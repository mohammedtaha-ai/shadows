//! One job: reading plan versions (spec §13.2) — one version as a reader sees
//! it, a project's plans, a thread's latest, and what the journal recorded a
//! version's commands answered. Its task and link rows are `task.rs`.

use sqlx::SqliteConnection;

use super::task::{links_of, tasks_of};
use super::{Storage, StorageError};
use crate::project::ProjectId;
use crate::thread::ThreadId;
use crate::workflow::{
    EditOutcome, LastEdit, Plan, PlanListing, WorkflowId, WorkflowState, approval_problems,
};

impl Storage {
    pub async fn get_plan(&self, workflow: &WorkflowId) -> Result<Plan, StorageError> {
        let mut conn = self.reader().acquire().await?;
        load_plan(&mut conn, workflow).await
    }

    /// Each thread's latest version in `project`, in the order the threads
    /// were created (by their `PlanningThreadCreated` event's sequence, for
    /// the reason `list_projects` gives).
    pub async fn list_plans(&self, project: &ProjectId) -> Result<Vec<PlanListing>, StorageError> {
        let rows: Vec<(String, String, String, i64, String, String)> = sqlx::query_as(
            "SELECT w.id, w.thread_id, w.title, w.version, w.state, w.updated_at
               FROM workflow w
               JOIN planning_thread t ON t.id = w.thread_id
               LEFT JOIN durable_event e
                 ON e.thread_id = t.id AND e.kind = 'PlanningThreadCreated'
              WHERE t.project_id = ?
                AND w.version = (SELECT MAX(version) FROM workflow WHERE thread_id = w.thread_id)
              ORDER BY e.seq, t.id",
        )
        .bind(project.as_str())
        .fetch_all(self.reader())
        .await?;
        rows.into_iter()
            .map(|(id, thread, title, version, state, updated_at)| {
                Ok(PlanListing {
                    id: WorkflowId::from_stored(id),
                    thread_id: ThreadId::from_stored(thread),
                    title,
                    version,
                    state: parse_state(&state)?,
                    updated_at,
                })
            })
            .collect()
    }

    /// The thread's latest version, if it has a plan.
    pub async fn thread_plan(&self, thread: &ThreadId) -> Result<Option<WorkflowId>, StorageError> {
        let mut conn = self.reader().acquire().await?;
        latest_version(&mut conn, thread).await
    }
}

pub(super) async fn latest_version(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
) -> Result<Option<WorkflowId>, StorageError> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM workflow WHERE thread_id = ? ORDER BY version DESC LIMIT 1",
    )
    .bind(thread.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(id.map(WorkflowId::from_stored))
}

fn parse_state(s: &str) -> Result<WorkflowState, StorageError> {
    match s {
        "Draft" => Ok(WorkflowState::Draft),
        "Frozen" => Ok(WorkflowState::Frozen),
        other => Err(StorageError::Constraint(format!(
            "a plan state this milestone does not use: {other}"
        ))),
    }
}

type VersionRow = (
    String,
    String,
    i64,
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
);

pub(super) async fn load_plan(
    conn: &mut SqliteConnection,
    id: &WorkflowId,
) -> Result<Plan, StorageError> {
    let (thread, project, version, revision, state, title, goal, previous, frozen_at, created_at): VersionRow =
        sqlx::query_as(
            "SELECT w.thread_id, t.project_id, w.version, w.revision, w.state, w.title, w.goal,
                    w.previous_version_id, w.frozen_at, w.created_at
               FROM workflow w JOIN planning_thread t ON t.id = w.thread_id
              WHERE w.id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("workflow"))?;
    let next: Option<String> =
        sqlx::query_scalar("SELECT id FROM workflow WHERE previous_version_id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    let thread_id = ThreadId::from_stored(thread);
    let last_edit = edits_of(conn, &thread_id, id)
        .await?
        .into_iter()
        .max_by_key(|e| e.revision)
        .map(|e| LastEdit {
            revision: e.revision,
            summary: e.summary,
            changed_tasks: e.changed_tasks,
        });
    let mut plan = Plan {
        id: id.clone(),
        thread_id,
        project_id: ProjectId::from_stored(project),
        version,
        revision,
        state: parse_state(&state)?,
        title,
        goal,
        previous: previous.map(WorkflowId::from_stored),
        next: next.map(WorkflowId::from_stored),
        tasks: tasks_of(conn, id).await?,
        links: links_of(conn, id).await?,
        blockers: Vec::new(),
        last_edit,
        frozen_at,
        created_at,
    };
    if plan.state == WorkflowState::Draft {
        plan.blockers = approval_problems(&plan.content());
    }
    Ok(plan)
}

/// Every `WorkflowEdited` of `workflow`, in journal order. Its payload is the
/// edit's [`EditOutcome`].
pub(super) async fn edits_of(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
    workflow: &WorkflowId,
) -> Result<Vec<EditOutcome>, StorageError> {
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM durable_event
          WHERE thread_id = ? AND kind = 'WorkflowEdited' ORDER BY seq",
    )
    .bind(thread.as_str())
    .fetch_all(&mut *conn)
    .await?;
    let mut edits = Vec::new();
    for payload in payloads {
        let edit: EditOutcome = serde_json::from_str(&payload)?;
        if &edit.workflow_id == workflow {
            edits.push(edit);
        }
    }
    Ok(edits)
}

/// The payload of the event a command recorded as its outcome: what the
/// command answered when it committed (§13.5).
pub(super) async fn recorded_outcome<T: serde::de::DeserializeOwned>(
    conn: &mut SqliteConnection,
    event_id: &str,
) -> Result<T, StorageError> {
    let payload: String =
        sqlx::query_scalar("SELECT payload_json FROM durable_event WHERE event_id = ?")
            .bind(event_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("durable_event"))?;
    Ok(serde_json::from_str(&payload)?)
}
