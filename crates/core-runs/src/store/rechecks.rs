//! Durable recheck batches and their per-check results.
use super::{RunsStore, checks::Finished};
use crate::{AttemptId, RunsError};
use core_db::{Storage, Tx};
use core_plans::{Plan, Plans, TaskCheck, TaskContext};

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
        cause_task: u32,
        command: &str,
    ) -> Result<Option<String>, RunsError> {
        let versions = Plans::version_ids_in(conn, &plan.plan_id)
            .await
            .map_err(RunsError::from_plans)?;
        let mut pending = Vec::new();
        for (workflow, version) in versions {
            let marks: Vec<(u32, Option<String>)> = sqlx::query_as(
                "SELECT task,judged_attempt_id FROM run_task_marks
                 WHERE workflow_id=? AND version=? AND done=1 ORDER BY task",
            )
            .bind(workflow.as_str())
            .bind(version)
            .fetch_all(&mut *conn)
            .await?;
            for (task, judged) in marks {
                if workflow == plan.id && task == cause_task {
                    continue;
                }
                let attempt = match judged {
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
             WHERE recheck_id=? ORDER BY version,task,ordinal,attempt_id",
        )
        .bind(id)
        .fetch_all(self.storage.reader())
        .await?)
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
