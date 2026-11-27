//! Event writes as the views see them: placement while saving and how each outcome ends.
//!
//! The snapshot only ever holds what Google confirmed. `events()` lays the unfinished edits on
//! top of it at render time, so a background snapshot can never overwrite a pending edit, and
//! restoring the previous position after a failure means dropping the edit.

use super::*;
use crate::service::{
    WriteResult,
    snapshot::{split_calendar_key, split_event_key},
};
use calendar_google::{Calendar, created_event_id};
use calendar_sync::{Change, WriteError, WriteRequest};
use std::borrow::Cow;

pub(super) struct InFlight {
    pub operation_id: String,
    pub event_key: String,
    /// Where the event is shown while saving; `None` for a delete.
    pub placement: Option<CalendarEvent>,
    pub expected_revision: i64,
    /// The outcome is not known: keep the placement until the pending marker leaves the snapshot.
    pub unknown: bool,
}

/// What the user asked for. Event ids are view keys; a create carries a fresh draft UUID.
pub(super) enum Action {
    Create(CalendarEvent),
    Update(CalendarEvent),
    Delete(CalendarEvent),
}

fn split_owned(event_key: &str) -> Result<(String, String, String), String> {
    split_event_key(event_key)
        .map(|(account_id, calendar_id, event_id)| {
            (
                account_id.to_owned(),
                calendar_id.to_owned(),
                event_id.to_owned(),
            )
        })
        .ok_or_else(|| "Unknown event.".to_owned())
}

impl CalendarApp {
    /// The cached events with unfinished edits, the drag preview and the open draft laid on top.
    pub(super) fn events(&self) -> Cow<'_, [CalendarEvent]> {
        let preview = self
            .gesture
            .as_ref()
            .and_then(|gesture| gesture.preview.as_ref());
        let draft = self
            .editor
            .as_ref()
            .filter(|editor| editor.create && editor.in_flight.is_none())
            .map(|editor| &editor.base);
        if self.writes.is_empty() && preview.is_none() && draft.is_none() {
            return Cow::Borrowed(&self.snapshot.events);
        }

        let mut events = self.snapshot.events.clone();
        for write in &self.writes {
            events.retain(|event| event.event_id != write.event_key);
            events.extend(write.placement.clone());
        }
        if let Some(preview) = preview {
            events.retain(|event| event.event_id != preview.event_id);
            events.push(preview.clone());
        }
        events.extend(draft.cloned());

        Cow::Owned(events)
    }

    pub(super) fn is_saving(&self, event_key: &str) -> bool {
        self.writes.iter().any(|write| write.event_key == event_key)
    }

    /// Accounts that granted write access and calendars whose role allows it, in sidebar order.
    pub(super) fn writable_calendars(&self) -> impl Iterator<Item = &CalendarView> {
        self.snapshot
            .accounts
            .iter()
            .filter(|account| {
                account
                    .permissions
                    .as_ref()
                    .is_some_and(|permissions| permissions.write_events)
            })
            .flat_map(|account| &account.calendars)
            .filter(|calendar| calendar.writable)
    }

    pub(super) fn can_write(&self, event: &CalendarEvent) -> bool {
        !event.read_only
            && self
                .writable_calendars()
                .any(|calendar| calendar.key == event.calendar_id)
    }

    /// Queues a write and shows its placement. Returns the operation id, or why nothing was sent.
    pub(super) fn submit(
        &mut self,
        action: Action,
        context: &mut Context<Self>,
    ) -> Result<String, String> {
        let (event, is_create) = match &action {
            Action::Create(event) => (event, true),
            Action::Update(event) | Action::Delete(event) => (event, false),
        };
        if is_create {
            if !self
                .writable_calendars()
                .any(|calendar| calendar.key == event.calendar_id)
            {
                return Err(WriteError::ReadOnly.to_string());
            }
        } else if !self.can_write(event) {
            return Err(WriteError::ReadOnly.to_string());
        }
        if !matches!(action, Action::Delete(_)) {
            event.validate()?;
        }

        let operation_id = uuid::Uuid::new_v4().to_string();
        let (account_id, calendar_id, change, event_key, placement) = match action {
            Action::Create(event) => {
                let (account_id, calendar_id) = split_calendar_key(&event.calendar_id)
                    .map(|(account_id, calendar_id)| {
                        (account_id.to_owned(), calendar_id.to_owned())
                    })
                    .ok_or("Unknown calendar.")?;
                let calendar = Calendar {
                    id: calendar_id.clone(),
                    summary: String::new(),
                    primary: false,
                    access_role: String::new(),
                    background_color: String::new(),
                    time_zone: String::new(),
                };
                let created = created_event_id(&calendar, &event.event_id)
                    .map_err(|error| error.to_string())?;
                let event_key = format!("{}/{created}", event.calendar_id);
                let request = CalendarEvent {
                    calendar_id: calendar_id.clone(),
                    ..event.clone()
                };
                let placement = CalendarEvent {
                    event_id: event_key.clone(),
                    ..event
                };
                (
                    account_id,
                    calendar_id,
                    Change::Create(request),
                    event_key,
                    Some(placement),
                )
            }
            Action::Update(event) | Action::Delete(event) if event.revision < 0 => {
                return Err(format!("{} has not been saved yet.", event.title));
            }
            Action::Update(event) => {
                let (account_id, calendar_id, raw_id) = split_owned(&event.event_id)?;
                let request = CalendarEvent {
                    event_id: raw_id,
                    calendar_id: calendar_id.clone(),
                    ..event.clone()
                };
                (
                    account_id,
                    calendar_id,
                    Change::Update {
                        expected_revision: event.revision,
                        event: request,
                    },
                    event.event_id.clone(),
                    Some(event),
                )
            }
            Action::Delete(event) => {
                let (account_id, calendar_id, raw_id) = split_owned(&event.event_id)?;
                (
                    account_id,
                    calendar_id,
                    Change::Delete {
                        event_id: raw_id,
                        expected_revision: event.revision,
                    },
                    event.event_id.clone(),
                    None,
                )
            }
        };
        if self.is_saving(&event_key) {
            return Err("Wait for the previous change to this event to finish.".into());
        }

        let expected_revision = match &change {
            Change::Update {
                expected_revision, ..
            }
            | Change::Delete {
                expected_revision, ..
            } => *expected_revision,
            Change::Create(_) => 0,
        };
        self.handle.write(WriteRequest {
            account_id,
            calendar_id,
            operation_id: operation_id.clone(),
            change,
        });
        self.writes.push(InFlight {
            operation_id: operation_id.clone(),
            event_key,
            placement,
            expected_revision,
            unknown: false,
        });
        self.write_notice = None;
        context.notify();

        Ok(operation_id)
    }

    pub(super) fn finish_write(&mut self, result: WriteResult, context: &mut Context<Self>) {
        let Some(index) = self
            .writes
            .iter()
            .position(|write| write.operation_id == result.operation_id)
        else {
            return;
        };
        let operation_id = result.operation_id;
        match result.outcome {
            Ok(_) => {
                self.writes.remove(index);
                if self.editing(&operation_id) {
                    self.close_editor();
                }
            }
            Err(WriteError::Unknown) => {
                self.writes[index].unknown = true;
                self.report(&operation_id, WriteError::Unknown.to_string());
            }
            Err(error) => {
                let write = self.writes.remove(index);
                let mut reason = error.to_string();
                if matches!(error, WriteError::Stale)
                    && let Some(latest) = self
                        .snapshot
                        .events
                        .iter()
                        .find(|event| event.event_id == write.event_key)
                        .cloned()
                {
                    reason = format!("{reason} Latest: {}.", self.describe(&latest));
                    if let Some(editor) = self.editor.as_mut().filter(|editor| !editor.create) {
                        editor.base = latest.clone();
                    }
                    self.selected = Some(latest);
                }
                self.report(&operation_id, reason);
            }
        }
        self.settle_writes();
        context.notify();
    }

    /// Ends unknown writes whose pending marker is gone: either the sync confirmed them or they
    /// did not reach Google. Run after every snapshot.
    pub(super) fn settle_writes(&mut self) {
        let snapshot = self.snapshot.clone();
        let mut settled = Vec::new();
        self.writes.retain(|write| {
            let waiting = write.unknown
                && snapshot
                    .pending
                    .iter()
                    .any(|pending| pending.operation_id == write.operation_id);
            if write.unknown && !waiting {
                let current = snapshot
                    .events
                    .iter()
                    .find(|event| event.event_id == write.event_key);
                let saved = match (&write.placement, current) {
                    (None, current) => current.is_none(),
                    (Some(_), None) => false,
                    (Some(_), Some(current)) => current.revision != write.expected_revision,
                };
                settled.push((write.operation_id.clone(), saved));
            }

            !write.unknown || waiting
        });
        for (operation_id, saved) in settled {
            if saved && self.editing(&operation_id) {
                self.close_editor();
            } else if !saved {
                self.report(
                    &operation_id,
                    "Google did not save the change. You can try again.".into(),
                );
            }
        }
    }

    /// Shows why a write did not finish: in its open editor, otherwise as a notice.
    fn report(&mut self, operation_id: &str, reason: String) {
        if self.editing(operation_id) {
            if let Some(editor) = &mut self.editor {
                editor.status = Some(reason);
                if !self
                    .writes
                    .iter()
                    .any(|write| write.operation_id == operation_id)
                {
                    editor.in_flight = None;
                }
            }
        } else {
            self.write_notice = Some(reason);
        }
    }

    fn editing(&self, operation_id: &str) -> bool {
        self.editor
            .as_ref()
            .is_some_and(|editor| editor.in_flight.as_deref() == Some(operation_id))
    }

    fn describe(&self, event: &CalendarEvent) -> String {
        let start = event.start.with_timezone(&self.timezone);
        format!("{}, {}", event.title, start.format("%a %b %-d %H:%M"))
    }
}
