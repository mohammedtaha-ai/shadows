//! Rechecking the recorded checks of the project's done tasks: after the done mark that
//! completes a plan, or when the person or a Planner asks.
use crate::checks::CONTRACT_CHECK;
use crate::git::Git;
use crate::notify::NoticeIntent;
use crate::store::checks::Finished;
use crate::store::rechecks::{Batch, ResultRow};
use crate::{
    Attempt, AttemptId, CheckOutcome, CheckResult, LastRecheck, ProjectRecheck, RecheckBegan,
    RecheckFailure, RecheckNotice, Runs, RunsError,
};
use core_db::CommandContext;
use core_plans::{TaskCheck, WorkflowId};
use core_threads::ThreadId;
use std::collections::BTreeMap;

impl Runs {
    /// Starts a recheck of the project of plan version `workflow`, asked for by the person
    /// (`asker` none: its notice goes to that plan's Planner) or by a Planner (its thread hears
    /// the notice). The batch runs in the background as a done mark's does; the same command
    /// answers the same batch.
    pub async fn recheck_project(
        &self,
        command: String,
        workflow: &WorkflowId,
        asker: Option<ThreadId>,
    ) -> Result<ProjectRecheck, RunsError> {
        let plan = self.read_plan(workflow).await?;
        let plans = self
            .inner
            .plans
            .list(&plan.project_id, true)
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?
            .into_iter()
            .map(|listing| listing.plan_id)
            .collect();
        let input = serde_json::json!({
            "workflow": workflow,
            "asker": asker.as_ref().map(|thread| thread.as_str().to_string()),
        });
        let command = CommandContext::user(command, "run.recheck", input);
        let (id, fresh) = self
            .inner
            .store
            .ask_recheck(command, plan, plans, asker)
            .await?;
        if let Some(batch) = id.clone().filter(|_| fresh) {
            let runs = self.clone();
            tokio::spawn(async move {
                runs.recheck_done(batch).await;
            });
        }
        Ok(ProjectRecheck { recheck_id: id })
    }

    /// The newest recheck batch of the project of plan version `workflow`, with its rows
    /// counted by outcome; none when the project has had no recheck.
    pub async fn last_recheck(
        &self,
        workflow: &WorkflowId,
    ) -> Result<Option<LastRecheck>, RunsError> {
        let plan = self.read_plan(workflow).await?;
        let Some((batch, counts)) = self
            .inner
            .store
            .last_recheck(plan.project_id.as_str())
            .await?
        else {
            return Ok(None);
        };
        let began = match (batch.asked, batch.asker_thread_id) {
            (true, Some(thread_id)) => RecheckBegan::Planner { thread_id },
            (true, None) => RecheckBegan::Person,
            (false, _) => {
                let cause = WorkflowId::from_recorded(batch.cause_workflow_id);
                let plan_title = if &cause == workflow {
                    plan.title
                } else {
                    self.read_plan(&cause).await?.title
                };
                RecheckBegan::Mark {
                    plan_title,
                    version: batch.cause_version,
                    task: batch.cause_task,
                }
            }
        };
        Ok(Some(LastRecheck {
            recheck_id: batch.id,
            began,
            created_at: batch.created_at,
            finished_at: batch.finished_at,
            counts,
        }))
    }

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
                        self.live_started(id, row.task, &attempt.id, &check.id);
                        self.recheck_contract(id, &attempt, row.task, &check)
                            .await?
                    } else {
                        self.run_check(&attempt, row.task, 0, &check, Some(id))
                            .await?
                    };
                    let outcome = result.outcome;
                    self.inner
                        .store
                        .finish_recheck(id, &attempt.id, &check.id, result)
                        .await?;
                    self.live_finished(id, outcome);
                }
                drop(held);
            }
            self.end_recheck(&batch).await?;
            self.live_ended(id);
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
                &format!("recheck:{}", attempt.id.as_str()),
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
        let failing: Vec<_> = rows
            .iter()
            .filter(|row| {
                row.outcome.as_deref() != Some("passed")
                    && !(row.check_id == CONTRACT_CHECK
                        && row.outcome.as_deref() == Some("not_run"))
            })
            .collect();
        for row in failing.iter().copied() {
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
            let check = Self::recheck_result(row)?;
            let output = check.output.as_deref().unwrap_or_default();
            lines.push(format!(
                "Plan \"{}\" version {}: {}\n{}",
                plan.title,
                row.version,
                Self::check_line(&check, 0, "the executor"),
                Self::output_excerpt(&row.check_id, output)
            ));
            let check = Self::card_check(check);
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
        // A batch tells a new failure; an asked one also tells its asker that every check
        // passed, since nothing else would answer it.
        let all_passed = batch.asked && failing.is_empty();
        // The batch ends whoever can be told: a Planner that cannot be told yet (no turn whose
        // settings a notice takes) is told nothing, as a plan with no Planner is, and the
        // failing rows stay readable on the plan page.
        let recipient = if failed.is_empty() && !all_passed {
            None
        } else {
            self.recheck_recipient(batch).await.unwrap_or_else(|error| {
                tracing::warn!(id=%batch.id, %error, "runs.recheck_untold");
                None
            })
        };
        let notice = if failed.is_empty() && !all_passed {
            None
        } else if let Some(recipient) = recipient {
            let why = match (batch.asked, &batch.asker_thread_id) {
                (false, _) => format!(
                    "After T{} of version {} was marked done and completed its plan",
                    batch.cause_task, batch.cause_version
                ),
                (true, Some(_)) => "As you asked".to_string(),
                (true, None) => "As the person asked".to_string(),
            };
            let body = if all_passed {
                // A `contract` row that could not run is not a pass; say it was not run.
                let unchecked = rows
                    .iter()
                    .filter(|row| row.outcome.as_deref() != Some("passed"))
                    .count();
                let passed = match unchecked {
                    0 => "Every check passed.".to_string(),
                    n => format!(
                        "Every check that ran passed; the contract check of {n} task(s) \
                        could not run."
                    ),
                };
                format!(
                    "[Shadows] {why}, Shadows reran the checks of the project's done tasks. \
                    {passed}"
                )
            } else {
                format!(
                    "[Shadows] {why}, Shadows reran the checks of the project's done tasks. \
                    These no longer pass:\n{}\nTheir done marks stand. Read the change with \
                    task_diff, then amend or plan a correction.",
                    lines.join("\n")
                )
            };
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
                body,
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

    /// Who hears a batch: the asking Planner's thread, else the cause plan's Planner.
    async fn recheck_recipient(
        &self,
        batch: &Batch,
    ) -> Result<Option<crate::notify::Recipient>, RunsError> {
        let workflow = WorkflowId::from_recorded(batch.cause_workflow_id.clone());
        let Some(thread) = &batch.asker_thread_id else {
            return self.recipient_for(&workflow).await;
        };
        let thread = ThreadId::from_recorded(thread.clone());
        let plan = self.read_plan(&workflow).await?;
        let invocation = self.last_invocation(&thread).await.ok_or_else(|| {
            RunsError::Unavailable("the Planner has no turn whose settings a notice takes".into())
        })?;
        Ok(Some(crate::notify::Recipient {
            plan,
            thread,
            invocation,
        }))
    }

    pub(crate) fn recheck_result(row: &ResultRow) -> Result<CheckResult, RunsError> {
        let check: TaskCheck = serde_json::from_str(&row.run_json)?;
        let outcome: Option<CheckOutcome> = row
            .outcome
            .as_ref()
            .map(|outcome| serde_json::from_value(serde_json::Value::String(outcome.clone())))
            .transpose()?;
        Ok(CheckResult {
            for_task: row.task,
            id: row.check_id.clone(),
            by_shadows: row.check_id == CONTRACT_CHECK,
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

    /// A batch a restart cut short runs its unfinished rows now, keeping the finished ones,
    /// then ends and delivers as it would have; a finished batch only delivers its notice.
    pub(crate) async fn recover_rechecks(&self, ids: Vec<String>) {
        for id in ids {
            if let Err(error) = self.run_recheck(&id).await {
                tracing::warn!(%id, %error, "runs.recheck_recovery_failed");
            }
        }
    }
}
