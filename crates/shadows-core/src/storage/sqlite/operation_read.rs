//! One job: reading operations back out.
//!
//! Writing one is `operation.rs`, where every transition carries its journal
//! event; a read has no such pairing, and keeping it apart keeps that file
//! about transitions only (the same split as `events.rs` / `events_read.rs`).

use std::collections::HashMap;

use sqlx::SqliteConnection;

use crate::operation::{InvocationView, Operation, OperationId};
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
        let mut conn = self.reader().acquire().await?;
        let invocation = invocation_of(&mut conn, op_id).await?;
        Ok(into_operation(row, invocation))
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
        let found: Vec<InvocationRow> = sqlx::query_as(
            "SELECT i.operation_id, i.harness_kind, i.harness_version, i.agent_version,
                    i.requested_model, i.requested_mode, i.requested_effort,
                    i.observed_model, i.context_used, i.context_window
               FROM agent_invocation i JOIN operation o ON o.id = i.operation_id
              WHERE o.thread_id = ?",
        )
        .bind(thread_id.as_str())
        .fetch_all(self.reader())
        .await?;
        let mut by_op: HashMap<String, InvocationView> = found
            .into_iter()
            .map(|r| (r.0.clone(), into_view(r)))
            .collect();
        Ok(rows
            .into_iter()
            .map(|r| {
                let invocation = by_op.remove(&r.0);
                into_operation(r, invocation)
            })
            .collect())
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

/// `agent_invocation`'s columns as a client sees them, led by its operation.
type InvocationRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
);

fn into_view(r: InvocationRow) -> InvocationView {
    InvocationView {
        harness_kind: r.1,
        harness_version: r.2,
        agent_version: r.3,
        requested_model: r.4,
        requested_mode: r.5,
        requested_effort: r.6,
        observed_model: r.7,
        context_used: r.8,
        context_window: r.9,
    }
}

/// The invocation of `op`, if one was recorded. Also read by the transition
/// that completes a turn, to put it in the `OperationCompleted` event.
pub(super) async fn invocation_of(
    conn: &mut SqliteConnection,
    op: &OperationId,
) -> Result<Option<InvocationView>, StorageError> {
    let row: Option<InvocationRow> = sqlx::query_as(
        "SELECT operation_id, harness_kind, harness_version, agent_version,
                requested_model, requested_mode, requested_effort,
                observed_model, context_used, context_window
           FROM agent_invocation WHERE operation_id = ?",
    )
    .bind(op.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(into_view))
}

fn into_operation(r: OperationRow, invocation: Option<InvocationView>) -> Operation {
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
        invocation,
    }
}
