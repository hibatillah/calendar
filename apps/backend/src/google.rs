mod authorization;
mod credentials;
mod events;
mod mapping;

use async_graphql::SimpleObject;
use axum::{Router, extract::Query, response::Html, routing::get};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use credentials::{restore_credentials, store_credentials};
use mapping::{calendar_api_error, map_google_event, provider_operation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

const EVENT_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
const SCOPE: &str = "openid email profile https://www.googleapis.com/auth/calendar.calendarlist.readonly https://www.googleapis.com/auth/calendar.events";
const CREDENTIAL_SERVICE: &str = "calendar.google.local";
// Read only during migration from the previous application name.
const LEGACY_CREDENTIAL_SERVICE: &str = "randevu.google.local";

#[derive(Clone, Deserialize, Serialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendar {
    pub id: String,
    pub summary: String,
    #[serde(default)]
    pub primary: bool,
    pub access_role: String,
    #[serde(default)]
    pub background_color: String,
    #[serde(default)]
    pub time_zone: String,
}

#[derive(Clone, Serialize, SimpleObject)]
pub struct GoogleConnection {
    pub configured: bool,
    pub connected: bool,
    pub event_access: bool,
    pub account_key: String,
    pub write_access: bool,
    pub name: String,
    pub email: String,
    pub picture: String,
    pub connecting: bool,
    pub message: String,
    pub calendars: Vec<GoogleCalendar>,
}

#[derive(Clone)]
struct Config {
    client_id: String,
    client_secret: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    picture: String,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: Option<String>,
}

struct Pending {
    state: String,
    verifier: String,
    redirect_uri: String,
}

#[derive(Default)]
struct State {
    loaded: bool,
    last_discovery: Option<std::time::Instant>,
    credentials: Option<Credentials>,
    pending: Option<Pending>,
    calendars: Vec<GoogleCalendar>,
    message: Option<String>,
    event_cache: Option<(
        chrono::NaiveDate,
        chrono::NaiveDate,
        std::time::Instant,
        Vec<calendar_core::CalendarEvent>,
    )>,
}

#[derive(Clone)]
pub struct GoogleAuth {
    pub slot: String,
    api_origin: String,
    config: Option<Config>,
    state: Arc<Mutex<State>>,
    client: reqwest::Client,
}

fn config() -> Option<Config> {
    let values: HashMap<_, _> = dotenvy::from_path_iter(".env")
        .map(|values| values.filter_map(Result::ok).collect())
        .unwrap_or_default();
    let value = |name: &str| {
        std::env::var(name)
            .ok()
            .or_else(|| values.get(name).cloned())
            .filter(|value| !value.trim().is_empty())
    };
    let client_id = value("GOOGLE_CLIENT_ID")?;
    if !client_id.ends_with(".apps.googleusercontent.com") {
        return None;
    }
    Some(Config {
        client_id,
        client_secret: value("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
    })
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

impl Default for GoogleAuth {
    fn default() -> Self {
        Self {
            slot: "desktop-account".into(),
            api_origin: "https://www.googleapis.com".into(),
            config: config(),
            state: Arc::new(Mutex::new(State::default())),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client"),
        }
    }
}

impl GoogleAuth {
    async fn initialize(&self, state: &mut State) {
        if state.loaded {
            return;
        }
        state.loaded = true;
        if self.config.is_none() {
            return;
        }
        let slot = self.slot.clone();
        let restored = tokio::task::spawn_blocking(move || restore_credentials(&slot)).await;
        match restored {
            Ok(Ok(Some(credentials)))
                if Some(&credentials.client_id)
                    == self.config.as_ref().map(|config| &config.client_id) =>
            {
                state.credentials = Some(credentials)
            }
            Ok(Ok(_)) => (),
            _ => {
                state.message =
                    Some("Cannot restore Google credentials. Check OS credential storage.".into())
            }
        }
    }

    async fn discover(&self, credentials: &mut Credentials) -> Result<Vec<GoogleCalendar>, String> {
        let config = self
            .config
            .as_ref()
            .ok_or("Google OAuth is not configured.")?;
        if credentials.expires_at <= chrono::Utc::now().timestamp() + 60 {
            let response = self
                .client
                .post("https://oauth2.googleapis.com/token")
                .form(&[
                    ("client_id", config.client_id.as_str()),
                    ("client_secret", config.client_secret.as_str()),
                    ("refresh_token", credentials.refresh_token.as_str()),
                    ("grant_type", "refresh_token"),
                ])
                .send()
                .await
                .map_err(|_| "Google token refresh unavailable. Try again online.")?;
            if !response.status().is_success() {
                return Err(
                    "Google authorization expired or was revoked. Disconnect and reconnect.".into(),
                );
            }
            let tokens: Tokens = response
                .json()
                .await
                .map_err(|_| "Invalid Google token response.")?;
            if let Some(scope) = tokens.scope {
                credentials.scope = scope;
            }
            credentials.access_token = tokens.access_token;
            credentials.expires_at = chrono::Utc::now().timestamp() + tokens.expires_in;
            if let Some(refresh_token) = tokens.refresh_token {
                credentials.refresh_token = refresh_token;
            }
            if let Ok(response) = self
                .client
                .get("https://openidconnect.googleapis.com/v1/userinfo")
                .bearer_auth(&credentials.access_token)
                .send()
                .await
                && response.status().is_success()
                && let Ok(profile) = response.json::<serde_json::Value>().await
            {
                credentials.name = profile["name"].as_str().unwrap_or_default().into();
                credentials.email = profile["email"].as_str().unwrap_or_default().into();
                credentials.picture = profile["picture"]
                    .as_str()
                    .filter(|url| url.starts_with("https://"))
                    .unwrap_or_default()
                    .into();
            }
            store_credentials(self.slot.clone(), Some(credentials.clone())).await?;
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Page {
            #[serde(default)]
            items: Vec<GoogleCalendar>,
            next_page_token: Option<String>,
        }
        let mut calendars = Vec::new();
        let mut page_token = String::new();
        loop {
            let mut request = self
                .client
                .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
                .bearer_auth(&credentials.access_token)
                .query(&[("maxResults", "250")]);
            if !page_token.is_empty() {
                request = request.query(&[("pageToken", page_token.as_str())]);
            }
            let response = request
                .send()
                .await
                .map_err(|_| "Google Calendar is unreachable. Try again online.")?;
            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body: serde_json::Value = response.json().await.unwrap_or_default();
                let message = calendar_api_error(status, &body);
                tracing::warn!(
                    action = "google_calendar_discovery_failed",
                    status,
                    "{message}"
                );
                return Err(message);
            }
            let page: Page = response
                .json()
                .await
                .map_err(|_| "Invalid Google calendar list.")?;
            calendars.extend(page.items);
            match page.next_page_token {
                Some(next) if next != page_token => page_token = next,
                Some(_) => return Err("Google repeated a pagination cursor.".into()),
                None => break,
            }
        }
        Ok(calendars)
    }

    pub async fn status(&self) -> GoogleConnection {
        let mut state = self.state.lock().await;
        self.initialize(&mut state).await;
        if state.pending.is_none()
            && state
                .last_discovery
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(60))
            && let Some(mut credentials) = state.credentials.clone()
            && (state.calendars.is_empty()
                || credentials.expires_at <= chrono::Utc::now().timestamp() + 60)
        {
            state.last_discovery = Some(std::time::Instant::now());
            match self.discover(&mut credentials).await {
                Ok(calendars) => {
                    state.credentials = Some(credentials);
                    state.calendars = calendars;
                    state.message = None;
                }
                Err(error) => state.message = Some(error),
            }
        }
        let event_access = state.credentials.as_ref().is_some_and(|credentials| {
            credentials.scope.split_whitespace().any(|scope| {
                scope == EVENT_SCOPE
                    || scope == "https://www.googleapis.com/auth/calendar.events.readonly"
                    || scope == "https://www.googleapis.com/auth/calendar"
                    || scope == "https://www.googleapis.com/auth/calendar.readonly"
            })
        });
        let account_key = state
            .calendars
            .iter()
            .find(|calendar| calendar.primary)
            .map(|calendar| {
                challenge(&format!(
                    "{}:{}",
                    self.config
                        .as_ref()
                        .map(|config| config.client_id.as_str())
                        .unwrap_or_default(),
                    calendar.id
                ))
            })
            .unwrap_or_default();
        GoogleConnection {
            write_access: state.credentials.as_ref().is_some_and(|credentials| {
                credentials.scope.split_whitespace().any(|scope| {
                    scope == EVENT_SCOPE || scope == "https://www.googleapis.com/auth/calendar"
                })
            }),
            name: state
                .credentials
                .as_ref()
                .map(|credentials| credentials.name.clone())
                .unwrap_or_default(),
            email: state
                .credentials
                .as_ref()
                .map(|credentials| credentials.email.clone())
                .filter(|email| !email.is_empty())
                .or_else(|| {
                    state
                        .calendars
                        .iter()
                        .find(|calendar| calendar.primary)
                        .map(|calendar| calendar.id.clone())
                })
                .unwrap_or_default(),
            picture: state
                .credentials
                .as_ref()
                .map(|credentials| credentials.picture.clone())
                .unwrap_or_default(),
            event_access,
            account_key,
            configured: self.config.is_some(),
            connected: state.credentials.is_some(),
            connecting: state.pending.is_some(),
            message: state.message.clone().unwrap_or_else(|| {
                if self.config.is_none() {
                    "Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET in .env, then restart."
                } else if state.pending.is_some() {
                    "Finish Google sign-in in your browser."
                } else if state.credentials.is_some() {
                    if event_access {
                        "Connected to Google"
                    } else {
                        "Approve event access to show your Google events."
                    }
                } else {
                    "Connect your Google account."
                }
                .into()
            }),
            calendars: state.calendars.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
