//! One job: reading the durable journal back out, after a cursor.
//!
//! Appending an event is `db/journal.rs`, and stays private because
//! cross-cutting rule 10 forbids appending one without the state write it
//! describes. Reading has no such hazard: the replay half of spec §2.10 is a
//! query, and it is `Events`' (§14.4).

use crate::db::{Storage, StorageError};
use crate::events::EventCursor;
use crate::projects::ProjectId;
use crate::threads::ThreadId;
use crate::turns::OperationId;

/// One replayed journal row, as a client resuming a thread sees it.
///
/// The payload stays a `String` and is not parsed here: it was written as JSON
/// by whichever capability produced the event, and re-parsing it to re-encode
/// it would let this module invent a shape the writer never agreed to.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredEvent {
    pub seq: i64,
    pub kind: String,
    pub operation_id: Option<OperationId>,
    pub thread_id: Option<ThreadId>,
    pub payload_json: String,
    pub created_at: String,
}

/// The six `durable_event` columns `read_events_after` selects, in select order.
type EventRow = (i64, String, Option<String>, Option<String>, String, String);

impl Storage {
    /// A project's plan event tail. Only identifiers are projected for client
    /// invalidation; plan content and edit details never enter this stream.
    /// Archive events name a plan, so they name its latest version to refetch.
    pub async fn read_project_events_after(
        &self,
        cursor: EventCursor,
        project: &ProjectId,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, StorageError> {
        type Row = (
            i64,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
        );
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT e.seq, e.kind, e.operation_id, e.thread_id, e.created_at, w.plan_id, w.id
               FROM durable_event e
               JOIN workflow w ON w.id = COALESCE(
                    json_extract(e.payload_json, '$.workflow_id'),
                    (SELECT id FROM workflow
                      WHERE plan_id = json_extract(e.payload_json, '$.plan')
                      ORDER BY version DESC LIMIT 1))
              WHERE e.project_id = ? AND e.seq > ?
                AND e.kind IN ('WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen',
                               'PlanArchived', 'PlanUnarchived')
              ORDER BY e.seq LIMIT ?",
        )
        .bind(project.as_str())
        .bind(cursor.0)
        .bind(limit)
        .fetch_all(self.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| StoredEvent {
                seq: r.0,
                kind: r.1,
                operation_id: r.2.map(OperationId::from_stored),
                thread_id: r.3.map(ThreadId::from_stored),
                created_at: r.4,
                payload_json: serde_json::json!({ "plan_id": r.5, "workflow_id": r.6 }).to_string(),
            })
            .collect())
    }

    /// The highest sequence committed so far. Spec §2.10: the snapshot and the
    /// cursor must come from the same read, so a caller building a snapshot
    /// takes this inside that same read transaction.
    #[cfg(feature = "test-support")]
    pub async fn current_cursor(&self) -> Result<EventCursor, StorageError> {
        let seq: Option<i64> = sqlx::query_scalar(crate::db::MAX_SEQ)
            .fetch_one(self.reader())
            .await?;
        Ok(EventCursor(seq.unwrap_or(0)))
    }

    /// Thread-scoped replay. Ordering is by `seq` explicitly — never by
    /// insertion order. On SQLite, sequence order is commit order; see the
    /// OPEN block in §6.18 before assuming that on another backend.
    pub async fn read_events_after(
        &self,
        cursor: EventCursor,
        thread_id: &ThreadId,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, StorageError> {
        let rows: Vec<EventRow> = sqlx::query_as(
            "SELECT seq, kind, operation_id, thread_id, payload_json, created_at
               FROM durable_event
              WHERE thread_id = ? AND seq > ?
              ORDER BY seq
              LIMIT ?",
        )
        .bind(thread_id.as_str())
        .bind(cursor.0)
        .bind(limit)
        .fetch_all(self.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| StoredEvent {
                seq: r.0,
                kind: r.1,
                // Read back from the column that stores it, which is the one
                // case `from_stored` exists for: outside code still cannot
                // build an id from arbitrary text.
                operation_id: r.2.map(OperationId::from_stored),
                thread_id: r.3.map(ThreadId::from_stored),
                payload_json: r.4,
                created_at: r.5,
            })
            .collect())
    }
}
