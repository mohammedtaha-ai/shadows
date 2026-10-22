use crate::db::JournalEntry;
use crate::db::{Actor, Storage, StorageError, append_event, now};
use crate::runtime::{Runtime, RuntimeInstanceId};
use sqlx::SqliteConnection;
pub(super) struct RuntimeStore<'a> {
    pub(super) storage: &'a Storage,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Graceful,
    Escalated,
}
impl StopKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Graceful => "Graceful",
            Self::Escalated => "Escalated",
        }
    }
}
impl RuntimeStore<'_> {
    pub async fn register_runtime_instance(
        &self,
        version: &str,
    ) -> Result<RuntimeInstanceId, StorageError> {
        let id = RuntimeInstanceId::generate();
        let ts = now();
        let (id2, version, ts2) = (id.as_str().to_string(), version.to_string(), ts.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO runtime_instance (id, version, started_at) VALUES (?,?,?)",
                    )
                    .bind(&id2)
                    .bind(&version)
                    .bind(&ts2)
                    .execute(&mut *conn)
                    .await?;
                    append_event(
                        conn,
                        RuntimeStore::event(
                            "RuntimeStarted",
                            serde_json::json!({ "runtime_instance_id": id2 }),
                        ),
                        &ts2,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await?;
        Ok(id)
    }
}
impl Runtime {
    pub(crate) async fn stop_in(
        conn: &mut SqliteConnection,
        id: &str,
        kind: StopKind,
        ts: &str,
    ) -> Result<(), StorageError> {
        let affected = sqlx::query(
            "UPDATE runtime_instance SET stopped_at = ?, stop_kind = ?
                     WHERE id = ? AND stopped_at IS NULL",
        )
        .bind(ts)
        .bind(kind.as_str())
        .bind(id)
        .execute(&mut *conn)
        .await?
        .rows_affected();
        if affected == 0 {
            return Err(StorageError::TransitionConflict {
                expected: "runtime with stopped_at IS NULL".into(),
                found: "already stopped or missing".into(),
            });
        }
        append_event(
            conn,
            RuntimeStore::event(
                "RuntimeStopped",
                serde_json::json!({
                    "runtime_instance_id": id, "stop_kind": kind.as_str()
                }),
            ),
            ts,
        )
        .await?;
        Ok(())
    }
    pub(crate) async fn require_active_in(
        conn: &mut SqliteConnection,
        id: &RuntimeInstanceId,
    ) -> Result<(), StorageError> {
        let active: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM runtime_instance WHERE id=? AND stopped_at IS NULL")
                .bind(id.as_str())
                .fetch_optional(conn)
                .await?;
        if active.is_none() {
            return Err(StorageError::TransitionConflict {
                expected: "a runtime that has not stopped".into(),
                found: "a stopped or unknown runtime".into(),
            });
        }
        Ok(())
    }
    pub(crate) async fn stop_kind_in(
        conn: &mut SqliteConnection,
        id: &str,
    ) -> Result<Option<String>, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT stop_kind FROM runtime_instance WHERE id=?")
                .bind(id)
                .fetch_one(conn)
                .await?,
        )
    }
}

impl RuntimeStore<'_> {
    fn event(kind: &str, payload: serde_json::Value) -> JournalEntry {
        JournalEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            actor: Actor::system(),
            causation: None,
            correlation_id: None,
            project_id: None,
            thread_id: None,
            operation_id: None,
            payload_json: payload.to_string(),
        }
    }
}
