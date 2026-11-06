//! Approved task context assembled in one caller snapshot.
use super::{PlansStore, read::ReadRows};
use crate::scope::Scope;
use crate::{ContextPlan, ContextStandards, Plans, PlansError, TaskContext, TaskPart, TaskPath};
use crate::{WorkflowId, WorkflowState};
use core_design::Design;
use core_grants::{Grant, Grants};

impl PlansStore {
    pub(crate) async fn task_context(
        &self,
        grant: &Grant,
        workflow: Option<&WorkflowId>,
        number: u32,
    ) -> Result<TaskContext, PlansError> {
        let workflow = Scope::named(workflow)?;
        let mut tx = self.storage.reader().begin().await?;
        if !Plans::workflow_in_project_in(&mut tx, workflow, &grant.project_id).await? {
            return Err(PlansError::GrantScope(
                "that plan is not in this grant's project".into(),
            ));
        }
        Grants::check_writer(&mut tx, &Scope::writer_of(grant)?, &grant.project_id).await?;
        let plan = ReadRows::load_plan(&mut tx, workflow, Some(&grant.project_id)).await?;
        if plan.state != WorkflowState::Frozen {
            return Err(PlansError::InvalidCommand(
                "task_context needs an approved plan version".into(),
            ));
        }
        let task = plan
            .tasks
            .iter()
            .find(|t| t.content.number == number)
            .ok_or_else(|| {
                PlansError::InvalidCommand(format!("T{number} does not exist in this plan"))
            })?
            .content
            .clone();
        let tied = Plans::plan_parts_in(&mut tx, &plan.project_id, &plan.plan_id).await?;
        let layout = Design::layout_in(&mut tx, &plan.project_id).await?;
        let mut paths = Vec::new();
        for path in task.reads.iter().chain(&task.writes) {
            if !paths.contains(path) {
                paths.push(path.clone());
            }
        }
        let claims = paths
            .iter()
            .map(|p| (p.clone(), layout.owners(p)))
            .collect::<Vec<_>>();
        let unowned_paths = claims
            .iter()
            .filter(|(_, owners)| owners.is_empty())
            .map(|(p, _)| p.clone())
            .collect();
        let parts = layout
            .parts
            .into_iter()
            .filter_map(|part| {
                let paths = claims
                    .iter()
                    .flat_map(|(path, owners)| {
                        owners
                            .iter()
                            .filter(|o| o.part == part.id)
                            .map(|o| TaskPath {
                                path: path.clone(),
                                via: o.via.clone(),
                            })
                    })
                    .collect::<Vec<_>>();
                (tied.contains(&part.id) || !paths.is_empty()).then_some(TaskPart { part, paths })
            })
            .collect();
        let context = TaskContext {
            plan: ContextPlan {
                plan_id: plan.plan_id,
                workflow_id: plan.id,
                version: plan.version,
                title: plan.title,
            },
            task,
            parts,
            unowned_paths,
            standards: ContextStandards {
                base: layout.standards_base,
                additions: layout.standards_additions,
            },
            map_revision: layout.map_revision,
        };
        tx.commit().await?;
        Ok(context)
    }
}
