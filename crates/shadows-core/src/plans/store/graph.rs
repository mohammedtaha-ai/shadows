//! Snapshot checks for dependencies reached through other plans (§16.7).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use sqlx::SqliteConnection;

use super::task::{links_of, tasks_of};
use crate::db::StorageError;
use crate::plans::dependency_graph::DependencyGraph;
use crate::plans::{Plan, PlanContent, PlanId, PlanOp, Problem, TaskParent, WorkflowId};
use crate::projects::ProjectId;

type Owner = (ProjectId, Option<WorkflowId>, Option<String>);

pub(super) async fn check_parents(
    conn: &mut SqliteConnection,
    source: &Plan,
    ops: &[PlanOp],
) -> Result<(), StorageError> {
    for op in ops {
        let PlanOp::LinkPut { link } = op else {
            continue;
        };
        let TaskParent::Plan { plan_id, task } = &link.after else {
            continue;
        };
        let invalid = StorageError::PlanLinkInvalid;
        if *plan_id == source.plan_id {
            return Err(invalid(
                "use a local task number for a link inside this plan".to_string(),
            ));
        }
        let Some((project, version, _)) = owner(conn, plan_id).await? else {
            return Err(invalid(format!("plan {plan_id} T{task} is unavailable")));
        };
        if !crate::code::project_reachable_in(conn, &source.project_id, &project).await? {
            return Err(invalid(format!(
                "a project link to plan {plan_id}'s project is missing"
            )));
        }
        let Some(version) = version else {
            return Err(invalid(format!("plan {plan_id} has no latest version")));
        };
        let known: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM task WHERE workflow_id=? AND number=?)",
        )
        .bind(version.as_str())
        .bind(task)
        .fetch_one(&mut *conn)
        .await?;
        if !known {
            return Err(invalid(format!(
                "plan {plan_id} T{task} is missing from its latest version"
            )));
        }
    }
    Ok(())
}

pub(super) async fn owner(
    conn: &mut SqliteConnection,
    plan: &PlanId,
) -> Result<Option<Owner>, StorageError> {
    let row: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT p.project_id, w.id, w.title FROM plan p
           JOIN project q ON q.id=p.project_id AND q.removed_at IS NULL
           LEFT JOIN workflow w ON w.plan_id=p.id AND w.version=(
             SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
          WHERE p.id=?",
    )
    .bind(plan.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(|(project, version, title)| {
        (
            ProjectId::from_stored(project),
            version.map(WorkflowId::from_stored),
            title,
        )
    }))
}

pub(super) async fn blockers(
    conn: &mut SqliteConnection,
    root: &Plan,
    origin: Option<&ProjectId>,
) -> Result<Vec<Problem>, StorageError> {
    if !root
        .links
        .iter()
        .any(|link| matches!(link.after, TaskParent::Plan { .. }))
    {
        return Ok(Vec::new());
    }
    let mut plans = BTreeMap::from([(
        root.plan_id.to_string(),
        (root.project_id.clone(), root.content()),
    )]);
    let mut titles = BTreeMap::from([(root.plan_id.to_string(), root.title.clone())]);
    let mut pending: VecDeque<_> = root
        .tasks
        .iter()
        .map(|task| (root.plan_id.to_string(), task.content.number))
        .collect();
    let mut reached = BTreeSet::new();
    let mut problems = Vec::new();
    while let Some((id, number)) = pending.pop_front() {
        if !reached.insert((id.clone(), number)) {
            continue;
        }
        let (source_project, links) = {
            let (project, content) = &plans[&id];
            (
                project.clone(),
                content
                    .links
                    .iter()
                    .filter(|link| link.task == number)
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        };
        for link in &links {
            if let TaskParent::Local(after) = link.after {
                pending.push_back((id.clone(), after));
                continue;
            }
            let TaskParent::Plan { plan_id, task } = &link.after else {
                continue;
            };
            let Some((project, version, title)) = owner(conn, plan_id).await? else {
                problems.push(Problem::new(format!(
                    "T{} links to unavailable plan {plan_id} T{task}",
                    link.task
                )));
                continue;
            };
            if !crate::code::project_reachable_in(conn, &source_project, &project).await? {
                problems.push(Problem::new(format!(
                    "T{} needs a project link to plan {plan_id} T{task}",
                    link.task,
                )));
                continue;
            }
            let key = plan_id.to_string();
            if !plans.contains_key(&key) {
                let Some(version) = version else {
                    problems.push(Problem::new(format!(
                        "plan {plan_id} has no version for T{task}"
                    )));
                    continue;
                };
                let tasks = tasks_of(conn, &version)
                    .await?
                    .into_iter()
                    .map(|task| (task.content.number, task.content))
                    .collect();
                let content = PlanContent {
                    title: title.clone().unwrap_or_default(),
                    goal: String::new(),
                    tasks,
                    links: links_of(conn, &version).await?,
                };
                if crate::code::project_reachable_in(
                    conn,
                    origin.unwrap_or(&root.project_id),
                    &project,
                )
                .await?
                {
                    titles.insert(key.clone(), title.unwrap_or_else(|| key.clone()));
                }
                plans.insert(key.clone(), (project, content));
            }
            if !plans[&key].1.tasks.contains_key(task) {
                problems.push(Problem::new(format!(
                    "{} T{task} is missing from its latest version",
                    titles.get(&key).unwrap_or(&key),
                )));
            } else {
                pending.push_back((key, *task));
            }
        }
    }
    let mut graph = DependencyGraph::new();
    for (id, number) in &reached {
        if plans[id].1.tasks.contains_key(number) {
            graph.task(id, *number);
        }
    }
    for (id, (_, content)) in &plans {
        for link in &content.links {
            if reached.contains(&(id.clone(), link.task)) {
                graph.link(id, link);
            }
        }
    }
    if let Some(cycle) = graph.cycle() {
        let names: Vec<_> = cycle
            .iter()
            .map(|(plan, task)| format!("{} T{task}", titles.get(plan).unwrap_or(plan)))
            .collect();
        problems.push(Problem::new(format!(
            "these links form a cycle: {}",
            names.join(", ")
        )));
    }
    Ok(problems)
}
