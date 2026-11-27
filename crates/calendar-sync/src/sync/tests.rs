//! Sync behaviour against a local fake Google and an in-memory cache. Google is never contacted.

use super::*;
use calendar_google::Account;
use chrono::TimeZone;
use chrono_tz::Asia::Jakarta;
use fake::{Fake, Request, serve};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(crate) mod fake;
mod zone;

const LIST: &str = "/calendar/v3/users/me/calendarList";

fn events_path(calendar_id: &str) -> String {
    format!("/calendar/v3/calendars/{calendar_id}/events")
}

/// Token endpoint and calendar list shared by most tests; `events` answers the event reads.
/// Access tokens are `access-<refresh token>`, so a handler can tell which account is asking.
fn google_fake(
    calendar_ids: &'static [&'static str],
    events: impl Fn(&Request) -> (u16, Value) + Send + 'static,
) -> Fake {
    serve(move |request| match request.path.as_str() {
        "/token" => (
            200,
            json!({"access_token": format!("access-{}", request.param("refresh_token").unwrap()), "expires_in": 3600}),
        ),
        LIST => (
            200,
            json!({"items": calendar_ids.iter().map(|id| json!({"id": id, "accessRole": "owner"})).collect::<Vec<_>>()}),
        ),
        _ => events(request),
    })
}

pub(crate) fn event(id: &str, title: &str) -> Value {
    json!({"id": id, "etag": format!("\"{title}\""), "summary": title,
        "start": {"dateTime": "2026-10-15T09:00:00+07:00"}, "end": {"dateTime": "2026-10-15T10:00:00+07:00"}})
}

fn tombstone(id: &str) -> Value {
    json!({"id": id, "status": "cancelled"})
}

pub(crate) fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap()
}

pub(crate) fn syncer(fake: &Fake) -> Syncer {
    Syncer::with_zone_source(Google::for_fake_server(&fake.origin, Jakarta), || {
        Some(Jakarta)
    })
}

pub(crate) fn store(account_ids: &[&str]) -> Store {
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

pub(crate) fn refresh_token(account_id: &str) -> Option<String> {
    Some(format!("refresh-{account_id}"))
}

pub(crate) fn sync(
    syncer: &mut Syncer,
    store: &mut Store,
    at: DateTime<Utc>,
) -> Vec<Result<(), Error>> {
    syncer
        .sync_all(store, at, refresh_token)
        .unwrap()
        .into_iter()
        .map(|outcome| outcome.result)
        .collect()
}

/// Titles of every cached event, as `account:title`, in start order.
pub(crate) fn titles(store: &Store) -> Vec<String> {
    let all = store
        .events_between(now() - Duration::days(365), now() + Duration::days(365))
        .unwrap();

    all.into_iter()
        .map(|cached| format!("{}:{}", cached.account_id, cached.google.event.title))
        .collect()
}

#[test]
fn full_sync_is_bounded_then_increments_use_only_the_token_and_apply_deletions() {
    let fake = google_fake(&["work"], |request| match request.param("syncToken") {
        None => (
            200,
            json!({"nextSyncToken": "t1", "items": [event("one", "One"), event("two", "Two")]}),
        ),
        Some("t1") => (
            200,
            json!({"nextSyncToken": "t2", "items": [event("one", "One moved"), tombstone("two")]}),
        ),
        Some(_) => (410, json!({})),
    });
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));

    assert!(sync(&mut syncer, &mut store, now())[0].is_ok());
    assert_eq!(titles(&store), ["a:One", "a:Two"]);
    let state = store.sync_state("a", "work").unwrap().unwrap();
    assert_eq!(state.sync_token, "t1");
    assert_eq!(state.window_start, now() - PAST);
    assert_eq!(state.window_end, now() + FUTURE);
    let full = &fake.queries(&events_path("work"))[0];
    assert!(
        full.contains("singleEvents=true")
            && full.contains("timeMin=")
            && full.contains("timeMax=")
    );

    assert!(sync(&mut syncer, &mut store, now() + Duration::seconds(60))[0].is_ok());
    assert_eq!(titles(&store), ["a:One moved"]);
    assert_eq!(
        store.sync_state("a", "work").unwrap().unwrap().sync_token,
        "t2"
    );
    let incremental = &fake.queries(&events_path("work"))[1];
    assert!(incremental.contains("syncToken=t1"));
    assert!(!incremental.contains("timeMin") && !incremental.contains("timeMax"));
    assert_eq!(
        fake.queries("/token").len(),
        1,
        "the access token is reused while fresh"
    );
}

#[test]
fn a_410_clears_the_token_and_the_full_resync_replaces_the_calendar() {
    let resync_fails = Arc::new(AtomicBool::new(false));
    let flag = resync_fails.clone();
    let fake = google_fake(&["work"], move |request| match request.param("syncToken") {
        Some("t1") => (
            410,
            json!({"error": {"errors": [{"reason": "fullSyncRequired"}]}}),
        ),
        Some(_) => (500, json!({})),
        None if flag.load(Ordering::Relaxed) => (500, json!({})),
        None => (
            200,
            json!({"nextSyncToken": "t9", "items": [event("fresh", "Fresh")]}),
        ),
    });
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));
    store.replace_calendars("a", &[calendar("work")]).unwrap();
    store
        .store_full_sync(
            "a",
            "work",
            &[change("stale", "Stale")],
            &SyncState {
                sync_token: "t1".into(),
                window_start: now() - PAST,
                window_end: now() + FUTURE,
            },
        )
        .unwrap();

    // The resync fails: the token is cleared, the cached event stays visible, and the error is reported.
    resync_fails.store(true, Ordering::Relaxed);
    let failed = sync(&mut syncer, &mut store, now());
    assert!(matches!(
        failed[0],
        Err(Error::Google(calendar_google::Error::ServerUnavailable))
    ));
    assert_eq!(titles(&store), ["a:Stale"]);
    assert!(store.sync_state("a", "work").unwrap().is_none());

    resync_fails.store(false, Ordering::Relaxed);
    assert!(sync(&mut syncer, &mut store, now())[0].is_ok());
    assert_eq!(titles(&store), ["a:Fresh"]);
    assert_eq!(
        store.sync_state("a", "work").unwrap().unwrap().sync_token,
        "t9"
    );
}

#[test]
fn a_failed_fetch_keeps_the_cache_while_an_empty_increment_changes_nothing() {
    let mode = Arc::new(Mutex::new("full"));
    let current = mode.clone();
    let fake = google_fake(&["work", "home"], move |request| {
        let calendar = request.path.split('/').rev().nth(1).unwrap().to_owned();
        match (*current.lock().unwrap(), calendar.as_str()) {
            ("full", _) => (
                200,
                json!({"nextSyncToken": "t1", "items": [event(&format!("{calendar}-one"), &calendar)]}),
            ),
            ("down", "work") => (503, json!({})),
            _ => (200, json!({"nextSyncToken": "t2", "items": []})),
        }
    });
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));
    sync(&mut syncer, &mut store, now());
    assert_eq!(titles(&store), ["a:home", "a:work"]);

    // One calendar failing is an error for the account, the other still syncs, nothing is cleared.
    *mode.lock().unwrap() = "down";
    let outcome = sync(&mut syncer, &mut store, now() + Duration::seconds(60));
    assert!(matches!(
        outcome[0],
        Err(Error::Google(calendar_google::Error::ServerUnavailable))
    ));
    assert_eq!(titles(&store), ["a:home", "a:work"]);
    assert_eq!(
        store.sync_state("a", "work").unwrap().unwrap().sync_token,
        "t1"
    );
    assert_eq!(
        store.sync_state("a", "home").unwrap().unwrap().sync_token,
        "t2"
    );

    // A successful empty increment is not a failure and does not delete anything either.
    *mode.lock().unwrap() = "empty";
    assert!(sync(&mut syncer, &mut store, now() + Duration::seconds(120))[0].is_ok());
    assert_eq!(titles(&store), ["a:home", "a:work"]);
}

#[test]
fn two_accounts_sharing_a_calendar_stay_separate() {
    let fake = google_fake(&["shared"], |request| {
        let owner = request
            .authorization
            .strip_prefix("Bearer access-refresh-")
            .unwrap()
            .to_owned();
        match request.param("syncToken") {
            None => (
                200,
                json!({"nextSyncToken": "t1", "items": [event("same", &format!("from {owner}"))]}),
            ),
            Some(_) if owner == "b" => (
                200,
                json!({"nextSyncToken": "t2", "items": [tombstone("same")]}),
            ),
            Some(_) => (200, json!({"nextSyncToken": "t2", "items": []})),
        }
    });
    let (mut syncer, mut store) = (syncer(&fake), store(&["a", "b"]));

    sync(&mut syncer, &mut store, now());
    assert_eq!(titles(&store), ["a:from a", "b:from b"]);

    // Account b loses the event; account a keeps its own copy.
    sync(&mut syncer, &mut store, now() + Duration::seconds(60));
    assert_eq!(titles(&store), ["a:from a"]);

    store.remove_account("a").unwrap();
    assert!(titles(&store).is_empty());
    assert!(store.sync_state("b", "shared").unwrap().is_some());
}

#[test]
fn a_window_close_to_its_end_triggers_a_full_sync_with_a_new_window() {
    let fake = google_fake(&["work"], |_| {
        (
            200,
            json!({"nextSyncToken": "next", "items": [event("one", "One")]}),
        )
    });
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));
    sync(&mut syncer, &mut store, now());

    // The window ends 180 days out: 61 days before that an increment is enough, 59 days before it
    // the next sync is full and moves the window.
    sync(&mut syncer, &mut store, now() + Duration::days(119));
    sync(&mut syncer, &mut store, now() + Duration::days(121));
    sync(&mut syncer, &mut store, now() + Duration::days(122));

    let queries = fake.queries(&events_path("work"));
    assert!(queries[1].contains("syncToken"));
    assert!(
        !queries[2].contains("syncToken"),
        "re-bound runs a full sync"
    );
    assert!(
        queries[3].contains("syncToken"),
        "the new window is good for months"
    );
    let state = store.sync_state("a", "work").unwrap().unwrap();
    assert_eq!(state.window_end, now() + Duration::days(121) + FUTURE);
}

#[test]
fn missing_credentials_fail_the_account_without_calling_google_or_touching_the_cache() {
    let fake = google_fake(&["work"], |_| {
        (200, json!({"nextSyncToken": "t", "items": []}))
    });
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));

    let outcomes = syncer.sync_all(&mut store, now(), |_| None).unwrap();

    assert!(matches!(outcomes[0].result, Err(Error::Vault)));
    assert!(fake.requests.lock().unwrap().is_empty());
}

#[test]
fn events_that_ended_before_the_window_are_pruned() {
    let mut store = store(&["a"]);
    store.replace_calendars("a", &[calendar("work")]).unwrap();
    let old = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let mut stale = change("stale", "Stale");
    if let calendar_google::EventChange::Upsert(google) = &mut stale {
        google.event.start = old;
        google.event.end = old + Duration::hours(1);
    }
    store
        .store_incremental_sync("a", "work", &[stale, change("current", "Current")], "t")
        .unwrap();

    store.prune_ended_before(now() - PAST).unwrap();

    assert_eq!(titles(&store), ["a:Current"]);
}

pub(crate) fn calendar(id: &str) -> Calendar {
    Calendar {
        id: id.into(),
        summary: id.into(),
        primary: false,
        access_role: "owner".into(),
        background_color: "#123456".into(),
        time_zone: "Asia/Jakarta".into(),
    }
}

/// An upsert as the mapper would produce it, for seeding the cache directly.
pub(crate) fn change(id: &str, title: &str) -> calendar_google::EventChange {
    let start = Utc.with_ymd_and_hms(2026, 10, 15, 2, 0, 0).unwrap();

    calendar_google::EventChange::Upsert(Box::new(calendar_google::GoogleEvent {
        event: calendar_core::CalendarEvent {
            event_id: format!("google:d29yaw:{id}"),
            calendar_id: "work".into(),
            title: title.into(),
            start,
            end: start + Duration::hours(1),
            timezone: "Asia/Jakarta".into(),
            all_day: false,
            description: String::new(),
            location: String::new(),
            color: 0x123456,
            revision: 0,
            read_only: false,
        },
        etag: format!("\"{title}\""),
        recurring_event_id: None,
        operation_id: None,
    }))
}
