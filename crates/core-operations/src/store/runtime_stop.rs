//! The durable shutdown claim over operations and their runtime owner.
use crate::Operations;
use core_db::{Storage, StorageError};
use core_runtime::{Runtime, RuntimeInstanceId, StopKind};
impl Operations {
    pub async fn stop_runtime(
        &self,
        runtime: &RuntimeInstanceId,
        kind: StopKind,
    ) -> Result<(), StorageError> {
        let id = runtime.clone();
        self.store
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if kind == StopKind::Graceful {
                        let unfinished: i64 = sqlx::query_scalar(
                            "SELECT COUNT(*) FROM operation WHERE runtime_instance_id=?
                     AND status_kind IN ('Pending','Running')",
                        )
                        .bind(id.as_str())
                        .fetch_one(&mut *conn)
                        .await?;
                        if unfinished > 0 {
                            return Err(StorageError::TransitionConflict {
                                expected: "every operation the runtime owns terminal".into(),
                                found: format!("{unfinished} still Pending or Running"),
                            });
                        }
                    }
                    Runtime::stop_in(conn, id.as_str(), kind, &Storage::now()).await
                })
            })
            .await?;
        tracing::info!(runtime_instance_id = %runtime, stop_kind = ?kind, "runtime.stop");
        Ok(())
    }
}
