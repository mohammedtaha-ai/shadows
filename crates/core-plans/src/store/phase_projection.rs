//! Pinned foreign-phase previews for one plan version.
use super::phase_refs::{PhaseRows, VersionSnapshot};
use crate::{LinkedPhase, PhaseRef, Plan, WorkflowId};
use core_db::StorageError;
use sqlx::SqliteConnection;

pub(super) struct PhaseProjection;
impl PhaseProjection {
    pub(super) async fn read(
        conn: &mut SqliteConnection,
        plan: &Plan,
    ) -> Result<Vec<LinkedPhase>, StorageError> {
        let root = VersionSnapshot::from_plan(plan);
        let workflows: Vec<String> = sqlx::query_scalar(
            "SELECT w.id FROM workflow w JOIN plan p ON p.id=w.plan_id
             WHERE p.project_id=? AND p.id<>? AND w.version=(
                 SELECT MAX(v.version) FROM workflow v WHERE v.plan_id=p.id)
             AND EXISTS (SELECT 1 FROM task t WHERE t.workflow_id=w.id
                 AND json_array_length(t.contract_json,'$.phases')>0)
             ORDER BY p.id",
        )
        .bind(plan.project_id.as_str())
        .bind(plan.plan_id.as_str())
        .fetch_all(&mut *conn)
        .await?;
        let mut sources = vec![root.clone()];
        for workflow in workflows {
            if let Some(source) = PhaseRows::load(conn, &WorkflowId::from_stored(workflow)).await? {
                sources.push(source);
            }
        }
        let mut previews = Vec::new();
        for source in sources {
            for task in source.content.tasks.values() {
                for phase in &task.phases {
                    for reference in &phase.waits_for {
                        let outgoing = source.workflow == root.workflow;
                        let incoming = match reference {
                            PhaseRef::Plan {
                                plan_id, version, ..
                            } => *plan_id == root.plan_id && *version == root.version,
                            PhaseRef::Local { task: target, .. } => {
                                outgoing && *target != task.number
                            }
                        };
                        let foreign = match reference {
                            PhaseRef::Local { task: target, .. } => *target != task.number,
                            PhaseRef::Plan { .. } => true,
                        };
                        if !(outgoing || incoming) || !foreign {
                            continue;
                        }
                        let (target, broken) =
                            match PhaseRows::resolve(conn, &source, reference).await {
                                Ok((target, number, phase)) => {
                                    (Some(target.endpoint(number, &phase)), None)
                                }
                                Err(StorageError::Constraint(reason)) => (None, Some(reason)),
                                Err(error) => return Err(error),
                            };
                        previews.push(LinkedPhase {
                            source: source.endpoint(task.number, phase),
                            reference: reference.clone(),
                            target,
                            broken,
                        });
                    }
                }
            }
        }
        Ok(previews)
    }
}
