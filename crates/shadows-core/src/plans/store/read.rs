//! One job: reading plan versions (spec §13.2) — one version as a reader sees
//! it, a project's plans, a thread's latest, and what the journal recorded a
//! version's commands answered. Its task and link rows are `task.rs`.

use sqlx::SqliteConnection;

use super::task::{links_of, tasks_of};
use crate::db::{Storage, StorageError};
use crate::plans::model::{
    EditOutcome, LastEdit, Plan, PlanId, PlanListing, PlanState, WorkflowId, WorkflowState,
};
use crate::plans::rules::approval_problems;
use crate::projects::ProjectId;
use crate::threads::ThreadId;

impl Storage {
    pub async fn get_plan(&self, workflow: &WorkflowId) -> Result<Plan, StorageError> {
        let mut conn = self.reader().acquire().await?;
        load_plan(&mut conn, workflow).await
    }

    /// Each plan's latest version in `project`, in the order their writing
    /// threads were created (by the `PlanningThreadCreated` event's sequence,
    /// for the reason `list_projects` gives).
    pub async fn list_plans(&self, project: &ProjectId) -> Result<Vec<PlanListing>, StorageError> {
        type Row = (String, String, String, String, String, i64, String, String);
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT p.id, p.state, w.id, w.written_by_thread, w.title, w.version, w.state,
                    w.updated_at
               FROM workflow w
               JOIN plan p ON p.id = w.plan_id
               LEFT JOIN durable_event e
                 ON e.thread_id = w.written_by_thread AND e.kind = 'PlanningThreadCreated'
              WHERE p.project_id = ?
                AND w.version = (SELECT MAX(version) FROM workflow WHERE plan_id = w.plan_id)
              ORDER BY e.seq, w.written_by_thread, p.id",
        )
        .bind(project.as_str())
        .fetch_all(self.reader())
        .await?;
        rows.into_iter()
            .map(
                |(plan, plan_state, id, thread, title, version, state, updated_at)| {
                    Ok(PlanListing {
                        plan_id: PlanId::from_stored(plan),
                        plan_state: parse_plan_state(&plan_state)?,
                        id: WorkflowId::from_stored(id),
                        thread_id: ThreadId::from_stored(thread),
                        title,
                        version,
                        state: parse_state(&state)?,
                        updated_at,
                    })
                },
            )
            .collect()
    }

    /// The thread's latest version, if it has a plan.
    pub async fn thread_plan(&self, thread: &ThreadId) -> Result<Option<WorkflowId>, StorageError> {
        let mut conn = self.reader().acquire().await?;
        match plan_of_thread(&mut conn, thread).await? {
            Some(plan) => latest_version(&mut conn, &plan).await,
            None => Ok(None),
        }
    }
}

/// The plan whose versions `thread` wrote (§16.2): until a Planner reaches
/// its project's plans (Task 3), a thread writes into one plan only.
pub(super) async fn plan_of_thread(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
) -> Result<Option<PlanId>, StorageError> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT plan_id FROM workflow WHERE written_by_thread = ? ORDER BY plan_id LIMIT 1",
    )
    .bind(thread.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(id.map(PlanId::from_stored))
}

pub(super) async fn latest_version(
    conn: &mut SqliteConnection,
    plan: &PlanId,
) -> Result<Option<WorkflowId>, StorageError> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM workflow WHERE plan_id = ? ORDER BY version DESC LIMIT 1",
    )
    .bind(plan.as_str())
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

fn parse_plan_state(s: &str) -> Result<PlanState, StorageError> {
    match s {
        "Active" => Ok(PlanState::Active),
        "Archived" => Ok(PlanState::Archived),
        other => Err(StorageError::Constraint(format!(
            "no such plan state: {other}"
        ))),
    }
}

type VersionRow = (
    Option<String>,
    String,
    i64,
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
);

pub(super) async fn load_plan(
    conn: &mut SqliteConnection,
    id: &WorkflowId,
) -> Result<Plan, StorageError> {
    let (
        thread,
        project,
        version,
        revision,
        state,
        title,
        goal,
        previous,
        frozen_at,
        created_at,
        plan_id,
        plan_state,
    ): VersionRow = sqlx::query_as(
        "SELECT w.written_by_thread, p.project_id, w.version, w.revision, w.state, w.title,
                    w.goal, w.previous_version_id, w.frozen_at, w.created_at, p.id, p.state
               FROM workflow w JOIN plan p ON p.id = w.plan_id
              WHERE w.id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("workflow"))?;
    let thread = thread.ok_or_else(|| {
        StorageError::Constraint(format!("plan version {id} has no writing thread"))
    })?;
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
        plan_id: PlanId::from_stored(plan_id),
        plan_state: parse_plan_state(&plan_state)?,
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
