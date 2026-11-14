//! One job: the group of tasks the Planner may start together — tasks of one
//! version that never ran, that Start allows now, and whose writes meet no
//! other task chosen into the group.
use crate::model::{ReadyGroup, ReadyTask};
use crate::scope::Scope;
use crate::{Runs, RunsError};
use core_grants::Grant;
use core_plans::{Plan, WorkflowId};

impl Runs {
    /// The ready group of `workflow` for its project's Planner. A version
    /// that admits no new attempt is refused as Start refuses it; a valid
    /// version with nothing ready answers an empty group.
    pub async fn ready_for(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
    ) -> Result<ReadyGroup, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let execution = self
            .inner
            .plans
            .execution_plan(workflow, plan.version)
            .await
            .map_err(RunsError::from_plans)?;
        if let Some(refusal) = execution.refusal {
            return Err(refusal.into());
        }
        Ok(ReadyGroup {
            workflow_id: workflow.clone(),
            version: plan.version,
            tasks: self.ready_group(&plan).await?,
        })
    }

    /// In task-number order, each task with no done mark and no attempt that
    /// Start allows now, unless its writes may meet a task already chosen.
    /// A task that already ran is never offered: a failure goes to the
    /// Planner's review, and a done task is never started twice. A version
    /// that admits no new attempt has no ready task, since Start refuses all.
    pub(crate) async fn ready_group(&self, plan: &Plan) -> Result<Vec<ReadyTask>, RunsError> {
        let mut contents: Vec<_> = plan.tasks.iter().map(|t| &t.content).collect();
        contents.sort_unstable_by_key(|content| content.number);
        let mut chosen: Vec<&[String]> = Vec::new();
        let mut tasks = Vec::new();
        let mut model = None;
        for content in contents {
            let number = content.number;
            if chosen
                .iter()
                .any(|writes| Scope::may_overlap(&content.writes, writes))
            {
                continue;
            }
            let marked = self
                .inner
                .store
                .mark(&plan.id, plan.version, number)
                .await?
                .is_some_and(|m| m.0);
            if marked
                || !self
                    .attempts(&plan.id, plan.version, number)
                    .await?
                    .is_empty()
                || !self.start_answer(plan, number).await?.allowed
            {
                continue;
            }
            if model.is_none() {
                model = Some(self.suggested_model(plan).await);
            }
            chosen.push(&content.writes);
            tasks.push(ReadyTask {
                task: number,
                title: content.title.clone(),
                suggested_model: model.clone().flatten(),
            });
        }
        Ok(tasks)
    }
}
