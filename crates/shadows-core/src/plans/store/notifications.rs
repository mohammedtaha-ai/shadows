//! Dependency invalidations committed with the plan change that caused them.

use std::collections::BTreeSet;

use sqlx::SqliteConnection;

use crate::db::{StorageError, append_event};
use crate::events::{Actor, DurableEvent};
use crate::plans::{Link, PlanId, TaskParent};
use crate::projects::ProjectId;

pub(super) async fn notify_plan(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    plan: &PlanId,
    previous: &[Link],
    actor: &Actor,
    ts: &str,
) -> Result<(), StorageError> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT p.project_id FROM task_plan_parent e JOIN workflow w ON w.id=e.workflow_id
           JOIN plan p ON p.id=w.plan_id JOIN project q ON q.id=p.project_id
          WHERE e.parent_plan_id=? AND q.removed_at IS NULL AND w.version=(
            SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
         UNION
         SELECT target.project_id FROM task_plan_parent e JOIN workflow w ON w.id=e.workflow_id
           JOIN plan target ON target.id=e.parent_plan_id
           JOIN project q ON q.id=target.project_id
          WHERE w.plan_id=? AND q.removed_at IS NULL AND w.version=(
            SELECT MAX(version) FROM workflow WHERE plan_id=w.plan_id)",
    )
    .bind(plan.as_str())
    .bind(plan.as_str())
    .fetch_all(&mut *conn)
    .await?;
    let mut affected: BTreeSet<String> = rows.into_iter().collect();
    // Removed edges still invalidate the target's former incoming preview.
    for link in previous {
        if let TaskParent::Plan { plan_id, .. } = &link.after
            && let Some((target, _, _)) = super::graph::owner(conn, plan_id).await?
        {
            affected.insert(target.to_string());
        }
    }
    publish(
        conn,
        project,
        affected,
        actor,
        ts,
        serde_json::json!({ "plan_id": plan }),
    )
    .await
}

/// Code link writes and Projects removal invalidate connected projects in their write.
pub(crate) async fn notify_project_dependencies_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    actor: &Actor,
    ts: &str,
) -> Result<(), StorageError> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT p.project_id FROM task_plan_parent e JOIN workflow w ON w.id=e.workflow_id
           JOIN plan p ON p.id=w.plan_id JOIN project q ON q.id=p.project_id
           JOIN plan target ON target.id=e.parent_plan_id
          WHERE target.project_id=? AND q.removed_at IS NULL AND w.version=(
            SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
         UNION
         SELECT target.project_id FROM task_plan_parent e JOIN workflow w ON w.id=e.workflow_id
           JOIN plan p ON p.id=w.plan_id JOIN plan target ON target.id=e.parent_plan_id
           JOIN project q ON q.id=target.project_id
          WHERE p.project_id=? AND q.removed_at IS NULL AND w.version=(
            SELECT MAX(version) FROM workflow WHERE plan_id=p.id)",
    )
    .bind(project.as_str())
    .bind(project.as_str())
    .fetch_all(&mut *conn)
    .await?;
    publish(
        conn,
        project,
        rows.into_iter().collect(),
        actor,
        ts,
        serde_json::json!({ "related_project_id": project }),
    )
    .await
}

async fn publish(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    mut affected: BTreeSet<String>,
    actor: &Actor,
    ts: &str,
    payload: serde_json::Value,
) -> Result<(), StorageError> {
    affected.remove(project.as_str());
    for target in affected {
        let event = DurableEvent::new("PlanDependenciesChanged", actor.clone())
            .with_project(&ProjectId::from_stored(target))
            .with_payload(payload.clone());
        append_event(conn, &event, ts).await?;
    }
    Ok(())
}
