//! One job: the Planner's calls on its project's tasks.
//!
//! The Planner starts a task and marks it done through the same `start` and
//! `mark_done` the HTTP routes call; only its grant is checked here. After a
//! done mark it hears which group of tasks may start next, and starts them on
//! its own.
use crate::diff::MAX_LINES;
use crate::model::{Attempt, AttemptFile, AttemptId, DoneMark, FileDiff};
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};
use core_plans::{Plan, WorkflowId};

impl Runs {
    /// Lists held scopes throughout the plan's version history.
    pub async fn live_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
    ) -> Result<Vec<crate::LiveAttempt>, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let versions = self
            .inner
            .plans
            .plan(&plan.plan_id)
            .await
            .map_err(RunsError::from_plans)?;
        let workflows = versions
            .versions
            .into_iter()
            .map(|v| v.workflow_id)
            .collect::<Vec<_>>();
        let mut live = Vec::new();
        for (attempt, conflict) in self.inner.store.live(&workflows).await? {
            let version = self.read_plan(&attempt.workflow_id).await?;
            live.push(crate::LiveAttempt {
                title: version
                    .tasks
                    .iter()
                    .find(|task| task.content.number == attempt.task)
                    .map(|task| task.content.title.clone())
                    .unwrap_or_else(|| format!("T{}", attempt.task)),
                attempt_id: attempt.id,
                workflow_id: attempt.workflow_id,
                version: attempt.version,
                task: attempt.task,
                number: attempt.number,
                state: attempt.state,
                commit: attempt.commit,
                conflict,
                started_at: attempt.started_at,
            });
        }
        Ok(live)
    }

    /// The newest committed attempt, after the same project grant check as Start.
    async fn planner_commit(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<AttemptId, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        self.attempts(workflow, plan.version, task)
            .await?
            .into_iter()
            .find(|attempt| attempt.commit.is_some())
            .map(|attempt| attempt.id)
            .ok_or(RunsError::NotFound)
    }

    /// The route's diff, with a total patch budget for the Planner's answer.
    pub async fn diff_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<Vec<FileDiff>, RunsError> {
        let attempt = self.planner_commit(grant, workflow, task).await?;
        let mut files = self.diff(&attempt).await?;
        let mut kept = 0;
        let mut full = false;
        for file in &mut files {
            let lines = file.patch.lines().count();
            if full || kept + lines > MAX_LINES {
                full = true;
                file.patch = "[not shown: read it with task_file]".into();
            } else {
                kept += lines;
            }
        }
        Ok(files)
    }

    /// Any file inside the task's writes as its latest committed attempt holds it.
    pub async fn file_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        path: &str,
    ) -> Result<AttemptFile, RunsError> {
        let attempt = self.planner_commit(grant, workflow, task).await?;
        self.file(&attempt, path).await
    }

    /// The plan version `workflow`, when `grant` is a Planner's of its project.
    pub(crate) async fn planner_plan(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
    ) -> Result<Plan, RunsError> {
        if grant.kind != GrantKind::Thread {
            return Err(RunsError::NotFound);
        }
        let plan = self.read_plan(workflow).await?;
        if plan.project_id != grant.project_id {
            return Err(RunsError::NotFound);
        }
        Ok(plan)
    }

    /// Reads Start's answer for one task of a plan in the Planner's project.
    pub async fn start_status_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<crate::StartAnswer, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        if !plan.tasks.iter().any(|item| item.content.number == task) {
            return Err(RunsError::NotFound);
        }
        self.start_answer(&plan, task).await
    }

    /// Starts the task on its saved choice, using Start's admission checks.
    pub async fn start_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<Attempt, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let command = format!("planner-start-{}", AttemptId::generate());
        self.start(command, workflow, plan.version, task).await
    }

    pub async fn set_executor_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        input: crate::ExecutorInput,
    ) -> Result<crate::ExecutorChoice, RunsError> {
        self.planner_plan(grant, workflow).await?;
        self.inner.store.grant_live(grant).await?;
        let command = format!("planner-executor-{}", AttemptId::generate());
        self.save_executor(command, workflow, task, input, Some(grant))
            .await
    }

    /// Confirmation results of the task's newest committed attempt, for its Planner.
    pub async fn checks_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<Vec<crate::CheckResult>, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let id = self
            .inner
            .store
            .latest_committed(workflow, plan.version, task)
            .await?
            .ok_or(RunsError::NotFound)?;
        self.check_results(&id).await
    }

    /// Marks a task for the Planner and answers the group that may start next.
    pub async fn mark_done_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        done: bool,
        reason: Option<String>,
    ) -> Result<DoneMark, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let command = format!("planner-mark-{}", AttemptId::generate());
        let run = self
            .mark_done(command, workflow, plan.version, task, done, reason)
            .await?;
        let next = if done {
            self.ready_group(&plan).await?
        } else {
            Vec::new()
        };
        Ok(DoneMark { task: run, next })
    }
}
