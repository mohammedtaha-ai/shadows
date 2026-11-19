//! Rechecking the recorded checks of other done tasks after a done mark.
use crate::checks::CONTRACT_CHECK;
use crate::git::Git;
use crate::notify::NoticeIntent;
use crate::store::checks::Finished;
use crate::store::rechecks::{Batch, ResultRow};
use crate::{
    Attempt, AttemptId, CheckOutcome, CheckResult, RecheckFailure, RecheckNotice, Runs, RunsError,
};
use core_plans::{TaskCheck, WorkflowId};
use std::collections::BTreeMap;

impl Runs {
    pub(crate) async fn recheck_done(&self, id: String) {
        if let Err(error) = self.run_recheck(&id).await {
            tracing::warn!(%id, %error, "runs.recheck_failed");
        }
    }

    async fn run_recheck(&self, id: &str) -> Result<(), RunsError> {
        let batch = self.inner.store.recheck_batch(id).await?;
        if batch.finished_at.is_none() {
            let rows = self.inner.store.recheck_rows(id).await?;
            if let Some(first) = rows.first() {
                let attempt = self
                    .inner
                    .store
                    .get(&AttemptId::from_recorded(first.attempt_id.clone()))
                    .await?;
                self.wait_for_quiet(&attempt).await?;
                let lock = self.checking_lock(&attempt.project_id).await;
                let held = lock.lock().await;
                self.wait_for_quiet(&attempt).await?;
                for row in rows.iter().filter(|row| row.finished_at.is_none()) {
                    let attempt = self
                        .inner
                        .store
                        .get(&AttemptId::from_recorded(row.attempt_id.clone()))
                        .await?;
                    let check: TaskCheck = serde_json::from_str(&row.run_json)?;
                    let result = if check.id == CONTRACT_CHECK {
                        self.recheck_contract(id, &attempt, row.task, &check)
                            .await?
                    } else {
                        self.run_check(&attempt, row.task, 0, &check, Some(id))
                            .await?
                    };
                    self.inner
                        .store
                        .finish_recheck(id, &attempt.id, &check.id, result)
                        .await?;
                }
                drop(held);
            }
            self.end_recheck(&batch).await?;
        }
        self.deliver_recheck(id).await
    }

    /// The judged attempt's contract, verified again as the folder stands now, under the
    /// recheck's own verdict key.
    async fn recheck_contract(
        &self,
        id: &str,
        attempt: &Attempt,
        task: u32,
        check: &TaskCheck,
    ) -> Result<Finished, RunsError> {
        self.inner
            .store
            .start_recheck(id, &attempt.id, check)
            .await?;
        let contract = self
            .inner
            .store
            .context(&attempt.id)
            .await?
            .and_then(|(context, _)| context.task.contract)
            .ok_or_else(|| RunsError::Unavailable("the attempt has no contract".into()))?;
        let verdict = self
            .inner
            .code_record
            .verify(
                &format!("recheck:{id}:{}", attempt.id.as_str()),
                &attempt.project_id,
                task,
                &contract,
            )
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?;
        let outcome = Self::contract_outcome(verdict.outcome());
        Ok(Finished {
            outcome,
            exit_code: None,
            head: Git::at(&self.folder(&attempt.project_id).await?)
                .head()
                .await,
            output: verdict.lines(),
            truncated: false,
            foreign_leftovers: false,
        })
    }

    async fn end_recheck(&self, batch: &Batch) -> Result<(), RunsError> {
        let rows = self.inner.store.recheck_rows(&batch.id).await?;
        // Rows come in plan version, task and check order; each attempt's failures stay together.
        let mut failed = Vec::<RecheckFailure>::new();
        let mut plans = BTreeMap::<String, core_plans::Plan>::new();
        let mut lines = Vec::new();
        // A `contract` row that could not run is recorded and never told.
        for row in rows.iter().filter(|row| {
            row.outcome.as_deref() != Some("passed")
                && !(row.check_id == CONTRACT_CHECK && row.outcome.as_deref() == Some("not_run"))
        }) {
            // Every check runs and is recorded; a failure already told stays untold until
            // the check has passed in between.
            if self.inner.store.failed_before(&batch.id, row).await? {
                continue;
            }
            if !plans.contains_key(&row.workflow_id) {
                let plan = self
                    .read_plan(&WorkflowId::from_recorded(row.workflow_id.clone()))
                    .await?;
                plans.insert(row.workflow_id.clone(), plan);
            }
            let plan = &plans[&row.workflow_id];
            let mut check = Self::recheck_result(row)?;
            let output = check.output.as_deref().unwrap_or_default();
            lines.push(format!(
                "Plan \"{}\" version {}: {}\n{}",
                plan.title,
                row.version,
                Self::check_line(&check, 0, "the executor"),
                Self::output_tail(output)
            ));
            check.output = None;
            if failed
                .last()
                .is_none_or(|last| last.attempt_id.as_str() != row.attempt_id)
            {
                let title = plan
                    .tasks
                    .iter()
                    .find(|task| task.content.number == row.task)
                    .map(|task| task.content.title.clone())
                    .unwrap_or_else(|| format!("T{}", row.task));
                failed.push(RecheckFailure {
                    workflow_id: plan.id.clone(),
                    version: row.version,
                    task: row.task,
                    title,
                    attempt_id: AttemptId::from_recorded(row.attempt_id.clone()),
                    checks: Vec::new(),
                });
            }
            failed.last_mut().unwrap().checks.push(check);
        }
        let notice = if failed.is_empty() {
            None
        } else if let Some(recipient) = self
            .recipient_for(&WorkflowId::from_recorded(batch.cause_workflow_id.clone()))
            .await?
        {
            let card = RecheckNotice {
                plan_id: recipient.plan.plan_id,
                cause_workflow_id: WorkflowId::from_recorded(batch.cause_workflow_id.clone()),
                cause_version: batch.cause_version,
                cause_task: batch.cause_task,
                recheck_id: batch.id.clone(),
                failed,
            };
            let intent = NoticeIntent {
                planner: recipient.thread,
                body: format!(
                    "[Shadows] After T{} of version {} was marked done, Shadows reran \
                    the checks of the project's done tasks. These no longer pass:\n{}\nTheir done \
                    marks stand. Read the change with task_diff, then amend or plan a correction.",
                    batch.cause_task,
                    batch.cause_version,
                    lines.join("\n")
                ),
                model: recipient.invocation.requested_model,
                mode: recipient.invocation.requested_mode,
                effort: recipient.invocation.requested_effort,
                card: serde_json::to_value(card)?,
            };
            Some(serde_json::to_string(&intent)?)
        } else {
            tracing::warn!(id=%batch.id, "runs.recheck_no_planner");
            None
        };
        self.inner.store.finish_batch(&batch.id, notice).await
    }

    fn recheck_result(row: &ResultRow) -> Result<CheckResult, RunsError> {
        let check: TaskCheck = serde_json::from_str(&row.run_json)?;
        let outcome: Option<CheckOutcome> = row
            .outcome
            .as_ref()
            .map(|outcome| serde_json::from_value(serde_json::Value::String(outcome.clone())))
            .transpose()?;
        Ok(CheckResult {
            for_task: row.task,
            id: row.check_id.clone(),
            run: check.run,
            folder: check.folder,
            reported: None,
            reported_note: None,
            outcome,
            matches: None,
            exit_code: row.exit_code,
            head_before: row.head.clone(),
            head_after: row.head.clone(),
            output: row.output.clone(),
            truncated: row.truncated,
            started_at: row.started_at.clone(),
            finished_at: row.finished_at.clone(),
        })
    }

    async fn deliver_recheck(&self, id: &str) -> Result<(), RunsError> {
        let batch = self.inner.store.recheck_batch(id).await?;
        if batch.notice_sent_at.is_some() {
            return Ok(());
        }
        if let Some(json) = batch.notice_json {
            let intent: NoticeIntent = serde_json::from_str(&json)?;
            let (thread, turn, notice) = intent.into_notice(format!("recheck-{id}"), None);
            self.inner
                .turns
                .queue_notice(thread, turn, notice)
                .await
                .map_err(|error| RunsError::Unavailable(error.to_string()))?;
            self.inner.store.recheck_sent(id).await?;
        }
        Ok(())
    }

    pub(crate) async fn recover_rechecks(&self, ids: Vec<String>) {
        for id in ids {
            let recovered = async {
                let batch = self.inner.store.recheck_batch(&id).await?;
                if batch.finished_at.is_none() {
                    self.inner.store.interrupt_recheck(&id).await?;
                    self.end_recheck(&batch).await?;
                }
                self.deliver_recheck(&id).await
            }
            .await;
            if let Err(error) = recovered {
                tracing::warn!(%id, %error, "runs.recheck_recovery_failed");
            }
        }
    }
}
