//! Calendar reads and writes against the fake Calendar API.

use super::*;

fn event_json(id: &str) -> Value {
    json!({"id": id, "etag": "\"e\"", "summary": id, "start": {"dateTime": "2026-10-15T09:00:00+07:00"}, "end": {"dateTime": "2026-10-15T10:00:00+07:00"}})
}

#[test]
fn calendar_list_follows_pages_and_rejects_a_repeated_cursor() {
    let fake = serve(|request| match request.query("pageToken").as_deref() {
        None => (
            200,
            json!({"nextPageToken": "two", "items": [{"id": "a", "accessRole": "owner"}]}),
        ),
        Some("two") => (
            200,
            json!({"items": [{"id": "b", "accessRole": "reader", "backgroundColor": "#ffffff"}]}),
        ),
        _ => (404, json!({})),
    });
    let calendars = google(&fake).list_calendars("token").unwrap();
    assert_eq!(
        calendars
            .iter()
            .map(|calendar| calendar.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(calendars[0].writable() && !calendars[1].writable());
    assert_eq!(fake.last().header("authorization"), Some("Bearer token"));

    let looping = serve(|_| (200, json!({"nextPageToken": "same", "items": []})));
    assert_eq!(
        google(&looping).list_calendars("token").err(),
        Some(Error::InvalidResponse)
    );
}

#[test]
fn full_sync_paginates_and_incremental_sync_sends_only_the_token() {
    let fake = serve(|request| {
        match (
            request.query("pageToken").as_deref(),
            request.query("syncToken").as_deref(),
        ) {
            (None, None) => (
                200,
                json!({"nextPageToken": "p2", "items": [event_json("one")]}),
            ),
            (Some("p2"), None) => (
                200,
                json!({"nextSyncToken": "sync-1", "items": [event_json("two")]}),
            ),
            (None, Some("sync-1")) => (
                200,
                json!({"nextSyncToken": "sync-2", "items": [{"id": "one", "status": "cancelled"}]}),
            ),
            (_, Some(_)) => (
                410,
                json!({"error": {"errors": [{"reason": "fullSyncRequired"}]}}),
            ),
            _ => (404, json!({})),
        }
    });
    let google = google(&fake);
    let calendar = test_calendar("cal", "owner");

    let time_min = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
    let time_max = Utc.with_ymd_and_hms(2027, 3, 1, 0, 0, 0).unwrap();
    let full = google
        .sync_events(
            "token",
            &calendar,
            EventQuery::Full {
                time_min: Some(time_min),
                time_max: Some(time_max),
            },
        )
        .ok()
        .unwrap();
    assert_eq!(full.changes.len(), 2);
    assert_eq!(full.next_sync_token, "sync-1");
    let first = fake.requests.lock().unwrap().remove(0);
    assert_eq!(first.query("singleEvents").as_deref(), Some("true"));
    assert_eq!(
        first.query("timeMin").as_deref(),
        Some("2026-09-01T00:00:00Z")
    );
    assert_eq!(
        first.query("timeMax").as_deref(),
        Some("2027-03-01T00:00:00Z")
    );
    assert!(first.query("syncToken").is_none());

    let incremental = google
        .sync_events(
            "token",
            &calendar,
            EventQuery::Incremental {
                sync_token: "sync-1",
            },
        )
        .ok()
        .unwrap();
    assert!(matches!(
        incremental.changes[..],
        [EventChange::Deleted { .. }]
    ));
    assert_eq!(incremental.next_sync_token, "sync-2");
    let sent = fake.last();
    assert!(sent.query("timeMin").is_none() && sent.query("timeMax").is_none());

    assert_eq!(
        google
            .sync_events(
                "token",
                &calendar,
                EventQuery::Incremental {
                    sync_token: "expired"
                }
            )
            .err(),
        Some(Error::Gone)
    );
}

#[test]
fn failed_fetches_are_errors_never_empty_results() {
    let calendar = test_calendar("cal", "owner");
    let first_page_ok_then_error = serve(|request| match request.query("pageToken") {
        None => (
            200,
            json!({"nextPageToken": "p2", "items": [event_json("one")]}),
        ),
        Some(_) => (500, json!({})),
    });
    let google_one = google(&first_page_ok_then_error);
    assert_eq!(
        google_one
            .sync_events(
                "t",
                &calendar,
                EventQuery::Full {
                    time_min: None,
                    time_max: None
                }
            )
            .err(),
        Some(Error::ServerUnavailable)
    );

    let no_token = serve(|_| (200, json!({"items": []})));
    assert_eq!(
        google(&no_token)
            .sync_events(
                "t",
                &calendar,
                EventQuery::Full {
                    time_min: None,
                    time_max: None
                }
            )
            .err(),
        Some(Error::InvalidResponse)
    );

    let unauthorized = serve(|_| (401, json!({})));
    assert_eq!(
        google(&unauthorized).list_calendars("t").err(),
        Some(Error::Unauthorized)
    );

    let dead = serve(|_| (200, json!({})));
    let offline = google(&dead);
    drop(dead);
    assert_eq!(
        offline
            .sync_events(
                "t",
                &calendar,
                EventQuery::Full {
                    time_min: None,
                    time_max: None
                }
            )
            .err(),
        Some(Error::Unreachable)
    );

    let empty = serve(|_| (200, json!({"nextSyncToken": "s", "items": []})));
    assert!(
        google(&empty)
            .sync_events(
                "t",
                &calendar,
                EventQuery::Full {
                    time_min: None,
                    time_max: None
                }
            )
            .ok()
            .unwrap()
            .changes
            .is_empty()
    );
}

// ---- writes ----

fn draft() -> CalendarEvent {
    CalendarEvent {
        event_id: "7f4b0e1c-2d3a-4b5c-8d6e-9f0a1b2c3d4e".into(),
        calendar_id: "cal".into(),
        title: "Draft".into(),
        start: Utc.with_ymd_and_hms(2026, 10, 15, 2, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 10, 15, 3, 0, 0).unwrap(),
        timezone: "Asia/Jakarta".into(),
        all_day: false,
        description: String::new(),
        location: String::new(),
        color: 0,
        revision: -1,
        read_only: false,
    }
}

/// A one-event server with Google's behaviour: duplicate ids conflict, stale `If-Match` fails with 412.
fn event_store() -> Fake {
    let stored: Mutex<Option<Value>> = Mutex::new(None);
    let version = Mutex::new(0);
    serve(move |request| {
        let mut stored = stored.lock().unwrap();
        let mut version = version.lock().unwrap();
        let body: Value = serde_json::from_str(&request.body).unwrap_or(Value::Null);
        match request.method.as_str() {
            "POST" if stored.is_some() => (409, json!({})),
            "POST" => {
                *version += 1;
                let mut event = body;
                event["etag"] = format!("v{version}").into();
                *stored = Some(event.clone());
                (200, event)
            }
            "GET" => stored
                .clone()
                .map_or((404, json!({})), |event| (200, event)),
            "PATCH"
                if request.header("if-match")
                    != stored.as_ref().and_then(|event| event["etag"].as_str()) =>
            {
                (412, json!({}))
            }
            "PATCH" => {
                *version += 1;
                let event = stored.as_mut().unwrap();
                for (key, value) in body.as_object().unwrap() {
                    event[key] = value.clone();
                }
                event["etag"] = format!("v{version}").into();
                (200, event.clone())
            }
            "DELETE" if stored.is_none() => (410, json!({})),
            _ => {
                *stored = None;
                (204, Value::Null)
            }
        }
    })
}

#[test]
fn create_is_retry_safe_and_patch_uses_if_match() {
    let fake = event_store();
    let google = google(&fake);
    let calendar = test_calendar("cal", "owner");

    let created = google
        .create_event("t", &calendar, &draft(), "operation-1")
        .unwrap();
    assert_eq!(created.etag, "v1");
    assert_eq!(created.operation_id.as_deref(), Some("operation-1"));
    assert_eq!(fake.last().query("sendUpdates").as_deref(), Some("all"));
    assert_eq!(
        draft().event_id.replace('-', ""),
        created.event.event_id.rsplit(':').next().unwrap()
    );

    // Unknown outcome, same operation: 409 is reconciled to the event the first attempt created.
    let retried = google
        .create_event("t", &calendar, &draft(), "operation-1")
        .unwrap();
    assert_eq!(retried, created);
    assert_eq!(
        google
            .create_event("t", &calendar, &draft(), "operation-2")
            .err(),
        Some(Error::Conflict)
    );

    let mut edit = created.event.clone();
    edit.title = "Renamed".into();
    let updated = google
        .update_event("t", &calendar, &edit, "v1", "operation-3")
        .unwrap();
    assert_eq!(
        (updated.event.title.as_str(), updated.etag.as_str()),
        ("Renamed", "v2")
    );
    assert!(updated.event.revision >= 0);

    // The patch went out with the ETag the caller read.
    let again = google
        .update_event("t", &calendar, &edit, "v2", "operation-4")
        .unwrap();
    assert_eq!(again.etag, "v3");
    assert_eq!(fake.last().header("if-match"), Some("v2"));
}

#[test]
fn stale_etag_fails_but_a_repeated_operation_is_recognised() {
    let fake = event_store();
    let google = google(&fake);
    let calendar = test_calendar("cal", "owner");
    let created = google
        .create_event("t", &calendar, &draft(), "operation-1")
        .unwrap();
    let mut edit = created.event.clone();
    edit.title = "Renamed".into();
    let updated = google
        .update_event("t", &calendar, &edit, "v1", "operation-2")
        .unwrap();

    // Replaying the applied write with the old ETag returns the stored result instead of failing.
    assert_eq!(
        google
            .update_event("t", &calendar, &edit, "v1", "operation-2")
            .unwrap(),
        updated
    );
    // A different operation holding the old ETag is a genuine stale write.
    assert_eq!(
        google
            .update_event("t", &calendar, &edit, "v1", "operation-9")
            .err(),
        Some(Error::PreconditionFailed)
    );
}

#[test]
fn delete_is_conditional_idempotent_and_respects_calendar_roles() {
    let fake = event_store();
    let google = google(&fake);
    let calendar = test_calendar("cal", "owner");
    let created = google
        .create_event("t", &calendar, &draft(), "operation-1")
        .unwrap();

    google
        .delete_event("t", &calendar, &created.event.event_id, Some("v1"))
        .unwrap();
    assert_eq!(fake.last().header("if-match"), Some("v1"));
    google
        .delete_event("t", &calendar, &created.event.event_id, None)
        .unwrap();

    let before = fake.requests.lock().unwrap().len();
    let reader = test_calendar("cal", "reader");
    assert_eq!(
        google.create_event("t", &reader, &draft(), "op").err(),
        Some(Error::ReadOnly)
    );
    assert_eq!(
        google
            .delete_event("t", &reader, &created.event.event_id, None)
            .err(),
        Some(Error::ReadOnly)
    );
    assert_eq!(
        google
            .update_event("t", &calendar, &draft(), "v1", "op")
            .err(),
        Some(Error::InvalidEvent)
    );
    assert_eq!(fake.requests.lock().unwrap().len(), before);
}

#[test]
fn get_event_tells_a_missing_event_from_a_failed_read_and_created_ids_are_known_in_advance() {
    let fake = event_store();
    let google = google(&fake);
    let calendar = test_calendar("cal", "owner");
    let expected = crate::created_event_id(&calendar, &draft().event_id).unwrap();

    assert_eq!(
        google.get_event("t", &calendar, &expected).unwrap(),
        None,
        "a 404 is a missing event, not an error"
    );
    let created = google
        .create_event("t", &calendar, &draft(), "operation-1")
        .unwrap();
    assert_eq!(created.event.event_id, expected);
    assert_eq!(
        google.get_event("t", &calendar, &expected).unwrap(),
        Some(created)
    );

    google
        .delete_event("t", &calendar, &expected, None)
        .unwrap();
    assert_eq!(google.get_event("t", &calendar, &expected).unwrap(), None);
    assert_eq!(
        crate::created_event_id(&calendar, "not-a-uuid"),
        Err(Error::InvalidEvent)
    );

    let down = serve(|_| (503, json!({})));
    assert_eq!(
        super::google(&down)
            .get_event("t", &calendar, &expected)
            .err(),
        Some(Error::ServerUnavailable)
    );
}
