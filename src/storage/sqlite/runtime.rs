use super::{Storage, StorageError, events::append_event};
use crate::events::{Actor, DurableEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Graceful,
    Escalated,
}

impl StopKind {
    fn as_str(self) -> &'static str {
        match self {
            StopKind::Graceful => "Graceful",
            StopKind::Escalated => "Escalated",
        }
    }
}

#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub interrupted: Vec<String>,
    /// Operations found under a `Graceful` runtime. Spec §8.5 says that cannot
    /// happen; if it does, §8.6 requires it be reported, not silently handled.
    pub anomalies: Vec<String>,
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting cannot fail")
}

impl Storage {
    pub async fn register_runtime_instance(&self, version: &str) -> Result<String, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now();
        let (id2, version, ts2) = (id.clone(), version.to_string(), ts.clone());
        self.write_txn(move |conn| {
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
                    &DurableEvent::new("RuntimeStarted", Actor::system())
                        .with_payload(serde_json::json!({ "runtime_instance_id": id2 })),
                    &ts2,
                )
                .await?;
                Ok(())
            })
        })
        .await?;
        Ok(id)
    }

    pub async fn stop_runtime_instance(
        &self,
        id: &str,
        kind: StopKind,
    ) -> Result<(), StorageError> {
        let (id, ts) = (id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE runtime_instance SET stopped_at = ?, stop_kind = ?
                     WHERE id = ? AND stopped_at IS NULL",
                )
                .bind(&ts)
                .bind(kind.as_str())
                .bind(&id)
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
                    &DurableEvent::new("RuntimeStopped", Actor::system()).with_payload(
                        serde_json::json!({
                            "runtime_instance_id": id, "stop_kind": kind.as_str()
                        }),
                    ),
                    &ts,
                )
                .await?;
                Ok(())
            })
        })
        .await
    }

    /// Spec §8.6. Selects on **ownership alone**: every non-terminal Operation
    /// whose owning runtime is not the current one, regardless of how that
    /// runtime ended. Filtering on `stopped_at IS NULL` would permanently
    /// strand everything an Escalated shutdown left behind.
    pub async fn reconcile_orphans(&self, current: &str) -> Result<ReconcileReport, StorageError> {
        let (current, ts) = (current.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
                    "SELECT o.id, o.status_kind, r.stop_kind
                       FROM operation o
                       JOIN runtime_instance r ON r.id = o.runtime_instance_id
                      WHERE o.status_kind IN ('Pending','Running')
                        AND o.runtime_instance_id <> ?
                      ORDER BY o.id",
                )
                .bind(&current)
                .fetch_all(&mut *conn)
                .await?;

                let mut report = ReconcileReport::default();
                for (op_id, status, stop_kind) in rows {
                    let reason = match status.as_str() {
                        "Pending" => "PreviousRuntimeEndedBeforeStart",
                        _ => "PreviousRuntimeEndedDuringRun",
                    };
                    // Exact CAS on id, expected status, and previous runtime.
                    let affected = sqlx::query(
                        "UPDATE operation
                            SET status_kind = 'Interrupted',
                                interrupt_reason = ?,
                                finished_at = ?
                          WHERE id = ? AND status_kind = ? AND runtime_instance_id <> ?",
                    )
                    .bind(reason)
                    .bind(&ts)
                    .bind(&op_id)
                    .bind(&status)
                    .bind(&current)
                    .execute(&mut *conn)
                    .await?
                    .rows_affected();
                    if affected == 0 {
                        continue;
                    }
                    append_event(
                        conn,
                        &DurableEvent::new("OperationInterrupted", Actor::system())
                            .with_operation(&op_id)
                            .with_payload(serde_json::json!({ "reason": reason })),
                        &ts,
                    )
                    .await?;
                    report.interrupted.push(op_id.clone());
                    if stop_kind.as_deref() == Some("Graceful") {
                        report.anomalies.push(op_id);
                    }
                }
                Ok(report)
            })
        })
        .await
    }
}
