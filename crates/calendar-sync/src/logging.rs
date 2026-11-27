//! Local diagnostics only: events go to a size-capped file under the app data directory.
//! Nothing is sent over the network, and call sites log identifiers and outcomes, never
//! tokens, consent URLs or event content.
use std::fs::{self, File, OpenOptions};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::level_filters::LevelFilter;

const LOG_FILE_NAME: &str = "calendar.log";
const PREVIOUS_LOG_FILE_NAME: &str = "calendar.previous.log";
const MAXIMUM_LOG_BYTES: u64 = 1_000_000;

/// Resolves `CALENDAR_LOG_DIR`, otherwise `%LOCALAPPDATA%\Calendar\logs`.
fn log_directory() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("CALENDAR_LOG_DIR") {
        return Some(PathBuf::from(directory));
    }

    std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("Calendar").join("logs"))
}

/// Opens the log file, moving an oversized one aside so disk use stays under two files.
fn open_log_file(directory: &PathBuf) -> std::io::Result<File> {
    fs::create_dir_all(directory)?;
    let path = directory.join(LOG_FILE_NAME);
    let is_oversized = fs::metadata(&path).is_ok_and(|metadata| metadata.len() > MAXIMUM_LOG_BYTES);
    if is_oversized {
        fs::rename(&path, directory.join(PREVIOUS_LOG_FILE_NAME))?;
    }

    OpenOptions::new().create(true).append(true).open(path)
}

/// Installs the file logger for `process` (`desktop` or `tray`), both appending to one file; logging stays disabled if the directory is not writable.
pub fn init(process: &str) {
    let Some(directory) = log_directory() else {
        return;
    };
    let Ok(file) = open_log_file(&directory) else {
        return;
    };
    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| value.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::INFO);
    let _ = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .try_init();
    tracing::info!(action = "application_started", process);
}
