//! The display zone: all-day rows are rebuilt when the system zone changes, and only then.

use super::*;
use chrono_tz::{America::New_York, Pacific::Auckland};

fn holiday() -> Value {
    json!({"id": "holiday", "etag": "\"h\"", "summary": "Holiday",
        "start": {"date": "2026-10-15"}, "end": {"date": "2026-10-16"}})
}

/// Full syncs return the holiday; increments return no changes.
fn fake() -> Fake {
    google_fake(&["work"], |request| match request.param("syncToken") {
        None => (200, json!({"nextSyncToken": "full", "items": [holiday()]})),
        Some(_) => (200, json!({"nextSyncToken": "increment", "items": []})),
    })
}

/// The cached holiday as `start..end` in UTC.
fn holiday_span(store: &Store) -> String {
    let events = store
        .events_between(now() - Duration::days(30), now() + Duration::days(60))
        .unwrap();
    let event = &events[0].google.event;
    assert!(event.all_day);

    format!("{}..{}", event.start.to_rfc3339(), event.end.to_rfc3339())
}

fn syncs_with_token(fake: &Fake) -> Vec<bool> {
    fake.queries(&events_path("work"))
        .iter()
        .map(|query| query.contains("syncToken="))
        .collect()
}

#[test]
fn a_zone_change_forces_a_full_resync_that_rewrites_all_day_rows_and_the_same_zone_does_not() {
    let fake = fake();
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));

    assert!(sync(&mut syncer, &mut store, now())[0].is_ok());
    assert_eq!(
        holiday_span(&store),
        "2026-10-14T17:00:00+00:00..2026-10-15T17:00:00+00:00"
    );

    assert!(sync(&mut syncer, &mut store, now() + Duration::seconds(60))[0].is_ok());
    assert_eq!(
        syncs_with_token(&fake),
        [false, true],
        "same zone: an increment"
    );

    // The system zone changes while the process runs.
    syncer.zone_source = || Some(New_York);
    assert!(sync(&mut syncer, &mut store, now() + Duration::seconds(120))[0].is_ok());
    assert_eq!(
        syncs_with_token(&fake),
        [false, true, false],
        "new zone: a full sync"
    );
    assert_eq!(
        holiday_span(&store),
        "2026-10-15T04:00:00+00:00..2026-10-16T04:00:00+00:00",
        "the end date stays exclusive"
    );
    assert_eq!(store.zone().unwrap(), Some(New_York));
    assert_eq!(
        store.sync_state("a", "work").unwrap().unwrap().sync_token,
        "full"
    );

    assert!(sync(&mut syncer, &mut store, now() + Duration::seconds(180))[0].is_ok());
    assert!(
        syncs_with_token(&fake)[3],
        "the new zone is settled: increments resume"
    );
}

#[test]
fn an_unresolvable_zone_keeps_the_recorded_zone_and_incremental_sync() {
    let fake = fake();
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));
    syncer.zone_source = || Some(Auckland);
    assert!(sync(&mut syncer, &mut store, now())[0].is_ok());
    let span = holiday_span(&store);

    syncer.zone_source = || None;
    assert!(sync(&mut syncer, &mut store, now() + Duration::seconds(60))[0].is_ok());

    assert_eq!(syncs_with_token(&fake), [false, true]);
    assert_eq!(store.zone().unwrap(), Some(Auckland));
    assert_eq!(holiday_span(&store), span);
}

#[test]
fn with_no_zone_at_all_rows_are_built_in_utc_and_nothing_is_recorded() {
    let fake = fake();
    let (mut syncer, mut store) = (syncer(&fake), store(&["a"]));
    syncer.zone_source = || None;

    assert!(sync(&mut syncer, &mut store, now())[0].is_ok());

    assert_eq!(store.zone().unwrap(), None);
    assert_eq!(
        holiday_span(&store),
        "2026-10-15T00:00:00+00:00..2026-10-16T00:00:00+00:00"
    );
}
