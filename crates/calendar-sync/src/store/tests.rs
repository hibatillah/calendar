//! Cache and journal behaviour on in-memory databases, plus one test-owned file for two connections.

use super::*;
use crate::sync::tests::{calendar, change, now, store};
use calendar_google::EventChange;
use chrono::{Duration, Utc};

fn state() -> SyncState {
    SyncState {
        sync_token: "t".into(),
        window_start: now(),
        window_end: now(),
    }
}

fn operation(operation_id: &str) -> Operation {
    Operation {
        operation_id: operation_id.into(),
        account_id: "a".into(),
        calendar_id: "work".into(),
        event_id: "event".into(),
        kind: OperationKind::Create,
        state: OperationState::Pending,
    }
}

fn count(store: &Store) -> usize {
    let (from, to) = (now() - Duration::days(365), now() + Duration::days(365));

    store.events_between(from, to).unwrap().len()
}

#[test]
fn a_repeated_operation_id_returns_the_stored_entry_instead_of_starting_over() {
    let mut store = store(&["a"]);

    let first = store.begin_operation(&operation("op-1")).unwrap();
    assert_eq!(first.state, OperationState::Pending);
    store
        .finish_operation(&first, OperationState::Applied, None)
        .unwrap();
    let retried = store.begin_operation(&operation("op-1")).unwrap();

    assert_eq!(retried.state, OperationState::Applied);
    assert!(store.pending_operations().unwrap().is_empty());
}

#[test]
fn pending_and_uncertain_writes_need_reconciling_and_disappear_with_the_account() {
    let mut store = store(&["a"]);
    for (operation_id, state) in [
        ("pending", None),
        ("uncertain", Some(OperationState::Uncertain)),
        ("applied", Some(OperationState::Applied)),
        ("failed", Some(OperationState::Failed)),
    ] {
        let begun = store.begin_operation(&operation(operation_id)).unwrap();
        if let Some(state) = state {
            store.finish_operation(&begun, state, None).unwrap();
        }
    }

    let unfinished: Vec<_> = store
        .pending_operations()
        .unwrap()
        .into_iter()
        .map(|operation| operation.operation_id)
        .collect();
    assert_eq!(unfinished, ["pending", "uncertain"]);

    store.remove_account("a").unwrap();
    assert!(store.pending_operations().unwrap().is_empty());
}

#[test]
fn finished_operations_are_pruned_by_age_but_unknown_outcomes_never_are() {
    let mut store = store(&["a"]);
    for (operation_id, state) in [
        ("applied", OperationState::Applied),
        ("failed", OperationState::Failed),
        ("uncertain", OperationState::Uncertain),
    ] {
        let begun = store.begin_operation(&operation(operation_id)).unwrap();
        store.finish_operation(&begun, state, None).unwrap();
    }

    store
        .prune_operations_before(chrono::DateTime::UNIX_EPOCH)
        .unwrap();
    assert!(store.operation("applied").unwrap().is_some());

    store
        .prune_operations_before(Utc::now() + Duration::days(1))
        .unwrap();
    assert!(store.operation("applied").unwrap().is_none());
    assert!(store.operation("failed").unwrap().is_none());
    assert!(store.operation("uncertain").unwrap().is_some());
}

#[test]
fn finishing_an_operation_updates_the_cached_event_in_the_same_transaction() {
    let mut store = store(&["a"]);
    store.replace_calendars("a", &[calendar("work")]).unwrap();
    let begun = store.begin_operation(&operation("op-1")).unwrap();

    store
        .finish_operation(&begun, OperationState::Applied, Some(&change("one", "One")))
        .unwrap();

    let cached = store
        .event("a", "work", "google:d29yaw:one")
        .unwrap()
        .unwrap();
    assert_eq!(cached.google.event.title, "One");
    assert_eq!(
        store.operation("op-1").unwrap().unwrap().state,
        OperationState::Applied
    );
}

#[test]
fn a_series_id_is_recognised_by_its_cached_instances() {
    let mut store = store(&["a"]);
    store.replace_calendars("a", &[calendar("work")]).unwrap();
    let mut instance = change("series_20261015T020000Z", "Standup");
    if let EventChange::Upsert(google) = &mut instance {
        google.recurring_event_id = Some("series".into());
    }
    store
        .store_incremental_sync("a", "work", &[instance], "t1")
        .unwrap();

    assert!(store.has_instances_of("a", "work", "series").unwrap());
    assert!(!store.has_instances_of("a", "work", "other").unwrap());
    assert!(!store.has_instances_of("b", "work", "series").unwrap());
}

#[test]
fn a_series_tombstone_removes_its_instances_and_nothing_else() {
    let mut store = store(&["a"]);
    store.replace_calendars("a", &[calendar("work")]).unwrap();
    let mut instance = change("series_20261015T020000Z", "Standup");
    if let EventChange::Upsert(google) = &mut instance {
        google.recurring_event_id = Some("series".into());
    }
    store
        .store_incremental_sync("a", "work", &[instance, change("other", "Other")], "t1")
        .unwrap();
    assert_eq!(count(&store), 2);

    let tombstone = EventChange::Deleted {
        event_id: "google:d29yaw:series".into(),
    };
    store
        .store_incremental_sync("a", "work", &[tombstone], "t2")
        .unwrap();

    let remaining = store
        .events_between(now() - Duration::days(365), now() + Duration::days(365))
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].google.event.title, "Other");
}

#[test]
fn a_calendar_missing_from_the_list_takes_its_events_with_it() {
    let mut store = store(&["a"]);
    store
        .replace_calendars("a", &[calendar("work"), calendar("home")])
        .unwrap();
    store
        .store_full_sync("a", "work", &[change("one", "One")], &state())
        .unwrap();

    store.replace_calendars("a", &[calendar("home")]).unwrap();

    assert_eq!(count(&store), 0);
    assert_eq!(store.calendars("a").unwrap().len(), 1);
    assert!(store.sync_state("a", "work").unwrap().is_none());
}

#[test]
fn events_keep_their_etag_and_operation_marker_through_the_cache() {
    let mut store = store(&["a"]);
    store.replace_calendars("a", &[calendar("work")]).unwrap();
    let mut marked = change("one", "One");
    if let EventChange::Upsert(google) = &mut marked {
        google.operation_id = Some("op-9".into());
    }

    store
        .store_full_sync("a", "work", &[marked], &state())
        .unwrap();

    let cached = store
        .events_between(now() - Duration::days(365), now() + Duration::days(365))
        .unwrap();
    assert_eq!(cached[0].google.etag, "\"One\"");
    assert_eq!(cached[0].google.operation_id.as_deref(), Some("op-9"));
}

#[test]
fn two_connections_share_one_file_in_wal_mode() {
    let directory =
        std::env::temp_dir().join(format!("calendar-sync-test-{}", uuid::Uuid::new_v4()));
    let writer = Store::open(&directory).unwrap();
    let reader = Store::open(&directory).unwrap();

    writer
        .upsert_account(&calendar_google::Account {
            account_id: "a".into(),
            email: "a@example.com".into(),
            name: "A".into(),
            picture: String::new(),
        })
        .unwrap();

    assert_eq!(reader.accounts().unwrap().len(), 1);
    let mode: String = reader
        .connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    drop((writer, reader));
    std::fs::remove_dir_all(directory).unwrap();
}
