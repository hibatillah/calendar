use async_graphql::SimpleObject;
use axum::{Router, extract::Query, response::Html, routing::get};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
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
fn calendar_api_error(status: u16, body: &serde_json::Value) -> String {
    let reasons: Vec<&str> = ["errors", "details"]
        .into_iter()
        .filter_map(|field| body["error"][field].as_array())
        .flatten()
        .filter_map(|entry| entry["reason"].as_str())
        .collect();
    let has = |expected: &[&str]| reasons.iter().any(|reason| expected.contains(reason));
    if has(&["SERVICE_DISABLED", "accessNotConfigured", "serviceDisabled"]) {
        "Google Calendar API is disabled for your OAuth project. Enable it in Google Cloud > APIs & Services > Library, wait a few minutes, then reconnect.".into()
    } else if has(&["ACCESS_TOKEN_SCOPE_INSUFFICIENT", "insufficientPermissions"]) {
        "Google did not grant the required calendar permission. Reconnect and approve access in Settings.".into()
    } else if status == 401 {
        "Google rejected the access token. Reconnect your account.".into()
    } else if status == 429
        || has(&[
            "rateLimitExceeded",
            "userRateLimitExceeded",
            "quotaExceeded",
            "RATE_LIMIT_EXCEEDED",
        ])
    {
        "Google Calendar rate limit reached. Wait a few minutes before reconnecting.".into()
    } else if status >= 500 {
        "Google Calendar is temporarily unavailable. Try connecting again later.".into()
    } else {
        format!(
            "Google Calendar rejected calendar discovery (HTTP {status}). Check Calendar API enablement, calendar-list consent, and any Workspace administrator restrictions."
        )
    }
}

fn credential_entry(slot: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(CREDENTIAL_SERVICE, slot)
        .map_err(|_| "OS credential storage unavailable.".into())
}

fn restore_credentials(slot: &str) -> Result<Option<Credentials>, String> {
    let current = credential_entry(slot)?;
    let value = match current.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => {
            let legacy = keyring::Entry::new(LEGACY_CREDENTIAL_SERVICE, slot)
                .map_err(|_| "OS credential storage unavailable.")?;
            match legacy.get_password() {
                Ok(value) => {
                    let _: Credentials = serde_json::from_str(&value)
                        .map_err(|_| "Stored Google credentials are invalid.")?;
                    current
                        .set_password(&value)
                        .map_err(|_| "Cannot migrate Google credentials to Calendar.")?;
                    legacy
                        .delete_credential()
                        .map_err(|_| "Cannot remove the migrated credential entry.")?;
                    tracing::info!(action = "google_credentials_migrated");
                    value
                }
                Err(keyring::Error::NoEntry) => return Ok(None),
                Err(_) => return Err("Cannot read Google credentials from the OS vault.".into()),
            }
        }
        Err(_) => return Err("Cannot read Google credentials from the OS vault.".into()),
    };
    serde_json::from_str(&value)
        .map(Some)
        .map_err(|_| "Stored Google credentials are invalid. Disconnect and reconnect.".into())
}

fn provider_operation(value: &serde_json::Value) -> Option<&str> {
    let private = &value["extendedProperties"]["private"];
    private["calendarOperation"]
        .as_str()
        .or_else(|| private["randevuOperation"].as_str())
}
async fn store_credentials(slot: String, credentials: Option<Credentials>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let entry = credential_entry(&slot)?;
        match credentials {
            Some(mut credentials) => {
                credentials.access_token.clear();
                credentials.expires_at = 0;
                entry
                    .set_password(
                        &serde_json::to_string(&credentials)
                            .map_err(|_| "Unable to encode credentials.")?,
                    )
                    .map_err(|_| "Unable to save credentials in the OS vault.".into())
            }
            None => {
                // Remove both namespaces so an interrupted migration cannot restore a signed-out account.
                for service in [CREDENTIAL_SERVICE, LEGACY_CREDENTIAL_SERVICE] {
                    let entry = keyring::Entry::new(service, &slot)
                        .map_err(|_| "OS credential storage unavailable.")?;
                    match entry.delete_credential() {
                        Ok(()) | Err(keyring::Error::NoEntry) => (),
                        Err(_) => {
                            return Err("Unable to remove credentials from the OS vault.".into());
                        }
                    }
                }
                Ok(())
            }
        }
    })
    .await
    .map_err(|_| "Credential storage task failed.")?
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
    pub async fn begin(&self) -> Result<String, String> {
        let config = self.config.as_ref().ok_or(
            "Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET in .env, then restart the backend.",
        )?;
        let mut state = self.state.lock().await;
        self.initialize(&mut state).await;
        if state.pending.is_some() {
            return Err("Sign-in is already open. Finish it or cancel before retrying.".into());
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| "Unable to start local sign-in callback.")?;
        let redirect_uri = format!(
            "http://127.0.0.1:{}/",
            listener
                .local_addr()
                .map_err(|_| "Cannot read callback port.")?
                .port()
        );
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let nonce = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
        url.query_pairs_mut().extend_pairs([
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", &redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("state", &nonce),
            ("code_challenge", &challenge(&verifier)),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ]);
        state.pending = Some(Pending {
            state: nonce.clone(),
            verifier,
            redirect_uri,
        });
        state.message = None;
        let auth = self.clone();
        tokio::spawn(async move {
            let callback_auth = auth.clone();
            let callback_nonce = nonce.clone();
            let (done, completed) = tokio::sync::watch::channel(false);
            let router = Router::new().route("/", get(move |Query(parameters): Query<HashMap<String, String>>| {
                let auth = callback_auth.clone(); let done = done.clone(); let expected_state = callback_nonce.clone();
                async move {
                    let result = if parameters.get("state") == Some(&expected_state) { auth.complete(parameters).await } else { Err(()) };
                    if result.is_ok() { let _ = done.send(true); }
                    ([("cache-control", "no-store"), ("referrer-policy", "no-referrer"), ("content-security-policy", "default-src 'none'")], Html(if result == Ok(true) { "<h1>Google connected</h1><p>Return to Calendar. You can close this tab.</p>" } else { "<h1>Sign-in did not complete</h1><p>Return to Calendar for details. You can close this tab.</p>" }))
                }
            }));
            let shutdown = async move {
                let mut completed = completed;
                let _ = completed.changed().await;
            };
            let _ = tokio::time::timeout(
                Duration::from_secs(300),
                axum::serve(listener, router)
                    .with_graceful_shutdown(shutdown)
                    .into_future(),
            )
            .await;
            let mut state = auth.state.lock().await;
            if state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.state == nonce)
            {
                state.pending = None;
                state.message = Some("Google sign-in timed out. Try connecting again.".into());
            }
        });
        tracing::info!(action = "google_sign_in_started");
        Ok(url.into())
    }
    async fn complete(&self, parameters: HashMap<String, String>) -> Result<bool, ()> {
        let mut state = self.state.lock().await;
        let pending = state.pending.as_ref().ok_or(())?;
        if parameters.get("state") != Some(&pending.state) {
            return Err(());
        }
        let pending = state.pending.take().unwrap();
        if parameters.contains_key("error") {
            state.message = Some("Google sign-in was cancelled or denied.".into());
            return Ok(false);
        }
        let result = self
            .exchange(
                parameters
                    .get("code")
                    .map(String::as_str)
                    .unwrap_or_default(),
                pending,
            )
            .await;
        match result {
            Ok((credentials, calendars)) => {
                state.credentials = Some(credentials);
                state.calendars = calendars;
                state.event_cache = None;
                state.message = None;
                tracing::info!(action = "google_connected");
                Ok(true)
            }
            Err(error) => {
                state.message = Some(error);
                tracing::warn!(action = "google_sign_in_failed");
                Ok(false)
            }
        }
    }
    async fn exchange(
        &self,
        code: &str,
        pending: Pending,
    ) -> Result<(Credentials, Vec<GoogleCalendar>), String> {
        if code.is_empty() {
            return Err("Google did not return an authorization code.".into());
        }
        let config = self
            .config
            .as_ref()
            .ok_or("Missing Google configuration.")?;
        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("code", code),
                ("code_verifier", &pending.verifier),
                ("redirect_uri", &pending.redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|_| "Google token exchange unavailable. Try connecting again.")?;
        if !response.status().is_success() {
            return Err(
                "Google rejected sign-in. Check Desktop OAuth credentials and reconnect.".into(),
            );
        }
        let tokens: Tokens = response
            .json()
            .await
            .map_err(|_| "Invalid Google token response.")?;
        let mut credentials = Credentials {
            client_id: config.client_id.clone(),
            access_token: tokens.access_token,
            refresh_token: tokens
                .refresh_token
                .ok_or("Google did not grant offline access. Reconnect and approve access.")?,
            expires_at: chrono::Utc::now().timestamp() + tokens.expires_in,
            scope: tokens.scope.unwrap_or_else(|| SCOPE.into()),
            name: String::new(),
            email: String::new(),
            picture: String::new(),
        };
        let calendars = self.discover(&mut credentials).await?;
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
        Ok((credentials, calendars))
    }
    pub async fn events(
        &self,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Result<Vec<calendar_core::CalendarEvent>, String> {
        if end <= start || (end - start).num_days() > 100 {
            return Err("Request a date range of 1 to 100 days.".into());
        }
        let connection = self.status().await;
        if !connection.connected {
            return Ok(Vec::new());
        }
        if !connection.event_access {
            return Err("Approve event access in the sidebar to load Google events.".into());
        }
        let mut state = self.state.lock().await;
        if let Some((cached_start, cached_end, fetched, events)) = &state.event_cache
            && *cached_start == start
            && *cached_end == end
            && fetched.elapsed() < Duration::from_secs(30)
        {
            return Ok(events.clone());
        }
        let credentials = state
            .credentials
            .as_ref()
            .ok_or("Google account disconnected.")?;
        let mut events = Vec::new();
        for calendar in &state.calendars {
            if calendar.access_role == "freeBusyReader" {
                continue;
            }
            let mut url =
                reqwest::Url::parse(&format!("{}/calendar/v3/calendars/", self.api_origin))
                    .unwrap();
            url.path_segments_mut()
                .unwrap()
                .pop_if_empty()
                .push(&calendar.id)
                .push("events");
            let mut page_token = String::new();
            loop {
                let mut request = self
                    .client
                    .get(url.clone())
                    .bearer_auth(&credentials.access_token)
                    .query(&[
                        ("singleEvents", "true".to_string()),
                        ("showDeleted", "false".to_string()),
                        ("maxResults", "2500".to_string()),
                        ("timeMin", format!("{start}T00:00:00+07:00")),
                        ("timeMax", format!("{end}T00:00:00+07:00")),
                    ]);
                if !page_token.is_empty() {
                    request = request.query(&[("pageToken", &page_token)]);
                }
                let response = request
                    .send()
                    .await
                    .map_err(|_| "Google Calendar is unreachable. Showing cached events.")?;
                if !response.status().is_success() {
                    let status = response.status().as_u16();
                    return Err(calendar_api_error(
                        status,
                        &response.json().await.unwrap_or_default(),
                    ));
                }
                let page: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|_| "Invalid Google event response.")?;
                for event in page["items"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                    if let Some(event) = map_google_event(event, calendar)? {
                        let mut event = event;
                        event.read_only |= !connection.write_access;
                        events.push(event);
                    }
                }
                match page["nextPageToken"].as_str() {
                    Some(next) if next != page_token => page_token = next.into(),
                    Some(_) => return Err("Google repeated an event pagination cursor.".into()),
                    None => break,
                }
            }
        }
        state.event_cache = Some((start, end, std::time::Instant::now(), events.clone()));
        tracing::info!(action = "google_events_synced", event_count = events.len());
        Ok(events)
    }

    pub async fn save(
        &self,
        command: calendar_core::EventCommand,
    ) -> Result<calendar_core::CalendarEvent, String> {
        command.event.validate()?;
        let connection = self.status().await;
        if !connection.write_access {
            return Err("Reconnect this account in Settings and approve calendar editing.".into());
        }
        let mut state = self.state.lock().await;
        let calendar = state
            .calendars
            .iter()
            .find(|calendar| calendar.id == command.event.calendar_id)
            .cloned()
            .ok_or("Calendar not found. Refresh and retry.")?;
        if !matches!(calendar.access_role.as_str(), "owner" | "writer") {
            return Err("This calendar is read-only.".into());
        }
        let credentials = state.credentials.as_ref().ok_or("Account disconnected.")?;
        let event = &command.event;
        let mut collection =
            reqwest::Url::parse(&format!("{}/calendar/v3/calendars/", self.api_origin)).unwrap();
        collection
            .path_segments_mut()
            .unwrap()
            .pop_if_empty()
            .push(&calendar.id)
            .push("events");
        let creating = command.expected_revision < 0;
        let provider_id = if creating {
            uuid::Uuid::parse_str(&event.event_id)
                .map_err(|_| "Invalid new event identifier.")?
                .simple()
                .to_string()
        } else {
            event
                .event_id
                .rsplit(':')
                .next()
                .ok_or("Invalid event identifier.")?
                .to_string()
        };
        let mut item = collection.clone();
        item.path_segments_mut().unwrap().push(&provider_id);
        let date_value = |instant: chrono::DateTime<chrono::Utc>| {
            if event.all_day {
                serde_json::json!({"date":instant.with_timezone(&chrono_tz::Asia::Jakarta).date_naive().to_string()})
            } else {
                serde_json::json!({"dateTime":instant.to_rfc3339(),"timeZone":event.timezone})
            }
        };
        let mut payload = serde_json::json!({"summary":event.title,"description":event.description,"location":event.location,"start":date_value(event.start),"end":date_value(event.end)});
        let request = if creating {
            payload["id"] = provider_id.clone().into();
            payload["extendedProperties"] =
                serde_json::json!({"private":{"calendarOperation":command.operation_id}});
            self.client.post(collection)
        } else {
            let current = self
                .client
                .get(item.clone())
                .bearer_auth(&credentials.access_token)
                .send()
                .await
                .map_err(|_| "Cannot check the current Google event.")?;
            if !current.status().is_success() {
                return Err("Event no longer available. Refresh before editing.".into());
            }
            let current: serde_json::Value =
                current.json().await.map_err(|_| "Invalid Google event.")?;
            if provider_operation(&current) == Some(command.operation_id.as_str()) {
                return map_google_event(&current, &calendar)?.ok_or("Event was removed.".into());
            }
            let mapped = map_google_event(&current, &calendar)?.ok_or("Event was removed.")?;
            if mapped.read_only {
                return Err("This Google event type is read-only.".into());
            }
            if mapped.revision != command.expected_revision {
                return Err("Event changed in Google. Refresh before saving again.".into());
            }
            let mut private = current["extendedProperties"]["private"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            private.insert(
                "calendarOperation".into(),
                command.operation_id.clone().into(),
            );
            payload["extendedProperties"] = serde_json::json!({"private":private});
            self.client.patch(item.clone()).header(
                "If-Match",
                current["etag"].as_str().ok_or("Missing event version.")?,
            )
        };
        let response = request
            .bearer_auth(&credentials.access_token)
            .query(&[("sendUpdates", "all")])
            .json(&payload)
            .send()
            .await
            .map_err(|_| "Google save could not be confirmed. Refresh before retrying.")?;
        let status = response.status().as_u16();
        let result: serde_json::Value = if creating && status == 409 {
            let response = self
                .client
                .get(item)
                .bearer_auth(&credentials.access_token)
                .send()
                .await
                .map_err(|_| "Cannot reconcile event creation.")?;
            if !response.status().is_success() {
                return Err("Cannot reconcile event creation.".into());
            }
            let value: serde_json::Value = response
                .json()
                .await
                .map_err(|_| "Invalid Google event response.")?;
            if provider_operation(&value) != Some(command.operation_id.as_str()) {
                return Err("Event identifier already exists.".into());
            }
            value
        } else {
            if status == 412 {
                return Err("Event changed in Google. Refresh before saving again.".into());
            }
            if !(200..300).contains(&status) {
                return Err(calendar_api_error(
                    status,
                    &response.json().await.unwrap_or_default(),
                ));
            }
            response.json().await.map_err(|_| "Google saved the event but returned an unreadable response. Refresh before retrying.")?
        };
        state.event_cache = None;
        tracing::info!(action = "google_event_saved", creating);
        map_google_event(&result, &calendar)?.ok_or("Event not available after saving.".into())
    }

    pub async fn forget_local(&self) -> Result<(), String> {
        store_credentials(self.slot.clone(), None).await?;
        let mut state = self.state.lock().await;
        state.credentials = None;
        state.calendars.clear();
        state.event_cache = None;
        Ok(())
    }

    pub async fn cancel(&self) {
        let mut state = self.state.lock().await;
        state.pending = None;
        state.message = Some("Sign-in cancelled. Existing account kept.".into());
    }

    pub async fn disconnect(&self) -> Result<bool, String> {
        let mut state = self.state.lock().await;
        self.initialize(&mut state).await;
        // Delete the local secret first, even when Google is unreachable.
        store_credentials(self.slot.clone(), None).await?;
        let credentials = state.credentials.take();
        state.pending = None;
        state.calendars.clear();
        state.event_cache = None;
        state.message = Some("Google disconnected from this machine.".into());
        if let Some(credentials) = credentials {
            let revoked = self
                .client
                .post("https://oauth2.googleapis.com/revoke")
                .form(&[("token", credentials.refresh_token)])
                .send()
                .await;
            if !revoked.is_ok_and(|response| response.status().is_success()) {
                state.message = Some("Disconnected locally. Remove Calendar access in your Google account to revoke the remote grant.".into());
            }
        }
        tracing::info!(action = "google_disconnected");
        Ok(true)
    }
}

fn map_google_event(
    value: &serde_json::Value,
    calendar: &GoogleCalendar,
) -> Result<Option<calendar_core::CalendarEvent>, String> {
    if value["status"] == "cancelled" {
        return Ok(None);
    }
    let all_day = value["start"]["date"].is_string();
    let parse_time = |field: &str| -> Result<chrono::DateTime<chrono::Utc>, String> {
        if all_day {
            let date = value[field]["date"]
                .as_str()
                .ok_or("Missing all-day date.")?
                .parse()
                .map_err(|_| "Invalid all-day date.")?;
            calendar_core::resolve_local(date, 0, chrono_tz::Asia::Jakarta)
        } else {
            value[field]["dateTime"]
                .as_str()
                .ok_or("Missing event time.")?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|_| "Invalid Google event time.".into())
        }
    };
    let provider_id = value["id"].as_str().ok_or("Missing Google event ID.")?;
    let start = parse_time("start")?;
    let end = parse_time("end")?;
    if end <= start {
        return Ok(None);
    }
    let timezone = value["start"]["timeZone"]
        .as_str()
        .unwrap_or(&calendar.time_zone);
    let timezone = if timezone.parse::<chrono_tz::Tz>().is_ok() {
        timezone
    } else {
        "Asia/Jakarta"
    };
    Ok(Some(calendar_core::CalendarEvent {
        event_id: format!(
            "google:{}:{}",
            URL_SAFE_NO_PAD.encode(calendar.id.as_bytes()),
            provider_id
        ),
        calendar_id: calendar.id.clone(),
        title: value["summary"].as_str().unwrap_or("Busy").into(),
        start,
        end,
        timezone: timezone.into(),
        all_day,
        description: value["description"].as_str().unwrap_or_default().into(),
        location: value["location"].as_str().unwrap_or_default().into(),
        color: u32::from_str_radix(calendar.background_color.trim_start_matches('#'), 16)
            .unwrap_or(0x6ba5d7),
        revision: value["updated"]
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.timestamp_millis())
            .unwrap_or(0),
        read_only: !matches!(calendar.access_role.as_str(), "owner" | "writer")
            || value["eventType"]
                .as_str()
                .is_some_and(|kind| kind != "default"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn browser_flow_uses_loopback_and_consumes_denial_once() {
        let auth = GoogleAuth {
            slot: "test".into(),
            api_origin: "https://www.googleapis.com".into(),
            config: Some(Config {
                client_id: "test.apps.googleusercontent.com".into(),
                client_secret: "test".into(),
            }),
            state: Arc::new(Mutex::new(State {
                loaded: true,
                ..State::default()
            })),
            client: reqwest::Client::new(),
        };
        let authorization = reqwest::Url::parse(&auth.begin().await.unwrap()).unwrap();
        assert_eq!(authorization.host_str(), Some("accounts.google.com"));
        let parameters: HashMap<_, _> = authorization.query_pairs().into_owned().collect();
        assert_eq!(parameters["code_challenge_method"], "S256");
        assert_eq!(parameters["scope"], SCOPE);
        assert!(!parameters.contains_key("client_secret"));
        let callback = reqwest::Url::parse(&parameters["redirect_uri"]).unwrap();
        assert_eq!(callback.host_str(), Some("127.0.0.1"));
        assert!(callback.port().is_some());
        assert!(auth.begin().await.is_err());
        let response = auth
            .client
            .get(callback)
            .query(&[
                ("state", parameters["state"].as_str()),
                ("error", "access_denied"),
            ])
            .send()
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(response.text().await.unwrap().contains("did not complete"));
        let status = auth.status().await;
        assert!(!status.connected);
        assert!(!status.connecting);
        assert!(status.message.contains("cancelled"));
    }

    #[test]
    fn calendar_errors_distinguish_disabled_api_from_missing_consent() {
        let disabled = serde_json::json!({"error":{"details":[{"reason":"SERVICE_DISABLED"}], "message":"private provider data"}});
        let permission =
            serde_json::json!({"error":{"errors":[{"reason":"insufficientPermissions"}]}});
        assert!(calendar_api_error(403, &disabled).contains("disabled"));
        assert!(!calendar_api_error(403, &disabled).contains("private provider data"));
        assert!(calendar_api_error(403, &permission).contains("approve access"));
        assert!(calendar_api_error(429, &serde_json::Value::Null).contains("rate limit"));
        assert!(calendar_api_error(400, &serde_json::Value::Null).contains("HTTP 400"));
    }

    fn test_calendar(identifier: &str) -> GoogleCalendar {
        GoogleCalendar {
            id: identifier.into(),
            summary: "Test".into(),
            primary: true,
            access_role: "owner".into(),
            background_color: "#123456".into(),
            time_zone: "Asia/Jakarta".into(),
        }
    }

    #[test]
    fn event_mapping_preserves_offsets_and_calendar_identity() {
        let value = serde_json::json!({"id":"recurring_20261015T020000Z", "summary":"Meeting", "start":{"dateTime":"2026-10-15T09:00:00+07:00"}, "end":{"dateTime":"2026-10-15T10:00:00+07:00"}});
        let event = map_google_event(&value, &test_calendar("one"))
            .unwrap()
            .unwrap();
        let other = map_google_event(&value, &test_calendar("two"))
            .unwrap()
            .unwrap();
        assert_eq!(event.start.to_rfc3339(), "2026-10-15T02:00:00+00:00");
        assert_eq!(event.end - event.start, chrono::Duration::hours(1));
        assert_ne!(event.event_id, other.event_id);
        assert!(!event.read_only);
        assert_eq!(event.color, 0x123456);
    }

    #[test]
    fn all_day_mapping_preserves_exclusive_dates_and_skips_cancellations() {
        let value = serde_json::json!({"id":"holiday", "start":{"date":"2026-10-15"}, "end":{"date":"2026-10-17"}});
        let event = map_google_event(&value, &test_calendar("one"))
            .unwrap()
            .unwrap();
        assert!(event.all_day);
        assert_eq!(
            event
                .start
                .with_timezone(&chrono_tz::Asia::Jakarta)
                .date_naive()
                .to_string(),
            "2026-10-15"
        );
        assert_eq!(
            event
                .end
                .with_timezone(&chrono_tz::Asia::Jakarta)
                .date_naive()
                .to_string(),
            "2026-10-17"
        );
        assert_eq!(event.end - event.start, chrono::Duration::days(2));
        assert!(
            map_google_event(
                &serde_json::json!({"status":"cancelled"}),
                &test_calendar("one")
            )
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn google_writes_are_retry_safe_and_reject_stale_edits() {
        use axum::{
            Json,
            extract::State as AxumState,
            http::{HeaderMap, StatusCode},
            routing::post,
        };
        let stored = Arc::new(Mutex::new(serde_json::Value::Null));
        let routes = Router::new()
            .route(
                "/calendar/v3/calendars/one/events",
                post(
                    |AxumState(stored): AxumState<Arc<Mutex<serde_json::Value>>>,
                     Json(mut value): Json<serde_json::Value>| async move {
                        let mut stored = stored.lock().await;
                        if !stored.is_null() {
                            return (StatusCode::CONFLICT, Json(stored.clone()));
                        }
                        value["updated"] = "2026-10-15T02:00:00Z".into();
                        value["etag"] = "version-one".into();
                        *stored = value.clone();
                        (StatusCode::OK, Json(value))
                    },
                ),
            )
            .route(
                "/calendar/v3/calendars/one/events/{event}",
                get(
                    |AxumState(stored): AxumState<Arc<Mutex<serde_json::Value>>>| async move {
                        Json(stored.lock().await.clone())
                    },
                )
                .patch(
                    |AxumState(stored): AxumState<Arc<Mutex<serde_json::Value>>>,
                     headers: HeaderMap,
                     Json(value): Json<serde_json::Value>| async move {
                        let mut stored = stored.lock().await;
                        assert_eq!(headers.get("if-match").unwrap(), "version-one");
                        for (key, value) in value.as_object().unwrap() {
                            stored[key] = value.clone();
                        }
                        stored["updated"] = "2026-10-15T02:01:00Z".into();
                        stored["etag"] = "version-two".into();
                        Json(stored.clone())
                    },
                ),
            )
            .with_state(stored.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, routes).await.unwrap();
        });
        let auth = GoogleAuth {
            slot: "test".into(),
            api_origin: origin,
            config: None,
            client: reqwest::Client::new(),
            state: Arc::new(Mutex::new(State {
                loaded: true,
                credentials: Some(Credentials {
                    client_id: "test".into(),
                    access_token: "test".into(),
                    refresh_token: "test".into(),
                    expires_at: chrono::Utc::now().timestamp() + 3600,
                    scope: SCOPE.into(),
                    name: String::new(),
                    email: String::new(),
                    picture: String::new(),
                }),
                calendars: vec![test_calendar("one")],
                ..Default::default()
            })),
        };
        let mut event = calendar_core::sample_events(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            chrono_tz::Asia::Jakarta,
        )
        .remove(0);
        event.event_id = uuid::Uuid::new_v4().to_string();
        event.calendar_id = "one".into();
        let command = calendar_core::EventCommand {
            operation_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: -1,
            event,
        };
        let saved = auth.save(command.clone()).await.unwrap();
        let retry = auth.save(command).await.unwrap();
        assert_eq!(saved.event_id, retry.event_id);
        assert!(!saved.read_only);
        let mut edit = calendar_core::EventCommand {
            operation_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: saved.revision,
            event: saved.clone(),
        };
        edit.event.title = "Updated".into();
        let updated = auth.save(edit.clone()).await.unwrap();
        assert_eq!(updated.title, "Updated");
        assert!(updated.revision > saved.revision);
        assert_eq!(auth.save(edit).await.unwrap().revision, updated.revision);
        let stale = calendar_core::EventCommand {
            operation_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: saved.revision,
            event: saved,
        };
        assert!(
            auth.save(stale)
                .await
                .unwrap_err()
                .contains("changed in Google")
        );
        server.abort();
    }

    #[test]
    fn provider_operation_reads_legacy_markers_but_prefers_calendar() {
        assert_eq!(
            provider_operation(
                &serde_json::json!({"extendedProperties":{"private":{"randevuOperation":"old"}}})
            ),
            Some("old")
        );
        assert_eq!(
            provider_operation(
                &serde_json::json!({"extendedProperties":{"private":{"randevuOperation":"old", "calendarOperation":"new"}}})
            ),
            Some("new")
        );
        assert_eq!(provider_operation(&serde_json::Value::Null), None);
    }

    #[test]
    fn pkce_matches_rfc7636_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
    #[tokio::test]
    async fn invalid_state_does_not_consume_pending_sign_in() {
        let auth = GoogleAuth {
            slot: "test".into(),
            api_origin: "https://www.googleapis.com".into(),
            config: None,
            state: Arc::new(Mutex::new(State::default())),
            client: reqwest::Client::new(),
        };
        auth.state.lock().await.pending = Some(Pending {
            state: "correct".into(),
            verifier: "test".into(),
            redirect_uri: "http://127.0.0.1:1/".into(),
        });
        assert!(
            auth.complete(HashMap::from([("state".into(), "wrong".into())]))
                .await
                .is_err()
        );
        assert!(auth.state.lock().await.pending.is_some());
        assert_eq!(
            auth.complete(HashMap::from([
                ("state".into(), "correct".into()),
                ("error".into(), "access_denied".into())
            ]))
            .await,
            Ok(false)
        );
        assert!(auth.state.lock().await.pending.is_none());
        assert!(
            auth.complete(HashMap::from([("state".into(), "correct".into())]))
                .await
                .is_err()
        );
    }
}
