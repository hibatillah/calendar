use super::*;
use crate::SyncState;
use calendar_google::{Account, Calendar};
use chrono::{TimeZone, Utc};
use chrono_tz::{America::New_York, Asia::Jakarta, Pacific::Auckland};

/// Two accounts, three calendars, every one with a sync token.
fn synced_store() -> Store {
    let mut store = Store::in_memory();
    for (account_id, calendar_ids) in [
        ("a", ["work", "home"].as_slice()),
        ("b", ["work"].as_slice()),
    ] {
        store
            .upsert_account(&Account {
                account_id: account_id.into(),
                email: format!("{account_id}@example.com"),
                name: account_id.into(),
                picture: String::new(),
            })
            .unwrap();
        let calendars: Vec<_> = calendar_ids
            .iter()
            .map(|id| Calendar {
                id: (*id).into(),
                summary: (*id).into(),
                primary: false,
                access_role: "owner".into(),
                background_color: "#123456".into(),
                time_zone: "Asia/Jakarta".into(),
            })
            .collect();
        store.replace_calendars(account_id, &calendars).unwrap();
    }
    restore_tokens(&mut store);

    store
}

fn restore_tokens(store: &mut Store) {
    let start = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
    let state = SyncState {
        sync_token: "token".into(),
        window_start: start,
        window_end: start + chrono::Duration::days(210),
    };
    for (account_id, calendar_id) in [("a", "work"), ("a", "home"), ("b", "work")] {
        store
            .store_full_sync(account_id, calendar_id, &[], &state)
            .unwrap();
    }
}

fn token_count(store: &Store) -> usize {
    [("a", "work"), ("a", "home"), ("b", "work")]
        .into_iter()
        .filter(|(account_id, calendar_id)| {
            store.sync_state(account_id, calendar_id).unwrap().is_some()
        })
        .count()
}

#[test]
fn zone_names_that_windows_returns_for_common_zones_parse() {
    // Windows answers with ICU's IANA names, including old aliases that `chrono-tz` must accept.
    for name in [
        "Asia/Jakarta",
        "Asia/Calcutta",
        "Asia/Saigon",
        "Asia/Rangoon",
        "Pacific/Auckland",
        "America/Argentina/Buenos_Aires",
        "Etc/GMT-7",
        "Etc/UTC",
        "UTC",
    ] {
        assert!(name.parse::<Tz>().is_ok(), "{name}");
    }
    for name in ["Etc/Unknown", "Windows Standard Time", ""] {
        assert!(name.parse::<Tz>().is_err(), "{name}");
    }
}

#[cfg(windows)]
#[test]
fn the_windows_system_zone_can_be_read() {
    // A read-only query of this machine's setting; the name differs per machine.
    assert!(system_zone().is_some());
}

#[test]
fn a_changed_zone_drops_every_sync_token_and_is_recorded() {
    let mut store = synced_store();
    assert_eq!(store.zone().unwrap(), None);

    // The first recording also drops tokens: the rows may come from a build with another zone.
    assert_eq!(
        align_cache_zone(&mut store, Some(Jakarta)).unwrap(),
        Jakarta
    );
    assert_eq!(
        (store.zone().unwrap(), token_count(&store)),
        (Some(Jakarta), 0)
    );

    restore_tokens(&mut store);
    assert_eq!(
        align_cache_zone(&mut store, Some(Jakarta)).unwrap(),
        Jakarta
    );
    assert_eq!(token_count(&store), 3, "the same zone keeps every token");

    assert_eq!(
        align_cache_zone(&mut store, Some(New_York)).unwrap(),
        New_York
    );
    assert_eq!(
        (store.zone().unwrap(), token_count(&store)),
        (Some(New_York), 0)
    );
}

#[test]
fn an_unknown_system_zone_keeps_the_recorded_zone_and_every_token() {
    let mut store = synced_store();
    store.record_zone(Auckland).unwrap();
    restore_tokens(&mut store);

    assert_eq!(align_cache_zone(&mut store, None).unwrap(), Auckland);
    assert_eq!(
        (store.zone().unwrap(), token_count(&store)),
        (Some(Auckland), 3)
    );

    let mut fresh = Store::in_memory();
    assert_eq!(align_cache_zone(&mut fresh, None).unwrap(), Tz::UTC);
    assert_eq!(fresh.zone().unwrap(), None, "a guess is never recorded");
}
