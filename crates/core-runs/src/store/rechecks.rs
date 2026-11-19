//! Durable recheck batches and their per-check results.
use super::{RunsStore, checks::Finished};
use crate::checks::CONTRACT_CHECK;
use crate::{AttemptId, RunsError};
use core_db::{Storage, Tx};
use core_plans::{Plan, PlanId, Plans, TaskCheck, TaskContext, WorkflowId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(sqlx::FromRow)]
pub(crate) struct Batch {
    pub id: String,
    pub cause_workflow_id: String,
    pub cause_version: i64,
    pub cause_task: u32,
    pub finished_at: Option<String>,
    pub notice_json: Option<String>,
    pub notice_sent_at: Option<String>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct ResultRow {
    pub attempt_id: String,
    pub workflow_id: String,
    pub version: i64,
    pub task: u32,
    pub check_id: String,
    pub run_json: String,
    pub outcome: Option<String>,
    pub exit_code: Option<i32>,
    pub head: Option<String>,
    pub output: Option<String>,
    pub truncated: bool,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

impl RunsStore {
    pub(super) async fn begin_recheck_in(
        conn: &mut Tx,
        plan: &Plan,
        plans: &[PlanId],
        cause_task: u32,
        command: &str,
    ) -> Result<Option<String>, RunsError> {
        let superseded = Self::superseded_in(conn, plans).await?;
        // Every done task of the project, in any plan and version: a done task is in the
        // project when one of its attempts ran there, and one with no attempt has no checks.
        let marks: Vec<(String, i64, u32, Option<String>)> = sqlx::query_as(
            "SELECT m.workflow_id,m.version,m.task,m.judged_attempt_id FROM run_task_marks m
             WHERE m.done=1 AND EXISTS (SELECT 1 FROM run_attempts a
               WHERE a.workflow_id=m.workflow_id AND a.version=m.version AND a.task=m.task
               AND a.project_id=?)
             ORDER BY m.workflow_id,m.version,m.task",
        )
        .bind(plan.project_id.as_str())
        .fetch_all(&mut *conn)
        .await?;
        let mut pending = Vec::new();
        for (workflow, version, task, judged) in marks {
            let workflow = WorkflowId::from_recorded(workflow);
            if (workflow == plan.id && task == cause_task)
                || superseded.contains(&(workflow.as_str().to_string(), task))
            {
                continue;
            }
            let attempt =
                match judged {
                    Some(id) => Some(id),
                    None => sqlx::query_scalar::<_, String>(
                        "SELECT id FROM run_attempts WHERE workflow_id=? AND version=? AND task=?
                     AND commit_sha IS NOT NULL ORDER BY number DESC LIMIT 1",
                    )
                    .bind(workflow.as_str())
                    .bind(version)
                    .bind(i64::from(task))
                    .fetch_optional(&mut *conn)
                    .await?,
                };
            let Some(attempt) = attempt else {
                continue;
            };
            let json: Option<String> = sqlx::query_scalar(
                "SELECT context_json FROM run_attempt_context WHERE attempt_id=?",
            )
            .bind(&attempt)
            .fetch_optional(&mut *conn)
            .await?;
            let Some(json) = json else {
                continue;
            };
            let context: TaskContext = serde_json::from_str(&json)?;
            if let Some(contract) = context.task.contract {
                let own = contract.checks.len();
                for (ordinal, check) in contract.checks.into_iter().enumerate() {
                    pending.push((
                        attempt.clone(),
                        workflow.clone(),
                        version,
                        task,
                        ordinal,
                        check,
                    ));
                }
                // The contract check is rechecked when the judged attempt had one, after the
                // task's own checks.
                let contract_row: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM run_checks
                     WHERE attempt_id=? AND for_task=? AND check_id=?)",
                )
                .bind(&attempt)
                .bind(i64::from(task))
                .bind(CONTRACT_CHECK)
                .fetch_one(&mut *conn)
                .await?;
                if contract_row {
                    let check = TaskCheck {
                        id: CONTRACT_CHECK.into(),
                        run: vec![],
                        folder: None,
                        timeout_secs: None,
                        proves: vec![],
                    };
                    pending.push((attempt.clone(), workflow.clone(), version, task, own, check));
                }
            }
        }
        if pending.is_empty() {
            return Ok(None);
        }
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO run_rechecks (id,project_id,plan_id,cause_workflow_id,cause_version,
             cause_task,cause_command,created_at) VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(&id)
        .bind(plan.project_id.as_str())
        .bind(plan.plan_id.as_str())
        .bind(plan.id.as_str())
        .bind(plan.version)
        .bind(i64::from(cause_task))
        .bind(command)
        .bind(Storage::now())
        .execute(&mut *conn)
        .await?;
        for (attempt, workflow, version, task, ordinal, check) in pending {
            sqlx::query(
                "INSERT INTO run_recheck_results (recheck_id,attempt_id,workflow_id,version,
                 task,check_id,ordinal,run_json,folder) VALUES (?,?,?,?,?,?,?,?,?)",
            )
            .bind(&id)
            .bind(attempt)
            .bind(workflow.as_str())
            .bind(version)
            .bind(i64::from(task))
            .bind(&check.id)
            .bind(ordinal as i64)
            .bind(serde_json::to_string(&check)?)
            .bind(&check.folder)
            .execute(&mut *conn)
            .await?;
        }
        Ok(Some(id))
    }

    pub async fn recheck_batch(&self, id: &str) -> Result<Batch, RunsError> {
        Ok(sqlx::query_as(
            "SELECT id,cause_workflow_id,cause_version,cause_task,finished_at,notice_json,
             notice_sent_at FROM run_rechecks WHERE id=?",
        )
        .bind(id)
        .fetch_one(self.storage.reader())
        .await?)
    }

    pub async fn recheck_rows(&self, id: &str) -> Result<Vec<ResultRow>, RunsError> {
        Ok(sqlx::query_as(
            "SELECT attempt_id,workflow_id,version,task,check_id,run_json,outcome,exit_code,
             head,output,truncated,started_at,finished_at FROM run_recheck_results
             WHERE recheck_id=? ORDER BY workflow_id,version,task,ordinal,attempt_id",
        )
        .bind(id)
        .fetch_all(self.storage.reader())
        .await?)
    }

    /// The done tasks a newer version of their plan marks done too: the newest mark stands
    /// for the task, so an older version's checks are not run again.
    async fn superseded_in(
        conn: &mut Tx,
        plans: &[PlanId],
    ) -> Result<BTreeSet<(String, u32)>, RunsError> {
        let mut plan_of = BTreeMap::<String, String>::new();
        for plan in plans {
            for (workflow, _) in Plans::version_ids_in(conn, plan)
                .await
                .map_err(|e| RunsError::Unavailable(e.to_string()))?
            {
                plan_of.insert(workflow.as_str().to_string(), plan.as_str().to_string());
            }
        }
        let marks: Vec<(String, i64, u32)> =
            sqlx::query_as("SELECT workflow_id,version,task FROM run_task_marks WHERE done=1")
                .fetch_all(&mut *conn)
                .await?;
        let mut newest = BTreeMap::<(&str, u32), i64>::new();
        for (workflow, version, task) in &marks {
            if let Some(plan) = plan_of.get(workflow) {
                let entry = newest.entry((plan.as_str(), *task)).or_insert(*version);
                *entry = (*entry).max(*version);
            }
        }
        Ok(marks
            .iter()
            .filter(|(workflow, version, task)| {
                plan_of
                    .get(workflow)
                    .and_then(|plan| newest.get(&(plan.as_str(), *task)))
                    .is_some_and(|newest| newest > version)
            })
            .map(|(workflow, _, task)| (workflow.clone(), *task))
            .collect())
    }

    /// Whether the same check of the same task did not pass either in the latest earlier batch
    /// that ran it: such a failure was told already, and is told again only after a pass.
    /// A `contract` row that could not run is skipped, so a server down once hides nothing.
    pub async fn failed_before(&self, batch: &str, row: &ResultRow) -> Result<bool, RunsError> {
        let previous: Option<Option<String>> = sqlx::query_scalar(
            "SELECT r.outcome FROM run_recheck_results r
             JOIN run_rechecks b ON b.id=r.recheck_id
             JOIN run_rechecks cur ON cur.id=?
             WHERE r.workflow_id=? AND r.task=? AND r.check_id=? AND r.finished_at IS NOT NULL
               AND NOT (r.check_id='contract' AND r.outcome='not_run')
               AND (b.created_at<cur.created_at OR (b.created_at=cur.created_at AND b.id<cur.id))
             ORDER BY b.created_at DESC,b.id DESC LIMIT 1",
        )
        .bind(batch)
        .bind(&row.workflow_id)
        .bind(i64::from(row.task))
        .bind(&row.check_id)
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(previous.is_some_and(|outcome| outcome.as_deref() != Some("passed")))
    }

    pub async fn start_recheck(
        &self,
        batch: &str,
        attempt: &AttemptId,
        check: &TaskCheck,
    ) -> Result<(), RunsError> {
        let (batch, attempt, check) = (batch.to_string(), attempt.clone(), check.id.clone());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_recheck_results SET started_at=?
                WHERE recheck_id=? AND attempt_id=? AND check_id=? AND finished_at IS NULL",
                    )
                    .bind(Storage::now())
                    .bind(batch)
                    .bind(attempt.as_str())
                    .bind(check)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn finish_recheck(
        &self,
        batch: &str,
        attempt: &AttemptId,
        check: &str,
        result: Finished,
    ) -> Result<(), RunsError> {
        let (batch, attempt, check) = (batch.to_string(), attempt.clone(), check.to_string());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_recheck_results SET outcome=?,exit_code=?,head=?,output=?,
                truncated=?,foreign_leftovers=?,finished_at=?
                WHERE recheck_id=? AND attempt_id=? AND check_id=?",
                    )
                    .bind(serde_json::to_value(result.outcome)?.as_str().unwrap())
                    .bind(result.exit_code)
                    .bind(result.head)
                    .bind(result.output)
                    .bind(result.truncated)
                    .bind(result.foreign_leftovers)
                    .bind(Storage::now())
                    .bind(batch)
                    .bind(attempt.as_str())
                    .bind(check)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn finish_batch(&self, id: &str, notice: Option<String>) -> Result<(), RunsError> {
        let id = id.to_string();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_rechecks SET finished_at=?,notice_json=?
                WHERE id=? AND finished_at IS NULL",
                    )
                    .bind(Storage::now())
                    .bind(notice)
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn recheck_sent(&self, id: &str) -> Result<(), RunsError> {
        let id = id.to_string();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_rechecks SET notice_sent_at=COALESCE(notice_sent_at,?)
                WHERE id=?",
                    )
                    .bind(Storage::now())
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn unfinished_rechecks(&self) -> Result<Vec<String>, RunsError> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM run_rechecks WHERE finished_at IS NULL OR
            (notice_json IS NOT NULL AND notice_sent_at IS NULL) ORDER BY created_at,id",
        )
        .fetch_all(self.storage.reader())
        .await?)
    }

    pub async fn interrupt_recheck(&self, id: &str) -> Result<(), RunsError> {
        let id = id.to_string();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_recheck_results SET outcome='not_run',
                output='Shadows restarted during the recheck',finished_at=?
                WHERE recheck_id=? AND finished_at IS NULL",
                    )
                    .bind(Storage::now())
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}
