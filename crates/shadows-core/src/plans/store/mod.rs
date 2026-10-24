//! One job: plans' SQLite queries. Each file uses the private PlansStore
//! over the shared pool, which stays in `db`: `edit.rs` changes a
//! version, `draft.rs` starts one, `read.rs` reads them, `plan.rs` archives
//! and reads a plan with its versions, `task.rs` holds a version's task
//! and link rows, `view.rs` shows one in its conversation.
//!
//! What another service's store may call inside one of its writes is
//! `pub(crate)` and declared in `plans/contract.yaml` under
//! `shared_in_transaction`; nothing else here is.

pub(super) mod binding_participants;
mod bindings;
mod dependencies;
mod draft;
mod edit;
mod graph;
mod linked;
mod map;
pub(super) mod notifications;
mod plan;
mod read;
pub(super) mod task;
mod view;

use crate::grants::Writer;
use crate::operations::OperationId;
use crate::projects::ProjectId;
use core_db::JournalEntry;

/// A plan event records its subject: always its project, with the writer as
/// the actor; the thread, and the turn when there is one, only for a Planner.
fn plan_event(
    kind: &str,
    writer: &Writer,
    project: &ProjectId,
    operation: Option<&OperationId>,
) -> JournalEntry {
    let event = JournalEntry::new(kind, writer.actor()).with_project(project.as_str());
    match (writer, operation) {
        (Writer::Planner { thread, .. }, Some(op)) => event
            .with_thread(thread.as_str())
            .with_operation(op.as_str()),
        (Writer::Planner { thread, .. }, None) => event.with_thread(thread.as_str()),
        _ => event,
    }
}

use core_db::Storage;
use std::sync::Arc;

pub(super) struct PlansStore {
    pub(super) storage: Arc<Storage>,
}

mod design_links;

mod project_links;

mod design_parts;

mod agreement_review;

mod event_projection;
