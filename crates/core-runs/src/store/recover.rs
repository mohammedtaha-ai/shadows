//! What startup does with attempts an earlier daemon left behind.
use super::RunsStore;
use crate::RunsError;
use core_db::{Actor, Storage};
use core_projects::ProjectId;
use core_threads::ThreadId;

impl RunsStore {
    /// Each project with an attempt still `Running`, once, in a stable order.
    pub async fn running_projects(&self) -> Result<Vec<ProjectId>, RunsError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT project_id FROM run_attempts
             WHERE state = 'running' ORDER BY project_id",
        )
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows.into_iter().map(ProjectId::from_recorded).collect())
    }

    /// Every `Running` attempt's grant is revoked. One whose commit git holds,
    /// `(attempt, commit)` in `found`, becomes `Committed` with it; every other
    /// becomes `Interrupted`. Every attempt still holding its scope releases it,
    /// because no session of an earlier daemon is left to write. Answers how
    /// many were committed and how many interrupted.
    pub async fn settle_running(
        &self,
        found: Vec<(String, String)>,
    ) -> Result<(u32, u32), RunsError> {
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let running: Vec<(String, String)> = sqlx::query_as(
                        "SELECT id, thread_id FROM run_attempts
                         WHERE state = 'running' ORDER BY started_at, id",
                    )
                    .fetch_all(&mut *conn)
                    .await?;
                    let mut committed = 0u32;
                    for (id, thread) in &running {
                        core_grants::Grants::revoke_thread_grants_in(
                            conn,
                            &ThreadId::from_recorded(thread.clone()),
                            Actor::system(),
                            &ts,
                        )
                        .await?;
                        if let Some((_, sha)) = found.iter().find(|(attempt, _)| attempt == id) {
                            sqlx::query(
                                "UPDATE run_attempts
                                 SET state = 'committed', commit_sha = ?, ended_at = ?
                                 WHERE id = ?",
                            )
                            .bind(sha)
                            .bind(&ts)
                            .bind(id)
                            .execute(&mut *conn)
                            .await?;
                            committed += 1;
                        }
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
                    let total = u32::try_from(running.len()).unwrap_or(u32::MAX);
                    Ok((committed, total - committed))
                })
            })
            .await
    }
}
