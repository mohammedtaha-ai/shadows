//! One job: reading operations back out.
//!
//! Writing one is `operation.rs`, where every transition carries its journal
//! event; a read has no such pairing, and keeping it apart keeps that file
//! about transitions only (the same split as `events.rs` / `events_read.rs`).

use crate::operation::{Operation, OperationId};
use crate::runtime::RuntimeInstanceId;
use crate::thread::ThreadId;

use super::{Storage, StorageError};

/// The thirteen `operation` columns both reads below select, in select order.
/// A row alias, not a domain type: [`into_operation`] maps it into `Operation`.
type OperationRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

impl Storage {
    pub async fn get_operation(&self, op_id: &OperationId) -> Result<Operation, StorageError> {
        let row: OperationRow = sqlx::query_as(
            "SELECT id, kind, status_kind, thread_id, runtime_instance_id, outcome_json,
                    failure_stage, failure_reason, interrupt_reason, cancel_requested_at,
                    created_at, started_at, finished_at
               FROM operation WHERE id = ?",
        )
        .bind(op_id.as_str())
        .fetch_optional(self.reader())
        .await?
        .ok_or(StorageError::NotFound("operation"))?;
        Ok(into_operation(row))
    }

    /// A thread's operations, newest first. A thread nobody knows has none.
    ///
    /// Ordered by the durable sequence of each operation's `OperationCreated`
    /// event, written in the transaction that created the row — not by
    /// `created_at`: RFC 3339 text with its trailing zeros trimmed does not
    /// sort lexically in time order within a second (`.1Z` sorts after
    /// `.12Z`), and CLAUDE.md names the journal's sequence an explicit
    /// ordering key.
    pub async fn list_operations_for_thread(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        let rows: Vec<OperationRow> = sqlx::query_as(
            "SELECT o.id, o.kind, o.status_kind, o.thread_id, o.runtime_instance_id,
                    o.outcome_json, o.failure_stage, o.failure_reason, o.interrupt_reason,
                    o.cancel_requested_at, o.created_at, o.started_at, o.finished_at
               FROM operation o
               JOIN durable_event e
                 ON e.operation_id = o.id AND e.kind = 'OperationCreated'
              WHERE o.thread_id = ?
              ORDER BY e.seq DESC",
        )
        .bind(thread_id.as_str())
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(into_operation).collect())
    }

    /// The operations `runtime` owns that are still `Pending` or `Running`,
    /// ordered by id. Spec §8.5: shutdown may record `Graceful` only once this
    /// is empty. The order is only for stable logs; nothing depends on it.
    pub async fn non_terminal_operations_owned_by(
        &self,
        runtime: &RuntimeInstanceId,
    ) -> Result<Vec<OperationId>, StorageError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM operation
              WHERE runtime_instance_id = ? AND status_kind IN ('Pending','Running')
              ORDER BY id",
        )
        .bind(runtime.as_str())
        .fetch_all(self.reader())
        .await?;
        Ok(ids.into_iter().map(OperationId::from_stored).collect())
    }
}

fn into_operation(r: OperationRow) -> Operation {
    Operation {
        id: OperationId::from_stored(r.0),
        kind: r.1,
        status_kind: r.2,
        thread_id: r.3,
        runtime_instance_id: RuntimeInstanceId::from_stored(r.4),
        outcome_json: r.5,
        failure_stage: r.6,
        failure_reason: r.7,
        interrupt_reason: r.8,
        cancel_requested_at: r.9,
        created_at: r.10,
        started_at: r.11,
        finished_at: r.12,
    }
}
