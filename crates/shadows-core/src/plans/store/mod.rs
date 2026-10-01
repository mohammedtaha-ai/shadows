//! One job: plans' SQLite queries (spec §14.4). Each file is `impl Storage`
//! over the shared pool, which stays in `db`: `edit.rs` changes a
//! version, `draft.rs` starts one, `read.rs` reads them, `task.rs` holds a
//! version's task and link rows, `view.rs` shows one in its conversation.
//!
//! What another service's store may call inside one of its writes is
//! `pub(crate)` and declared in `plans/contract.yaml` under
//! `shared_in_transaction`; nothing else here is.

mod draft;
mod edit;
mod read;
mod task;
mod view;

pub(crate) use task::task_of;

use crate::command::Writer;
use crate::events::DurableEvent;
use crate::projects::ProjectId;
use crate::turns::OperationId;

/// A plan event as §16.3 records it: always its project, with the writer as
/// the actor; the thread, and the turn when there is one, only for a Planner.
fn plan_event(
    kind: &str,
    writer: &Writer,
    project: &ProjectId,
    operation: Option<&OperationId>,
) -> DurableEvent {
    let event = DurableEvent::new(kind, writer.actor()).with_project(project);
    match (writer, operation) {
        (Writer::Planner { thread, .. }, Some(op)) => event.with_thread(thread).with_operation(op),
        (Writer::Planner { thread, .. }, None) => event.with_thread(thread),
        _ => event,
    }
}
