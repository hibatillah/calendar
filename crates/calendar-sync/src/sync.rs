//! Sync engine: calendar list, a bounded full sync per calendar, then `syncToken` increments.
//!
//! Google forbids `timeMin`/`timeMax` together with `syncToken`, so the window is applied to the
//! full sync only. Incremental syncs return whatever changed, and `Store::prune_ended_before`
//! trims what fell out of the past edge. Once less than `REBOUND_MARGIN` of the future edge is
//! left, the next sync is a fresh full sync with a new window. See docs/google.md.

use crate::{Error, Store, SyncState, align_cache_zone, system_zone};
use calendar_google::{AccessToken, Calendar, EventQuery, Google};
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use std::collections::HashMap;

mod write;

pub use write::{Change, WriteError, WriteRequest, Written};

/// How far back the full-sync window starts; the cache holds nothing older.
pub const PAST: Duration = Duration::days(30);
/// How far ahead the full-sync window ends.
const FUTURE: Duration = Duration::days(180);
/// How long finished writes stay in the journal.
const FINISHED_WRITES: Duration = Duration::days(7);
/// A full sync with a fresh window runs when the cached window ends within this long.
const REBOUND_MARGIN: Duration = Duration::days(60);

/// One account's result. An `Err` means the fetch failed and its cache was kept as it was; it is
/// never the same as an account with no events.
pub struct AccountOutcome {
    pub account_id: String,
    pub result: Result<(), Error>,
}

/// Holds the HTTP client and the in-memory access tokens. Own it on the sync thread.
pub struct Syncer {
    google: Google,
    access_tokens: HashMap<String, AccessToken>,
    /// Read at the start of every sync run; the system zone can change while the process runs.
    zone_source: fn() -> Option<Tz>,
}

impl Syncer {
    pub fn new(google: Google) -> Self {
        Self {
            google,
            access_tokens: HashMap::new(),
            zone_source: system_zone,
        }
    }

    /// For tests, here and in dependants (`test-support`): the system zone is whatever `zone_source` says.
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_zone_source(google: Google, zone_source: fn() -> Option<Tz>) -> Self {
        Self {
            zone_source,
            ..Self::new(google)
        }
    }

    /// Syncs every cached account. `refresh_token` reads the account's token from the vault.
    /// One account or calendar failing never stops the others.
    pub fn sync_all(
        &mut self,
        store: &mut Store,
        now: DateTime<Utc>,
        refresh_token: impl Fn(&str) -> Option<String>,
    ) -> Result<Vec<AccountOutcome>, Error> {
        // A changed zone drops every sync token, so this run does full syncs and rebuilds all-day rows.
        let zone = align_cache_zone(store, (self.zone_source)())?;
        self.google.set_display_zone(zone);
        let accounts = store.accounts()?;
        self.access_tokens.retain(|account_id, _| {
            accounts
                .iter()
                .any(|account| &account.account_id == account_id)
        });

        let mut outcomes = Vec::new();
        for account in accounts {
            let result = self.sync_account(store, now, &account.account_id, &refresh_token);
            if matches!(
                result,
                Err(Error::Google(calendar_google::Error::Unauthorized))
            ) {
                self.access_tokens.remove(&account.account_id);
            }
            tracing::info!(
                action = "sync_account",
                account_id = %account.account_id,
                outcome = result.as_ref().map_or_else(|error| error.to_string(), |_| "ok".into())
            );
            outcomes.push(AccountOutcome {
                account_id: account.account_id,
                result,
            });
        }
        store.prune_ended_before(now - PAST)?;
        store.prune_operations_before(now - FINISHED_WRITES)?;

        Ok(outcomes)
    }

    fn sync_account(
        &mut self,
        store: &mut Store,
        now: DateTime<Utc>,
        account_id: &str,
        refresh_token: &impl Fn(&str) -> Option<String>,
    ) -> Result<(), Error> {
        self.settle_unfinished(store, account_id, refresh_token)?;
        let access_token = self.access_token(account_id, refresh_token)?;
        let calendars = self.google.list_calendars(&access_token)?;
        store.replace_calendars(account_id, &calendars)?;

        // Calendars are independent: keep going after a failure and report the first one.
        let mut first_error = None;
        for calendar in &calendars {
            if let Err(error) = self.sync_calendar(store, now, account_id, &access_token, calendar)
            {
                first_error.get_or_insert(error);
            }
        }

        first_error.map_or(Ok(()), Err)
    }

    fn sync_calendar(
        &self,
        store: &mut Store,
        now: DateTime<Utc>,
        account_id: &str,
        access_token: &str,
        calendar: &Calendar,
    ) -> Result<(), Error> {
        let state = store.sync_state(account_id, &calendar.id)?;
        if let Some(state) = state.filter(|state| state.window_end - now > REBOUND_MARGIN) {
            let query = EventQuery::Incremental {
                sync_token: &state.sync_token,
            };
            match self.google.sync_events(access_token, calendar, query) {
                Ok(sync) => {
                    return store.store_incremental_sync(
                        account_id,
                        &calendar.id,
                        &sync.changes,
                        &sync.next_sync_token,
                    );
                }
                Err(calendar_google::Error::Gone) => {
                    store.clear_sync_token(account_id, &calendar.id)?;
                    tracing::info!(action = "sync_token_expired");
                }
                Err(error) => return Err(error.into()),
            }
        }

        let window = (now - PAST, now + FUTURE);
        let query = EventQuery::Full {
            time_min: Some(window.0),
            time_max: Some(window.1),
        };
        let sync = self.google.sync_events(access_token, calendar, query)?;
        let state = SyncState {
            sync_token: sync.next_sync_token,
            window_start: window.0,
            window_end: window.1,
        };

        store.store_full_sync(account_id, &calendar.id, &sync.changes, &state)
    }

    /// The cached token while it is fresh, otherwise a new one from the stored refresh token.
    fn access_token(
        &mut self,
        account_id: &str,
        refresh_token: &impl Fn(&str) -> Option<String>,
    ) -> Result<String, Error> {
        if let Some(token) = self
            .access_tokens
            .get(account_id)
            .filter(|token| token.is_fresh())
        {
            return Ok(token.value().to_owned());
        }

        let refresh_token = refresh_token(account_id).ok_or(Error::Vault)?;
        let token = self.google.refresh_access_token(&refresh_token)?;
        let value = token.value().to_owned();
        self.access_tokens.insert(account_id.to_owned(), token);

        Ok(value)
    }
}

#[cfg(test)]
pub(crate) mod tests;
