//! SQLite cache: accounts, calendars with their sync state, events and the write journal.
//!
//! Everything is namespaced by `account_id`, so a calendar shared between two accounts is two
//! separate cached calendars. Refresh tokens are never stored here (see `vault`). Two processes
//! (tray and window) may open the same file: WAL plus `busy_timeout`, and every write transaction
//! is short.

use crate::Error;
use calendar_core::CalendarEvent;
use calendar_google::{Account, Calendar, EventChange, GoogleEvent};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use std::{path::Path, time::Duration};

mod journal;
mod zone;

pub use journal::{Operation, OperationKind, OperationState};

const SCHEMA: &str = "
PRAGMA user_version = 1;
CREATE TABLE IF NOT EXISTS accounts (
    account_id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    picture TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS calendars (
    account_id TEXT NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    calendar_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    is_primary INTEGER NOT NULL,
    access_role TEXT NOT NULL,
    background_color TEXT NOT NULL,
    time_zone TEXT NOT NULL,
    sync_token TEXT,
    window_start INTEGER,
    window_end INTEGER,
    PRIMARY KEY (account_id, calendar_id)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS events (
    account_id TEXT NOT NULL,
    calendar_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    title TEXT NOT NULL,
    start_at INTEGER NOT NULL,
    end_at INTEGER NOT NULL,
    timezone TEXT NOT NULL,
    all_day INTEGER NOT NULL,
    description TEXT NOT NULL,
    location TEXT NOT NULL,
    color INTEGER NOT NULL,
    revision INTEGER NOT NULL,
    read_only INTEGER NOT NULL,
    etag TEXT NOT NULL,
    recurring_event_id TEXT,
    operation_id TEXT,
    PRIMARY KEY (account_id, calendar_id, event_id),
    FOREIGN KEY (account_id, calendar_id) REFERENCES calendars(account_id, calendar_id) ON DELETE CASCADE
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS operations (
    operation_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    calendar_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    state TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
";

const EVENT_COLUMNS: &str = "account_id, calendar_id, event_id, title, start_at, end_at, timezone, all_day, description, location, color, revision, read_only, etag, recurring_event_id, operation_id";

/// What the next incremental sync needs, plus the range the cached events cover.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncState {
    pub sync_token: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CachedEvent {
    pub account_id: String,
    pub google: GoogleEvent,
}

pub struct Store {
    connection: Connection,
}

fn seconds(row: &Row, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    let value: i64 = row.get(index)?;

    DateTime::from_timestamp(value, 0).ok_or(rusqlite::Error::IntegralValueOutOfRange(index, value))
}

impl Store {
    /// Opens `calendar.db` in `directory`, creating the directory and schema when missing.
    pub fn open(directory: &Path) -> Result<Self, Error> {
        // A failure here surfaces as a `CantOpen` error from the open below.
        let _ = std::fs::create_dir_all(directory);

        Self::configure(Connection::open(directory.join("calendar.db"))?)
    }

    /// For tests, here and in dependants (`test-support`); never touches the disk.
    #[cfg(any(test, feature = "test-support"))]
    pub fn in_memory() -> Self {
        Self::configure(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn configure(connection: Connection) -> Result<Self, Error> {
        // Wait for the other process instead of failing; set before the first write.
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.pragma_update(None, "foreign_keys", true)?;
        // About 0.5 MiB of page cache instead of the 2 MiB default.
        connection.pragma_update(None, "cache_size", -512)?;
        connection.execute_batch(SCHEMA)?;

        Ok(Self { connection })
    }

    pub fn upsert_account(&self, account: &Account) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO accounts (account_id, email, name, picture) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id) DO UPDATE SET email = ?2, name = ?3, picture = ?4",
            params![
                account.account_id,
                account.email,
                account.name,
                account.picture
            ],
        )?;

        Ok(())
    }

    pub fn accounts(&self) -> Result<Vec<Account>, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT account_id, email, name, picture FROM accounts ORDER BY email")?;
        let accounts = statement
            .query_map([], |row| {
                Ok(Account {
                    account_id: row.get(0)?,
                    email: row.get(1)?,
                    name: row.get(2)?,
                    picture: row.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(accounts)
    }

    /// Disconnect: removes the account with its calendars, events and journal.
    pub fn remove_account(&self, account_id: &str) -> Result<(), Error> {
        self.connection
            .execute("DELETE FROM accounts WHERE account_id = ?1", [account_id])?;

        Ok(())
    }

    /// Stores the account's current calendar list. Calendars missing from it are removed with
    /// their events; sync state of the others is kept.
    pub fn replace_calendars(
        &mut self,
        account_id: &str,
        calendars: &[Calendar],
    ) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        let cached_ids: Vec<String> = transaction
            .prepare("SELECT calendar_id FROM calendars WHERE account_id = ?1")?
            .query_map([account_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        for removed in cached_ids
            .iter()
            .filter(|cached| !calendars.iter().any(|calendar| &calendar.id == *cached))
        {
            transaction.execute(
                "DELETE FROM calendars WHERE account_id = ?1 AND calendar_id = ?2",
                [account_id, removed],
            )?;
        }
        for calendar in calendars {
            transaction.execute(
                "INSERT INTO calendars (account_id, calendar_id, summary, is_primary, access_role, background_color, time_zone)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(account_id, calendar_id) DO UPDATE SET
                   summary = ?3, is_primary = ?4, access_role = ?5, background_color = ?6, time_zone = ?7",
                params![
                    account_id,
                    calendar.id,
                    calendar.summary,
                    calendar.primary,
                    calendar.access_role,
                    calendar.background_color,
                    calendar.time_zone
                ],
            )?;
        }
        transaction.commit()?;

        Ok(())
    }

    pub fn calendars(&self, account_id: &str) -> Result<Vec<Calendar>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT calendar_id, summary, is_primary, access_role, background_color, time_zone
             FROM calendars WHERE account_id = ?1 ORDER BY is_primary DESC, summary, calendar_id",
        )?;
        let calendars = statement
            .query_map([account_id], |row| {
                Ok(Calendar {
                    id: row.get(0)?,
                    summary: row.get(1)?,
                    primary: row.get(2)?,
                    access_role: row.get(3)?,
                    background_color: row.get(4)?,
                    time_zone: row.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(calendars)
    }

    /// `None` until the first full sync of the calendar succeeded, and after `clear_sync_token`.
    pub fn sync_state(
        &self,
        account_id: &str,
        calendar_id: &str,
    ) -> Result<Option<SyncState>, Error> {
        let state = self
            .connection
            .query_row(
                "SELECT sync_token, window_start, window_end FROM calendars
                 WHERE account_id = ?1 AND calendar_id = ?2 AND sync_token IS NOT NULL",
                [account_id, calendar_id],
                |row| {
                    Ok(SyncState {
                        sync_token: row.get(0)?,
                        window_start: seconds(row, 1)?,
                        window_end: seconds(row, 2)?,
                    })
                },
            )
            .optional()?;

        Ok(state)
    }

    /// After a 410: the token is dead but the cached events stay visible until the resync replaces them.
    pub fn clear_sync_token(&self, account_id: &str, calendar_id: &str) -> Result<(), Error> {
        self.connection.execute(
            "UPDATE calendars SET sync_token = NULL WHERE account_id = ?1 AND calendar_id = ?2",
            [account_id, calendar_id],
        )?;

        Ok(())
    }

    /// Replaces all cached events of the calendar and records the new state in one transaction,
    /// so a failure leaves the previous events and token untouched.
    pub fn store_full_sync(
        &mut self,
        account_id: &str,
        calendar_id: &str,
        changes: &[EventChange],
        state: &SyncState,
    ) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM events WHERE account_id = ?1 AND calendar_id = ?2",
            [account_id, calendar_id],
        )?;
        apply(&transaction, account_id, calendar_id, changes)?;
        transaction.execute(
            "UPDATE calendars SET sync_token = ?3, window_start = ?4, window_end = ?5
             WHERE account_id = ?1 AND calendar_id = ?2",
            params![
                account_id,
                calendar_id,
                state.sync_token,
                state.window_start.timestamp(),
                state.window_end.timestamp()
            ],
        )?;
        transaction.commit()?;

        Ok(())
    }

    /// Applies changes in order and advances the token in one transaction.
    pub fn store_incremental_sync(
        &mut self,
        account_id: &str,
        calendar_id: &str,
        changes: &[EventChange],
        sync_token: &str,
    ) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        apply(&transaction, account_id, calendar_id, changes)?;
        transaction.execute(
            "UPDATE calendars SET sync_token = ?3 WHERE account_id = ?1 AND calendar_id = ?2",
            [account_id, calendar_id, sync_token],
        )?;
        transaction.commit()?;

        Ok(())
    }

    /// Drops events that ended before `cutoff`; incremental sync cannot be bounded, so this keeps
    /// the cache inside its window.
    pub fn prune_ended_before(&self, cutoff: DateTime<Utc>) -> Result<(), Error> {
        self.connection
            .execute("DELETE FROM events WHERE end_at < ?1", [cutoff.timestamp()])?;

        Ok(())
    }

    /// Events overlapping `[from, to)` across all accounts, ordered by start.
    pub fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CachedEvent>, Error> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {EVENT_COLUMNS} FROM events WHERE end_at > ?1 AND start_at < ?2
             ORDER BY start_at, account_id, event_id"
        ))?;
        let events = statement
            .query_map([from.timestamp(), to.timestamp()], read_event)?
            .collect::<Result<_, _>>()?;

        Ok(events)
    }

    /// One cached event, as Google last reported it.
    pub fn event(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<Option<CachedEvent>, Error> {
        let event = self
            .connection
            .query_row(
                &format!(
                    "SELECT {EVENT_COLUMNS} FROM events
                     WHERE account_id = ?1 AND calendar_id = ?2 AND event_id = ?3"
                ),
                [account_id, calendar_id, event_id],
                read_event,
            )
            .optional()?;

        Ok(event)
    }

    /// True when a cached instance belongs to the recurring series `series_id` (a provider id).
    /// Expanded reads never contain the series itself, so this is how a series id is recognised.
    pub fn has_instances_of(
        &self,
        account_id: &str,
        calendar_id: &str,
        series_id: &str,
    ) -> Result<bool, Error> {
        let found = self
            .connection
            .query_row(
                "SELECT 1 FROM events
                 WHERE account_id = ?1 AND calendar_id = ?2 AND recurring_event_id = ?3 LIMIT 1",
                [account_id, calendar_id, series_id],
                |_| Ok(()),
            )
            .optional()?;

        Ok(found.is_some())
    }
}

fn read_event(row: &Row) -> rusqlite::Result<CachedEvent> {
    Ok(CachedEvent {
        account_id: row.get(0)?,
        google: GoogleEvent {
            event: CalendarEvent {
                calendar_id: row.get(1)?,
                event_id: row.get(2)?,
                title: row.get(3)?,
                start: seconds(row, 4)?,
                end: seconds(row, 5)?,
                timezone: row.get(6)?,
                all_day: row.get(7)?,
                description: row.get(8)?,
                location: row.get(9)?,
                color: row.get(10)?,
                revision: row.get(11)?,
                read_only: row.get(12)?,
            },
            etag: row.get(13)?,
            recurring_event_id: row.get(14)?,
            operation_id: row.get(15)?,
        },
    })
}

/// Upserts and deletions in the order Google sent them. A deleted id also removes instances whose
/// `recurring_event_id` is that id, in case Google reports a whole series as one tombstone.
fn apply(
    transaction: &Transaction,
    account_id: &str,
    calendar_id: &str,
    changes: &[EventChange],
) -> rusqlite::Result<()> {
    let mut upsert = transaction.prepare(&format!(
        "INSERT OR REPLACE INTO events ({EVENT_COLUMNS})
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
    ))?;
    let mut delete = transaction.prepare(
        "DELETE FROM events WHERE account_id = ?1 AND calendar_id = ?2
         AND (event_id = ?3 OR recurring_event_id = ?4)",
    )?;

    for change in changes {
        match change {
            EventChange::Upsert(google) => {
                let event = &google.event;
                upsert.execute(params![
                    account_id,
                    calendar_id,
                    event.event_id,
                    event.title,
                    event.start.timestamp(),
                    event.end.timestamp(),
                    event.timezone,
                    event.all_day,
                    event.description,
                    event.location,
                    event.color,
                    event.revision,
                    event.read_only,
                    google.etag,
                    google.recurring_event_id,
                    google.operation_id
                ])?;
            }
            EventChange::Deleted { event_id } => {
                let provider_id = event_id
                    .rsplit_once(':')
                    .map_or(event_id.as_str(), |parts| parts.1);
                delete.execute(params![account_id, calendar_id, event_id, provider_id])?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
