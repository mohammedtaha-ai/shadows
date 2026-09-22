//! One job: the journal record of an operation's state transition.
//!
//! Every operation transition writes its row and its event in one transaction.
//! The event names the operation's thread as well as the operation, because a
//! client follows a thread (spec §2.10): an operation event with no thread is a
//! row no thread's replay or live stream can ever select. The thread is read
//! from the operation row inside the same write transaction, so no caller can
//! pass the wrong one and no raw id crosses a boundary.

use sqlx::SqliteConnection;

use super::{StorageError, events::append_event};
use crate::events::DurableEvent;
use crate::operation::OperationId;
use crate::thread::ThreadId;

/// Appends the transition's event, scoped to the operation and to the thread
/// its row names. Called inside `write_txn` after the transition's
/// compare-and-swap, whose `BEGIN IMMEDIATE` holds the write lock, so the row
/// read here is the row just written. The schema allows an operation with no
/// thread; that event keeps a NULL thread, as it has no stream to reach.
pub(super) async fn record(
    conn: &mut SqliteConnection,
    op_id: &OperationId,
    event: DurableEvent,
    ts: &str,
) -> Result<(), StorageError> {
    let thread: Option<String> = sqlx::query_scalar("SELECT thread_id FROM operation WHERE id = ?")
        .bind(op_id.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .flatten();
    let mut event = event.with_operation(op_id);
    // Read back from the column that stores it: the case `from_stored` exists for.
    if let Some(thread) = thread.map(ThreadId::from_stored) {
        event = event.with_thread(&thread);
    }
    append_event(conn, &event, ts).await?;
    Ok(())
}
