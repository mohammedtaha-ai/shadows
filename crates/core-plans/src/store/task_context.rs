//! Approved task context assembled in one caller snapshot.
use super::{PlansStore, read::ReadRows};
use crate::scope::Scope;
use crate::{ContextPlan, ContextStandards, Plans, PlansError, TaskContext, TaskPart, TaskPath};
use crate::{WorkflowId, WorkflowState};
use core_db::Tx;
use core_design::Design;
use core_grants::{Grant, Grants};
use core_projects::ProjectId;

impl PlansStore {
    pub(crate) async fn task_context(
        &self,
        grant: &Grant,
        workflow: Option<&WorkflowId>,
        number: u32,
        phase_id: Option<&crate::PhaseId>,
    ) -> Result<TaskContext, PlansError> {
        let workflow = &Scope::named_or_own(grant, workflow)?;
        let mut tx = self.storage.reader().begin().await?;
        if !Plans::workflow_in_project_in(&mut tx, workflow, &grant.project_id).await? {
            return Err(PlansError::GrantScope(
                "that plan is not in this grant's project".into(),
            ));
        }
        // An executor grant reads exactly its own task: it writes no plan, so
        // there is no writer to check.
        if let Some(scope) = &grant.scope {
            if scope.workflow != workflow.as_str() || scope.task != number {
                return Err(PlansError::OutsideGrant);
            }
            let plan = ReadRows::load_plan(&mut tx, workflow, Some(&grant.project_id)).await?;
            if scope.version != plan.version {
                return Err(PlansError::OutsideGrant);
            }
        } else {
            Grants::check_writer(&mut tx, &Scope::writer_of(grant)?, &grant.project_id).await?;
        }
        let context =
            Self::task_context_in(&mut tx, &grant.project_id, workflow, number, phase_id).await?;
        tx.commit().await?;
        Ok(context)
    }

    /// The context of task `number` of the approved version `workflow` of
    /// `project`, its phase `phase_id` when named, as `tx` sees it.
    pub(crate) async fn task_context_in(
        tx: &mut Tx,
        project: &ProjectId,
        workflow: &WorkflowId,
        number: u32,
        phase_id: Option<&crate::PhaseId>,
    ) -> Result<TaskContext, PlansError> {
        let plan = ReadRows::load_plan(tx, workflow, Some(project)).await?;
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
        let contracts = crate::task_contract::ContractRules::resolved(&plan.content(), number);
        let phase = phase_id
            .map(|id| {
                task.phases
                    .iter()
                    .find(|p| p.id == *id)
                    .cloned()
                    .ok_or_else(|| {
                        PlansError::InvalidCommand(format!("T{number} has no phase {id}"))
                    })
            })
            .transpose()?;
        let mut phase_dependencies = Vec::new();
        if let Some(phase) = &phase {
            use super::phase_refs::{PhaseRows, VersionSnapshot};
            let source = VersionSnapshot::from_plan(&plan);
            for reference in &phase.waits_for {
                let (target, phase, broken) = match PhaseRows::resolve(tx, &source, reference).await
                {
                    Ok((target, number, phase)) => {
                        (Some(target.endpoint(number, &phase)), Some(phase), None)
                    }
                    Err(core_db::StorageError::Constraint(reason)) => (None, None, Some(reason)),
                    Err(error) => return Err(error.into()),
                };
                phase_dependencies.push(crate::PhaseDependency {
                    reference: reference.clone(),
                    target,
                    phase,
                    broken,
                });
            }
        }
        let tied = Plans::plan_parts_in(tx, &plan.project_id, &plan.plan_id).await?;
        let layout = Design::layout_in(tx, &plan.project_id).await?;
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
        Ok(TaskContext {
            phase,
            phase_dependencies,
            plan: ContextPlan {
                plan_id: plan.plan_id,
                workflow_id: plan.id,
                version: plan.version,
                title: plan.title,
            },
            task,
            contracts,
            parts,
            unowned_paths,
            standards: ContextStandards {
                base: layout.standards_base,
                additions: layout.standards_additions,
            },
            map_revision: layout.map_revision,
        })
    }
}
