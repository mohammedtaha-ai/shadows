//! One job: a plan version's task and link rows (spec §13.3, §13.15) —
//! the columns a task's content is stored in, reading them back (one task by
//! id or by number included), and making them say what an edit left. The
//! version row itself is `edit.rs`, `draft.rs` and `read.rs`.

use std::collections::HashMap;

use sqlx::SqliteConnection;

use crate::plans::model::{
    AcceptanceItem, Link, LinkKind, PlanContent, PlanTask, TaskContent, TaskId, WorkflowId,
};
use crate::storage::StorageError;

/// `contract_json` as stored: title, goal and acceptance (§13.3).
#[derive(serde::Serialize, serde::Deserialize)]
struct Contract {
    title: String,
    goal: String,
    acceptance: Vec<AcceptanceItem>,
}

/// `scope_json` as stored: the declared scope (§4 `DeclaredScope`).
#[derive(serde::Serialize, serde::Deserialize)]
struct Scope {
    reads: Vec<String>,
    writes: Vec<String>,
}

pub(super) async fn tasks_of(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<Vec<PlanTask>, StorageError> {
    let rows: Vec<(String, u32, String, String)> = sqlx::query_as(
        "SELECT id, number, contract_json, scope_json FROM task
          WHERE workflow_id = ? ORDER BY number",
    )
    .bind(workflow.as_str())
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter()
        .map(|(id, number, contract, scope)| {
            let contract: Contract = serde_json::from_str(&contract)?;
            let scope: Scope = serde_json::from_str(&scope)?;
            Ok(PlanTask {
                id: TaskId::from_stored(id),
                content: TaskContent {
                    number,
                    title: contract.title,
                    goal: contract.goal,
                    reads: scope.reads,
                    writes: scope.writes,
                    acceptance: contract.acceptance,
                },
            })
        })
        .collect()
}

/// The number and title of task `id`, if it is a task of `workflow`.
pub(crate) async fn task_of(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
    id: &TaskId,
) -> Result<Option<(u32, String)>, StorageError> {
    let row: Option<(u32, String)> =
        sqlx::query_as("SELECT number, contract_json FROM task WHERE id = ? AND workflow_id = ?")
            .bind(id.as_str())
            .bind(workflow.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    row.map(|(number, contract)| Ok((number, title(&contract)?)))
        .transpose()
}

/// The id and title of `workflow`'s task numbered `number`, if it has one.
pub(super) async fn task_numbered(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
    number: u32,
) -> Result<Option<(TaskId, String)>, StorageError> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, contract_json FROM task WHERE workflow_id = ? AND number = ?")
            .bind(workflow.as_str())
            .bind(number)
            .fetch_optional(&mut *conn)
            .await?;
    row.map(|(id, contract)| Ok((TaskId::from_stored(id), title(&contract)?)))
        .transpose()
}

fn title(contract_json: &str) -> Result<String, StorageError> {
    Ok(serde_json::from_str::<Contract>(contract_json)?.title)
}

/// In an explicit order: the waiting task's number, the number it waits
/// for, then the kind.
pub(super) async fn links_of(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<Vec<Link>, StorageError> {
    let rows: Vec<(u32, u32, String, String, Option<String>)> = sqlx::query_as(
        "SELECT t.number, a.number, p.kind, p.label, p.waiting_items
           FROM task_parent p
           JOIN task t ON t.id = p.task_id
           JOIN task a ON a.id = p.parent_id
          WHERE p.workflow_id = ?
          ORDER BY t.number, a.number, p.kind",
    )
    .bind(workflow.as_str())
    .fetch_all(&mut *conn)
    .await?;
    rows.into_iter()
        .map(|(task, after, kind, label, waiting)| {
            let kind = match kind.as_str() {
                "needs" => LinkKind::Needs,
                "completes_after" => LinkKind::CompletesAfter,
                other => {
                    return Err(StorageError::Constraint(format!(
                        "unknown link kind: {other}"
                    )));
                }
            };
            Ok(Link {
                task,
                after,
                kind,
                label,
                waiting_items: match waiting {
                    Some(json) => serde_json::from_str(&json)?,
                    None => Vec::new(),
                },
            })
        })
        .collect()
}

/// Makes the version's task and link rows say `after`, given the rows it
/// holds now (`before`): removed tasks go, changed ones are rewritten, new
/// ones get an id, unchanged ones are left alone; the links are replaced,
/// their ends resolved by task number.
pub(super) async fn write_content(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
    before: &[PlanTask],
    after: &PlanContent,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM task_parent WHERE workflow_id = ?")
        .bind(workflow.as_str())
        .execute(&mut *conn)
        .await?;
    let mut ids: HashMap<u32, TaskId> = HashMap::new();
    for old in before {
        let number = old.content.number;
        match after.tasks.get(&number) {
            None => {
                sqlx::query("DELETE FROM task WHERE id = ?")
                    .bind(old.id.as_str())
                    .execute(&mut *conn)
                    .await?;
            }
            Some(new) => {
                if *new != old.content {
                    let (contract, scope) = columns(new)?;
                    sqlx::query(
                        "UPDATE task SET contract_json = ?, scope_json = ?, updated_at = ?
                          WHERE id = ?",
                    )
                    .bind(contract)
                    .bind(scope)
                    .bind(ts)
                    .bind(old.id.as_str())
                    .execute(&mut *conn)
                    .await?;
                }
                ids.insert(number, old.id.clone());
            }
        }
    }
    for (number, task) in &after.tasks {
        if ids.contains_key(number) {
            continue;
        }
        let id = TaskId::generate();
        let (contract, scope) = columns(task)?;
        sqlx::query(
            "INSERT INTO task
               (id, workflow_id, number, contract_json, scope_json, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?)",
        )
        .bind(id.as_str())
        .bind(workflow.as_str())
        .bind(number)
        .bind(contract)
        .bind(scope)
        .bind(ts)
        .bind(ts)
        .execute(&mut *conn)
        .await?;
        ids.insert(*number, id);
    }
    for link in &after.links {
        let end = |n: u32| {
            ids.get(&n).ok_or_else(|| {
                StorageError::Constraint(format!("a link names T{n}, which is not stored"))
            })
        };
        let waiting = match link.kind {
            LinkKind::Needs => None,
            LinkKind::CompletesAfter => Some(serde_json::to_string(&link.waiting_items)?),
        };
        sqlx::query(
            "INSERT INTO task_parent
               (workflow_id, task_id, parent_id, kind, label, waiting_items)
             VALUES (?,?,?,?,?,?)",
        )
        .bind(workflow.as_str())
        .bind(end(link.task)?.as_str())
        .bind(end(link.after)?.as_str())
        .bind(link.kind.as_str())
        .bind(&link.label)
        .bind(waiting)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// A task's `contract_json` and `scope_json` (§13.3).
fn columns(task: &TaskContent) -> Result<(String, String), StorageError> {
    let contract = Contract {
        title: task.title.clone(),
        goal: task.goal.clone(),
        acceptance: task.acceptance.clone(),
    };
    let scope = Scope {
        reads: task.reads.clone(),
        writes: task.writes.clone(),
    };
    Ok((
        serde_json::to_string(&contract)?,
        serde_json::to_string(&scope)?,
    ))
}
