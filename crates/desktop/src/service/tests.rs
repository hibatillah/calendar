//! Snapshot flow against a local fake Google, an in-memory cache and a test-owned Credential
//! Manager service. Google and the user's real vault entries are never touched.

use super::*;
use calendar_google::Account;
use chrono::{Duration, TimeZone};
use percent_encoding::percent_decode_str;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::atomic::AtomicUsize,
};

const LIST: &str = "/calendar/v3/users/me/calendarList";
const EVENTS: &str = "/calendar/v3/calendars/work/events";
const READ_AND_WRITE: &str = "https://www.googleapis.com/auth/calendar.calendarlist.readonly https://www.googleapis.com/auth/calendar.events";

fn now() -> DateTime<Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap()
}

struct Fake {
    origin: String,
    address: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
    }
}

/// Answers each request with the status and JSON the handler picks from `(path, query + body)`.
fn serve(handler: impl Fn(&str, &str) -> (u16, Value) + Send + 'static) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let halt = stop.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if halt.load(Ordering::Relaxed) {
                break;
            }
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let target = line.split_whitespace().nth(1).unwrap().to_owned();
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let Some((name, value)) = header.trim_end().split_once(": ") else {
                    break;
                };
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();

            let (path, query) = target.split_once('?').unwrap_or((&target, ""));
            let (status, reply) = handler(
                path,
                &format!("{query}&{}", String::from_utf8(body).unwrap()),
            );
            let reply = reply.to_string();
            let _ = write!(
                reader.get_mut(),
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });

    Fake {
        origin: format!("http://{address}"),
        address,
        stop,
    }
}

/// Test-owned service name, so no real credential is read or overwritten. Entries are removed on drop.
struct TestVault(
    Vault,
    #[allow(dead_code)] std::sync::MutexGuard<'static, ()>,
);

/// Credential Manager returned nothing right after a write when tests ran in parallel, so vault tests take turns.
static VAULT_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl TestVault {
    fn new() -> Self {
        let turn = VAULT_TESTS
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Self(
            Vault::new(&format!("calendar.test.{}", uuid::Uuid::new_v4())),
            turn,
        )
    }

    fn grant(&self, account_id: &str, scope: &str) {
        self.0
            .save(
                account_id,
                &Secret {
                    refresh_token: format!("refresh-{account_id}"),
                    scope: scope.into(),
                },
            )
            .unwrap();
    }
}

impl Drop for TestVault {
    fn drop(&mut self) {
        for account_id in ["a", "b", "subject-1"] {
            let _ = self.0.delete(account_id);
        }
    }
}

fn store_with(account_ids: &[&str]) -> Store {
    let store = Store::in_memory();
    for account_id in account_ids {
        store
            .upsert_account(&Account {
                account_id: (*account_id).into(),
                email: format!("{account_id}@example.com"),
                name: (*account_id).into(),
                picture: String::new(),
            })
            .unwrap();
    }

    store
}

fn google(fake: &Fake) -> Google {
    Google::for_fake_server(&fake.origin, chrono_tz::Asia::Jakarta)
}

fn param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then(|| percent_decode_str(value).decode_utf8_lossy().into_owned())
    })
}

#[test]
fn cached_events_come_first_then_each_account_keeps_its_own_copy_and_failures_keep_them() {
    let healthy = Arc::new(AtomicBool::new(true));
    let switch = healthy.clone();
    let fake = serve(move |path, query| match path {
        "/token" => (200, json!({"access_token": "access", "expires_in": 3600})),
        LIST => (
            200,
            json!({"items": [{"id": "work", "summary": "Work", "accessRole": "owner", "backgroundColor": "#112233"}]}),
        ),
        EVENTS if switch.load(Ordering::Relaxed) => {
            let token = param(query, "syncToken");
            let items = if token.is_some() {
                json!([])
            } else {
                json!([{"id": "one", "etag": "\"1\"", "summary": "Planning",
                    "start": {"dateTime": "2026-10-15T09:00:00+07:00"},
                    "end": {"dateTime": "2026-10-15T10:00:00+07:00"}}])
            };
            (200, json!({"nextSyncToken": "next", "items": items}))
        }
        EVENTS => (503, json!({})),
        _ => (404, json!({})),
    });
    let vault = TestVault::new();
    vault.grant("a", READ_AND_WRITE);
    vault.grant("b", READ_AND_WRITE);
    let mut store = store_with(&["a", "b"]);
    let mut syncer = Syncer::with_zone_source(google(&fake), || Some(chrono_tz::Asia::Jakarta));
    let errors = Mutex::default();

    // Before any network call: accounts are listed, nothing is claimed about events.
    let cached = build_snapshot(&store, &vault.0, &HashMap::new(), true, now()).unwrap();
    assert_eq!(cached.accounts.len(), 2);
    assert!(cached.events.is_empty() && cached.covered.is_none());
    assert_eq!(
        cached.zone, None,
        "no zone is claimed before a sync records one"
    );

    refresh(&mut syncer, &mut store, &vault.0, &errors, now()).unwrap();
    let synced = build_snapshot(&store, &vault.0, &errors.lock().unwrap(), true, now()).unwrap();
    assert_eq!(
        synced.zone,
        Some(chrono_tz::Asia::Jakarta),
        "the snapshot carries the cache's zone"
    );
    // Google event ids are provider-scoped, not account-scoped: the view key adds the account.
    let ids: Vec<_> = synced
        .events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    assert!(ids[0].starts_with("a/work/") && ids[1].starts_with("b/work/"));
    assert!(ids[0].ends_with(":one") && ids[1].ends_with(":one"));
    assert_ne!(
        ids[0], ids[1],
        "one calendar shared by two accounts stays two"
    );
    assert_eq!(synced.events[0].calendar_id, "a/work");
    assert_eq!(synced.accounts[0].calendars[0].color, 0x112233);
    assert!(
        synced
            .accounts
            .iter()
            .all(|account| account.error.is_none())
    );
    let (start, end) = synced.covered.unwrap();
    assert_eq!(
        (start, end),
        (now() - CACHE_PAST, now() + Duration::days(180))
    );

    healthy.store(false, Ordering::Relaxed);
    refresh(
        &mut syncer,
        &mut store,
        &vault.0,
        &errors,
        now() + Duration::seconds(60),
    )
    .unwrap();
    let failed = build_snapshot(&store, &vault.0, &errors.lock().unwrap(), true, now()).unwrap();
    assert_eq!(
        failed.events.len(),
        2,
        "a failed refresh keeps the saved events"
    );
    assert!(failed.accounts.iter().all(|account| {
        account
            .error
            .as_deref()
            .is_some_and(|error| error.contains("temporarily unavailable"))
    }));

    healthy.store(true, Ordering::Relaxed);
    refresh(
        &mut syncer,
        &mut store,
        &vault.0,
        &errors,
        now() + Duration::seconds(120),
    )
    .unwrap();
    assert!(errors.lock().unwrap().is_empty());
}

#[test]
fn permissions_are_reported_per_grant_and_a_missing_credential_is_unknown() {
    let vault = TestVault::new();
    vault.grant(
        "a",
        "https://www.googleapis.com/auth/calendar.calendarlist.readonly",
    );
    let store = store_with(&["a", "b"]);

    let snapshot = build_snapshot(&store, &vault.0, &HashMap::new(), true, now()).unwrap();
    let permissions = snapshot.accounts[0].permissions.unwrap();

    assert!(permissions.calendar_list && !permissions.read_events && !permissions.write_events);
    assert!(snapshot.accounts[1].permissions.is_none());
}

/// Plays the browser: one GET to the redirect the consent URL names.
fn browse(url: &str, outcome: &str) {
    let query = url.split_once('?').unwrap().1;
    let redirect = param(query, "redirect_uri").unwrap();
    let state = param(query, "state").unwrap();
    let mut stream =
        TcpStream::connect(redirect.trim_start_matches("http://").trim_end_matches('/')).unwrap();
    write!(
        stream,
        "GET /?state={state}&{outcome} HTTP/1.1\r\nHost: localhost\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
}

/// One whole sign-in: consent URL, the browser's redirect on another thread, then the exchange.
fn sign_in(
    google: &Google,
    store: &Store,
    vault: &Vault,
    outcome: &'static str,
) -> Result<&'static str, Error> {
    let sign_in = google.begin_sign_in().unwrap();
    let url = sign_in.authorization_url().to_owned();
    let browser = std::thread::spawn(move || browse(&url, outcome));
    let result = finish_sign_in(google, sign_in, store, vault);
    browser.join().unwrap();

    result
}

#[test]
fn sign_in_adds_an_account_once_refreshes_a_duplicate_and_keeps_accounts_when_denied() {
    let grants = Arc::new(AtomicUsize::new(0));
    let counter = grants.clone();
    let fake = serve(move |path, _| match path {
        "/token" => (
            200,
            json!({"access_token": "access", "refresh_token": format!("refresh-{}", counter.fetch_add(1, Ordering::Relaxed)),
                "expires_in": 3600, "scope": READ_AND_WRITE}),
        ),
        "/userinfo" => (
            200,
            json!({"sub": "subject-1", "email": "s@example.com", "name": "S"}),
        ),
        _ => (404, json!({})),
    });
    let google = google(&fake);
    let vault = TestVault::new();
    let store = store_with(&["a"]);
    // New account.
    let outcome = sign_in(&google, &store, &vault.0, "code=one");
    assert_eq!(outcome.unwrap(), "Account connected. Fetching calendars…");
    assert_eq!(store.accounts().unwrap().len(), 2);
    assert_eq!(
        vault.0.load("subject-1").unwrap().unwrap().refresh_token,
        "refresh-0"
    );

    // Same account again: credentials replaced, no second row.
    let outcome = sign_in(&google, &store, &vault.0, "code=two");
    assert!(outcome.unwrap().contains("already connected"));
    assert_eq!(store.accounts().unwrap().len(), 2);
    assert_eq!(
        vault.0.load("subject-1").unwrap().unwrap().refresh_token,
        "refresh-1"
    );

    // Denied consent: an error, and nothing changes.
    let outcome = sign_in(&google, &store, &vault.0, "error=access_denied");
    assert!(outcome.is_err());
    assert_eq!(store.accounts().unwrap().len(), 2);
    assert_eq!(
        vault.0.load("subject-1").unwrap().unwrap().refresh_token,
        "refresh-1"
    );
}

#[test]
fn disconnect_removes_the_credential_and_every_cached_row_and_returns_the_token_to_revoke() {
    let vault = TestVault::new();
    vault.grant("a", READ_AND_WRITE);
    vault.grant("b", READ_AND_WRITE);
    let store = store_with(&["a", "b"]);

    let token = remove_account(&store, &vault.0, "a").unwrap();

    assert_eq!(token.as_deref(), Some("refresh-a"));
    assert!(vault.0.load("a").unwrap().is_none());
    assert!(
        vault.0.load("b").unwrap().is_some(),
        "other accounts are untouched"
    );
    let remaining: Vec<_> = store
        .accounts()
        .unwrap()
        .into_iter()
        .map(|account| account.account_id)
        .collect();
    assert_eq!(remaining, ["b"]);
    assert_eq!(
        remove_account(&store, &vault.0, "a").unwrap(),
        None,
        "disconnecting twice is harmless"
    );
}

#[test]
fn an_unconfigured_build_publishes_cached_data_and_refuses_to_connect() {
    let directory = std::env::temp_dir().join(format!("calendar-test-{}", uuid::Uuid::new_v4()));
    let (updates, receiver) = async_channel::unbounded();
    let inner = Inner::new(
        None,
        Some(directory.clone()),
        format!("calendar.test.{}", uuid::Uuid::new_v4()),
        updates,
    );
    let (_sender, messages) = mpsc::channel();

    inner.run_sync(&messages);
    let handle = Handle {
        inner: inner.clone(),
        sync: mpsc::channel().0,
    };
    handle.connect();

    let Ok(Update::Snapshot(snapshot)) = receiver.try_recv() else {
        panic!("the cached snapshot is sent first");
    };
    assert!(!snapshot.configured && snapshot.accounts.is_empty());
    assert!(receiver.try_recv().is_err(), "no sign-in update, no thread");
    assert!(!inner.connecting.load(Ordering::SeqCst));
    drop(inner);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_snapshot_clears_the_cache_error_and_updates_replace_state() {
    let mut model = Model::default();

    model.apply(Update::Error(
        "The local calendar cache is unavailable.".into(),
    ));
    model.apply(Update::Syncing(true));
    model.apply(Update::SignIn(SignInState::Waiting));
    assert!(model.error.is_some() && model.syncing && model.sign_in == SignInState::Waiting);

    model.apply(Update::Snapshot(Snapshot::default()));
    assert!(model.error.is_none());
}

#[test]
fn unfinished_writes_appear_in_the_snapshot_under_the_views_event_key() {
    use calendar_sync::{Operation, OperationKind, OperationState};
    use snapshot::{split_calendar_key, split_event_key};

    let vault = TestVault::new();
    let mut store = store_with(&["a"]);
    let operation = |operation_id: &str| Operation {
        operation_id: operation_id.into(),
        account_id: "a".into(),
        calendar_id: "work".into(),
        event_id: format!("google:d29yaw:{operation_id}"),
        kind: OperationKind::Create,
        state: OperationState::Pending,
    };
    let done = store.begin_operation(&operation("done")).unwrap();
    store
        .finish_operation(&done, OperationState::Applied, None)
        .unwrap();
    store.begin_operation(&operation("open")).unwrap();

    let snapshot = build_snapshot(&store, &vault.0, &HashMap::new(), true, now()).unwrap();

    assert_eq!(snapshot.pending.len(), 1, "a finished write is not pending");
    let open = &snapshot.pending[0];
    assert_eq!(open.event_id, "a/work/google:d29yaw:open");
    assert_eq!(
        split_event_key(&open.event_id),
        Some(("a", "work", "google:d29yaw:open"))
    );
    assert_eq!(split_calendar_key("a/work"), Some(("a", "work")));
    assert_eq!(split_event_key("no-separator"), None);
}
