//! One job: carrying an attempt on with the Planner's addition.
//!
//! An addition is a new attempt on the same executor thread: the committed
//! attempt waiting for the Planner's judgment is released into it, and its
//! turn runs through the same `run_turn` Start uses. The plan version never
//! changes. A task takes `ADDITIONS` additions after its Start; the next one
//! stops the task, which goes back to the person.
use crate::git::Git;
use crate::store::Continued;
use crate::{Attempt, Runs, RunsError};
use core_grants::Grant;
use core_plans::WorkflowId;

/// How many additions a task takes after its Start (Mohammed, 2026-11-11).
pub(crate) const ADDITIONS: u32 = 3;

/// The turn an addition sends the executor.
const ADDITION: &str = "The Planner reviewed your commit {sha} for task T{n} and asks for more:

{addition}

Do it inside the task's writes. When it is done, commit your own files again with git and call \
task_report with the new commit's sha and an honest report of what you did, ran and found.";

impl Runs {
    /// Sends the executor of task `task` the Planner's `addition`: the
    /// committed attempt waiting for judgment carries on as a new attempt in
    /// the same session. The fourth addition after a Start stops the task.
    pub async fn amend(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        addition: String,
    ) -> Result<Attempt, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let addition = addition.trim().to_string();
        if addition.is_empty() {
            return Err(RunsError::EmptyAddition);
        }
        let (awaiting, _) = self
            .inner
            .store
            .awaiting(workflow, plan.version, task)
            .await?
            .ok_or(RunsError::NotAwaiting { task })?;
        let spent = self
            .inner
            .store
            .additions_since_start(workflow, plan.version, task)
            .await?;
        if spent >= ADDITIONS {
            let stopped = self.stop(&awaiting).await?;
            tracing::info!(%workflow, task, "runs.additions_spent");
            self.record_lesson(&plan, &stopped).await;
            return Err(RunsError::AdditionsSpent { task });
        }
        let previous = self.inner.store.get(&awaiting).await?;
        let start_head = Git::at(&self.folder(&plan.project_id).await?).head().await;
        let next = self
            .inner
            .store
            .continue_attempt(
                &awaiting,
                Continued {
                    addition: addition.clone(),
                    start_head,
                },
            )
            .await?;
        let prompt = ADDITION
            .replace("{sha}", previous.commit.as_deref().unwrap_or_default())
            .replace("{n}", &task.to_string())
            .replace("{addition}", &addition);
        self.run_turn(next, prompt).await
    }
}
