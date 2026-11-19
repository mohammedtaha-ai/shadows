//! A person's done marks, which `needs` links wait for, and the plan's progress they make.
use super::RunsStore;
use crate::checks::CONTRACT_CHECK;
use crate::{AttemptId, RunsError};
use code_record::{CodeRecord, DoneRecord};
use core_db::{CommandContext, Storage, StorageError, Tx};
use core_plans::WorkflowId;
use core_threads::ThreadId;

/// One row of `progress`: `(task, done, done_at, state)`.
type ProgressRow = (i64, Option<bool>, Option<String>, Option<String>);

/// The command record's scope for done marks: one id names one mark command.
const MARK_SCOPE: &str = "run.mark_done";

/// What a done mark's write needs that only other services know, read before it.
pub struct DoneFacts {
    /// The code record's entry for the task, when it has a contract.
    pub record: Option<DoneRecord>,
    /// Every plan of the project, so the recheck can tell which versions are superseded.
    pub plans: Vec<core_plans::PlanId>,
}

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
        plan: core_plans::Plan,
        task: u32,
        done: bool,
        reason: Option<String>,
        facts: DoneFacts,
    ) -> Result<super::Marked, RunsError> {
        let DoneFacts { record, plans } = facts;
        let (workflow, version, ts) = (plan.id.clone(), plan.version, Storage::now());
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
                        let judged =
                            Self::judged_scope(conn, &workflow, version, task, &command.command_id)
                                .await?;
                        return Ok(super::Marked {
                            judged,
                            recheck: None,
                        });
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
                            // A due `contract` check not started yet, or unfinished, has no
                            // result; one that could not be run judged nothing, so it asks no
                            // reason.
                            if !rows
                                .iter()
                                .any(|(t, c, _)| *t == task && c == CONTRACT_CHECK)
                                && Self::contract_due_in(conn, &id).await?
                            {
                                failed.insert(format!("T{task} {CONTRACT_CHECK}"));
                            }
                            for (t, c, o) in rows {
                                let unjudged =
                                    c == CONTRACT_CHECK && o.as_deref() == Some("not_run");
                                if o.as_deref() != Some("passed") && !unjudged {
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
                    Self::record_in(conn, &workflow, version, task, done, record).await?;
                    let recheck = if done {
                        Self::begin_recheck_in(conn, &plan, &plans, task, command_id).await?
                    } else {
                        None
                    };
                    let judged =
                        Self::judged_scope(conn, &workflow, version, task, command_id).await?;
                    Ok(super::Marked { judged, recheck })
                })
            })
            .await
    }

    /// The code record follows the mark in its own write: done records the
    /// task's contract, without the verdict when another attempt committed
    /// after `record` was read; not done, or no contract, removes it.
    async fn record_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
        done: bool,
        record: Option<DoneRecord>,
    ) -> Result<(), RunsError> {
        let Some(mut record) = record.filter(|_| done) else {
            CodeRecord::forget_done_in(tx, workflow, version, task).await?;
            return Ok(());
        };
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT id FROM run_attempts WHERE workflow_id=? AND version=?
             AND task=? AND state='committed' ORDER BY number DESC LIMIT 1",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_optional(&mut *tx)
        .await?;
        if latest != record.attempt {
            record.verdict = None;
        }
        CodeRecord::record_done_in(tx, &record).await?;
        Ok(())
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

    /// The version's rows: one per marked task with `state` none, and one per
    /// attempted task with its newest attempt's state and `done` none.
    pub async fn progress(
        &self,
        workflow: &WorkflowId,
        version: i64,
    ) -> Result<Vec<ProgressRow>, RunsError> {
        let rows: Vec<ProgressRow> = sqlx::query_as(
            "SELECT task, done, done_at, NULL AS state FROM run_task_marks
             WHERE workflow_id = ?1 AND version = ?2
             UNION ALL
             SELECT a.task, NULL, NULL, a.state FROM run_attempts a
             WHERE a.workflow_id = ?1 AND a.version = ?2
               AND a.number = (SELECT MAX(b.number) FROM run_attempts b
                               WHERE b.workflow_id = a.workflow_id
                                 AND b.version = a.version AND b.task = a.task)",
        )
        .bind(workflow.as_str())
        .bind(version)
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows)
    }
}
