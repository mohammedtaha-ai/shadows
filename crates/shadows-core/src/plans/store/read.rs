//! One job: reading plan versions (spec §13.2, §16.2) — one version as a
//! reader sees it, with its writer, a project's plans, a thread's latest, and
//! what the journal recorded a version's commands answered. Its task and link
//! rows are `task.rs`.

use sqlx::SqliteConnection;

use super::task::{links_of, tasks_of};
use crate::db::{Storage, StorageError};
use crate::grants::GrantId;
use crate::plans::model::{
    EditOutcome, LastEdit, Plan, PlanId, PlanListing, PlanState, WorkflowId, WorkflowState,
    WrittenBy,
};
use crate::plans::rules::approval_problems;
use crate::projects::ProjectId;
use crate::threads::ThreadId;

impl Storage {
    pub async fn get_plan(&self, workflow: &WorkflowId) -> Result<Plan, StorageError> {
        let mut conn = self.reader().acquire().await?;
        load_plan(&mut conn, workflow).await
    }

    /// Each plan of `project` by its latest version: the Active ones, or all
    /// of them when `archived`. Ordered by when the plan was created, then by
    /// its id; by `julianday`, because RFC 3339 with trimmed fractional
    /// seconds does not sort in time order as text.
    pub async fn list_plans(
        &self,
        project: &ProjectId,
        archived: bool,
    ) -> Result<Vec<PlanListing>, StorageError> {
        type Row = (String, String, String, String, i64, String, String);
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT p.id, p.state, w.id, w.title, w.version, w.state, w.updated_at
               FROM workflow w
               JOIN plan p ON p.id = w.plan_id
              WHERE p.project_id = ?
                AND (? OR p.state = 'Active')
                AND w.version = (SELECT MAX(version) FROM workflow WHERE plan_id = w.plan_id)
              ORDER BY julianday(p.created_at), p.id",
        )
        .bind(project.as_str())
        .bind(archived)
        .fetch_all(self.reader())
        .await?;
        rows.into_iter()
            .map(
                |(plan, plan_state, id, title, version, state, updated_at)| {
                    Ok(PlanListing {
                        plan_id: PlanId::from_stored(plan),
                        plan_state: parse_plan_state(&plan_state)?,
                        id: WorkflowId::from_stored(id),
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

/// A version's writer columns: its thread, operation and grant.
type Writers = (Option<String>, Option<String>, Option<String>);

type VersionRow = (
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
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(super) async fn load_plan(
    conn: &mut SqliteConnection,
    id: &WorkflowId,
) -> Result<Plan, StorageError> {
    let (
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
        change_reason,
        thread,
        operation,
        grant,
    ): VersionRow = sqlx::query_as(
        "SELECT p.project_id, w.version, w.revision, w.state, w.title, w.goal,
                w.previous_version_id, w.frozen_at, w.created_at, p.id, p.state,
                w.change_reason, w.written_by_thread, w.written_by_operation,
                w.written_by_grant
           FROM workflow w JOIN plan p ON p.id = w.plan_id
          WHERE w.id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("workflow"))?;
    let written_by = written_by(conn, id, (thread, operation, grant)).await?;
    let next: Option<String> =
        sqlx::query_scalar("SELECT id FROM workflow WHERE previous_version_id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    let project_id = ProjectId::from_stored(project);
    let last_edit = edits_of(conn, &project_id, id)
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
        project_id,
        written_by,
        change_reason,
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

/// §16.3: a version with a grant is an external agent's, whether or not a
/// thread stands beside it (the versions 2a wrote have both); one with a
/// thread only is its Planner's, with the model and harness of the turn's
/// invocation when a turn was recorded.
async fn written_by(
    conn: &mut SqliteConnection,
    id: &WorkflowId,
    (thread, operation, grant): Writers,
) -> Result<WrittenBy, StorageError> {
    if let Some(grant) = grant {
        return Ok(WrittenBy::External {
            grant_id: GrantId::from_stored(grant),
        });
    }
    let thread = thread
        .ok_or_else(|| StorageError::Constraint(format!("plan version {id} has no writer")))?;
    let (thread_title, removed_at): (String, Option<String>) =
        sqlx::query_as("SELECT title, removed_at FROM planning_thread WHERE id = ?")
            .bind(&thread)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("planning_thread"))?;
    let invocation: Option<(String, String)> = match operation {
        Some(op) => {
            sqlx::query_as(
                "SELECT COALESCE(observed_model, requested_model), harness_kind
                   FROM agent_invocation WHERE operation_id = ?",
            )
            .bind(op)
            .fetch_optional(&mut *conn)
            .await?
        }
        None => None,
    };
    let (model, harness) = invocation.map_or((None, None), |(m, h)| (Some(m), Some(h)));
    Ok(WrittenBy::Planner {
        thread_id: ThreadId::from_stored(thread),
        thread_title,
        thread_removed: removed_at.is_some(),
        model,
        harness,
    })
}

/// Every `WorkflowEdited` of `workflow`, in journal order, read by its plan's
/// project: an edit names a thread only when a Planner made it (§16.3). Its
/// payload is the edit's [`EditOutcome`].
pub(super) async fn edits_of(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    workflow: &WorkflowId,
) -> Result<Vec<EditOutcome>, StorageError> {
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM durable_event
          WHERE project_id = ? AND kind = 'WorkflowEdited' ORDER BY seq",
    )
    .bind(project.as_str())
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
