use super::transition::{Before, Transition};
use crate::operations::{OperationId, Operations, ReconcileReport};
use core_db::{Actor, Storage, StorageError};
use core_runtime::{Runtime, RuntimeInstanceId};
impl Operations {
    pub(crate) async fn recover(
        &self,
        current: &RuntimeInstanceId,
    ) -> Result<ReconcileReport, StorageError> {
        let runtime_id = current.clone();
        let (current, ts) = (current.as_str().to_string(), Storage::now());
        let (report, transitions) = self
            .store
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let rows: Vec<(String, String, String)> = sqlx::query_as(
                        "SELECT o.id, o.status_kind, o.runtime_instance_id
                           FROM operation o
                          WHERE o.status_kind IN ('Pending','Running')
                            AND o.runtime_instance_id <> ?
                          ORDER BY o.id",
                    )
                    .bind(&current)
                    .fetch_all(&mut *conn)
                    .await?;

                    let mut report = ReconcileReport::default();
                    let mut transitions = Vec::new();
                    for (op_id, status, owner) in rows {
                        let stop_kind = Runtime::stop_kind_in(conn, &owner).await?;
                        let op_id = OperationId::from_stored(op_id);
                        let reason = match status.as_str() {
                            "Pending" => "PreviousRuntimeEndedBeforeStart",
                            _ => "PreviousRuntimeEndedDuringRun",
                        };
                        let before = Before::read_before(conn, &op_id).await?;
                        let affected = sqlx::query(
                            "UPDATE operation
                                SET status_kind = 'Interrupted',
                                    interrupt_reason = ?,
                                    finished_at = ?
                              WHERE id = ? AND status_kind = ? AND runtime_instance_id <> ?",
                        )
                        .bind(reason)
                        .bind(&ts)
                        .bind(op_id.as_str())
                        .bind(&status)
                        .bind(&current)
                        .execute(&mut *conn)
                        .await?
                        .rows_affected();
                        if affected == 0 {
                            continue;
                        }
                        transitions.push(
                            Transition::record(
                                conn,
                                &op_id,
                                Before::existed(before)?,
                                "Interrupted",
                                Transition::event(
                                    "OperationInterrupted",
                                    Actor::system(),
                                    Some(serde_json::json!({ "reason": reason })),
                                ),
                                &ts,
                            )
                            .await?
                            .with_detail(reason.to_string()),
                        );
                        report.interrupted.push(op_id.clone());
                        if stop_kind.as_deref() == Some("Graceful") {
                            report.anomalies.push(op_id);
                        }
                    }
                    Ok((report, transitions))
                })
            })
            .await?;
        for transition in &transitions {
            transition.log();
        }
        tracing::info!(
            runtime_instance_id = %runtime_id,
            interrupted = report.interrupted.len(), anomalies = report.anomalies.len(),
            "recovery.reconcile"
        );
        for op in &report.anomalies {
            tracing::error!(operation_id = %op,
                "recovery.anomaly: a Graceful runtime owned a non-terminal operation");
        }
        Ok(report)
    }
}
