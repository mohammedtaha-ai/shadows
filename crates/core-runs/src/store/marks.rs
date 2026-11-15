//! A person's done marks, which `needs` links wait for.
use super::RunsStore;
use crate::{AttemptId, RunsError};
use core_db::{CommandContext, Storage, StorageError, Tx};
use core_plans::WorkflowId;
use core_threads::ThreadId;

/// The command record's scope for done marks: one id names one mark command.
const MARK_SCOPE: &str = "run.mark_done";

impl RunsStore {
    /// `(done, done_at, reason)`, when the task was ever marked.
    pub async fn mark(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Option<(bool, Option<String>, Option<String>)>, RunsError> {
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
    ) -> Result<Option<(bool, Option<String>, Option<String>)>, RunsError> {
        let row: Option<(bool, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT done, done_at, reason FROM run_task_marks
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
            .is_some_and(|(done, _, _)| done))
    }

    /// A replay of the same command with the same request writes nothing, and
    /// with another request is a conflict. The command record keeps every
    /// command, so a later mark of the same task does not forget an earlier one.
    /// Answers the attempt the mark judged while it still holds its scope and
    /// the mark is still this command's, also on a replay.
    pub async fn put_mark(
        &self,
        command: CommandContext,
        (workflow, version, task): (&WorkflowId, i64, u32),
        done: bool,
        reason: Option<String>,
    ) -> Result<Option<(AttemptId, ThreadId)>, RunsError> {
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
                        return Self::judged_scope(
                            conn,
                            &workflow,
                            version,
                            task,
                            &command.command_id,
                        )
                        .await;
                    }
                    if done && reason.is_none() {
                        let candidate: Option<String> = sqlx::query_scalar(
                            "SELECT id FROM run_attempts WHERE workflow_id=? AND version=?
                             AND task=? AND state='committed' ORDER BY number DESC LIMIT 1",
                        )
                        .bind(workflow.as_str())
                        .bind(version)
                        .bind(i64::from(task))
                        .fetch_optional(&mut *conn)
                        .await?;
                        if let Some(id) = candidate {
                            let id = crate::AttemptId::from_recorded(id);
                            let expected = Self::specs_in(conn, &id).await?;
                            let rows:Vec<(u32,String,Option<String>)>=sqlx::query_as(
                                "SELECT for_task,check_id,outcome FROM run_checks WHERE attempt_id=?
                                 ORDER BY for_task,ordinal")
                                .bind(id.as_str()).fetch_all(&mut *conn).await?;
                            let mut failed = std::collections::BTreeSet::new();
                            for (for_task, _, check) in expected {
                                if !rows.iter().any(|(t, c, o)| {
                                    *t == for_task
                                        && *c == check.id
                                        && o.as_deref() == Some("passed")
                                }) {
                                    failed.insert(format!("T{for_task} {}", check.id));
                                }
                            }
                            for (t, c, o) in rows {
                                if o.as_deref() != Some("passed") {
                                    failed.insert(format!("T{t} {c}"));
                                }
                            }
                            if !failed.is_empty() {
                                return Err(RunsError::ChecksNotPassed {
                                    checks: failed.into_iter().collect(),
                                });
                            }
                        }
                    }
                    let (command_id, fingerprint) =
                        (&command.command_id, &command.request_fingerprint);
                    // The attempt this mark judges: the task's latest, when it is
                    // committed and still holds its scope. A replay frees only it.
                    let judged: Option<String> = if done {
                        let latest: Option<(String, String, Option<String>)> = sqlx::query_as(
                            "SELECT id, state, released_at FROM run_attempts
                            WHERE workflow_id=? AND version=? AND task=?
                            ORDER BY number DESC LIMIT 1",
                        )
                        .bind(workflow.as_str())
                        .bind(version)
                        .bind(i64::from(task))
                        .fetch_optional(&mut *conn)
                        .await?;
                        latest
                            .filter(|(_, state, released)| {
                                state == "committed" && released.is_none()
                            })
                            .map(|(id, _, _)| id)
                    } else {
                        None
                    };
                    sqlx::query(
                        "INSERT INTO run_task_marks
                           (workflow_id, version, task, done, done_at,
                            command_id, fingerprint,reason,judged_attempt_id)
                         VALUES (?,?,?,?,?,?,?,?,?)
                         ON CONFLICT (workflow_id, version, task) DO UPDATE SET
                           done = excluded.done, done_at = excluded.done_at,
                           command_id = excluded.command_id,
                           fingerprint = excluded.fingerprint, reason=excluded.reason,
                           judged_attempt_id=excluded.judged_attempt_id",
                    )
                    .bind(workflow.as_str())
                    .bind(version)
                    .bind(i64::from(task))
                    .bind(done)
                    .bind(done.then_some(&ts))
                    .bind(command_id)
                    .bind(fingerprint)
                    .bind(reason)
                    .bind(judged)
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
                    Self::judged_scope(conn, &workflow, version, task, command_id).await
                })
            })
            .await
    }

    /// The unreleased attempt judged by the current mark, when `command` made it.
    async fn judged_scope(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
        command: &str,
    ) -> Result<Option<(AttemptId, ThreadId)>, RunsError> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT a.id,a.thread_id FROM run_task_marks m
             JOIN run_attempts a ON a.id=m.judged_attempt_id
             WHERE m.workflow_id=? AND m.version=? AND m.task=? AND m.command_id=?
             AND m.done=1 AND a.released_at IS NULL",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .bind(command)
        .fetch_optional(&mut *tx)
        .await?;
        Ok(row.map(|(id, thread)| {
            (
                AttemptId::from_recorded(id),
                ThreadId::from_recorded(thread),
            )
        }))
    }
}
