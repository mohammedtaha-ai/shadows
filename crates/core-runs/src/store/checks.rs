//! Persistence of confirmations and their delivery on the owning attempt.
use super::RunsStore;
use crate::{AttemptId, CheckOutcome, CheckResult, ReportedOutcome, RunsError};
use core_db::Storage;
use core_plans::{TaskCheck, WorkflowId};
use core_projects::ProjectId;
use core_threads::ThreadId;
use sqlx::Row;

pub(crate) struct Finished {
    pub outcome: CheckOutcome,
    pub exit_code: Option<i32>,
    pub head: Option<String>,
    pub output: String,
    pub truncated: bool,
    /// Inconclusive only because of uncommitted files in another attempt's writes.
    pub foreign_leftovers: bool,
}

impl RunsStore {
    /// The attempt's committed or failed notice as first resolved, if it was.
    pub async fn ended_notice(&self, id: &AttemptId) -> Result<Option<String>, RunsError> {
        Ok(sqlx::query_scalar::<_, Option<String>>(
            "SELECT ended_notice_json FROM run_attempts WHERE id=?",
        )
        .bind(id.as_str())
        .fetch_optional(self.storage.reader())
        .await?
        .flatten())
    }

    /// Saves `notice` unless one is saved already, and answers the saved one.
    pub async fn keep_ended_notice(
        &self,
        id: &AttemptId,
        notice: String,
    ) -> Result<String, RunsError> {
        let id = id.clone();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query_scalar(
                        "UPDATE run_attempts
                SET ended_notice_json=COALESCE(ended_notice_json,?)
                WHERE id=? RETURNING ended_notice_json",
                    )
                    .bind(notice)
                    .bind(id.as_str())
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(Into::into)
                })
            })
            .await
    }

    pub async fn check_specs(
        &self,
        id: &AttemptId,
    ) -> Result<Vec<(u32, usize, TaskCheck)>, RunsError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(core_db::StorageError::from)?;
        Self::specs_in(&mut conn, id).await
    }

    pub(super) async fn specs_in(
        tx: &mut core_db::Tx,
        id: &AttemptId,
    ) -> Result<Vec<(u32, usize, TaskCheck)>, RunsError> {
        let own: Option<(String, i64, u32, String)> = sqlx::query_as(
            "SELECT a.workflow_id,a.version,a.task,c.context_json FROM run_attempts a
             JOIN run_attempt_context c ON c.attempt_id=a.id WHERE a.id=?",
        )
        .bind(id.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((workflow, version, own, json)) = own else {
            return Ok(vec![]);
        };
        let context: core_plans::TaskContext = serde_json::from_str(&json)?;
        let Some(contract) = context.task.contract else {
            return Ok(vec![]);
        };
        let mut checks: Vec<_> = contract
            .checks
            .into_iter()
            .enumerate()
            .map(|(i, c)| (own, i, c))
            .collect();
        let mut used = std::collections::BTreeSet::new();
        for usage in contract.uses {
            if let core_plans::ContractSource::Task {
                target: core_plans::ContractTarget::Local { task },
                ..
            } = usage.source
                && used.insert(task)
            {
                let json: Option<String> = sqlx::query_scalar(
                    "SELECT c.context_json FROM run_attempts a JOIN run_attempt_context c
                     ON c.attempt_id=a.id WHERE a.workflow_id=? AND a.version=? AND a.task=?
                     AND a.state='committed' ORDER BY a.number DESC LIMIT 1",
                )
                .bind(&workflow)
                .bind(version)
                .bind(i64::from(task))
                .fetch_optional(&mut *tx)
                .await?;
                if let Some(json) = json {
                    let context: core_plans::TaskContext = serde_json::from_str(&json)?;
                    if let Some(contract) = context.task.contract {
                        checks.extend(
                            contract
                                .checks
                                .into_iter()
                                .enumerate()
                                .map(|(i, c)| (task, i, c)),
                        );
                    }
                }
            }
        }
        Ok(checks)
    }

    pub async fn check_results(&self, id: &AttemptId) -> Result<Vec<CheckResult>, RunsError> {
        let rows = sqlx::query(
            "SELECT c.* FROM run_checks c JOIN run_attempts a ON a.id=c.attempt_id
             WHERE c.attempt_id=? ORDER BY (c.for_task != a.task),c.for_task,c.ordinal",
        )
        .bind(id.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        rows.into_iter()
            .map(|r| {
                let reported: Option<String> = r.try_get("reported")?;
                let outcome: Option<String> = r.try_get("outcome")?;
                let reported: Option<ReportedOutcome> = reported
                    .map(|s| serde_json::from_value(serde_json::Value::String(s)))
                    .transpose()?;
                let outcome: Option<CheckOutcome> = outcome
                    .map(|s| serde_json::from_value(serde_json::Value::String(s)))
                    .transpose()?;
                let matches = reported.zip(outcome).map(|(r, o)| {
                    matches!(
                        (r, o),
                        (ReportedOutcome::Passed, CheckOutcome::Passed)
                            | (
                                ReportedOutcome::Failed,
                                CheckOutcome::Failed | CheckOutcome::TimedOut
                            )
                            | (ReportedOutcome::NotRun, CheckOutcome::NotRun)
                    )
                });
                Ok(CheckResult {
                    for_task: r.try_get::<u32, _>("for_task")?,
                    id: r.try_get("check_id")?,
                    run: serde_json::from_str(r.try_get::<&str, _>("run_json")?)?,
                    folder: r.try_get("folder")?,
                    reported,
                    reported_note: r.try_get("reported_note")?,
                    outcome,
                    matches,
                    exit_code: r.try_get("exit_code")?,
                    head_before: r.try_get("head_before")?,
                    head_after: r.try_get("head_after")?,
                    output: r.try_get("output")?,
                    truncated: r.try_get("truncated")?,
                    started_at: r.try_get("started_at")?,
                    finished_at: r.try_get("finished_at")?,
                })
            })
            .collect()
    }

    /// Whether any check of the attempt did not pass for a reason its executor can act on:
    /// every non-passed result but one made inconclusive only by another attempt's files,
    /// and a `contract` check that could not be run (no language server reads its files).
    pub async fn returnable_checks(&self, id: &AttemptId) -> Result<bool, RunsError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM run_checks WHERE attempt_id=?
             AND (outcome IS NULL OR outcome<>'passed') AND foreign_leftovers=0
             AND NOT (check_id='contract' AND outcome='not_run'))",
        )
        .bind(id.as_str())
        .fetch_one(self.storage.reader())
        .await?)
    }

    pub async fn start_check(
        &self,
        id: &AttemptId,
        task: u32,
        check: &TaskCheck,
        ordinal: usize,
        head: Option<String>,
    ) -> Result<(), RunsError> {
        let (id, check) = (id.clone(), check.clone());
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO run_checks
                (attempt_id,for_task,check_id,ordinal,run_json,folder,started_at,head_before)
                VALUES (?,?,?,?,?,?,?,?) ON CONFLICT (attempt_id,for_task,check_id) DO UPDATE SET
                started_at=excluded.started_at,head_before=excluded.head_before,
                outcome=NULL,finished_at=NULL,exit_code=NULL,head_after=NULL,
                 output=NULL,truncated=0,foreign_leftovers=0",
                    )
                    .bind(id.as_str())
                    .bind(i64::from(task))
                    .bind(&check.id)
                    .bind(ordinal as i64)
                    .bind(serde_json::to_string(&check.run)?)
                    .bind(&check.folder)
                    .bind(Storage::now())
                    .bind(head)
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn finish_check(
        &self,
        id: &AttemptId,
        task: u32,
        check: &str,
        result: Finished,
    ) -> Result<(), RunsError> {
        let (id, check) = (id.clone(), check.to_string());
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_checks SET outcome=?,exit_code=?,head_after=?,output=?,
                truncated=?,foreign_leftovers=?,finished_at=?
                WHERE attempt_id=? AND for_task=? AND check_id=?",
                    )
                    .bind(serde_json::to_value(result.outcome)?.as_str().unwrap())
                    .bind(result.exit_code)
                    .bind(result.head)
                    .bind(result.output)
                    .bind(result.truncated)
                    .bind(result.foreign_leftovers)
                    .bind(Storage::now())
                    .bind(id.as_str())
                    .bind(i64::from(task))
                    .bind(check)
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn clear_unfinished(&self, id: &AttemptId) -> Result<(), RunsError> {
        let id = id.clone();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "DELETE FROM run_checks WHERE attempt_id=? AND finished_at IS NULL
                AND for_task != (SELECT task FROM run_attempts WHERE id=?)",
                    )
                    .bind(id.as_str())
                    .bind(id.as_str())
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query(
                        "UPDATE run_checks SET outcome=NULL,started_at=NULL,head_before=NULL,
                head_after=NULL,output=NULL,truncated=0,exit_code=NULL,foreign_leftovers=0
                WHERE attempt_id=? AND finished_at IS NULL",
                    )
                    .bind(id.as_str())
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn quiet_threads(&self, project: &ProjectId) -> Result<Vec<ThreadId>, RunsError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT thread_id FROM run_attempts
            WHERE project_id=? AND released_at IS NULL
            UNION SELECT m.thread_id FROM run_agent_messages m JOIN run_attempts a
              ON a.id=m.attempt_id WHERE a.project_id=? AND m.state='sent' ORDER BY thread_id",
        )
        .bind(project.as_str())
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(ids.into_iter().map(ThreadId::from_recorded).collect())
    }

    pub async fn executor_threads(&self, project: &ProjectId) -> Result<Vec<ThreadId>, RunsError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT thread_id FROM run_attempts
            WHERE project_id=? ORDER BY thread_id",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(ids.into_iter().map(ThreadId::from_recorded).collect())
    }

    pub async fn attempt_writes(&self, project: &ProjectId) -> Result<Vec<Vec<String>>, RunsError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT writes_json FROM run_attempts
            WHERE project_id=? ORDER BY writes_json",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        rows.into_iter()
            .map(|s| serde_json::from_str(&s).map_err(Into::into))
            .collect()
    }

    pub async fn latest_committed(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Option<AttemptId>, RunsError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM run_attempts
            WHERE workflow_id=? AND version=? AND task=? AND state='committed'
            ORDER BY number DESC LIMIT 1",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(id.map(AttemptId::from_recorded))
    }

    pub async fn previous_commit(&self, id: &AttemptId) -> Result<Option<String>, RunsError> {
        Ok(sqlx::query_scalar::<_, Option<String>>(
            "SELECT p.commit_sha FROM run_attempts a
            JOIN run_attempts p ON p.workflow_id=a.workflow_id AND p.version=a.version
            AND p.task=a.task AND p.number=a.number-1 WHERE a.id=?",
        )
        .bind(id.as_str())
        .fetch_optional(self.storage.reader())
        .await?
        .flatten())
    }

    pub async fn returns_since(&self, id: &AttemptId) -> Result<u32, RunsError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT p.origin FROM run_attempts a
            JOIN run_attempts p ON p.workflow_id=a.workflow_id AND p.version=a.version
            AND p.task=a.task AND p.number<=a.number WHERE a.id=? ORDER BY p.number DESC",
        )
        .bind(id.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows.iter().take_while(|s| s.as_str() == "checks").count() as u32)
    }

    pub async fn continued(&self, id: &AttemptId) -> Result<bool, RunsError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM run_attempts a JOIN run_attempts p
            ON p.workflow_id=a.workflow_id AND p.version=a.version AND p.task=a.task
            AND p.number>a.number WHERE a.id=?)",
        )
        .bind(id.as_str())
        .fetch_one(self.storage.reader())
        .await?)
    }

    pub async fn mark_notice_sent(&self, id: &AttemptId) -> Result<(), RunsError> {
        let id = id.clone();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_attempts SET notice_sent_at=COALESCE(notice_sent_at,?)
                     WHERE id=?",
                    )
                    .bind(Storage::now())
                    .bind(id.as_str())
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn unsent_committed(&self) -> Result<Vec<AttemptId>, RunsError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT a.id FROM run_attempts a
            WHERE a.state='committed' AND a.notice_sent_at IS NULL AND NOT EXISTS
            (SELECT 1 FROM run_attempts n WHERE n.workflow_id=a.workflow_id AND n.version=a.version
             AND n.task=a.task AND n.number>a.number) ORDER BY a.started_at,a.id",
        )
        .fetch_all(self.storage.reader())
        .await?;
        Ok(ids.into_iter().map(AttemptId::from_recorded).collect())
    }
}
