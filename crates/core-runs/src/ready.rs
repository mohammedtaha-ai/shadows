//! One job: whether a task may start now — its `needs` links and Start's
//! checks, answered without writing.
use crate::model::{Need, StartAnswer};
use crate::store::NewAttempt;
use crate::{Runs, RunsError};
use core_plans::{LinkKind, Plan, TaskParent, WorkflowId};

impl Runs {
    /// Whether Start may run now, by the same checks in the same order as
    /// Start, without writing. A refusal is an answer; a failure to read is
    /// an error.
    pub async fn can_start(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<StartAnswer, RunsError> {
        let plan = self.plan(workflow, version).await?;
        self.start_answer(&plan, task).await
    }

    pub(crate) async fn start_answer(
        &self,
        plan: &Plan,
        task: u32,
    ) -> Result<StartAnswer, RunsError> {
        let checked = match self.checks(plan, task).await {
            Ok(content) => {
                self.inner
                    .store
                    .check_held(&NewAttempt {
                        command_id: String::new(),
                        fingerprint: String::new(),
                        project: plan.project_id.clone(),
                        workflow: plan.id.clone(),
                        version: plan.version,
                        task,
                        model: String::new(),
                        writes: content.writes,
                        start_head: None,
                    })
                    .await
            }
            Err(error) => Err(error),
        };
        match checked {
            Ok(()) => Ok(StartAnswer {
                allowed: true,
                code: None,
                reason: None,
            }),
            Err(error) if error.refuses_start() => Ok(StartAnswer {
                allowed: false,
                code: Some(error.code().to_string()),
                reason: Some(error.to_string()),
            }),
            Err(error) => Err(error),
        }
    }

    /// The tasks `task` needs, in link order, each with whether the person
    /// marked it done. A link to another plan reads the mark of the version the
    /// link reaches, that plan's latest; one that reaches no task is never done.
    pub(crate) async fn needs(&self, plan: &Plan, task: u32) -> Result<Vec<Need>, RunsError> {
        let mut needs = Vec::new();
        for link in plan
            .links
            .iter()
            .filter(|link| link.task == task && link.kind == LinkKind::Needs)
        {
            let need = match &link.after {
                TaskParent::Local(number) => Need {
                    task: link.after.clone(),
                    title: plan
                        .tasks
                        .iter()
                        .find(|t| t.content.number == *number)
                        .map(|t| t.content.title.clone())
                        .unwrap_or_default(),
                    done: self.marked(&plan.id, plan.version, *number).await?,
                },
                TaskParent::Plan { .. } => {
                    let reached = plan
                        .linked_tasks
                        .iter()
                        .find(|linked| !linked.incoming && linked.link == *link);
                    let target = reached.and_then(|linked| {
                        match (&linked.workflow_id, linked.version, &linked.task) {
                            (Some(workflow), Some(version), Some(preview))
                                if linked.broken.is_none() =>
                            {
                                Some((workflow, version, preview))
                            }
                            _ => None,
                        }
                    });
                    match target {
                        Some((workflow, version, preview)) => Need {
                            task: link.after.clone(),
                            title: preview.title.clone(),
                            done: self.marked(workflow, version, preview.number).await?,
                        },
                        None => Need {
                            task: link.after.clone(),
                            title: reached
                                .and_then(|linked| linked.broken.clone())
                                .unwrap_or_else(|| format!("{} is unavailable", link.after)),
                            done: false,
                        },
                    }
                }
            };
            if !needs.iter().any(|seen: &Need| seen.task == need.task) {
                needs.push(need);
            }
        }
        Ok(needs)
    }

    /// The tasks `task` needs that nobody has marked done, in link order.
    pub(crate) async fn waiting_on(
        &self,
        plan: &Plan,
        task: u32,
    ) -> Result<Vec<TaskParent>, RunsError> {
        Ok(self
            .needs(plan, task)
            .await?
            .into_iter()
            .filter(|need| !need.done)
            .map(|need| need.task)
            .collect())
    }

    async fn marked(
        &self,
        workflow: &core_plans::WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<bool, RunsError> {
        Ok(self
            .inner
            .store
            .mark(workflow, version, task)
            .await?
            .is_some_and(|(done, _)| done))
    }
}
