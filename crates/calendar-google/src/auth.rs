//! OAuth 2.0 authorization code flow with PKCE (S256) and a loopback redirect, plus refresh and revoke.

use crate::{Error, Google, check, parse, sign_in::SignIn};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const SCOPE_BASE: &str = "https://www.googleapis.com/auth/";
/// Refresh this long before the access token expires.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);

fn scope(name: &str) -> String {
    format!("{SCOPE_BASE}{name}")
}

pub(crate) fn sign_in_scope() -> String {
    format!(
        "openid email profile {} {}",
        scope("calendar.calendarlist.readonly"),
        scope("calendar.events")
    )
}

/// Calendar discovery, event reading and event writing are granted and tracked separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Permissions {
    pub calendar_list: bool,
    pub read_events: bool,
    pub write_events: bool,
}

impl Permissions {
    pub fn from_scope(granted: &str) -> Self {
        let has = |names: &[&str]| {
            granted
                .split_whitespace()
                .any(|entry| names.iter().any(|name| entry == scope(name)))
        };

        Self {
            calendar_list: has(&[
                "calendar.calendarlist.readonly",
                "calendar.calendarlist",
                "calendar",
                "calendar.readonly",
            ]),
            read_events: has(&[
                "calendar.events",
                "calendar.events.readonly",
                "calendar",
                "calendar.readonly",
            ]),
            write_events: has(&["calendar.events", "calendar"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Google's stable subject identifier; the same person signing in twice yields the same value.
    pub account_id: String,
    pub email: String,
    pub name: String,
    pub picture: String,
}

/// What the vault persists per account. No `Debug`: it holds the refresh token.
#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub account: Account,
    pub refresh_token: String,
    /// Space-separated scopes Google actually granted.
    pub scope: String,
}

impl Credentials {
    pub fn permissions(&self) -> Permissions {
        Permissions::from_scope(&self.scope)
    }
}

/// Kept in memory only.
#[derive(Clone)]
pub struct AccessToken {
    value: String,
    scope: String,
    expires_at: Instant,
}

impl AccessToken {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn permissions(&self) -> Permissions {
        Permissions::from_scope(&self.scope)
    }

    pub fn is_fresh(&self) -> bool {
        Instant::now() + EXPIRY_MARGIN < self.expires_at
    }
}

pub struct SignedIn {
    pub credentials: Credentials,
    pub access_token: AccessToken,
    /// The account was already known: update its credentials instead of adding a second account.
    pub already_connected: bool,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: Option<String>,
}

impl TokenResponse {
    fn access_token(&self, fallback_scope: &str) -> AccessToken {
        AccessToken {
            value: self.access_token.clone(),
            scope: self.scope.clone().unwrap_or_else(|| fallback_scope.into()),
            expires_at: Instant::now() + Duration::from_secs(self.expires_in),
        }
    }
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    picture: String,
}

impl Google {
    pub(crate) fn exchange_code(
        &self,
        sign_in: &SignIn,
        code: &str,
        existing_account_ids: &[String],
    ) -> Result<SignedIn, Error> {
        let tokens = self.token_request(&[
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("code", code),
            ("code_verifier", &sign_in.verifier),
            ("redirect_uri", &sign_in.redirect_uri),
            ("grant_type", "authorization_code"),
        ])?;
        let refresh_token = tokens
            .refresh_token
            .clone()
            .ok_or(Error::MissingRefreshToken)?;
        let access_token = tokens.access_token(&sign_in_scope());
        let user: UserInfo = parse(
            self.agent
                .get(&self.endpoints.userinfo)
                .header("Authorization", format!("Bearer {}", access_token.value))
                .call(),
        )?;
        let already_connected = existing_account_ids.contains(&user.sub);

        Ok(SignedIn {
            credentials: Credentials {
                account: Account {
                    account_id: user.sub,
                    email: user.email,
                    name: user.name,
                    picture: Some(user.picture)
                        .filter(|url| url.starts_with("https://"))
                        .unwrap_or_default(),
                },
                refresh_token,
                scope: access_token.scope.clone(),
            },
            access_token,
            already_connected,
        })
    }

    /// `Error::TokenRejected` means the grant expired or was revoked and the account must reconnect.
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<AccessToken, Error> {
        let tokens = self.token_request(&[
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])?;

        Ok(tokens.access_token(""))
    }

    /// Best effort remote revoke; callers delete the local credential first.
    pub fn revoke(&self, refresh_token: &str) -> Result<(), Error> {
        check(
            self.agent
                .post(&self.endpoints.revoke)
                .send_form([("token", refresh_token)]),
        )?;

        Ok(())
    }

    fn token_request(&self, form: &[(&str, &str)]) -> Result<TokenResponse, Error> {
        match parse(
            self.agent
                .post(&self.endpoints.token)
                .send_form(form.iter().copied()),
        ) {
            Err(Error::Unauthorized | Error::Rejected(_)) => Err(Error::TokenRejected),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_keep_discovery_reading_and_writing_apart() {
        let only_list = Permissions::from_scope(&scope("calendar.calendarlist.readonly"));
        let read_only = Permissions::from_scope(&scope("calendar.events.readonly"));
        let full = Permissions::from_scope(&sign_in_scope());

        assert_eq!(
            only_list,
            Permissions {
                calendar_list: true,
                read_events: false,
                write_events: false
            }
        );
        assert_eq!(
            read_only,
            Permissions {
                calendar_list: false,
                read_events: true,
                write_events: false
            }
        );
        assert_eq!(
            full,
            Permissions {
                calendar_list: true,
                read_events: true,
                write_events: true
            }
        );
    }
}
