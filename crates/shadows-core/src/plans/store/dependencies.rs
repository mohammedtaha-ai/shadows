//! Persisted links to another plan's latest task (§16.7).

use sqlx::SqliteConnection;

use super::task::stored_kind;
use crate::db::StorageError;
use crate::plans::{Link, LinkKind, PlanId, TaskId, TaskParent, WorkflowId};

pub(super) async fn links_of(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<Vec<Link>, StorageError> {
    type Row = (u32, String, u32, String, String, Option<String>);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT t.number, p.parent_plan_id, p.parent_task, p.kind, p.label, p.waiting_items
           FROM task_plan_parent p JOIN task t ON t.id = p.task_id
          WHERE p.workflow_id = ?
          ORDER BY t.number, p.parent_plan_id, p.parent_task, p.kind",
    )
    .bind(workflow.as_str())
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter()
        .map(|(task, plan, number, kind, label, waiting)| {
            Ok(Link {
                task,
                after: TaskParent::Plan {
                    plan_id: PlanId::from_stored(plan),
                    task: number,
                },
                kind: stored_kind(&kind)?,
                label,
                waiting_items: match waiting {
                    Some(json) => serde_json::from_str(&json)?,
                    None => Vec::new(),
                },
            })
        })
        .collect()
}

pub(super) async fn clear(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM task_plan_parent WHERE workflow_id = ?")
        .bind(workflow.as_str())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub(super) async fn put(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
    task: &TaskId,
    link: &Link,
    plan: &PlanId,
    number: u32,
) -> Result<(), StorageError> {
    let waiting = match link.kind {
        LinkKind::Needs => None,
        LinkKind::CompletesAfter => Some(serde_json::to_string(&link.waiting_items)?),
    };
    sqlx::query(
        "INSERT INTO task_plan_parent
           (workflow_id, task_id, parent_plan_id, parent_task, kind, label, waiting_items)
         VALUES (?,?,?,?,?,?,?)",
    )
    .bind(workflow.as_str())
    .bind(task.as_str())
    .bind(plan.as_str())
    .bind(number)
    .bind(link.kind.as_str())
    .bind(&link.label)
    .bind(waiting)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
