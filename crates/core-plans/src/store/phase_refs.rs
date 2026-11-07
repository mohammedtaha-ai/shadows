//! Exact phase target resolution inside the caller snapshot.
use super::task::TaskRows;
use crate::{Phase, PhaseEndpoint, PhaseRef, Plan, PlanContent, PlanId, WorkflowId};
use core_db::StorageError;
use core_projects::ProjectId;
use sqlx::SqliteConnection;

#[derive(Clone)]
pub(super) struct VersionSnapshot {
    pub(super) project: ProjectId,
    pub(super) plan_id: PlanId,
    pub(super) workflow: WorkflowId,
    pub(super) version: i64,
    pub(super) title: String,
    pub(super) content: std::sync::Arc<PlanContent>,
}

impl VersionSnapshot {
    pub(super) fn from_plan(plan: &Plan) -> Self {
        Self {
            project: plan.project_id.clone(),
            plan_id: plan.plan_id.clone(),
            workflow: plan.id.clone(),
            version: plan.version,
            title: plan.title.clone(),
            content: std::sync::Arc::new(plan.content()),
        }
    }
    pub(super) fn endpoint(&self, task: u32, phase: &Phase) -> PhaseEndpoint {
        PhaseEndpoint {
            plan_id: self.plan_id.clone(),
            workflow_id: self.workflow.clone(),
            version: self.version,
            plan_title: self.title.clone(),
            task,
            phase_id: phase.id.clone(),
            phase_number: phase.number,
            title: phase.title.clone(),
        }
    }
}

pub(super) struct PhaseRows;
impl PhaseRows {
    pub(super) async fn load(
        conn: &mut SqliteConnection,
        workflow: &WorkflowId,
    ) -> Result<Option<VersionSnapshot>, StorageError> {
        let row: Option<(String, String, i64, String)> = sqlx::query_as(
            "SELECT p.project_id,p.id,w.version,w.title FROM workflow w
             JOIN plan p ON p.id=w.plan_id WHERE w.id=?",
        )
        .bind(workflow.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        let Some((project, plan, version, title)) = row else {
            return Ok(None);
        };
        let project = ProjectId::from_recorded(project);
        if !core_projects::Projects::is_live_in(conn, &project).await? {
            return Ok(None);
        }
        Ok(Some(VersionSnapshot {
            project,
            plan_id: PlanId::from_stored(plan),
            workflow: workflow.clone(),
            version,
            title: title.clone(),
            content: std::sync::Arc::new(PlanContent {
                title,
                goal: String::new(),
                bindings: Vec::new(),
                tasks: TaskRows::tasks_of(conn, workflow)
                    .await?
                    .into_iter()
                    .map(|t| (t.content.number, t.content))
                    .collect(),
                links: TaskRows::links_of(conn, workflow).await?,
            }),
        }))
    }

    pub(super) async fn resolve(
        conn: &mut SqliteConnection,
        source: &VersionSnapshot,
        reference: &PhaseRef,
    ) -> Result<(VersionSnapshot, u32, Phase), StorageError> {
        let (target, task, id) = match reference {
            PhaseRef::Local { task, phase } => (source.clone(), *task, phase),
            PhaseRef::Plan {
                plan_id,
                version,
                task,
                phase,
            } => {
                let name = format!("plan {plan_id} v{version} T{task} phase {phase}");
                if *plan_id == source.plan_id {
                    return Err(StorageError::Constraint(format!(
                        "{name}: use a local phase reference inside this plan"
                    )));
                }
                let workflow: Option<String> = sqlx::query_scalar(
                    "SELECT w.id FROM workflow w JOIN plan p ON p.id=w.plan_id
                     WHERE p.id=? AND p.project_id=? AND w.version=? AND w.state='Frozen'",
                )
                .bind(plan_id.as_str())
                .bind(source.project.as_str())
                .bind(version)
                .fetch_optional(&mut *conn)
                .await?;
                let target = if let Some(workflow) = workflow {
                    Self::load(conn, &WorkflowId::from_stored(workflow)).await?
                } else {
                    None
                };
                let target = target.ok_or_else(|| {
                    StorageError::Constraint(format!(
                        "{name}: target is unavailable or not approved in this project"
                    ))
                })?;
                (target, *task, phase)
            }
        };
        let phase = target
            .content
            .tasks
            .get(&task)
            .and_then(|t| t.phases.iter().find(|p| p.id == *id))
            .cloned()
            .ok_or_else(|| {
                StorageError::Constraint(format!(
                    "plan {} v{}: missing T{task} phase {id}",
                    target.plan_id, target.version
                ))
            })?;
        Ok((target, task, phase))
    }
}
