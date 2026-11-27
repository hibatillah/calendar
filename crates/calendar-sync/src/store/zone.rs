//! The zone the cached rows were built in. All-day rows are midnight instants of that zone, so the
//! cache records it and every process (tray and window) reads it instead of asking the system.

use super::Store;
use crate::Error;
use chrono_tz::Tz;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

const ZONE_KEY: &str = "cache_zone";

impl Store {
    /// `None` on a cache that never recorded one (or holds a name this build cannot parse).
    pub fn zone(&self) -> Result<Option<Tz>, Error> {
        let name: Option<String> = self
            .connection
            .query_row("SELECT value FROM meta WHERE key = ?1", [ZONE_KEY], |row| {
                row.get(0)
            })
            .optional()?;

        Ok(name.and_then(|name| name.parse().ok()))
    }

    /// Records `zone`. When it differs from the recorded one, every calendar's sync token is
    /// dropped in the same transaction, so the next sync of each calendar is a full one that
    /// rebuilds its all-day rows; cached events stay visible until then. Returns whether the
    /// zone changed. `IMMEDIATE` makes the tray and the window agree on who changed it.
    pub fn record_zone(&mut self, zone: Tz) -> Result<bool, Error> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let recorded: Option<String> = transaction
            .query_row("SELECT value FROM meta WHERE key = ?1", [ZONE_KEY], |row| {
                row.get(0)
            })
            .optional()?;
        if recorded.as_deref() == Some(zone.name()) {
            return Ok(false);
        }

        transaction.execute("UPDATE calendars SET sync_token = NULL", [])?;
        transaction.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![ZONE_KEY, zone.name()],
        )?;
        transaction.commit()?;

        Ok(true)
    }
}
