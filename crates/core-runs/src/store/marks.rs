//! A person's done marks, which `needs` links wait for.
use super::RunsStore;
use crate::RunsError;
use core_db::{CommandContext, Storage, StorageError, Tx};
use core_plans::WorkflowId;

/// The command record's scope for done marks: one id names one mark command.
const MARK_SCOPE: &str = "run.mark_done";

impl RunsStore {
    /// `(done, done_at)`, when the task was ever marked.
    pub async fn mark(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Option<(bool, Option<String>)>, RunsError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        Self::mark_in(&mut conn, workflow, version, task).await
    }

    /// `mark`, as `tx` sees it.
    async fn mark_in(
        tx: &mut Tx,
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
        .fetch_optional(&mut *tx)
        .await?;
        Ok(row)
    }

    /// Whether the task is marked done, as `tx` sees it: what `needs` waits for.
    pub async fn done_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<bool, RunsError> {
        Ok(Self::mark_in(tx, workflow, version, task)
            .await?
            .is_some_and(|(done, _)| done))
    }

    /// A replay of the same command with the same request writes nothing, and
    /// with another request is a conflict. The command record keeps every
    /// command, so a later mark of the same task does not forget an earlier one.
    pub async fn put_mark(
        &self,
        command: CommandContext,
        (workflow, version, task): (&WorkflowId, i64, u32),
        done: bool,
    ) -> Result<(), RunsError> {
        let (workflow, ts) = (workflow.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let seen = CommandContext::classify(conn, &command, MARK_SCOPE, "")
                        .await
                        .map_err(|error| match error {
                            StorageError::CommandConflict => RunsError::CommandConflict,
                            other => other.into(),
                        })?;
                    if seen.is_some() {
                        return Ok(());
                    }
                    let (command_id, fingerprint) =
                        (&command.command_id, &command.request_fingerprint);
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
                    .bind(done.then_some(&ts))
                    .bind(command_id)
                    .bind(fingerprint)
                    .execute(&mut *conn)
                    .await?;
                    let mark = format!("{}/{version}/{task}", workflow.as_str());
                    CommandContext::record_command(
                        conn,
                        &command,
                        MARK_SCOPE,
                        "",
                        "run_task_mark",
                        &mark,
                        &ts,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}
