//! One job: showing a plan version in its conversation (spec §13.9) — the
//! `PlanView` card a Planner's `plan_show` writes, under the running turn's
//! command identity.

use super::read::{load_plan, recorded_outcome};
use super::task::task_numbered;
use crate::command::{CommandContext, Writer};
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::grants::check_writer;
use crate::plans::conversation::{Place, PlanShown};
use crate::plans::model::WorkflowId;
use crate::threads::{EntryRef, NewThreadEntry, ThreadEntryKind, append_entry_in};
use crate::turns::OperationId;

impl Storage {
    /// Writes the card — a `PlanView` entry of `turn`, headed by the version
    /// or the task — and its `PlanShown` event, with the command record. The
    /// grant is checked inside the transaction (§13.7); a command already
    /// recorded answers what it recorded, `replayed`, and writes nothing.
    pub async fn show_plan(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        turn: &OperationId,
        workflow: &WorkflowId,
        task_number: Option<u32>,
        place: Place,
    ) -> Result<PlanShown, StorageError> {
        let (ctx, writer, turn, workflow, ts) = (
            ctx.clone(),
            writer.clone(),
            turn.clone(),
            workflow.clone(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                let plan = load_plan(conn, &workflow).await?;
                // The card goes in the Planner's own conversation (§13.9).
                let Writer::Planner { thread, .. } = &writer else {
                    return Err(StorageError::GrantScope);
                };
                let thread = thread.clone();
                check_writer(conn, &writer, &plan.project_id).await?;
                if let Some(event) = classify(conn, &ctx, "Thread", thread.as_str()).await? {
                    let recorded: PlanShown = recorded_outcome(conn, &event).await?;
                    return Ok(PlanShown {
                        replayed: true,
                        ..recorded
                    });
                }
                let mut refs = vec![EntryRef::Workflow(workflow.clone())];
                let body = match task_number {
                    None => format!("Plan v{}", plan.version),
                    Some(n) => {
                        let (id, title) =
                            task_numbered(conn, &workflow, n).await?.ok_or_else(|| {
                                StorageError::TaskNotInPlan(format!(
                                    "T{n} does not exist in this plan"
                                ))
                            })?;
                        refs.push(EntryRef::Task(id));
                        format!("T{n} · {title}")
                    }
                };
                let entry = NewThreadEntry {
                    kind: ThreadEntryKind::PlanView,
                    author: writer.actor(),
                    body: &body,
                    refs: &refs,
                    operation_id: Some(&turn),
                };
                let entry = append_entry_in(conn, &thread, entry, &ts).await?;
                let shown = PlanShown {
                    workflow_id: workflow.clone(),
                    version: plan.version,
                    task_number,
                    place,
                    entry_id: entry.id,
                    replayed: false,
                };
                let event = crate::events::DurableEvent::new("PlanShown", writer.actor())
                    .with_project(&plan.project_id)
                    .with_thread(&thread)
                    .with_operation(&turn)
                    .with_payload(serde_json::to_value(&shown)?);
                append_event(conn, &event, &ts).await?;
                record_command(
                    conn,
                    &ctx,
                    "Thread",
                    thread.as_str(),
                    "PlanShown",
                    &event.event_id,
                    &ts,
                )
                .await?;
                Ok(shown)
            })
        })
        .await
    }
}
