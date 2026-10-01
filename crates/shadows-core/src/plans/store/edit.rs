//! One job: changing a plan version (spec §13.2–§13.5): an edit under a
//! revision check, or the approval that freezes it. Every command here
//! answers only what was fixed when it committed: a replay reads that answer
//! back from its event, never the plan as it is later. Starting a version is
//! `draft.rs`; reading one is `read.rs`.

use sqlx::SqliteConnection;

use super::plan_event;
use super::read::{edits_of, load_plan, recorded_outcome};
use super::task::write_content;
use crate::command::{CommandContext, Writer};
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::grants::check_writer;
use crate::plans::model::{Approved, EditOutcome, PlanId, WorkflowId, WorkflowState, WrittenBy};
use crate::plans::ops::{PlanOp, apply};
use crate::plans::rules::approval_problems;
use crate::projects::ProjectId;
use crate::threads::{EntryRef, NewThreadEntry, ThreadEntryKind, append_entry_in};
use crate::turns::OperationId;

impl Storage {
    /// §13.5's order of work: the grant, then a recorded command (answered
    /// from its event, before any revision check), then the revision, then
    /// the change with its event and command record in one transaction.
    /// `operation` is a Planner's running turn, which its event names.
    pub async fn edit_plan(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        operation: Option<&OperationId>,
        workflow: &WorkflowId,
        expected_revision: i64,
        ops: &[PlanOp],
    ) -> Result<EditOutcome, StorageError> {
        let (ctx, writer, workflow, ops, ts) = (
            ctx.clone(),
            writer.clone(),
            workflow.clone(),
            ops.to_vec(),
            now(),
        );
        let operation = operation.cloned();
        self.write_txn(move |conn| {
            Box::pin(async move {
                let (_, project) = owner(conn, &workflow).await?;
                check_writer(conn, &writer, &project).await?;
                if let Some(event) = classify(conn, &ctx, "Workflow", workflow.as_str()).await? {
                    return recorded_outcome(conn, &event).await;
                }
                let plan = load_plan(conn, &workflow).await?;
                writable(
                    conn,
                    &plan.state,
                    plan.revision,
                    expected_revision,
                    &project,
                    &workflow,
                )
                .await?;
                let applied = apply(&plan.content(), &ops).map_err(StorageError::PlanInvalid)?;
                write_content(conn, &workflow, &plan.tasks, &applied.content, &ts).await?;
                sqlx::query(
                    "UPDATE workflow SET title = ?, goal = ?, revision = revision + 1,
                            updated_at = ?
                      WHERE id = ?",
                )
                .bind(&applied.content.title)
                .bind(&applied.content.goal)
                .bind(&ts)
                .bind(workflow.as_str())
                .execute(&mut *conn)
                .await?;
                let outcome = EditOutcome {
                    workflow_id: workflow.clone(),
                    version: plan.version,
                    revision: plan.revision + 1,
                    summary: applied.summary,
                    changed_tasks: applied.changed_tasks,
                };
                let event = plan_event("WorkflowEdited", &writer, &project, operation.as_ref())
                    .with_payload(serde_json::to_value(&outcome)?);
                append_event(conn, &event, &ts).await?;
                record_command(
                    conn,
                    &ctx,
                    "Workflow",
                    workflow.as_str(),
                    "WorkflowEdited",
                    &event.event_id,
                    &ts,
                )
                .await?;
                Ok(outcome)
            })
        })
        .await
    }

    /// §13.2: a person's approval freezes a Draft that has no blockers, and
    /// says so in the conversation that wrote it (§13.9, §16.3): none when
    /// that conversation is deleted or an external agent wrote the version.
    /// A replay answers the recorded `WorkflowFrozen` payload, never the
    /// plan, whose `next` changes once the next version exists.
    pub async fn approve_plan(
        &self,
        ctx: &CommandContext,
        workflow: &WorkflowId,
        expected_revision: i64,
    ) -> Result<Approved, StorageError> {
        let (ctx, workflow, ts) = (ctx.clone(), workflow.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let (_, project) = owner(conn, &workflow).await?;
                if let Some(event) = classify(conn, &ctx, "Workflow", workflow.as_str()).await? {
                    return recorded_outcome(conn, &event).await;
                }
                let plan = load_plan(conn, &workflow).await?;
                writable(
                    conn,
                    &plan.state,
                    plan.revision,
                    expected_revision,
                    &project,
                    &workflow,
                )
                .await?;
                let blockers = approval_problems(&plan.content());
                if !blockers.is_empty() {
                    return Err(StorageError::PlanInvalid(blockers));
                }
                sqlx::query(
                    "UPDATE workflow SET state = 'Frozen', frozen_at = ?, updated_at = ?
                      WHERE id = ?",
                )
                .bind(&ts)
                .bind(&ts)
                .bind(workflow.as_str())
                .execute(&mut *conn)
                .await?;
                let actor = Actor::user(&ctx.principal_id);
                let thread = match &plan.written_by {
                    WrittenBy::Planner {
                        thread_id,
                        thread_removed: false,
                        ..
                    } => Some(thread_id),
                    _ => None,
                };
                if let Some(thread) = thread {
                    let body = format!("Plan v{} approved", plan.version);
                    let refs = [EntryRef::Workflow(workflow.clone())];
                    let entry = NewThreadEntry {
                        kind: ThreadEntryKind::PlanApproved,
                        author: actor.clone(),
                        body: &body,
                        refs: &refs,
                        operation_id: None,
                    };
                    append_entry_in(conn, thread, entry, &ts).await?;
                }
                let approved = Approved {
                    workflow_id: workflow.clone(),
                    version: plan.version,
                    revision: plan.revision,
                    frozen_at: ts.clone(),
                };
                let mut event = DurableEvent::new("WorkflowFrozen", actor).with_project(&project);
                if let Some(thread) = thread {
                    event = event.with_thread(thread);
                }
                let event = event.with_payload(serde_json::to_value(&approved)?);
                append_event(conn, &event, &ts).await?;
                record_command(
                    conn,
                    &ctx,
                    "Workflow",
                    workflow.as_str(),
                    "WorkflowFrozen",
                    &event.event_id,
                    &ts,
                )
                .await?;
                Ok(approved)
            })
        })
        .await
    }
}

/// A version's plan, and that plan's project (§16.2).
async fn owner(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<(PlanId, ProjectId), StorageError> {
    let (plan, project): (String, String) = sqlx::query_as(
        "SELECT p.id, p.project_id
           FROM workflow w JOIN plan p ON p.id = w.plan_id
          WHERE w.id = ?",
    )
    .bind(workflow.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("workflow"))?;
    Ok((PlanId::from_stored(plan), ProjectId::from_stored(project)))
}

/// A version may change only while it is a Draft at the revision the writer
/// read. The conflict's summary joins what every edit since then did.
async fn writable(
    conn: &mut SqliteConnection,
    state: &WorkflowState,
    current: i64,
    expected: i64,
    project: &ProjectId,
    workflow: &WorkflowId,
) -> Result<(), StorageError> {
    if *state == WorkflowState::Frozen {
        return Err(StorageError::WorkflowFrozen);
    }
    if current == expected {
        return Ok(());
    }
    let summary = edits_of(conn, project, workflow)
        .await?
        .into_iter()
        .filter(|e| e.revision > expected)
        .map(|e| e.summary)
        .collect::<Vec<_>>()
        .join("; ");
    Err(StorageError::RevisionConflict { current, summary })
}
