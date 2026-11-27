//! Local cache, `syncToken` sync, polling schedule and the Windows credential vault.
//!
//! Blocking by design: the caller owns one background thread that runs the [`scheduler`] loop.
//! No GUI and no async runtime. Tokens never reach SQLite or logs.

#[cfg(windows)]
pub mod instance;
pub mod logging;
pub mod scheduler;
mod store;
mod sync;
#[cfg(windows)]
pub mod vault;
mod zone;

use std::{fmt, path::PathBuf};

pub use store::{CachedEvent, Operation, OperationKind, OperationState, Store, SyncState};
pub use sync::{
    AccountOutcome, Change, PAST as CACHE_PAST, Syncer, WriteError, WriteRequest, Written,
};
pub use zone::{align_cache_zone, system_zone};

/// Messages are safe to show: they never contain provider data, tokens or SQL values.
#[derive(Debug)]
pub enum Error {
    Google(calendar_google::Error),
    Storage(rusqlite::Error),
    /// No refresh token is available for the account (never stored, deleted, or the vault failed).
    Vault,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Google(error) => error.fmt(formatter),
            Self::Storage(_) => formatter.write_str("The local calendar cache is unavailable."),
            Self::Vault => {
                formatter.write_str("Google credentials are unavailable. Reconnect the account.")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<calendar_google::Error> for Error {
    fn from(error: calendar_google::Error) -> Self {
        Self::Google(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error)
    }
}

/// `CALENDAR_DATA_DIR`, otherwise `%LOCALAPPDATA%\Calendar`. Never relative to the working directory.
pub fn data_directory() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("CALENDAR_DATA_DIR") {
        return Some(PathBuf::from(directory));
    }

    std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("Calendar"))
}
