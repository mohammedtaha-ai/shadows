//! Transaction-local journal insertion.

use super::{StorageError, Tx};

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Actor {
    pub kind: String,
    pub id: String,
}

impl Actor {
    pub fn system() -> Self {
        Self {
            kind: "System".into(),
            id: "daemon".into(),
        }
    }
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            kind: "User".into(),
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Causation {
    pub kind: String,
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub event_id: String,
    pub kind: String,
    pub project_id: Option<String>,
    pub thread_id: Option<String>,
    pub operation_id: Option<String>,
    pub actor: Actor,
    pub causation: Option<Causation>,
    pub correlation_id: Option<String>,
    pub payload_json: String,
}

impl JournalEntry {
    pub async fn append_event(
        conn: &mut Tx,
        event: impl Into<JournalEntry>,
        now: &str,
    ) -> Result<i64, StorageError> {
        let event: JournalEntry = event.into();
        let (causation_kind, causation_ref) = match &event.causation {
            Some(c) => (Some(c.kind.as_str()), Some(c.reference.as_str())),
            None => (None, None),
        };

        let seq: i64 = sqlx::query_scalar(
            "INSERT INTO durable_event
           (event_id, kind, project_id, thread_id, operation_id,
            actor_kind, actor_id, causation_kind, causation_ref, correlation_id,
            payload_json, created_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
         RETURNING seq",
        )
        .bind(&event.event_id)
        .bind(&event.kind)
        .bind(&event.project_id)
        .bind(&event.thread_id)
        .bind(&event.operation_id)
        .bind(&event.actor.kind)
        .bind(&event.actor.id)
        .bind(causation_kind)
        .bind(causation_ref)
        .bind(&event.correlation_id)
        .bind(&event.payload_json)
        .bind(now)
        .fetch_one(&mut *conn)
        .await?;
        Ok(seq)
    }

    #[cfg(feature = "test-support")]
    pub async fn append_event_for_test(
        conn: &mut Tx,
        event: impl Into<JournalEntry>,
        now: &str,
    ) -> Result<i64, StorageError> {
        Self::append_event(conn, event, now).await
    }
}
