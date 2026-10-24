//! One job: starting a plan version — a
//! new plan's first version, the next one copied from a frozen version with
//! its reason, or the Draft already there. `DraftStarted` names the version,
//! which never changes as the draft is edited, so a replay answers the same.
//! Changing a version is `edit.rs`. A start may also tie its plan to parts,
//! in the same transaction.

use super::PlansStore;
use sqlx::SqliteConnection;

use super::plan::PlanRows;
use super::plan_event;
use super::read::ReadRows;
use super::task::TaskRows;
use crate::design::PartId;
use crate::plans::model::{DraftStarted, PlanContent, PlanId, WorkflowId, WorkflowState};
use crate::plans::rules::Problem;
use core_db::{CommandContext, JournalEntry, Storage, StorageError};
use core_grants::Grants;
use core_grants::Writer;
use core_operations::OperationId;
use core_projects::ProjectId;

impl PlansStore {
    /// draft_start in `project`. Without a `plan`, a new plan with v1
    /// from `fresh`. With one: its Draft, unchanged, when it has one; else
    /// its next version copied from the latest, which needs a non-blank
    /// `reason`. A `source`, when given, must be that latest version;
    /// it is checked after the replay, because the version a start answered
    /// is not the latest one. `operation` is the Planner's running turn,
    /// recorded as the version's writer beside its thread.
    #[expect(
        clippy::too_many_arguments,
        reason = "one start's request, each part checked inside the one transaction"
    )]
    pub async fn start_draft(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        project: &ProjectId,
        plan: Option<&PlanId>,
        source: Option<&WorkflowId>,
        fresh: Option<(&str, &str)>,
        reason: Option<&str>,
        operation: Option<&OperationId>,
        draft_ref: Option<&str>,
        parts: &[PartId],
    ) -> Result<DraftStarted, crate::CoreError> {
        let (ctx, writer, project, ts) =
            (ctx.clone(), writer.clone(), project.clone(), Storage::now());
        let (plan, source, operation) = (plan.cloned(), source.cloned(), operation.cloned());
        let fresh = fresh.map(|(t, g)| (t.to_string(), g.to_string()));
        let reason = reason.map(str::to_string);
        let draft_ref = draft_ref.map(str::to_string);
        let parts = parts.to_vec();
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    Grants::check_writer(conn, &writer, &project).await?;
                    if let Some(plan) = &plan {
                        plan_in(conn, plan, &project).await?;
                    }
                    let scope = match &plan {
                        Some(plan) => ("Plan", plan.as_str()),
                        None => ("Project", project.as_str()),
                    };
                    if let Some(id) = recorded(conn, &ctx, scope, &project).await? {
                        return Ok(started(conn, &WorkflowId::from_stored(id)).await?);
                    }
                    crate::design::Design::require_roadmap_in(conn, &project).await?;
                    if let Some(plan) = &plan {
                        PlanRows::require_active(conn, plan).await?;
                    }
                    let latest = match &plan {
                        Some(plan) => {
                            let id = ReadRows::latest_version(conn, plan)
                                .await?
                                .ok_or(StorageError::NotFound("workflow"))?;
                            Some(ReadRows::load_plan(conn, &id, None).await?)
                        }
                        None => None,
                    };
                    if let Some(source) = &source
                        && let Some(latest) = &latest
                        && &latest.id != source
                    {
                        return Err(
                            crate::plans::PlansError::NotLatestVersion(latest.id.clone()).into(),
                        );
                    }
                    if matches!(writer, Writer::External { .. })
                        && draft_ref.is_none()
                        && !matches!(&latest, Some(plan) if plan.state == WorkflowState::Draft)
                    {
                        return Err(core_grants::GrantsError::GrantScope.into());
                    }
                    let by = Writing {
                        writer: &writer,
                        operation: operation.as_ref(),
                        project: &project,
                    };
                    let id = match (latest, fresh) {
                        (Some(draft), _) if draft.state == WorkflowState::Draft => draft.id,
                        (Some(frozen), _) => {
                            let reason = reason
                                .as_deref()
                                .map(str::trim)
                                .filter(|r| !r.is_empty())
                                .ok_or(crate::plans::PlansError::ReasonMissing)?;
                            let copy = Version {
                                plan: &frozen.plan_id,
                                number: frozen.version + 1,
                                previous: Some(&frozen.id),
                                reason: Some(reason),
                            };
                            let before = frozen.content();
                            match insert_version(conn, &by, copy, &before, &ts).await {
                                Err(StorageError::Constraint(m)) if m.contains(ONE_DRAFT) => {
                                    the_draft(conn, &frozen.plan_id).await?
                                }
                                inserted => inserted?,
                            }
                        }
                        (None, Some((title, goal))) => {
                            let plan = insert_plan(conn, &project, &ts).await?;
                            let first = Version {
                                plan: &plan,
                                number: 1,
                                previous: None,
                                reason: None,
                            };
                            insert_version(conn, &by, first, &empty(title, goal), &ts).await?
                        }
                        (None, None) => {
                            return Err(crate::plans::PlansError::PlanInvalid(vec![Problem {
                                message: "a plan's first version needs a title and a goal".into(),
                            }])
                            .into());
                        }
                    };
                    if let Some(r) = &draft_ref {
                        Grants::bind_draft_ref(conn, &writer, r, id.as_str(), &project, &ts)
                            .await?;
                    }
                    let started = started(conn, &id).await?;
                    crate::plans::Plans::link_plan_parts_in(
                        conn,
                        &project,
                        &started.plan_id,
                        &parts,
                    )
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        scope.0,
                        scope.1,
                        "Workflow",
                        id.as_str(),
                        &ts,
                    )
                    .await?;
                    Ok(started)
                })
            })
            .await
    }
}

/// Who writes a new version, and where its events go.
struct Writing<'a> {
    writer: &'a Writer,
    operation: Option<&'a OperationId>,
    project: &'a ProjectId,
}

/// Where a new version goes: its plan's chain, with its reason from v2.
struct Version<'a> {
    plan: &'a PlanId,
    number: i64,
    previous: Option<&'a WorkflowId>,
    reason: Option<&'a str>,
}

/// What SQLite says when `workflow_one_draft` (or the version number beside
/// it) refuses a second Draft of one plan.
const ONE_DRAFT: &str = "UNIQUE constraint failed: workflow.plan_id";

fn empty(title: String, goal: String) -> PlanContent {
    PlanContent {
        title,
        goal,
        tasks: Default::default(),
        links: Vec::new(),
        bindings: Vec::new(),
    }
}

/// `plan` must be a plan of `project`: a writer reaches no other.
async fn plan_in(
    conn: &mut SqliteConnection,
    plan: &PlanId,
    project: &ProjectId,
) -> Result<(), crate::CoreError> {
    let of: String = sqlx::query_scalar("SELECT project_id FROM plan WHERE id = ?")
        .bind(plan.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("plan"))?;
    if of == project.as_str() {
        Ok(())
    } else {
        Err(core_grants::GrantsError::GrantScope.into())
    }
}

/// A start already recorded under `scope`, or, for a start that names a plan,
/// under its project: a Planner's first start names none, and its retry in the
/// same turn finds the plan that start created.
async fn recorded(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope: (&str, &str),
    project: &ProjectId,
) -> Result<Option<String>, StorageError> {
    if let Some(id) = CommandContext::classify(conn, ctx, scope.0, scope.1).await? {
        return Ok(Some(id));
    }
    if scope.0 == "Project" {
        return Ok(None);
    }
    CommandContext::classify(conn, ctx, "Project", project.as_str()).await
}

/// A new Active plan of `project`, for its first version.
async fn insert_plan(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    ts: &str,
) -> Result<PlanId, StorageError> {
    let id = PlanId::generate();
    sqlx::query("INSERT INTO plan (id, project_id, state, created_at) VALUES (?, ?, 'Active', ?)")
        .bind(id.as_str())
        .bind(project.as_str())
        .bind(ts)
        .execute(&mut *conn)
        .await?;
    Ok(id)
}

/// The plan's Draft, which a start that lost the race to `workflow_one_draft`
/// answers instead of the database's refusal.
async fn the_draft(conn: &mut SqliteConnection, plan: &PlanId) -> Result<WorkflowId, StorageError> {
    let id: Option<String> =
        sqlx::query_scalar("SELECT id FROM workflow WHERE plan_id = ? AND state = 'Draft'")
            .bind(plan.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    id.map(WorkflowId::from_stored).ok_or_else(|| {
        StorageError::Constraint(format!(
            "plan {plan} refused a new version yet has no Draft"
        ))
    })
}

/// A new Draft at revision 0 holding `content`, journalled as
/// `WorkflowDraftStarted`. Its writer is recorded once: a Planner as
/// its thread and running turn, an external agent as its grant.
async fn insert_version(
    conn: &mut SqliteConnection,
    by: &Writing<'_>,
    v: Version<'_>,
    content: &PlanContent,
    ts: &str,
) -> Result<WorkflowId, StorageError> {
    let id = WorkflowId::generate();
    let (thread, operation, grant) = match by.writer {
        Writer::Planner { thread, .. } => (Some(thread.as_str()), by.operation, None),
        Writer::External { grant } => (None, None, Some(grant.as_str())),
        Writer::Person => (None, None, None),
    };
    sqlx::query(
        "INSERT INTO workflow
           (id, plan_id, state, previous_version_id, version, revision, title, goal,
            change_reason, written_by_thread, written_by_operation, written_by_grant,
            created_at, updated_at)
         VALUES (?,?, 'Draft', ?,?, 0, ?,?,?,?,?,?,?,?)",
    )
    .bind(id.as_str())
    .bind(v.plan.as_str())
    .bind(v.previous.map(WorkflowId::as_str))
    .bind(v.number)
    .bind(&content.title)
    .bind(&content.goal)
    .bind(v.reason)
    .bind(thread)
    .bind(operation.map(OperationId::as_str))
    .bind(grant)
    .bind(ts)
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    TaskRows::write_content(conn, &id, &[], content, ts).await?;
    let event = plan_event("WorkflowDraftStarted", by.writer, by.project, by.operation)
        .with_payload(serde_json::json!({ "workflow_id": id, "version": v.number }));
    JournalEntry::append_event(conn, event.clone(), ts).await?;
    super::notifications::NotificationsRows::notify_plan(
        conn,
        by.project,
        v.plan,
        &[],
        &by.writer.actor(),
        ts,
    )
    .await?;
    Ok(id)
}

/// What `draft_start` answers for a version.
async fn started(
    conn: &mut SqliteConnection,
    id: &WorkflowId,
) -> Result<DraftStarted, StorageError> {
    let (plan, version): (String, i64) =
        sqlx::query_as("SELECT plan_id, version FROM workflow WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("workflow"))?;
    Ok(DraftStarted {
        workflow_id: id.clone(),
        plan_id: PlanId::from_stored(plan),
        version,
    })
}
