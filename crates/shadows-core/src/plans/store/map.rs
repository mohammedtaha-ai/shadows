//! A captured map of Active project plans with directly related foreign plans.

use std::collections::BTreeSet;

use sqlx::SqliteConnection;

use super::read::{parse_plan_state, parse_state};
use crate::db::{Storage, StorageError};
use crate::plans::{MapLink, MapPlan, PlanId, PlanMap, WorkflowId};
use crate::projects::ProjectId;

impl Storage {
    pub async fn plan_map(&self, project: &ProjectId) -> Result<PlanMap, StorageError> {
        let mut snapshot = self.reader().begin().await?;
        let live: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM project WHERE id=? AND removed_at IS NULL)",
        )
        .bind(project.as_str())
        .fetch_one(&mut *snapshot)
        .await?;
        if !live {
            return Err(StorageError::NotFound("project"));
        }
        let roots: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM plan WHERE project_id=? AND state='Active' ORDER BY id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *snapshot)
        .await?;
        let mut ids: BTreeSet<_> = roots.into_iter().collect();
        type EdgeRow = (String, String, i64, bool);
        let edges: Vec<EdgeRow> = sqlx::query_as(
            "WITH roots AS (SELECT id FROM plan WHERE project_id=? AND state='Active')
             SELECT p.id, target.id, COUNT(*), MAX(CASE
               WHEN q.removed_at IS NOT NULL OR tq.removed_at IS NOT NULL
                 OR (p.project_id<>target.project_id AND NOT EXISTS(
                   SELECT 1 FROM project_link l WHERE l.project_id=p.project_id
                     AND l.linked_project_id=target.project_id))
                 OR NOT EXISTS(SELECT 1 FROM task t JOIN workflow tw ON tw.id=t.workflow_id
                   WHERE tw.plan_id=target.id AND t.number=e.parent_task
                     AND tw.version=(SELECT MAX(version) FROM workflow WHERE plan_id=target.id))
               THEN 1 ELSE 0 END)
               FROM task_plan_parent e JOIN workflow w ON w.id=e.workflow_id
               JOIN plan p ON p.id=w.plan_id JOIN project q ON q.id=p.project_id
               JOIN plan target ON target.id=e.parent_plan_id
               JOIN project tq ON tq.id=target.project_id
              WHERE w.version=(SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
                AND (p.id IN (SELECT id FROM roots) OR target.id IN (SELECT id FROM roots))
              GROUP BY p.id, target.id ORDER BY p.id, target.id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *snapshot)
        .await?;
        let mut links = Vec::with_capacity(edges.len());
        for (source, target, count, broken) in edges {
            ids.insert(source.clone());
            ids.insert(target.clone());
            links.push(MapLink {
                plan_id: PlanId::from_stored(source),
                after: PlanId::from_stored(target),
                count: count_into(count)?,
                broken,
            });
        }
        let mut plans = Vec::with_capacity(ids.len());
        for id in ids {
            plans.push(node(&mut snapshot, &PlanId::from_stored(id)).await?);
        }
        snapshot.commit().await?;
        Ok(PlanMap {
            project_id: project.clone(),
            plans,
            links,
        })
    }
}

type NodeRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<String>,
    i64,
    bool,
);

async fn node(conn: &mut SqliteConnection, plan: &PlanId) -> Result<MapPlan, StorageError> {
    let row: NodeRow = sqlx::query_as(
        "SELECT p.project_id, q.name, p.state, w.id, w.title, w.goal, w.version, w.state,
               (SELECT COUNT(*) FROM task WHERE workflow_id=w.id), q.removed_at IS NOT NULL
               FROM plan p JOIN project q ON q.id=p.project_id
               LEFT JOIN workflow w ON w.plan_id=p.id AND w.version=(
                 SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
              WHERE p.id=?",
    )
    .bind(plan.as_str())
    .fetch_one(conn)
    .await?;
    let (project, name, plan_state, workflow, title, goal, version, state, count, removed) = row;
    Ok(MapPlan {
        plan_id: plan.clone(),
        project_id: ProjectId::from_stored(project),
        project_name: name,
        plan_state: parse_plan_state(&plan_state)?,
        workflow_id: workflow.map(WorkflowId::from_stored),
        title: title.unwrap_or_else(|| format!("Plan {plan}")),
        goal: goal.unwrap_or_default(),
        version,
        state: state.as_deref().map(parse_state).transpose()?,
        task_count: count_into(count)?,
        removed,
    })
}

fn count_into(count: i64) -> Result<u32, StorageError> {
    u32::try_from(count).map_err(|_| StorageError::Constraint("plan map count exceeds u32".into()))
}
