//! Outgoing/incoming task dependency views for one captured read (§16.7).

use sqlx::SqliteConnection;

use super::{
    graph,
    read::{parse_plan_state, parse_state},
    task::{preview, stored_kind},
};
use crate::db::StorageError;
use crate::plans::{Link, LinkedTask, Plan, PlanId, TaskParent, WorkflowId};
use crate::projects::ProjectId;

pub(super) async fn read(
    conn: &mut SqliteConnection,
    root: &Plan,
    origin: Option<&ProjectId>,
) -> Result<Vec<LinkedTask>, StorageError> {
    let mut result = Vec::new();
    for link in &root.links {
        let TaskParent::Plan { plan_id, task } = &link.after else {
            continue;
        };
        if let Some(mut view) = related(conn, plan_id, *task, None, link, false, origin).await? {
            if let Some(project) = &view.project_id
                && !crate::code::project_reachable_in(conn, &root.project_id, project).await?
            {
                view.broken = Some("the target project's link was removed".into());
            }
            result.push(view);
        }
    }
    type Row = (String, String, u32, u32, String, String, Option<String>);
    let incoming: Vec<Row> = sqlx::query_as(
        "SELECT p.id, w.id, t.number, e.parent_task, e.kind, e.label, e.waiting_items
           FROM task_plan_parent e JOIN workflow w ON w.id=e.workflow_id
           JOIN plan p ON p.id=w.plan_id JOIN task t ON t.id=e.task_id
          WHERE e.parent_plan_id=? AND w.version=(
            SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
          ORDER BY p.id, t.number, e.parent_task, e.kind",
    )
    .bind(root.plan_id.as_str())
    .fetch_all(&mut *conn)
    .await?;
    for (plan, workflow, task, number, kind, label, waiting) in incoming {
        let plan = PlanId::from_stored(plan);
        let link = Link {
            task,
            after: TaskParent::Plan {
                plan_id: root.plan_id.clone(),
                task: number,
            },
            kind: stored_kind(&kind)?,
            label,
            waiting_items: waiting
                .map(|value| serde_json::from_str(&value))
                .transpose()?
                .unwrap_or_default(),
        };
        let workflow = WorkflowId::from_stored(workflow);
        if let Some(mut view) =
            related(conn, &plan, task, Some(&workflow), &link, true, origin).await?
        {
            if let Some((_, Some(latest), _)) = graph::owner(conn, &root.plan_id).await?
                && preview(conn, &latest, number).await?.is_none()
            {
                view.broken = Some(format!(
                    "T{number} is missing from the target's latest version"
                ));
            }
            if let Some(project) = &view.project_id
                && !crate::code::project_reachable_in(conn, project, &root.project_id).await?
            {
                view.broken = Some("the source project's link was removed".into());
            }
            result.push(view);
        }
    }
    Ok(result)
}

async fn related(
    conn: &mut SqliteConnection,
    plan: &PlanId,
    number: u32,
    version: Option<&WorkflowId>,
    link: &Link,
    incoming: bool,
    origin: Option<&ProjectId>,
) -> Result<Option<LinkedTask>, StorageError> {
    let mut view = LinkedTask {
        link: link.clone(),
        incoming,
        plan_id: plan.clone(),
        project_id: None,
        project_name: None,
        workflow_id: None,
        version: None,
        plan_title: None,
        plan_state: None,
        state: None,
        task: None,
        broken: None,
    };
    type Row = (
        String,
        String,
        Option<String>,
        String,
        i64,
        String,
        String,
        String,
    );
    let row: Option<Row> = sqlx::query_as(
        "SELECT p.project_id, q.name, q.removed_at, w.id, w.version, w.title, p.state, w.state
           FROM plan p JOIN project q ON q.id=p.project_id
           JOIN workflow w ON w.plan_id=p.id AND w.id=COALESCE(?, (
             SELECT id FROM workflow WHERE plan_id=p.id ORDER BY version DESC LIMIT 1))
          WHERE p.id=?",
    )
    .bind(version.map(WorkflowId::as_str))
    .bind(plan.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    let Some((project, name, removed, workflow, number_version, title, plan_state, state)) = row
    else {
        view.broken = Some(format!("plan {plan} T{number} is unavailable"));
        return Ok(Some(view));
    };
    let project = ProjectId::from_stored(project);
    if let Some(origin) = origin
        && !crate::code::project_reachable_in(conn, origin, &project).await?
    {
        if incoming {
            return Ok(None);
        }
        view.broken = Some("the target is outside the grant's project links".into());
        return Ok(Some(view));
    }
    view.project_id = Some(project);
    view.project_name = Some(name);
    view.plan_title = Some(title);
    view.plan_state = Some(parse_plan_state(&plan_state)?);
    view.state = Some(parse_state(&state)?);
    view.version = Some(number_version);
    let workflow = WorkflowId::from_stored(workflow);
    if removed.is_some() {
        view.broken = Some("the related project was removed".into());
    } else {
        view.task = preview(conn, &workflow, number).await?;
        if view.task.is_none() {
            view.broken = Some(format!("T{number} is missing from the latest version"));
        }
    }
    view.workflow_id = Some(workflow);
    Ok(Some(view))
}
