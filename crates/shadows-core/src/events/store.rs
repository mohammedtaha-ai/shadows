//! Private journal replay persistence.
use super::Events;
use crate::events::EventCursor;
use crate::plans::Plans;
use crate::threads::ThreadId;
use crate::turns::OperationId;
use core_db::{Storage, StorageError};
use core_projects::ProjectId;
use std::sync::Arc;

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

#[derive(Clone)]
pub(super) struct EventsStore {
    storage: Arc<Storage>,
}

impl EventsStore {
    pub(super) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// A project's plan/design event tail. Only identifiers are projected for client
    /// invalidation; plan content and edit details never enter this stream.
    /// Archive events name a plan, so they name its latest version to refetch.
    pub async fn read_project_events_after(
        &self,
        cursor: EventCursor,
        project: &ProjectId,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, StorageError> {
        let mut snapshot = self.storage.reader().begin().await?;
        let rows: Vec<EventRow> = sqlx::query_as(
            "SELECT e.seq, e.kind, e.operation_id, e.thread_id, e.payload_json, e.created_at
               FROM durable_event e
              WHERE e.project_id = ? AND e.seq > ?
                AND e.kind IN ('WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen',
                               'PlanArchived', 'PlanUnarchived', 'ProjectDesignChanged',
                               'PlanDependenciesChanged', 'ProjectLinked', 'ProjectUnlinked',
                               'ProjectRemoved', 'AgreementChanged', 'ProjectStandardsSaved',
                               'VisionProposed','MapProposed','ProjectStructureWritten',
                               'RoadmapProposed','RoadmapApproved')
              ORDER BY e.seq LIMIT ?",
        )
        .bind(project.as_str())
        .bind(cursor.0)
        .bind(limit)
        .fetch_all(&mut *snapshot)
        .await?;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let mut event = Self::stored(row);
            event.payload_json =
                Plans::project_event_payload_in(&mut snapshot, &event.kind, &event.payload_json)
                    .await?;
            events.push(event);
        }
        snapshot.commit().await?;
        Ok(events)
    }

    /// Thread-scoped replay. Ordering is by `seq` explicitly — never by
    /// insertion order. Sequence order is commit order on SQLite; verify the
    /// backend choice before relying on that property for another database.
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
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows.into_iter().map(Self::stored).collect())
    }

    fn stored(r: EventRow) -> StoredEvent {
        StoredEvent {
            seq: r.0,
            kind: r.1,
            operation_id: r.2.map(OperationId::from_stored),
            thread_id: r.3.map(ThreadId::from_stored),
            payload_json: r.4,
            created_at: r.5,
        }
    }
}

impl Events {
    /// The journal cursor for fixture assertions, without exposing a domain Storage method.
    #[cfg(feature = "test-support")]
    pub async fn current_cursor_for_test(storage: &Storage) -> Result<EventCursor, StorageError> {
        let seq: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM durable_event")
            .fetch_one(storage.reader())
            .await?;
        Ok(EventCursor(seq.unwrap_or(0)))
    }

    /// Replay through the journal owner using a fixture's explicit storage handle.
    #[cfg(feature = "test-support")]
    pub async fn read_events_after_for_test(
        storage: Arc<Storage>,
        cursor: EventCursor,
        thread: &ThreadId,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, StorageError> {
        EventsStore::with_storage(storage)
            .read_events_after(cursor, thread, limit)
            .await
    }
}
