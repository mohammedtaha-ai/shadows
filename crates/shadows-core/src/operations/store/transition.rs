use sqlx::SqliteConnection;

use crate::operations::OperationId;
use crate::threads::ThreadId;
use core_db::StorageError;
use core_db::{Actor, JournalEntry};

pub(crate) struct Before {
    status: String,
    thread: Option<ThreadId>,
}

impl Before {
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

impl Before {
    pub(crate) fn existed(before: Option<Before>) -> Result<Before, StorageError> {
        before.ok_or(StorageError::NotFound("operation"))
    }
}

impl Before {
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
            thread: thread.map(ThreadId::from_stored),
        }))
    }
}

#[must_use]
pub(crate) struct Transition {
    op_id: OperationId,
    thread: Option<ThreadId>,
    event: String,
    from: String,
    to: String,
    detail: Option<String>,
}

impl Transition {
    pub(crate) fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

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

impl Transition {
    pub(crate) async fn record(
        conn: &mut SqliteConnection,
        op_id: &OperationId,
        before: Before,
        to: &str,
        mut event: JournalEntry,
        ts: &str,
    ) -> Result<Transition, StorageError> {
        event.operation_id = Some(op_id.as_str().to_owned());
        if let Some(thread) = &before.thread {
            event.thread_id = Some(thread.as_str().to_owned());
        }
        let kind = event.kind.clone();
        JournalEntry::append_event(conn, event, ts).await?;
        Ok(Transition {
            op_id: op_id.clone(),
            thread: before.thread,
            event: kind,
            from: before.status,
            to: to.to_string(),
            detail: None,
        })
    }
}

impl Transition {
    pub(super) fn event(
        kind: &str,
        actor: Actor,
        payload: Option<serde_json::Value>,
    ) -> JournalEntry {
        JournalEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            actor,
            causation: None,
            correlation_id: None,
            project_id: None,
            thread_id: None,
            operation_id: None,
            payload_json: payload.map_or_else(|| "{}".into(), |p| p.to_string()),
        }
    }
}
