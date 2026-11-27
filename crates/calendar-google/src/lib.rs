//! Google OAuth (authorization code + PKCE, loopback redirect) and the Calendar REST client.
//!
//! Blocking by design: callers run these methods on a background thread. No GUI, storage or async runtime.
//! Types that hold secrets deliberately do not implement `Debug`, so they cannot be logged by accident.

mod api;
mod auth;
mod mapping;
mod sign_in;
#[cfg(test)]
mod tests;

use chrono_tz::Tz;
use serde_json::Value;
use std::{fmt, time::Duration};

pub use api::{EventQuery, EventSync};
pub use auth::{AccessToken, Account, Credentials, Permissions, SignedIn};
pub use mapping::{Calendar, EventChange, GoogleEvent, created_event_id};
pub use sign_in::{CancelHandle, SignIn, open_in_browser};

/// OAuth client credentials. A Desktop-app client secret is not confidential; PKCE protects the code.
#[derive(Clone)]
pub struct Config {
    client_id: String,
    client_secret: String,
}

impl Config {
    /// Values compiled in from `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`; the process environment overrides them.
    /// `None` means sign-in is not configured in this build.
    pub fn from_environment() -> Option<Self> {
        Self::resolve(
            (
                std::env::var("GOOGLE_CLIENT_ID").ok(),
                option_env!("GOOGLE_CLIENT_ID"),
            ),
            (
                std::env::var("GOOGLE_CLIENT_SECRET").ok(),
                option_env!("GOOGLE_CLIENT_SECRET"),
            ),
        )
    }

    fn resolve(
        client_id: (Option<String>, Option<&str>),
        client_secret: (Option<String>, Option<&str>),
    ) -> Option<Self> {
        let pick = |(runtime, built_in): (Option<String>, Option<&str>)| {
            runtime
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .or_else(|| built_in.map(|value| value.trim().to_owned()))
                .filter(|value| !value.is_empty())
        };
        let client_id = pick(client_id).filter(|id| id.ends_with(".apps.googleusercontent.com"))?;
        let client_secret = pick(client_secret)?;

        Some(Self {
            client_id,
            client_secret,
        })
    }
}

/// Everything that can go wrong; messages are safe to show and never contain provider data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Transport failure: a failed fetch, never an empty result.
    Unreachable,
    InvalidResponse,
    Unauthorized,
    InsufficientPermissions,
    ApiDisabled,
    RateLimited,
    ServerUnavailable,
    NotFound,
    /// 410: the sync token expired; drop the calendar's token and run a full sync.
    Gone,
    /// 412: the event changed since its ETag was read.
    PreconditionFailed,
    /// 409: the event identifier is already taken by a different operation.
    Conflict,
    Rejected(u16),
    ReadOnly,
    InvalidEvent,
    ListenerUnavailable,
    SignInDenied,
    SignInTimedOut,
    SignInCancelled,
    TokenRejected,
    MissingRefreshToken,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unreachable => "Google is unreachable. Try again online.".into(),
            Self::InvalidResponse => "Google returned an unreadable response.".into(),
            Self::Unauthorized => {
                "Google rejected the access token. Reconnect your account.".into()
            }
            Self::InsufficientPermissions => {
                "Google did not grant the required calendar permission. Reconnect and approve access in Settings.".into()
            }
            Self::ApiDisabled => {
                "Google Calendar API is disabled for your OAuth project. Enable it in Google Cloud > APIs & Services > Library, wait a few minutes, then reconnect.".into()
            }
            Self::RateLimited => {
                "Google Calendar rate limit reached. Wait a few minutes and try again.".into()
            }
            Self::ServerUnavailable => {
                "Google Calendar is temporarily unavailable. Try again later.".into()
            }
            Self::NotFound => "Google could not find this calendar or event.".into(),
            Self::Gone => "Google invalidated the sync state. A full refresh is needed.".into(),
            Self::PreconditionFailed => {
                "Event changed in Google. Refresh before saving again.".into()
            }
            Self::Conflict => "Event identifier already exists.".into(),
            Self::Rejected(status) => {
                format!("Google Calendar rejected the request (HTTP {status}).")
            }
            Self::ReadOnly => "This calendar is read-only.".into(),
            Self::InvalidEvent => "Invalid event identifier.".into(),
            Self::ListenerUnavailable => "Unable to start the local sign-in callback.".into(),
            Self::SignInDenied => "Google sign-in was cancelled or denied.".into(),
            Self::SignInTimedOut => "Google sign-in timed out. Try connecting again.".into(),
            Self::SignInCancelled => "Sign-in cancelled. Existing accounts kept.".into(),
            Self::TokenRejected => {
                "Google rejected the authorization. Reconnect the account.".into()
            }
            Self::MissingRefreshToken => {
                "Google did not grant offline access. Reconnect and approve access.".into()
            }
        };

        formatter.write_str(&message)
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Maps a non-success Google response to an error without keeping any provider text.
    fn from_status(status: u16, body: &Value) -> Self {
        let reasons: Vec<&str> = ["errors", "details"]
            .into_iter()
            .filter_map(|field| body["error"][field].as_array())
            .flatten()
            .filter_map(|entry| entry["reason"].as_str())
            .collect();
        let has = |expected: &[&str]| reasons.iter().any(|reason| expected.contains(reason));

        match status {
            404 => Self::NotFound,
            409 => Self::Conflict,
            410 => Self::Gone,
            412 => Self::PreconditionFailed,
            _ if has(&["SERVICE_DISABLED", "accessNotConfigured", "serviceDisabled"]) => {
                Self::ApiDisabled
            }
            _ if has(&["ACCESS_TOKEN_SCOPE_INSUFFICIENT", "insufficientPermissions"]) => {
                Self::InsufficientPermissions
            }
            401 => Self::Unauthorized,
            _ if status == 429
                || has(&[
                    "rateLimitExceeded",
                    "userRateLimitExceeded",
                    "quotaExceeded",
                    "RATE_LIMIT_EXCEEDED",
                ]) =>
            {
                Self::RateLimited
            }
            500.. => Self::ServerUnavailable,
            _ => Self::Rejected(status),
        }
    }
}

struct Endpoints {
    token: String,
    userinfo: String,
    revoke: String,
    api: String,
}

impl Endpoints {
    fn google() -> Self {
        Self {
            token: "https://oauth2.googleapis.com/token".into(),
            userinfo: "https://openidconnect.googleapis.com/v1/userinfo".into(),
            revoke: "https://oauth2.googleapis.com/revoke".into(),
            api: "https://www.googleapis.com/calendar/v3".into(),
        }
    }
}

/// One blocking HTTP client plus the OAuth client credentials; keep it for the process lifetime.
pub struct Google {
    agent: ureq::Agent,
    config: Config,
    endpoints: Endpoints,
    /// All-day dates start at midnight in this zone and are written back as dates in it. UTC until
    /// the caller sets the cache's zone with [`Google::set_display_zone`].
    display_zone: Tz,
}

impl Google {
    pub fn new(config: Config) -> Self {
        // Small buffers and few idle connections keep the resident footprint low.
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(30)))
            .input_buffer_size(16 * 1024)
            .output_buffer_size(16 * 1024)
            .max_idle_connections(2)
            .max_idle_connections_per_host(1)
            .build()
            .into();

        Self {
            agent,
            config,
            endpoints: Endpoints::google(),
            display_zone: Tz::UTC,
        }
    }

    /// The zone for all-day dates in the requests that follow. Callers set it from the cache's
    /// recorded zone, so every row and every written date uses the same one.
    pub fn set_display_zone(&mut self, display_zone: Tz) {
        self.display_zone = display_zone;
    }
}

#[cfg(feature = "test-support")]
impl Google {
    /// For other crates' tests: every endpoint points at `origin` (a local fake server).
    pub fn for_fake_server(origin: &str, display_zone: Tz) -> Self {
        let config = Config {
            client_id: "test.apps.googleusercontent.com".into(),
            client_secret: "test-secret".into(),
        };
        let mut google = Self::new(config);
        google.set_display_zone(display_zone);
        google.endpoints = Endpoints {
            token: format!("{origin}/token"),
            userinfo: format!("{origin}/userinfo"),
            revoke: format!("{origin}/revoke"),
            api: format!("{origin}/calendar/v3"),
        };

        google
    }
}

type HttpResult = Result<ureq::http::Response<ureq::Body>, ureq::Error>;

/// Turns a transport failure or non-2xx status into an `Error`; 2xx responses are returned unread.
fn check(result: HttpResult) -> Result<ureq::http::Response<ureq::Body>, Error> {
    let mut response = result.map_err(|_| Error::Unreachable)?;
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status().as_u16();
    let body: Value = response.body_mut().read_json().unwrap_or_default();

    Err(Error::from_status(status, &body))
}

fn parse<T: serde::de::DeserializeOwned>(result: HttpResult) -> Result<T, Error> {
    check(result)?
        .body_mut()
        .read_json()
        .map_err(|_| Error::InvalidResponse)
}
