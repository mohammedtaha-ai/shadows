//! One job: showing a plan version in its conversation — the
//! `PlanView` card a Planner's `plan_show` writes, under the running turn's
//! command identity.

use super::PlansStore;
use super::read::ReadRows;
use super::task::TaskRows;
use crate::grants::Grants;
use crate::grants::Writer;
use crate::operations::OperationId;
use crate::plans::conversation::{Place, PlanShown};
use crate::plans::model::WorkflowId;
use crate::threads::{EntryRef, NewThreadEntry, ThreadEntryKind, Threads};
use core_db::{CommandContext, JournalEntry, Storage};

impl PlansStore {
    /// Writes the card — a `PlanView` entry of `turn`, headed by the version
    /// or the task — and its `PlanShown` event, with the command record. The
    /// grant is checked inside the transaction; a command already
    /// recorded answers what it recorded, `replayed`, and writes nothing.
    pub async fn show_plan(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        turn: &OperationId,
        workflow: &WorkflowId,
        task_number: Option<u32>,
        place: Place,
    ) -> Result<PlanShown, crate::CoreError> {
        let (ctx, writer, turn, workflow, ts) = (
            ctx.clone(),
            writer.clone(),
            turn.clone(),
            workflow.clone(),
            Storage::now(),
        );
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    let plan = ReadRows::load_plan(conn, &workflow, None).await?;
                    // The card goes in the Planner's own conversation.
                    let Writer::Planner { thread, .. } = &writer else {
                        return Err(crate::grants::GrantsError::GrantScope.into());
                    };
                    let thread = thread.clone();
                    Grants::check_writer(conn, &writer, &plan.project_id).await?;
                    if let Some(event) =
                        CommandContext::classify(conn, &ctx, "Thread", thread.as_str()).await?
                    {
                        let recorded: PlanShown = ReadRows::recorded_outcome(conn, &event).await?;
                        return Ok(PlanShown {
                            replayed: true,
                            ..recorded
                        });
                    }
                    let mut refs = vec![EntryRef::Workflow(workflow.clone().as_str().into())];
                    let body = match task_number {
                        None => format!("Plan v{}", plan.version),
                        Some(n) => {
                            let (id, title) = TaskRows::task_numbered(conn, &workflow, n)
                                .await?
                                .ok_or_else(|| {
                                    crate::plans::PlansError::TaskNotInPlan(format!(
                                        "T{n} does not exist in this plan"
                                    ))
                                })?;
                            refs.push(EntryRef::Task(id.as_str().into()));
                            format!("T{n} · {title}")
                        }
                    };
                    let entry = NewThreadEntry {
                        kind: ThreadEntryKind::PlanView,
                        author: writer.actor(),
                        body: &body,
                        refs: &refs,
                        card: None,
                        operation_id: Some(turn.as_str()),
                    };
                    let entry = Threads::append_entry_in(conn, &thread, entry, &ts).await?;
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
                    JournalEntry::append_event(conn, event.journal_entry(), &ts).await?;
                    CommandContext::record_command(
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
