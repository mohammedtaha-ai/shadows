//! One job: the Planner's calls on its project's tasks.
//!
//! The Planner starts a task and marks it done through the same `start` and
//! `mark_done` the HTTP routes call; only its grant is checked here. After a
//! done mark it hears which task to start next, and starts it on its own.
use crate::diff::MAX_LINES;
use crate::model::{Attempt, AttemptFile, AttemptId, DoneMark, FileDiff};
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};
use core_plans::{Plan, WorkflowId};

impl Runs {
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

    /// Starts task `task` of `workflow` for the Planner, on `model` or, left
    /// out, the task's suggested model. Start's own checks and answers.
    pub async fn start_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        model: Option<String>,
    ) -> Result<Attempt, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let picked = model
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty());
        let model = match picked {
            Some(model) => model,
            None => self
                .suggested_model(&plan)
                .await
                .ok_or(RunsError::NoModel)?,
        };
        let command = format!("planner-start-{}", AttemptId::generate());
        self.start(command, workflow, plan.version, task, model)
            .await
    }

    /// Marks task `task` of `workflow` done or not done for the Planner. A done
    /// mark answers the task to start next, if any.
    pub async fn mark_done_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        done: bool,
    ) -> Result<DoneMark, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let command = format!("planner-mark-{}", AttemptId::generate());
        let run = self
            .mark_done(command, workflow, plan.version, task, done)
            .await?;
        let next = if done {
            self.next_ready(&plan).await?
        } else {
            None
        };
        Ok(DoneMark { task: run, next })
    }

    /// The lowest-numbered task with no done mark and no attempt that Start
    /// allows now. A task that already ran is never offered: a failure goes
    /// to the person, and a done task is never started twice.
    async fn next_ready(&self, plan: &Plan) -> Result<Option<u32>, RunsError> {
        let mut numbers: Vec<u32> = plan.tasks.iter().map(|t| t.content.number).collect();
        numbers.sort_unstable();
        for number in numbers {
            let marked = self
                .inner
                .store
                .mark(&plan.id, plan.version, number)
                .await?
                .is_some_and(|m| m.0);
            if marked {
                continue;
            }
            if !self
                .attempts(&plan.id, plan.version, number)
                .await?
                .is_empty()
            {
                continue;
            }
            if self.start_answer(plan, number).await?.allowed {
                return Ok(Some(number));
            }
        }
        Ok(None)
    }
}
