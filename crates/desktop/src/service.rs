//! Bridge between the UI and background work. The UI never touches the network, SQLite or the
//! credential vault: it sends commands through a [`Handle`] and renders the [`Snapshot`]s that
//! arrive on the update channel.
//!
//! Threads: one sync thread (the `calendar-sync` scheduler loop) for the process lifetime, plus a
//! short-lived thread per sign-in or disconnect, because both block on the network. Each thread
//! opens its own `Store`; SQLite runs in WAL mode so they do not block each other.

use async_channel::{Receiver, Sender};
use calendar_core::CalendarEvent;
use calendar_google::{CancelHandle, Config, Google, Permissions, SignIn, open_in_browser};
use calendar_sync::{
    CACHE_PAST, Error, Store, Syncer, WriteError, WriteRequest, Written, align_cache_zone,
    data_directory,
    scheduler::{self, Message, Mode},
    system_zone,
    vault::{SERVICE, Secret, Vault},
};
use chrono::{DateTime, Utc};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

pub mod snapshot;
#[cfg(test)]
mod tests;

use snapshot::build_snapshot;
pub use snapshot::{AccountView, Snapshot};

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub enum SignInState {
    #[default]
    Idle,
    /// The browser consent page is open; cancel or finish it there.
    Waiting,
    /// How the last attempt ended, for display.
    Message(String),
}

/// How one write ended, matched to the request by `operation_id`.
pub struct WriteResult {
    pub operation_id: String,
    /// `WriteError::Unknown` means the write may have been saved: keep it pending, do not retry.
    /// The snapshot sent just before this result already includes a confirmed write.
    pub outcome: Result<Written, WriteError>,
}

pub enum Update {
    Snapshot(Snapshot),
    Write(WriteResult),
    Syncing(bool),
    SignIn(SignInState),
    /// The cache itself could not be read or written.
    Error(String),
}

/// What the GPUI entity holds; views copy from it when it changes.
#[derive(Default)]
pub struct Model {
    pub snapshot: Arc<Snapshot>,
    pub syncing: bool,
    pub sign_in: SignInState,
    pub error: Option<String>,
    /// Results the views have not handled yet; take them with `std::mem::take`.
    pub finished_writes: Vec<WriteResult>,
}

impl Model {
    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Snapshot(snapshot) => {
                self.snapshot = Arc::new(snapshot);
                self.error = None;
            }
            Update::Write(result) => self.finished_writes.push(result),
            Update::Syncing(syncing) => self.syncing = syncing,
            Update::SignIn(state) => self.sign_in = state,
            Update::Error(message) => self.error = Some(message),
        }
    }
}

struct Inner {
    /// `None`: sign-in is not configured in this build.
    config: Option<Config>,
    directory: Option<PathBuf>,
    vault_service: String,
    updates: Sender<Update>,
    /// Last refresh error per account id; cleared by the next successful refresh.
    errors: Mutex<HashMap<String, String>>,
    connecting: AtomicBool,
    cancel: Mutex<Option<CancelHandle>>,
    /// Writes wait here for the sync thread, which owns the HTTP client and the access tokens.
    writes: Mutex<VecDeque<WriteRequest>>,
}

/// Commands from the UI. Cheap to clone; every send ignores a stopped worker.
#[derive(Clone)]
pub struct Handle {
    inner: Arc<Inner>,
    sync: mpsc::Sender<Message>,
}

pub fn start() -> (Handle, Receiver<Update>) {
    let (updates, receiver) = async_channel::unbounded();
    let inner = Inner::new(
        Config::from_environment(),
        data_directory(),
        SERVICE.into(),
        updates,
    );
    let (sync, messages) = mpsc::channel();
    let worker = inner.clone();
    std::thread::Builder::new()
        .name("calendar-sync".into())
        .spawn(move || worker.run_sync(&messages))
        .expect("spawn sync thread");

    (Handle { inner, sync }, receiver)
}

impl Handle {
    /// Window opened or focused. The scheduler skips it right after a sync.
    pub fn sync_now(&self) {
        let _ = self.sync.send(Message::SyncNow);
    }

    /// Queues a create, update or delete. It runs on the sync thread ahead of the next poll; the
    /// outcome arrives as `Update::Write`. Reuse `operation_id` to retry an unknown outcome.
    pub fn write(&self, request: WriteRequest) {
        if self.inner.config.is_none() {
            // No OAuth client, so there is no credential to write with.
            return self.inner.send(Update::Write(WriteResult {
                operation_id: request.operation_id,
                outcome: Err(Error::Vault.into()),
            }));
        }

        self.inner.writes.lock().unwrap().push_back(request);
        let _ = self.sync.send(Message::Run);
    }

    /// Opens browser consent and waits for it on its own thread. One attempt at a time.
    pub fn connect(&self) {
        let Some(config) = self.inner.config.clone() else {
            return;
        };
        if self.inner.connecting.swap(true, Ordering::SeqCst) {
            return;
        }

        let (inner, sync) = (self.inner.clone(), self.sync.clone());
        std::thread::spawn(move || {
            let state = match inner.connect(config) {
                Ok(notice) => {
                    let _ = sync.send(Message::SyncNow);
                    SignInState::Message(notice.into())
                }
                Err(error) => SignInState::Message(error),
            };
            *inner.cancel.lock().unwrap() = None;
            inner.connecting.store(false, Ordering::SeqCst);
            inner.send(Update::SignIn(state));
        });
    }

    pub fn cancel_connect(&self) {
        if let Some(cancel) = self.inner.cancel.lock().unwrap().as_ref() {
            cancel.cancel();
        }
    }

    /// Removes the credential and every cached row of the account, then revokes at Google.
    pub fn disconnect(&self, account_id: String) {
        let inner = self.inner.clone();
        std::thread::spawn(move || inner.disconnect(&account_id));
    }

    /// A handle whose worker never runs, for view tests.
    #[cfg(test)]
    pub fn disconnected() -> Self {
        let (updates, _) = async_channel::unbounded();
        let (sync, _) = mpsc::channel();

        Self {
            inner: Inner::new(None, None, "calendar.test.unused".into(), updates),
            sync,
        }
    }
}

impl Inner {
    fn new(
        config: Option<Config>,
        directory: Option<PathBuf>,
        vault_service: String,
        updates: Sender<Update>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            directory,
            vault_service,
            updates,
            errors: Mutex::default(),
            connecting: AtomicBool::new(false),
            cancel: Mutex::default(),
            writes: Mutex::default(),
        })
    }

    fn send(&self, update: Update) {
        let _ = self.updates.send_blocking(update);
    }

    fn open_store(&self) -> Result<Store, String> {
        let directory = self
            .directory
            .as_deref()
            .ok_or("No data directory: LOCALAPPDATA is not set.")?;

        Store::open(directory).map_err(|error| error.to_string())
    }

    /// Sends the current cache contents to the UI.
    fn publish(&self, store: &Store, vault: &Vault) {
        let errors = self.errors.lock().unwrap().clone();
        match build_snapshot(store, vault, &errors, self.config.is_some(), Utc::now()) {
            Ok(snapshot) => self.send(Update::Snapshot(snapshot)),
            Err(error) => {
                tracing::warn!(action = "snapshot_failed", error = %error);
                self.send(Update::Error(error.to_string()));
            }
        }
    }

    /// Cached events go out before the first network call, then every sync result follows.
    fn run_sync(&self, messages: &mpsc::Receiver<Message>) {
        let mut store = match self.open_store() {
            Ok(store) => store,
            Err(error) => return self.send(Update::Error(error)),
        };
        let vault = Vault::new(&self.vault_service);
        // The first snapshot already carries the zone the system uses now.
        if let Err(error) = align_cache_zone(&mut store, system_zone()) {
            self.send(Update::Error(error.to_string()));
        }
        self.publish(&store, &vault);
        let Some(config) = self.config.clone() else {
            return;
        };

        let mut syncer = Syncer::new(Google::new(config));
        scheduler::run(Mode::WindowOpen, messages, || {
            // Each `Message::Run` queued one write: do it instead of a poll.
            let queued = self.writes.lock().unwrap().pop_front();
            if let Some(request) = queued {
                let outcome = syncer.write(&mut store, &request, refresh_token(&vault));
                self.publish(&store, &vault);
                self.send(Update::Write(WriteResult {
                    operation_id: request.operation_id,
                    outcome,
                }));
                return;
            }

            self.send(Update::Syncing(true));
            if let Err(error) = refresh(&mut syncer, &mut store, &vault, &self.errors, Utc::now()) {
                self.send(Update::Error(error.to_string()));
            }
            self.publish(&store, &vault);
            self.send(Update::Syncing(false));
        });
    }

    fn connect(&self, config: Config) -> Result<&'static str, String> {
        let google = Google::new(config);
        let sign_in = google.begin_sign_in().map_err(|error| error.to_string())?;
        *self.cancel.lock().unwrap() = Some(sign_in.cancel_handle());
        open_in_browser(sign_in.authorization_url()).map_err(|error| error.to_string())?;
        self.send(Update::SignIn(SignInState::Waiting));

        let store = self.open_store()?;
        let vault = Vault::new(&self.vault_service);
        let notice =
            finish_sign_in(&google, sign_in, &store, &vault).map_err(|error| error.to_string())?;
        self.publish(&store, &vault);

        Ok(notice)
    }

    fn disconnect(&self, account_id: &str) {
        let vault = Vault::new(&self.vault_service);
        let removed = self
            .open_store()
            .and_then(|store| {
                let token = remove_account(&store, &vault, account_id)
                    .map_err(|error| error.to_string())?;
                self.errors.lock().unwrap().remove(account_id);
                self.publish(&store, &vault);

                Ok(token)
            })
            .inspect_err(|error| self.send(Update::Error(error.clone())));
        tracing::info!(
            action = "account_disconnected",
            account_id,
            removed = removed.is_ok()
        );

        // Local data is already gone; revoking is best effort and may take a while.
        if let (Ok(Some(token)), Some(config)) = (removed, self.config.clone()) {
            let outcome = Google::new(config).revoke(&token);
            tracing::info!(action = "account_revoked", succeeded = outcome.is_ok());
        }
    }
}

/// Reads an account's refresh token from the vault; `None` when it is missing or unreadable.
fn refresh_token(vault: &Vault) -> impl Fn(&str) -> Option<String> + '_ {
    |account_id| {
        vault
            .load(account_id)
            .ok()
            .flatten()
            .map(|secret| secret.refresh_token)
    }
}

/// Runs one sync of every cached account and records each account's outcome.
fn refresh(
    syncer: &mut Syncer,
    store: &mut Store,
    vault: &Vault,
    errors: &Mutex<HashMap<String, String>>,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let outcomes = syncer.sync_all(store, now, refresh_token(vault))?;

    let mut errors = errors.lock().unwrap();
    for outcome in outcomes {
        match outcome.result {
            Ok(()) => errors.remove(&outcome.account_id),
            Err(error) => errors.insert(outcome.account_id, error.to_string()),
        };
    }

    Ok(())
}

/// Waits for the consent redirect and stores the account. Nothing is written when consent fails,
/// so existing accounts are untouched; a repeat sign-in updates the stored credential in place.
fn finish_sign_in(
    google: &Google,
    sign_in: SignIn,
    store: &Store,
    vault: &Vault,
) -> Result<&'static str, Error> {
    let existing: Vec<String> = store
        .accounts()?
        .into_iter()
        .map(|account| account.account_id)
        .collect();
    let signed_in = google.complete_sign_in(sign_in, &existing)?;
    let credentials = signed_in.credentials;

    vault.save(
        &credentials.account.account_id,
        &Secret {
            refresh_token: credentials.refresh_token,
            scope: credentials.scope,
        },
    )?;
    store.upsert_account(&credentials.account)?;

    Ok(if signed_in.already_connected {
        "That account was already connected. Its access was refreshed."
    } else {
        "Account connected. Fetching calendars…"
    })
}

/// Deletes the credential, then the cached rows. Returns the refresh token to revoke, if any.
fn remove_account(store: &Store, vault: &Vault, account_id: &str) -> Result<Option<String>, Error> {
    let secret = vault.load(account_id).ok().flatten();
    vault.delete(account_id)?;
    store.remove_account(account_id)?;

    Ok(secret.map(|secret| secret.refresh_token))
}
