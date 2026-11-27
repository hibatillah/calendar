//! Write journal: one row per write, keyed by a stable `operation_id`, so an uncertain outcome
//! (timeout, crash) is reconciled by reading the event instead of replaying the write. The journal
//! stores no event content: it can tell what happened, not redo it.

use super::{Store, apply};
use crate::Error;
use calendar_google::EventChange;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row, params};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationState {
    /// Recorded before the request is sent.
    Pending,
    Applied,
    /// Google rejected it or never received it; nothing changed.
    Failed,
    /// The outcome is unknown (timeout, crash); reconcile by reading the event, never blindly replay.
    Uncertain,
}

impl OperationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applied => "applied",
            Self::Failed => "failed",
            Self::Uncertain => "uncertain",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "applied" => Self::Applied,
            "failed" => Self::Failed,
            "uncertain" => Self::Uncertain,
            _ => Self::Pending,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Create,
    Update,
    Delete,
}

impl OperationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "create" => Self::Create,
            "delete" => Self::Delete,
            _ => Self::Update,
        }
    }
}

/// One journaled write. `operation_id` is stable across retries. For a create, `event_id` is the
/// id the event will have once Google accepts it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub operation_id: String,
    pub account_id: String,
    pub calendar_id: String,
    pub event_id: String,
    pub kind: OperationKind,
    pub state: OperationState,
}

const OPERATION_COLUMNS: &str = "operation_id, account_id, calendar_id, event_id, kind, state";

impl Store {
    /// Records a write before it is sent. A repeated `operation_id` returns the stored entry
    /// unchanged, so a retry can see that the write already happened.
    pub fn begin_operation(&self, operation: &Operation) -> Result<Operation, Error> {
        self.connection.execute(
            "INSERT OR IGNORE INTO operations (operation_id, account_id, calendar_id, event_id, kind, state)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
            params![
                operation.operation_id,
                operation.account_id,
                operation.calendar_id,
                operation.event_id,
                operation.kind.as_str()
            ],
        )?;

        Ok(self
            .operation(&operation.operation_id)?
            .expect("the row was just inserted or already present"))
    }

    pub fn operation(&self, operation_id: &str) -> Result<Option<Operation>, Error> {
        let operation = self
            .connection
            .query_row(
                &format!("SELECT {OPERATION_COLUMNS} FROM operations WHERE operation_id = ?1"),
                [operation_id],
                read_operation,
            )
            .optional()?;

        Ok(operation)
    }

    /// Records the outcome and, in the same short transaction, the event as Google reported it
    /// (`cache`), so the cache and the journal never disagree after a crash. The cache may be
    /// refreshed even when the operation failed: it is Google's state either way.
    pub fn finish_operation(
        &mut self,
        operation: &Operation,
        state: OperationState,
        cache: Option<&EventChange>,
    ) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        if let Some(change) = cache {
            apply(
                &transaction,
                &operation.account_id,
                &operation.calendar_id,
                std::slice::from_ref(change),
            )?;
        }
        transaction.execute(
            "UPDATE operations SET state = ?2 WHERE operation_id = ?1",
            [operation.operation_id.as_str(), state.as_str()],
        )?;
        transaction.commit()?;

        Ok(())
    }

    /// Writes whose outcome is not known yet (in flight, crashed or uncertain). A view keeps its
    /// optimistic placement of these events until they leave this list.
    pub fn pending_operations(&self) -> Result<Vec<Operation>, Error> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {OPERATION_COLUMNS} FROM operations
             WHERE state IN ('pending', 'uncertain') ORDER BY created_at, operation_id"
        ))?;
        let operations = statement
            .query_map([], read_operation)?
            .collect::<Result<_, _>>()?;

        Ok(operations)
    }

    /// Forgets finished writes older than `cutoff`; unknown outcomes are never dropped.
    pub fn prune_operations_before(&self, cutoff: DateTime<Utc>) -> Result<(), Error> {
        self.connection.execute(
            "DELETE FROM operations WHERE state IN ('applied', 'failed') AND created_at < ?1",
            [cutoff.timestamp()],
        )?;

        Ok(())
    }
}

fn read_operation(row: &Row) -> rusqlite::Result<Operation> {
    Ok(Operation {
        operation_id: row.get(0)?,
        account_id: row.get(1)?,
        calendar_id: row.get(2)?,
        event_id: row.get(3)?,
        kind: OperationKind::parse(&row.get::<_, String>(4)?),
        state: OperationState::parse(&row.get::<_, String>(5)?),
    })
}
