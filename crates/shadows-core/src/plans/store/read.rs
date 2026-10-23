//! One job: reading plan versions — one version as a
//! reader sees it, with its writer, a project's plans, a thread's latest, and
//! what the journal recorded a version's commands answered. Its task and link
//! rows are `task.rs`.

use super::PlansStore;
use sqlx::SqliteConnection;

use super::task::TaskRows;
use crate::db::StorageError;
use crate::grants::GrantId;
use crate::plans::model::{
    EditOutcome, LastEdit, Plan, PlanId, PlanListing, PlanState, WorkflowId, WorkflowState,
    WrittenBy,
};
use crate::plans::rules::Rules;
use crate::projects::ProjectId;
use crate::threads::ThreadId;

impl PlansStore {
    pub async fn get_plan(&self, workflow: &WorkflowId) -> Result<Plan, crate::CoreError> {
        self.get_plan_scoped(workflow, None, None).await
    }

    pub async fn get_plan_scoped(
        &self,
        workflow: &WorkflowId,
        origin: Option<&ProjectId>,
        project: Option<&str>,
    ) -> Result<Plan, crate::CoreError> {
        let mut snapshot = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        if let Some(origin) = origin {
            let selected =
                crate::code::Code::selected_project_in(&mut snapshot, origin, project).await?;
            let owner: Option<String> = sqlx::query_scalar(
                "SELECT p.project_id FROM workflow w JOIN plan p ON p.id=w.plan_id WHERE w.id=?",
            )
            .bind(workflow.as_str())
            .fetch_optional(&mut *snapshot)
            .await
            .map_err(StorageError::from)?;
            if owner.as_deref() != Some(selected.as_str()) {
                return Err(crate::grants::GrantsError::GrantScope.into());
            }
        }
        let mut plan = ReadRows::load_plan(&mut snapshot, workflow, origin).await?;
        plan.linked_tasks = super::linked::LinkedRows::read(&mut snapshot, &plan, origin).await?;
        snapshot.commit().await.map_err(StorageError::from)?;
        Ok(plan)
    }

    pub async fn list_plans_scoped(
        &self,
        origin: &ProjectId,
        archived: bool,
        project: Option<&str>,
    ) -> Result<Vec<PlanListing>, crate::CoreError> {
        let mut snapshot = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        let selected =
            crate::code::Code::selected_project_in(&mut snapshot, origin, project).await?;
        let plans = listings(&mut snapshot, &selected, archived).await?;
        snapshot.commit().await.map_err(StorageError::from)?;
        Ok(plans)
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
        let mut conn = self.storage.reader().acquire().await?;
        listings(&mut conn, project, archived).await
    }

    /// The latest version of the first plan the thread wrote, if any. Tests
    /// use it to find a conversation's plan; the product names plans by id.
    #[cfg(feature = "test-support")]
    pub async fn thread_plan(&self, thread: &ThreadId) -> Result<Option<WorkflowId>, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        match plan_of_thread(&mut conn, thread).await? {
            Some(plan) => ReadRows::latest_version(&mut conn, &plan).await,
            None => Ok(None),
        }
    }
}

async fn listings(
    conn: &mut SqliteConnection,
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
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(
            |(plan, plan_state, id, title, version, state, updated_at)| {
                Ok(PlanListing {
                    plan_id: PlanId::from_stored(plan),
                    plan_state: ReadRows::parse_plan_state(&plan_state)?,
                    id: WorkflowId::from_stored(id),
                    title,
                    version,
                    state: ReadRows::parse_state(&state)?,
                    updated_at,
                })
            },
        )
        .collect()
}

/// The first plan whose versions `thread` wrote. Only `thread_plan` uses it.
#[cfg(feature = "test-support")]
async fn plan_of_thread(
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

impl ReadRows {
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
}

impl ReadRows {
    pub(super) fn parse_state(s: &str) -> Result<WorkflowState, StorageError> {
        match s {
            "Draft" => Ok(WorkflowState::Draft),
            "Frozen" => Ok(WorkflowState::Frozen),
            other => Err(StorageError::Constraint(format!(
                "a plan state this milestone does not use: {other}"
            ))),
        }
    }
}

impl ReadRows {
    pub(super) fn parse_plan_state(s: &str) -> Result<PlanState, StorageError> {
        match s {
            "Active" => Ok(PlanState::Active),
            "Archived" => Ok(PlanState::Archived),
            other => Err(StorageError::Constraint(format!(
                "no such plan state: {other}"
            ))),
        }
    }
}

/// A version's writer columns: its thread, operation and grant.
pub(super) type Writers = (Option<String>, Option<String>, Option<String>);

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

impl ReadRows {
    pub(super) async fn load_plan(
        conn: &mut SqliteConnection,
        id: &WorkflowId,
        origin: Option<&ProjectId>,
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
        let written_by = ReadRows::written_by(conn, id, (thread, operation, grant)).await?;
        let next: Option<String> =
            sqlx::query_scalar("SELECT id FROM workflow WHERE previous_version_id = ?")
                .bind(id.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        let project_id = ProjectId::from_stored(project);
        let last_edit = ReadRows::edits_of(conn, &project_id, id)
            .await?
            .into_iter()
            .max_by_key(|e| e.revision)
            .map(|e| LastEdit {
                revision: e.revision,
                summary: e.summary,
                changed_tasks: e.changed_tasks,
            });
        let mut plan = Plan {
            approved_by: {
                let (principal, agent, request): (Option<String>, Option<String>, Option<String>) =
                    sqlx::query_as(
                        "SELECT approved_principal,approved_agent,approved_request_entry
                    FROM workflow WHERE id=?",
                    )
                    .bind(id.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                crate::grants::Grants::read_in(conn, principal, agent, request).await?
            },
            id: id.clone(),
            plan_id: PlanId::from_stored(plan_id),
            plan_state: ReadRows::parse_plan_state(&plan_state)?,
            project_id,
            written_by,
            change_reason,
            version,
            revision,
            state: ReadRows::parse_state(&state)?,
            title,
            goal,
            previous: previous.map(WorkflowId::from_stored),
            next: next.map(WorkflowId::from_stored),
            tasks: TaskRows::tasks_of(conn, id).await?,
            links: TaskRows::links_of(conn, id).await?,
            blockers: Vec::new(),
            linked_tasks: Vec::new(),
            bindings: super::bindings::BindingsRows::load(conn, id).await?,
            last_edit,
            frozen_at,
            created_at,
        };
        if plan.state == WorkflowState::Draft {
            plan.blockers = Rules::approval_problems(&plan.content());
            plan.blockers
                .extend(super::graph::GraphRows::blockers(conn, &plan, origin).await?);
        }
        Ok(plan)
    }
}

/// Who wrote a version: one with a grant is an external agent's, whether or not a
/// thread stands beside it (the versions 2a wrote have both); one with a
/// thread only is its Planner's, with the model and harness of the turn's
/// invocation when a turn was recorded.
impl ReadRows {
    pub(super) async fn written_by(
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
        let metadata =
            crate::threads::Threads::get_in(conn, &ThreadId::from_stored(thread.clone())).await?;
        let (thread_title, removed_at) = (metadata.title, metadata.removed_at);
        let invocation: Option<(String, String)> = match operation {
            Some(op) => crate::harness::Harness::invocation_model_in(conn, &op).await?,
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
}

/// Every `WorkflowEdited` of `workflow`, in journal order, read by its plan's
/// project: an edit names a thread only when a Planner made it. Its
/// payload is the edit's [`EditOutcome`].
impl ReadRows {
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
}

/// The payload of the event a command recorded as its outcome: what the
/// command answered when it committed.
impl ReadRows {
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
}

pub(in crate::plans) struct ReadRows;
