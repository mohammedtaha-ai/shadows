//! One job: whether a task may start now — the plan's state and its `needs`
//! links, read in one snapshot or inside admission's write, and Start's
//! answer without writing.
use crate::model::{Need, StartAnswer};
use crate::scope::Scope;
use crate::store::{NewAttempt, RunsStore};
use crate::{Runs, RunsError};
use core_db::Tx;
use core_plans::{LinkKind, Plan, Plans, TaskContent, TaskParent, WorkflowId};

impl Runs {
    /// Whether Start may run now, by the same checks in the same order as
    /// Start, without writing. A refusal is an answer; a failure to read is
    /// an error.
    pub(crate) async fn start_answer(
        &self,
        plan: &Plan,
        task: u32,
    ) -> Result<StartAnswer, RunsError> {
        let checked = match self.checks(&plan.id, plan.version, task).await {
            Ok((admitted, content)) => {
                self.inner
                    .store
                    .check_held(&NewAttempt {
                        command_id: String::new(),
                        fingerprint: String::new(),
                        project: admitted.project_id,
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

    /// The durable half of admission, as `tx` sees it, in order: the version
    /// may start work, the task exists, its `writes` are patterns, and every
    /// task it needs is marked done. Start's preflight reads it in a snapshot;
    /// the attempt's insert checks it again inside the same write.
    pub(crate) async fn admit_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<(Plan, TaskContent), RunsError> {
        let execution = Plans::execution_plan_in(tx, workflow, version)
            .await
            .map_err(RunsError::from_plans)?;
        if let Some(refusal) = execution.refusal {
            return Err(refusal.into());
        }
        let plan = execution.plan;
        let content = plan
            .tasks
            .iter()
            .find(|t| t.content.number == task)
            .ok_or(RunsError::NotFound)?
            .content
            .clone();
        Scope::new(&content.writes).map_err(RunsError::BadScope)?;
        let waiting: Vec<TaskParent> = Self::needs_in(tx, &plan, task)
            .await?
            .into_iter()
            .filter(|need| !need.done)
            .map(|need| need.task)
            .collect();
        if !waiting.is_empty() {
            return Err(RunsError::Waiting { tasks: waiting });
        }
        Ok((plan, content))
    }

    /// The tasks `task` needs, in link order, each with whether the person
    /// marked it done.
    pub(crate) async fn needs(&self, plan: &Plan, task: u32) -> Result<Vec<Need>, RunsError> {
        self.inner.store.needs(plan, task).await
    }

    /// `needs`, as `tx` sees the marks. A link to another plan reads the mark
    /// of the version the link reaches, that plan's latest; one that reaches no
    /// task is never done.
    pub(crate) async fn needs_in(
        tx: &mut Tx,
        plan: &Plan,
        task: u32,
    ) -> Result<Vec<Need>, RunsError> {
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
                    done: RunsStore::done_in(tx, &plan.id, plan.version, *number).await?,
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
                            done: RunsStore::done_in(tx, workflow, version, preview.number).await?,
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
}
