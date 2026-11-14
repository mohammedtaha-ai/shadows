//! What startup does with attempts an earlier daemon left behind.
use super::RunsStore;
use crate::RunsError;
use core_db::{Actor, Storage};
use core_threads::ThreadId;

impl RunsStore {
    /// Every `Running` attempt's grant is revoked and the attempt becomes
    /// `Interrupted`: an accepted report is the only way to `Committed`, and
    /// none came. Every other attempt still holding its scope releases it,
    /// because no session of an earlier daemon is left to write, except a
    /// committed one: it keeps its scope while it waits for the Planner's
    /// judgment. A contract conflict and its saved notice stay as they are.
    /// Answers how many were interrupted.
    pub async fn settle_running(&self) -> Result<u32, RunsError> {
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
                        "UPDATE run_attempts SET released_at = ?
                         WHERE released_at IS NULL AND state <> 'committed'",
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
