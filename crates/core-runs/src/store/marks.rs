//! A person's done marks, which `needs` links wait for.
use super::RunsStore;
use crate::RunsError;
use core_db::Storage;
use core_plans::WorkflowId;

impl RunsStore {
    /// `(done, done_at)`, when the task was ever marked.
    pub(crate) async fn mark(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Option<(bool, Option<String>)>, RunsError> {
        let row: Option<(bool, Option<String>)> = sqlx::query_as(
            "SELECT done, done_at FROM run_task_marks
             WHERE workflow_id = ? AND version = ? AND task = ?",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(row)
    }

    /// Every task of the version marked done, ascending.
    pub(crate) async fn done_tasks(
        &self,
        workflow: &WorkflowId,
        version: i64,
    ) -> Result<Vec<u32>, RunsError> {
        let rows: Vec<i64> = sqlx::query_scalar(
            "SELECT task FROM run_task_marks
             WHERE workflow_id = ? AND version = ? AND done = 1 ORDER BY task",
        )
        .bind(workflow.as_str())
        .bind(version)
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|n| u32::try_from(n).ok())
            .collect())
    }

    /// A replay of the same command with the same request writes nothing.
    pub(crate) async fn put_mark(
        &self,
        command_id: String,
        fingerprint: String,
        (workflow, version, task): (&WorkflowId, i64, u32),
        done: bool,
    ) -> Result<(), RunsError> {
        let (workflow, ts) = (workflow.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let seen: Option<String> = sqlx::query_scalar(
                        "SELECT fingerprint FROM run_task_marks WHERE command_id = ?",
                    )
                    .bind(&command_id)
                    .fetch_optional(&mut *conn)
                    .await?;
                    if let Some(seen) = seen {
                        return if seen == fingerprint {
                            Ok(())
                        } else {
                            Err(RunsError::CommandConflict)
                        };
                    }
                    sqlx::query(
                        "INSERT INTO run_task_marks
                           (workflow_id, version, task, done, done_at, command_id, fingerprint)
                         VALUES (?,?,?,?,?,?,?)
                         ON CONFLICT (workflow_id, version, task) DO UPDATE SET
                           done = excluded.done, done_at = excluded.done_at,
                           command_id = excluded.command_id,
                           fingerprint = excluded.fingerprint",
                    )
                    .bind(workflow.as_str())
                    .bind(version)
                    .bind(i64::from(task))
                    .bind(done)
                    .bind(done.then_some(ts))
                    .bind(&command_id)
                    .bind(&fingerprint)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}
