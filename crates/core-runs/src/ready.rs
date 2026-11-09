//! One job: whether a task's `needs` links have been met.
use crate::model::Need;
use crate::{Runs, RunsError};
use core_plans::{LinkKind, Plan, TaskParent};

impl Runs {
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
