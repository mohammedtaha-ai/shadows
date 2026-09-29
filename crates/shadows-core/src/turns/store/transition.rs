//! One job: the record of an operation's state transition — its journal event
//! and its log line.
//!
//! Every operation transition writes its row and its event in one transaction.
//! The event names the operation's thread as well as the operation, because a
//! client follows a thread (spec §2.10): an operation event with no thread is a
//! row no thread's replay or live stream can ever select. The thread is read
//! from the operation row inside the same write transaction, so no caller can
//! pass the wrong one and no raw id crosses a boundary.
//!
//! The log line is the one place a transition is logged (spec §8.7,
//! `operation.transition.committed`). It is emitted by [`Transition::log`],
//! which the caller invokes only after `write_txn` has returned `Ok` — after
//! `COMMIT` — so a transition that rolled back is never logged.

use sqlx::SqliteConnection;

use crate::db::{StorageError, append_event};
use crate::events::DurableEvent;
use crate::threads::ThreadId;
use crate::turns::model::OperationId;

/// An operation's status and thread as the open write transaction found them,
/// before the transition changes the status.
pub(crate) struct Before {
    status: String,
    thread: Option<ThreadId>,
}

impl Before {
    /// An operation about to be created: it has a thread and no status yet.
    pub(super) fn creating(thread: ThreadId) -> Self {
        Self {
            status: "None".into(),
            thread: Some(thread),
        }
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }
}

/// The state [`read_before`] found, once the transition's compare-and-swap has
/// matched a row. A matched row existed when it was read, so `None` here means
/// the CAS and the read disagree — reported, not assumed away.
pub(crate) fn existed(before: Option<Before>) -> Result<Before, StorageError> {
    before.ok_or(StorageError::NotFound("operation"))
}

/// Reads the operation's current state. Called inside `write_txn`, whose
/// `BEGIN IMMEDIATE` holds the write lock, so nothing changes the row between
/// this read and the compare-and-swap that follows it. `None` when no such
/// operation exists.
pub(crate) async fn read_before(
    conn: &mut SqliteConnection,
    op_id: &OperationId,
) -> Result<Option<Before>, StorageError> {
    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT status_kind, thread_id FROM operation WHERE id = ?")
            .bind(op_id.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    Ok(row.map(|(status, thread)| Before {
        status,
        // Read back from the column that stores it: the case `from_stored`
        // exists for.
        thread: thread.map(ThreadId::from_stored),
    }))
}

/// A transition written inside a transaction that has not committed yet.
/// `#[must_use]`: dropping it instead of calling [`Transition::log`] after the
/// commit is a transition nobody can see in the logs.
#[must_use]
pub(crate) struct Transition {
    op_id: OperationId,
    thread: Option<ThreadId>,
    event: String,
    from: String,
    to: String,
    /// Why, when the row records a reason (a failure stage and reason).
    detail: Option<String>,
}

impl Transition {
    pub(crate) fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Exactly one line per committed transition. Call only after `write_txn`
    /// returned `Ok`.
    pub(crate) fn log(&self) {
        tracing::info!(
            operation_id = %self.op_id,
            thread_id = %self.thread.as_ref().map_or("none", |t| t.as_str()),
            event = %self.event,
            from = %self.from,
            to = %self.to,
            detail = self.detail.as_deref().map(tracing::field::display),
            "operation.transition.committed"
        );
    }
}

/// Appends the transition's event, scoped to the operation and — when it has
/// one — its thread. The schema allows an operation with no thread; that event
/// keeps a NULL thread, as it has no stream to reach. `to` is the status the
/// transition leaves the row in; a cancellation request leaves it unchanged.
pub(crate) async fn record(
    conn: &mut SqliteConnection,
    op_id: &OperationId,
    before: Before,
    to: &str,
    event: DurableEvent,
    ts: &str,
) -> Result<Transition, StorageError> {
    let mut event = event.with_operation(op_id);
    if let Some(thread) = &before.thread {
        event = event.with_thread(thread);
    }
    append_event(conn, &event, ts).await?;
    Ok(Transition {
        op_id: op_id.clone(),
        thread: before.thread,
        event: event.kind,
        from: before.status,
        to: to.to_string(),
        detail: None,
    })
}
