//! Writes against a stateful fake Google calendar and an in-memory cache. Google is never contacted.

use super::*;
use crate::sync::tests::{change, titles};
use serde_json::json;
use support::{DRAFT_ID, Writes, draft, harness};

mod support;

#[test]
fn repeating_an_operation_id_never_sends_a_second_request() {
    let mut harness = harness(&["a"], "owner");

    let created = harness
        .write("a", "op-1", Change::Create(draft("Planning")))
        .unwrap();
    let expected = format!("google:d29yaw:{}", DRAFT_ID.replace('-', ""));
    assert_eq!(created.event_id, expected);
    assert_eq!(titles(&harness.store), ["a:Planning"]);
    assert_eq!(harness.state("op-1"), OperationState::Applied);

    let repeated = harness
        .write("a", "op-1", Change::Create(draft("Planning")))
        .unwrap();

    assert_eq!(repeated, created);
    assert_eq!(harness.fake.count("POST"), 1);
    assert_eq!(titles(&harness.store), ["a:Planning"]);

    // The applied update answers a repeat from the journal instead of calling itself stale.
    harness.update("a", &expected, "op-2", "Renamed").unwrap();
    let cached = harness.cached("a", &expected).google.event;
    let again = harness.write(
        "a",
        "op-2",
        Change::Update {
            event: cached.clone(),
            expected_revision: 0,
        },
    );
    assert!(again.is_ok());
    assert_eq!(harness.fake.count("PATCH"), 1);
}

#[test]
fn a_stale_etag_is_rejected_and_the_cache_shows_the_newer_event() {
    let mut harness = harness(&["a"], "owner");
    let event_id = harness.create("a", "op-1");
    let seen = harness.cached("a", &event_id).google.event.revision;
    {
        // Another client edits the event; this cache has not heard of it yet.
        let mut remote = harness.remote.lock().unwrap();
        let key = format!("a/work/{}", DRAFT_ID.replace('-', ""));
        let mut event = remote.events[&key].clone();
        event["summary"] = "Changed elsewhere".into();
        remote.stamp(&mut event);
        remote.events.insert(key, event);
    }

    let result = harness.update("a", &event_id, "op-2", "My edit");

    assert!(matches!(result, Err(WriteError::Stale)));
    assert_eq!(harness.state("op-2"), OperationState::Failed);
    let cached = harness.cached("a", &event_id);
    assert_eq!(cached.google.event.title, "Changed elsewhere");
    assert_ne!(cached.google.event.revision, seen);
    let remote = harness.remote.lock().unwrap();
    assert_eq!(
        remote.events[&format!("a/work/{}", DRAFT_ID.replace('-', ""))]["summary"],
        "Changed elsewhere",
        "no silent overwrite"
    );
    drop(remote);

    // Editing the version the user saw is stale against the refreshed cache without any request.
    let mut old_view = cached.google.event.clone();
    old_view.title = "Late edit".into();
    let late = harness.write(
        "a",
        "op-3",
        Change::Update {
            event: old_view,
            expected_revision: seen,
        },
    );
    assert!(matches!(late, Err(WriteError::Stale)));
    assert_eq!(harness.fake.count("PATCH"), 1);
}

#[test]
fn an_uncertain_create_is_settled_by_reading_never_replayed() {
    let mut harness = harness(&["a"], "owner");
    harness.set_writes(Writes::LoseResponse);

    let result = harness.write("a", "op-1", Change::Create(draft("Planning")));

    assert!(matches!(result, Err(WriteError::Unknown)));
    assert_eq!(harness.state("op-1"), OperationState::Uncertain);
    assert_eq!(harness.pending(), ["op-1"]);
    assert!(titles(&harness.store).is_empty());

    // The next sync finds the event with this operation's marker: applied.
    harness.set_writes(Writes::Apply);
    assert!(harness.sync()[0].is_ok());

    assert_eq!(harness.state("op-1"), OperationState::Applied);
    assert!(harness.pending().is_empty());
    assert_eq!(titles(&harness.store), ["a:Planning"]);
    assert_eq!(harness.fake.count("POST"), 1);
}

#[test]
fn repeating_an_uncertain_operation_settles_it_without_a_second_write() {
    let mut harness = harness(&["a"], "owner");
    harness.set_writes(Writes::LoseResponse);
    harness
        .write("a", "op-1", Change::Create(draft("Planning")))
        .unwrap_err();
    harness.set_writes(Writes::Apply);

    let repeated = harness.write("a", "op-1", Change::Create(draft("Planning")));

    assert!(repeated.is_ok());
    assert_eq!(harness.fake.count("POST"), 1);
    assert_eq!(titles(&harness.store), ["a:Planning"]);
}

#[test]
fn an_uncertain_create_that_never_reached_google_is_settled_as_failed() {
    let mut harness = harness(&["a"], "owner");
    harness.set_writes(Writes::Drop);
    harness
        .write("a", "op-1", Change::Create(draft("Planning")))
        .unwrap_err();
    harness.set_writes(Writes::Apply);

    assert!(harness.sync()[0].is_ok());

    assert_eq!(harness.state("op-1"), OperationState::Failed);
    assert!(harness.pending().is_empty());
    assert!(titles(&harness.store).is_empty());
    assert_eq!(
        harness.fake.count("POST"),
        1,
        "settling reads, it does not replay"
    );
    let again = harness.write("a", "op-1", Change::Create(draft("Planning")));
    assert!(matches!(again, Err(WriteError::NotApplied)));
    assert_eq!(harness.fake.count("POST"), 1);
}

#[test]
fn uncertain_updates_and_deletes_are_settled_from_the_event_google_holds() {
    let mut harness = harness(&["a"], "owner");
    let event_id = harness.create("a", "op-1");

    harness.set_writes(Writes::LoseResponse);
    harness.update("a", &event_id, "op-2", "Moved").unwrap_err();
    harness.set_writes(Writes::Apply);
    harness.sync();
    assert_eq!(harness.state("op-2"), OperationState::Applied);
    assert_eq!(titles(&harness.store), ["a:Moved"]);

    harness.set_writes(Writes::Drop);
    harness.delete("a", &event_id, "op-3").unwrap_err();
    harness.set_writes(Writes::Apply);
    harness.sync();
    assert_eq!(harness.state("op-3"), OperationState::Failed);
    assert_eq!(
        titles(&harness.store),
        ["a:Moved"],
        "still there: not deleted"
    );

    harness.set_writes(Writes::LoseResponse);
    harness.delete("a", &event_id, "op-4").unwrap_err();
    harness.set_writes(Writes::Apply);
    harness.sync();
    assert_eq!(harness.state("op-4"), OperationState::Applied);
    assert!(titles(&harness.store).is_empty());
}

#[test]
fn a_rejected_write_leaves_the_cached_event_unchanged() {
    let mut harness = harness(&["a"], "owner");
    let event_id = harness.create("a", "op-1");
    let before = harness.cached("a", &event_id);
    harness.set_writes(Writes::Reject);

    let result = harness.update("a", &event_id, "op-2", "Rejected title");

    assert!(matches!(
        result,
        Err(WriteError::Failed(Error::Google(
            calendar_google::Error::Rejected(400)
        )))
    ));
    assert_eq!(harness.state("op-2"), OperationState::Failed);
    assert_eq!(harness.cached("a", &event_id), before);

    let repeated = harness.update("a", &event_id, "op-2", "Rejected title");
    assert!(matches!(repeated, Err(WriteError::NotApplied)));
    assert_eq!(harness.fake.count("PATCH"), 1);
}

#[test]
fn read_only_calendars_and_events_send_no_request() {
    let mut reader = harness(&["a"], "reader");
    let result = reader.write("a", "op-1", Change::Create(draft("Planning")));
    assert!(matches!(result, Err(WriteError::ReadOnly)));
    assert!(reader.fake.requests.lock().unwrap().is_empty());
    assert!(reader.store.operation("op-1").unwrap().is_none());

    // A writable calendar can still hold a read-only event, such as a birthday.
    let mut owner = harness(&["a"], "owner");
    let mut birthday = change("birthday", "Birthday");
    if let EventChange::Upsert(google) = &mut birthday {
        google.event.read_only = true;
    }
    owner
        .store
        .store_incremental_sync("a", "work", &[birthday], "token")
        .unwrap();
    let id = "google:d29yaw:birthday";

    assert!(matches!(
        owner.update("a", id, "op-2", "Renamed"),
        Err(WriteError::ReadOnly)
    ));
    assert!(matches!(
        owner.delete("a", id, "op-3"),
        Err(WriteError::ReadOnly)
    ));
    assert!(owner.fake.requests.lock().unwrap().is_empty());
}

#[test]
fn one_occurrence_can_change_but_a_whole_series_is_refused() {
    let mut harness = harness(&["a"], "owner");
    let instance = "series_20261015T020000Z";
    let mut standup = change(instance, "Standup");
    if let EventChange::Upsert(google) = &mut standup {
        google.recurring_event_id = Some("series".into());
    }
    harness
        .store
        .store_incremental_sync("a", "work", &[standup], "token")
        .unwrap();
    harness.remote.lock().unwrap().events.insert(
        format!("a/work/{instance}"),
        json!({"id": instance, "etag": "\"Standup\"", "summary": "Standup", "recurringEventId": "series",
            "start": {"dateTime": "2026-10-15T09:00:00+07:00"}, "end": {"dateTime": "2026-10-15T10:00:00+07:00"}}),
    );
    let instance_id = format!("google:d29yaw:{instance}");

    harness
        .update("a", &instance_id, "op-1", "Standup moved")
        .unwrap();

    let path = "/calendar/v3/calendars/work/events/series%5F20261015T020000Z";
    assert_eq!(
        harness.fake.queries(path).len(),
        1,
        "only this occurrence was patched"
    );
    assert_eq!(
        harness.cached("a", &instance_id).google.event.title,
        "Standup moved"
    );

    // The series itself never appears in the cache, but its instances name it.
    let mut as_series = harness.cached("a", &instance_id).google.event;
    as_series.event_id = "google:d29yaw:series".into();
    let edit = Change::Update {
        event: as_series,
        expected_revision: 0,
    };
    assert!(matches!(
        harness.write("a", "op-2", edit),
        Err(WriteError::SeriesEdit)
    ));
    let removal = Change::Delete {
        event_id: "google:d29yaw:series".into(),
        expected_revision: 0,
    };
    assert!(matches!(
        harness.write("a", "op-3", removal),
        Err(WriteError::SeriesEdit)
    ));
    assert_eq!(harness.fake.count("PATCH"), 1);
    assert_eq!(harness.fake.count("DELETE"), 0);
}

#[test]
fn two_accounts_with_the_same_calendar_and_draft_stay_isolated() {
    let mut harness = harness(&["a", "b"], "owner");

    let from_a = harness.create("a", "op-a");
    let from_b = harness.create("b", "op-b");
    assert_eq!(from_a, from_b);
    assert_eq!(titles(&harness.store), ["a:Planning", "b:Planning"]);

    harness.delete("a", &from_a, "op-a2").unwrap();

    assert_eq!(titles(&harness.store), ["b:Planning"]);
    assert_eq!(
        harness.store.operation("op-b").unwrap().unwrap().account_id,
        "b"
    );
    let remote = harness.remote.lock().unwrap();
    let keys: Vec<_> = remote.events.keys().map(|key| &key[..1]).collect();
    assert_eq!(keys, ["b"]);
}

#[test]
fn a_background_sync_does_not_drop_the_pending_marker() {
    let mut harness = harness(&["a"], "owner");
    harness.set_writes(Writes::LoseResponse);
    harness
        .write("a", "op-1", Change::Create(draft("Planning")))
        .unwrap_err();
    harness.set_writes(Writes::Apply);

    // Google cannot be asked about the event right now, but the sync itself works.
    harness.remote.lock().unwrap().reads_fail = true;
    assert!(harness.sync()[0].is_ok());

    assert_eq!(
        harness.pending(),
        ["op-1"],
        "still pending: nothing proved the outcome"
    );
    assert_eq!(harness.state("op-1"), OperationState::Uncertain);

    harness.remote.lock().unwrap().reads_fail = false;
    harness.sync();
    assert!(harness.pending().is_empty());
}

#[test]
fn invalid_drafts_unknown_events_and_missing_credentials_fail_without_a_write() {
    let mut harness = harness(&["a"], "owner");

    let untitled = harness.write("a", "op-1", Change::Create(draft(" ")));
    assert!(matches!(untitled, Err(WriteError::Invalid(_))));
    let removal = Change::Delete {
        event_id: "google:d29yaw:nothing".into(),
        expected_revision: 0,
    };
    let missing = harness.write("a", "op-2", removal);
    assert!(matches!(
        missing,
        Err(WriteError::Failed(Error::Google(
            calendar_google::Error::NotFound
        )))
    ));

    let request = WriteRequest {
        account_id: "a".into(),
        calendar_id: "work".into(),
        operation_id: "op-3".into(),
        change: Change::Create(draft("Planning")),
    };
    let result = harness.syncer.write(&mut harness.store, &request, |_| None);
    assert!(matches!(result, Err(WriteError::Failed(Error::Vault))));
    assert_eq!(harness.state("op-3"), OperationState::Failed);
    assert!(harness.fake.requests.lock().unwrap().is_empty());
}

#[test]
fn all_day_writes_use_the_cache_zone_before_any_sync_has_run() {
    use chrono::TimeZone;

    let mut harness = harness(&["a"], "owner");
    harness
        .store
        .record_zone(chrono_tz::Pacific::Auckland)
        .unwrap();
    // A syncer that has not synced yet must not fall back to its default zone.
    harness.syncer.google.set_display_zone(chrono_tz::Tz::UTC);

    let mut holiday = draft("Holiday");
    holiday.all_day = true;
    // Midnight of 2026-10-15 and 2026-10-16 in Auckland (UTC+13).
    holiday.start = chrono::Utc
        .with_ymd_and_hms(2026, 10, 14, 11, 0, 0)
        .unwrap();
    holiday.end = chrono::Utc
        .with_ymd_and_hms(2026, 10, 15, 11, 0, 0)
        .unwrap();
    harness.write("a", "op-1", Change::Create(holiday)).unwrap();

    let remote = harness.remote.lock().unwrap();
    let stored = remote.events.values().next().unwrap();
    assert_eq!(stored["start"]["date"], "2026-10-15");
    assert_eq!(stored["end"]["date"], "2026-10-16");
}
