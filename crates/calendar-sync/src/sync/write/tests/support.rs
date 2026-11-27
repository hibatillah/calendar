//! A stateful fake Google calendar, and a syncer with a cache wired to it.

use super::super::*;
use crate::sync::tests::{
    calendar,
    fake::{Fake, Request, serve},
    now, refresh_token, store, sync, syncer,
};
use chrono::{TimeZone, Utc};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub(super) const DRAFT_ID: &str = "0b8e1b86-3f23-4c5a-9a3e-1d5a7a6b9c01";
const LIST: &str = "/calendar/v3/users/me/calendarList";

/// What the fake does with a POST, PATCH or DELETE.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Writes {
    Apply,
    /// 400, nothing changes.
    Reject,
    /// 503 before anything changes: the write never happened.
    Drop,
    /// The write happens, the answer is 503: the caller cannot know.
    LoseResponse,
}

pub(super) struct Remote {
    /// `owner/calendar/provider id` to the event as Google stores it.
    pub(super) events: BTreeMap<String, Value>,
    pub(super) version: u32,
    pub(super) writes: Writes,
    /// Reading a single event fails with 503 (the collection still works).
    pub(super) reads_fail: bool,
}

impl Remote {
    pub(super) fn stamp(&mut self, event: &mut Value) {
        self.version += 1;
        event["etag"] = format!("\"v{}\"", self.version).into();
        event["updated"] = format!("2026-10-01T00:{:02}:00Z", self.version).into();
    }

    fn handle(&mut self, request: &Request) -> (u16, Value) {
        let owner = request
            .authorization
            .strip_prefix("Bearer access-refresh-")
            .unwrap();
        let rest = request
            .path
            .strip_prefix("/calendar/v3/calendars/")
            .unwrap();
        let (calendar, id) = rest.split_once("/events").unwrap();
        // Google ids such as `series_2026...` arrive percent-encoded (`_` is `%5F`).
        let id = id.trim_start_matches('/').replace("%5F", "_");
        let (id, prefix) = (id.as_str(), format!("{owner}/{calendar}/"));
        let body: Value = serde_json::from_str(&request.body).unwrap_or(Value::Null);

        if request.method == "GET" {
            return if id.is_empty() {
                let items: Vec<_> = self
                    .events
                    .iter()
                    .filter(|(key, _)| key.starts_with(&prefix))
                    .map(|(_, event)| event.clone())
                    .collect();
                (200, json!({"nextSyncToken": "token", "items": items}))
            } else if self.reads_fail {
                (503, json!({}))
            } else {
                self.events
                    .get(&format!("{prefix}{id}"))
                    .map_or((404, json!({})), |event| (200, event.clone()))
            };
        }
        match self.writes {
            Writes::Reject => return (400, json!({})),
            Writes::Drop => return (503, json!({})),
            Writes::Apply | Writes::LoseResponse => {}
        }

        let key = format!("{prefix}{}", body["id"].as_str().unwrap_or(id));
        let reply = match request.method.as_str() {
            "POST" if self.events.contains_key(&key) => (409, json!({})),
            "POST" => {
                let mut event = body;
                self.stamp(&mut event);
                self.events.insert(key, event.clone());
                (200, event)
            }
            _ if !self.events.contains_key(&key) => (410, json!({})),
            _ if !request.if_match.is_empty()
                && self.events[&key]["etag"] != request.if_match.as_str() =>
            {
                (412, json!({}))
            }
            "PATCH" => {
                let mut event = self.events[&key].clone();
                for (name, value) in body.as_object().unwrap() {
                    event[name] = value.clone();
                }
                self.stamp(&mut event);
                self.events.insert(key, event.clone());
                (200, event)
            }
            _ => {
                self.events.remove(&key);
                (204, Value::Null)
            }
        };

        if self.writes == Writes::LoseResponse {
            (503, json!({}))
        } else {
            reply
        }
    }
}

pub(super) struct Harness {
    pub(super) fake: Fake,
    pub(super) remote: Arc<Mutex<Remote>>,
    pub(super) syncer: Syncer,
    pub(super) store: Store,
}

pub(super) fn harness(account_ids: &[&str], role: &'static str) -> Harness {
    let remote = Arc::new(Mutex::new(Remote {
        events: BTreeMap::new(),
        version: 0,
        writes: Writes::Apply,
        reads_fail: false,
    }));
    let shared = remote.clone();
    let fake = serve(move |request| match request.path.as_str() {
        "/token" => (
            200,
            json!({"access_token": format!("access-{}", request.param("refresh_token").unwrap()), "expires_in": 3600}),
        ),
        LIST => (200, json!({"items": [{"id": "work", "accessRole": role}]})),
        _ => shared.lock().unwrap().handle(request),
    });
    let (syncer, mut store) = (syncer(&fake), store(account_ids));
    for account_id in account_ids {
        let mut work = calendar("work");
        work.access_role = role.into();
        store.replace_calendars(account_id, &[work]).unwrap();
    }

    Harness {
        fake,
        remote,
        syncer,
        store,
    }
}

impl Harness {
    pub(super) fn write(
        &mut self,
        account_id: &str,
        operation_id: &str,
        change: Change,
    ) -> Result<Written, WriteError> {
        let request = WriteRequest {
            account_id: account_id.into(),
            calendar_id: "work".into(),
            operation_id: operation_id.into(),
            change,
        };

        self.syncer.write(&mut self.store, &request, refresh_token)
    }

    pub(super) fn sync(&mut self) -> Vec<Result<(), Error>> {
        sync(&mut self.syncer, &mut self.store, now())
    }

    pub(super) fn set_writes(&self, writes: Writes) {
        self.remote.lock().unwrap().writes = writes;
    }

    pub(super) fn state(&self, operation_id: &str) -> OperationState {
        self.store.operation(operation_id).unwrap().unwrap().state
    }

    pub(super) fn pending(&self) -> Vec<String> {
        let pending = self.store.pending_operations().unwrap();

        pending.into_iter().map(|op| op.operation_id).collect()
    }

    pub(super) fn cached(&self, account_id: &str, event_id: &str) -> crate::CachedEvent {
        self.store
            .event(account_id, "work", event_id)
            .unwrap()
            .unwrap()
    }

    /// Creates the draft and returns its cache id.
    pub(super) fn create(&mut self, account_id: &str, operation_id: &str) -> String {
        self.write(account_id, operation_id, Change::Create(draft("Planning")))
            .unwrap()
            .event_id
    }

    pub(super) fn update(
        &mut self,
        account_id: &str,
        event_id: &str,
        operation_id: &str,
        title: &str,
    ) -> Result<Written, WriteError> {
        let cached = self.cached(account_id, event_id);
        let mut event = cached.google.event.clone();
        event.title = title.into();

        self.write(
            account_id,
            operation_id,
            Change::Update {
                event,
                expected_revision: cached.google.event.revision,
            },
        )
    }

    pub(super) fn delete(
        &mut self,
        account_id: &str,
        event_id: &str,
        operation_id: &str,
    ) -> Result<Written, WriteError> {
        let expected_revision = self.cached(account_id, event_id).google.event.revision;

        self.write(
            account_id,
            operation_id,
            Change::Delete {
                event_id: event_id.into(),
                expected_revision,
            },
        )
    }
}

pub(super) fn draft(title: &str) -> CalendarEvent {
    let start = Utc.with_ymd_and_hms(2026, 10, 15, 2, 0, 0).unwrap();

    CalendarEvent {
        event_id: DRAFT_ID.into(),
        calendar_id: "work".into(),
        title: title.into(),
        start,
        end: start + chrono::Duration::hours(1),
        timezone: "Asia/Jakarta".into(),
        all_day: false,
        description: String::new(),
        location: String::new(),
        color: 0x123456,
        revision: 0,
        read_only: false,
    }
}
