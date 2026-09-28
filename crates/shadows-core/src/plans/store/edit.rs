//! One job: changing a plan version (spec §13.2–§13.5): an edit under a
//! revision check, or the approval that freezes it. Every command here
//! answers only what was fixed when it committed: a replay reads that answer
//! back from its event, never the plan as it is later. Starting a version is
//! `draft.rs`; reading one is `read.rs`.

use sqlx::SqliteConnection;

use super::read::{edits_of, load_plan, recorded_outcome};
use super::task::write_content;
use crate::command::{CommandContext, Writer};
use crate::events::{Actor, DurableEvent};
use crate::grants::check_writer;
use crate::plans::model::{Approved, EditOutcome, WorkflowId, WorkflowState};
use crate::plans::ops::{PlanOp, apply};
use crate::plans::rules::approval_problems;
use crate::project::ProjectId;
use crate::storage::{
    Storage, StorageError, append_entry_in, append_event, classify, now, record_command,
};
use crate::thread::{EntryRef, NewThreadEntry, ThreadEntryKind, ThreadId};

impl Storage {
    /// §13.5's order of work: the grant, then a recorded command (answered
    /// from its event, before any revision check), then the revision, then
    /// the change with its event and command record in one transaction.
    pub async fn edit_plan(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
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
        self.write_txn(move |conn| {
            Box::pin(async move {
                let (thread, project) = owner(conn, &workflow).await?;
                check_writer(conn, &writer, &project, Some(&thread)).await?;
                if let Some(event) = classify(conn, &ctx, "Workflow", workflow.as_str()).await? {
                    return recorded_outcome(conn, &event).await;
                }
                let plan = load_plan(conn, &workflow).await?;
                writable(
                    conn,
                    &plan.state,
                    plan.revision,
                    expected_revision,
                    &thread,
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
                let event = DurableEvent::new("WorkflowEdited", writer.actor())
                    .with_project(&project)
                    .with_thread(&thread)
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
    /// says so in the conversation (§13.9). A replay answers the recorded
    /// `WorkflowFrozen` payload, never the plan, whose `next` changes once the
    /// next version exists.
    pub async fn approve_plan(
        &self,
        ctx: &CommandContext,
        workflow: &WorkflowId,
        expected_revision: i64,
    ) -> Result<Approved, StorageError> {
        let (ctx, workflow, ts) = (ctx.clone(), workflow.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let (thread, project) = owner(conn, &workflow).await?;
                if let Some(event) = classify(conn, &ctx, "Workflow", workflow.as_str()).await? {
                    return recorded_outcome(conn, &event).await;
                }
                let plan = load_plan(conn, &workflow).await?;
                writable(
                    conn,
                    &plan.state,
                    plan.revision,
                    expected_revision,
                    &thread,
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
                let body = format!("Plan v{} approved", plan.version);
                let refs = [EntryRef::Workflow(workflow.clone())];
                let entry = NewThreadEntry {
                    kind: ThreadEntryKind::PlanApproved,
                    author: actor.clone(),
                    body: &body,
                    refs: &refs,
                    operation_id: None,
                };
                append_entry_in(conn, &thread, entry, &ts).await?;
                let approved = Approved {
                    workflow_id: workflow.clone(),
                    version: plan.version,
                    revision: plan.revision,
                    frozen_at: ts.clone(),
                };
                let event = DurableEvent::new("WorkflowFrozen", actor)
                    .with_project(&project)
                    .with_thread(&thread)
                    .with_payload(serde_json::to_value(&approved)?);
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

/// The thread and project a version belongs to.
async fn owner(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<(ThreadId, ProjectId), StorageError> {
    let (thread, project): (String, String) = sqlx::query_as(
        "SELECT w.thread_id, t.project_id
           FROM workflow w JOIN planning_thread t ON t.id = w.thread_id
          WHERE w.id = ?",
    )
    .bind(workflow.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("workflow"))?;
    Ok((
        ThreadId::from_stored(thread),
        ProjectId::from_stored(project),
    ))
}

/// A version may change only while it is a Draft at the revision the writer
/// read. The conflict's summary joins what every edit since then did.
async fn writable(
    conn: &mut SqliteConnection,
    state: &WorkflowState,
    current: i64,
    expected: i64,
    thread: &ThreadId,
    workflow: &WorkflowId,
) -> Result<(), StorageError> {
    if *state == WorkflowState::Frozen {
        return Err(StorageError::WorkflowFrozen);
    }
    if current == expected {
        return Ok(());
    }
    let summary = edits_of(conn, thread, workflow)
        .await?
        .into_iter()
        .filter(|e| e.revision > expected)
        .map(|e| e.summary)
        .collect::<Vec<_>>()
        .join("; ");
    Err(StorageError::RevisionConflict { current, summary })
}
