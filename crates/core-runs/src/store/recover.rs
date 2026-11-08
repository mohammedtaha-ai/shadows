//! What startup does with attempts an earlier daemon left behind.
use super::RunsStore;
use crate::RunsError;
use core_db::{Actor, Storage};
use core_threads::ThreadId;

impl RunsStore {
    /// Every `Running` attempt becomes `Interrupted` and its grant is
    /// revoked; every attempt still holding its scope releases it, because no
    /// session of an earlier daemon is left to write. Answers how many were
    /// interrupted.
    pub(crate) async fn interrupt_running(&self) -> Result<u32, RunsError> {
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let running: Vec<String> = sqlx::query_scalar(
                        "SELECT thread_id FROM run_attempts
                         WHERE state = 'running' ORDER BY started_at, id",
                    )
                    .fetch_all(&mut *conn)
                    .await?;
                    for thread in &running {
                        core_grants::Grants::revoke_thread_grants_in(
                            conn,
                            &ThreadId::from_recorded(thread.clone()),
                            Actor::system(),
                            &ts,
                        )
                        .await?;
                    }
                    sqlx::query(
                        "UPDATE run_attempts SET state = 'interrupted', ended_at = ?
                         WHERE state = 'running'",
                    )
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query(
                        "UPDATE run_attempts SET released_at = ? WHERE released_at IS NULL",
                    )
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    Ok(u32::try_from(running.len()).unwrap_or(u32::MAX))
                })
            })
            .await
    }
}
