//! Event writes: create, update (which includes move and resize) and delete.
//!
//! Every write is journaled under a caller-chosen `operation_id` before it is sent, so repeating
//! the id never sends a second request: an applied or failed operation answers from the journal, and
//! a pending or uncertain one is settled by reading the event back, never by replaying the write.
//! The journal stores no payload, so an unknown outcome is settled by looking at Google's state:
//! the `calendarOperation` marker that `calendar-google` writes into every event says whether this
//! operation got through.
//!
//! Scope: single events, and one occurrence of a recurring series (an instance id is patched or
//! deleted on its own). Whole-series edits are rejected with [`WriteError::SeriesEdit`].

use super::Syncer;
use crate::{Error, Operation, OperationKind, OperationState, Store};
use calendar_core::CalendarEvent;
use calendar_google::{Calendar, EventChange, GoogleEvent, created_event_id};
use chrono_tz::Tz;
use std::fmt;

pub struct WriteRequest {
    pub account_id: String,
    pub calendar_id: String,
    /// Chosen once per user action and reused for every retry of it. A new attempt after a
    /// failure is a new action: a new `operation_id` (and a new draft id for a create).
    pub operation_id: String,
    pub change: Change,
}

pub enum Change {
    /// `event.event_id` must be a fresh UUID: it fixes the provider id the event gets.
    Create(CalendarEvent),
    /// `event` is the wanted state (new title, or new start and end for a move or resize);
    /// `event.event_id` is the cached id. `expected_revision` is the `revision` the user was
    /// looking at: if the cache moved on since, the write is stale and nothing is sent.
    Update {
        event: CalendarEvent,
        expected_revision: i64,
    },
    Delete {
        event_id: String,
        expected_revision: i64,
    },
}

/// A confirmed write. For a create, `event_id` is the id the event has in the cache.
#[derive(Debug, PartialEq, Eq)]
pub struct Written {
    pub event_id: String,
}

#[derive(Debug)]
pub enum WriteError {
    /// The calendar role or the event type forbids it. Nothing was sent or journaled.
    ReadOnly,
    /// A whole recurring series; only single events and single occurrences are supported.
    SeriesEdit,
    /// The event changed in Google since the user read it. The cache holds the newer version.
    Stale,
    /// The event failed validation. Nothing was sent or journaled.
    Invalid(String),
    /// Google did not apply it (rejected, or never received it). Retry from the form.
    NotApplied,
    /// The request may or may not have reached Google. The operation stays pending and the next
    /// sync (or a repeat of the same `operation_id`) settles it. Do not offer a plain retry.
    Unknown,
    Failed(Error),
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnly => formatter.write_str("This calendar or event is read-only."),
            Self::SeriesEdit => formatter.write_str(
                "Editing a whole recurring series is not supported. Edit a single occurrence.",
            ),
            Self::Stale => formatter.write_str(
                "The event changed in Google. The latest version is shown; review it and save again.",
            ),
            Self::Invalid(message) => formatter.write_str(message),
            Self::NotApplied => formatter.write_str("Google did not apply the change."),
            Self::Unknown => formatter.write_str(
                "Could not confirm that Google saved the change. It stays pending and is checked on the next sync.",
            ),
            Self::Failed(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WriteError {}

impl From<Error> for WriteError {
    fn from(error: Error) -> Self {
        Self::Failed(error)
    }
}

/// What a request resolved to before anything is sent.
struct Plan {
    calendar: Calendar,
    kind: OperationKind,
    event_id: String,
    etag: String,
}

impl Syncer {
    /// Runs one write to its outcome. `refresh_token` reads the account's token from the vault.
    pub fn write(
        &mut self,
        store: &mut Store,
        request: &WriteRequest,
        refresh_token: impl Fn(&str) -> Option<String>,
    ) -> Result<Written, WriteError> {
        let result = self.write_inner(store, request, &refresh_token);
        tracing::info!(
            action = "event_write",
            operation_id = %request.operation_id,
            account_id = %request.account_id,
            outcome = result.as_ref().map_or_else(|error| error.to_string(), |_| "applied".into())
        );

        result
    }

    fn write_inner(
        &mut self,
        store: &mut Store,
        request: &WriteRequest,
        refresh_token: &impl Fn(&str) -> Option<String>,
    ) -> Result<Written, WriteError> {
        // Writes use the zone the cached rows were built in, not a newer system zone that no sync has applied yet.
        self.google
            .set_display_zone(store.zone()?.unwrap_or(Tz::UTC));
        // A repeated operation id answers from the journal; it never reaches the checks below,
        // which would call an already applied update stale.
        if let Some(existing) = store.operation(&request.operation_id)? {
            return self.resume(store, &existing, refresh_token);
        }

        let plan = plan(store, request)?;
        let operation = store.begin_operation(&Operation {
            operation_id: request.operation_id.clone(),
            account_id: request.account_id.clone(),
            calendar_id: request.calendar_id.clone(),
            event_id: plan.event_id.clone(),
            kind: plan.kind,
            state: OperationState::Pending,
        })?;

        // Nothing has been sent yet, so failing here is a certain failure.
        let access_token = match self.access_token(&request.account_id, refresh_token) {
            Ok(token) => token,
            Err(error) => {
                store.finish_operation(&operation, OperationState::Failed, None)?;
                return Err(error.into());
            }
        };
        let sent = match &request.change {
            Change::Create(event) => self
                .google
                .create_event(
                    &access_token,
                    &plan.calendar,
                    event,
                    &operation.operation_id,
                )
                .map(|created| EventChange::Upsert(Box::new(created))),
            Change::Update { event, .. } => self
                .google
                .update_event(
                    &access_token,
                    &plan.calendar,
                    event,
                    &plan.etag,
                    &operation.operation_id,
                )
                .map(|updated| EventChange::Upsert(Box::new(updated))),
            Change::Delete { .. } => self
                .google
                .delete_event(
                    &access_token,
                    &plan.calendar,
                    &plan.event_id,
                    Some(&plan.etag),
                )
                .map(|()| EventChange::Deleted {
                    event_id: plan.event_id.clone(),
                }),
        };

        match sent {
            Ok(change) => {
                store.finish_operation(&operation, OperationState::Applied, Some(&change))?;

                Ok(Written {
                    event_id: operation.event_id,
                })
            }
            Err(calendar_google::Error::PreconditionFailed) => {
                // Load the newer version so the user sees what they would have overwritten.
                let current = self
                    .google
                    .get_event(&access_token, &plan.calendar, &operation.event_id)
                    .ok()
                    .map(|current| cache_change(current, &operation.event_id));
                store.finish_operation(&operation, OperationState::Failed, current.as_ref())?;

                Err(WriteError::Stale)
            }
            Err(
                calendar_google::Error::Unreachable
                | calendar_google::Error::ServerUnavailable
                | calendar_google::Error::InvalidResponse,
            ) => {
                store.finish_operation(&operation, OperationState::Uncertain, None)?;

                Err(WriteError::Unknown)
            }
            Err(error) => {
                if error == calendar_google::Error::Unauthorized {
                    self.access_tokens.remove(&request.account_id);
                }
                store.finish_operation(&operation, OperationState::Failed, None)?;

                Err(Error::Google(error).into())
            }
        }
    }

    /// A repeated `operation_id`: answer from the journal, and settle what is still open.
    fn resume(
        &mut self,
        store: &mut Store,
        operation: &Operation,
        refresh_token: &impl Fn(&str) -> Option<String>,
    ) -> Result<Written, WriteError> {
        let state = match operation.state {
            OperationState::Pending | OperationState::Uncertain => self
                .settle(store, operation, refresh_token)
                .map_err(|error| match error {
                    Error::Storage(_) => WriteError::Failed(error),
                    _ => WriteError::Unknown,
                })?,
            settled => settled,
        };

        match state {
            OperationState::Applied => Ok(Written {
                event_id: operation.event_id.clone(),
            }),
            _ => Err(WriteError::NotApplied),
        }
    }

    /// Settles every open write of `account_id` before its sync, so the sync starts from a journal
    /// without leftovers. A write that cannot be settled now stays open for the next sync.
    pub(super) fn settle_unfinished(
        &mut self,
        store: &mut Store,
        account_id: &str,
        refresh_token: &impl Fn(&str) -> Option<String>,
    ) -> Result<(), Error> {
        for operation in store
            .pending_operations()?
            .iter()
            .filter(|operation| operation.account_id == account_id)
        {
            let outcome = self.settle(store, operation, refresh_token);
            tracing::info!(
                action = "write_settled",
                operation_id = %operation.operation_id,
                outcome = outcome.as_ref().map_or_else(|error| error.to_string(), |state| format!("{state:?}"))
            );
        }

        Ok(())
    }

    /// Reads the event back and records whether the operation got through. Never sends the write
    /// again. `Err` means Google could not be asked; the operation is left as it was.
    fn settle(
        &mut self,
        store: &mut Store,
        operation: &Operation,
        refresh_token: &impl Fn(&str) -> Option<String>,
    ) -> Result<OperationState, Error> {
        let calendar = store
            .calendars(&operation.account_id)?
            .into_iter()
            .find(|calendar| calendar.id == operation.calendar_id);
        let Some(calendar) = calendar else {
            // The calendar is gone, and so is whatever the write targeted.
            store.finish_operation(operation, OperationState::Failed, None)?;
            return Ok(OperationState::Failed);
        };

        let access_token = self.access_token(&operation.account_id, refresh_token)?;
        let current = self
            .google
            .get_event(&access_token, &calendar, &operation.event_id)?;
        let applied = match (operation.kind, &current) {
            (OperationKind::Delete, found) => found.is_none(),
            (_, Some(event)) => event.operation_id.as_deref() == Some(&operation.operation_id),
            (_, None) => false,
        };
        let cache = cache_change(current, &operation.event_id);
        let state = if applied {
            OperationState::Applied
        } else {
            OperationState::Failed
        };
        store.finish_operation(operation, state, Some(&cache))?;

        Ok(state)
    }
}

/// A read result in the form the cache stores: Google no longer having the event deletes it.
fn cache_change(current: Option<GoogleEvent>, event_id: &str) -> EventChange {
    current.map_or_else(
        || EventChange::Deleted {
            event_id: event_id.to_owned(),
        },
        |event| EventChange::Upsert(Box::new(event)),
    )
}

/// Checks that need no network: role, event type, scope, staleness against the cache.
fn plan(store: &Store, request: &WriteRequest) -> Result<Plan, WriteError> {
    let calendar = store
        .calendars(&request.account_id)?
        .into_iter()
        .find(|calendar| calendar.id == request.calendar_id)
        .ok_or(Error::Google(calendar_google::Error::NotFound))?;
    if !calendar.writable() {
        return Err(WriteError::ReadOnly);
    }

    let (kind, event_id, etag) = match &request.change {
        Change::Create(event) => {
            event.validate().map_err(WriteError::Invalid)?;
            let event_id = created_event_id(&calendar, &event.event_id).map_err(Error::Google)?;

            (OperationKind::Create, event_id, String::new())
        }
        Change::Update {
            event,
            expected_revision,
        } => {
            event.validate().map_err(WriteError::Invalid)?;
            let etag = current_etag(store, request, &event.event_id, *expected_revision)?;

            (OperationKind::Update, event.event_id.clone(), etag)
        }
        Change::Delete {
            event_id,
            expected_revision,
        } => {
            let etag = current_etag(store, request, event_id, *expected_revision)?;

            (OperationKind::Delete, event_id.clone(), etag)
        }
    };

    Ok(Plan {
        calendar,
        kind,
        event_id,
        etag,
    })
}

/// The cached ETag of an event the user may change, or the reason they may not.
fn current_etag(
    store: &Store,
    request: &WriteRequest,
    event_id: &str,
    expected_revision: i64,
) -> Result<String, WriteError> {
    let Some(cached) = store.event(&request.account_id, &request.calendar_id, event_id)? else {
        // Expanded reads never contain a series itself, only its instances.
        let provider_id = event_id.rsplit_once(':').map_or(event_id, |parts| parts.1);
        let is_series =
            store.has_instances_of(&request.account_id, &request.calendar_id, provider_id)?;

        return Err(if is_series {
            WriteError::SeriesEdit
        } else {
            Error::Google(calendar_google::Error::NotFound).into()
        });
    };
    if cached.google.event.read_only {
        return Err(WriteError::ReadOnly);
    }
    if cached.google.event.revision != expected_revision {
        return Err(WriteError::Stale);
    }

    Ok(cached.google.etag)
}

#[cfg(test)]
mod tests;
