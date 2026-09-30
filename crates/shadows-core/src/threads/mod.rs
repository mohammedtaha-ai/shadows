//! One job: the planning threads and what they recorded (spec §4, §12.6,
//! §12.9, §14.4) — the `Threads` service.
//!
//! A thread's harness is one Shadows knows, `claude-code` when none is named;
//! it changes only before the thread's first operation, never on a fork, and
//! a change closes the thread's open session through `Harness`. A fork is
//! taken only from the source's last entry, written by a completed turn.
//!
//! `model` holds the types, `rules` the harness check `Projects` shares, and
//! `store` the queries.

mod model;
mod rules;
mod store;

use std::sync::Arc;

use shadows_agent::policy;

pub use model::{
    EntryRef, NewThreadEntry, PlanningThread, ThreadEntry, ThreadEntryId, ThreadEntryKind,
    ThreadId, TurnContext,
};
pub(crate) use rules::known_harness;
// What another write calls inside its own transaction (spec §14.6): a plan
// draft creates its thread, and a turn, an edit or a shown plan appends an
// entry.
pub(crate) use store::{append_entry_in, insert_thread};

use crate::app::user_command;
use crate::code::Code;
use crate::db::Storage;
use crate::error::CoreError;
use crate::harness::Harness;
use crate::projects::ProjectId;
use crate::turns::Operation;

/// Threads: what storage holds, the harness whose session a harness change
/// closes, and the code index a listed project is touched in (§15.6).
pub struct Threads {
    storage: Arc<Storage>,
    harness: Arc<Harness>,
    code: Code,
}

impl Threads {
    pub(crate) fn new(storage: Arc<Storage>, harness: Arc<Harness>, code: Code) -> Self {
        Self {
            storage,
            harness,
            code,
        }
    }

    /// A project's planning threads, oldest first. An unknown project has none.
    /// The web client opening a project lists them: the project is used (§15.6).
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanningThread>, CoreError> {
        let threads = self.storage.list_threads_for_project(project).await?;
        self.code.touch(project).await;
        Ok(threads)
    }

    /// Creates a planning thread: "thread.create", params { "project",
    /// "title", "harness" }, the harness `claude-code` when none is named.
    pub async fn create(
        &self,
        command_id: String,
        project: &ProjectId,
        title: &str,
        harness: Option<&str>,
    ) -> Result<PlanningThread, CoreError> {
        let harness = harness.unwrap_or(policy::CLAUDE_CODE);
        known_harness(harness)?;
        let params = serde_json::json!({
            "project": project, "title": title, "harness": harness,
        });
        let c = user_command(command_id, "thread.create", params);
        Ok(self
            .storage
            .create_planning_thread(&c, project, title, harness)
            .await?)
    }

    /// Changes the thread's harness before its first turn (spec §12.6):
    /// "thread.harness", params { "thread_id", "harness" }. When the harness
    /// changed, the thread's session is closed (§12.2); a failure to close is
    /// logged and the change still answers.
    pub async fn set_harness(
        &self,
        command_id: String,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<PlanningThread, CoreError> {
        known_harness(harness)?;
        let before = self.storage.turn_context(thread).await?.harness;
        let params = serde_json::json!({ "thread_id": thread, "harness": harness });
        let c = user_command(command_id, "thread.harness", params);
        let updated = self.storage.set_thread_harness(&c, thread, harness).await?;
        if updated.harness != before
            && let Err(error) = self.harness.close_session(thread).await
        {
            tracing::error!(%error, thread_id = %thread, "thread.harness_change_close_failed");
        }
        Ok(updated)
    }

    /// Forks the thread from its last completed entry (spec §12.9):
    /// "thread.fork", params { "thread_id", "at_entry_id" }.
    pub async fn fork(
        &self,
        command_id: String,
        thread: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<PlanningThread, CoreError> {
        let params = serde_json::json!({ "thread_id": thread, "at_entry_id": at });
        let c = user_command(command_id, "thread.fork", params);
        Ok(self.storage.fork_thread(&c, thread, at).await?)
    }

    /// A thread's entries in ordinal order.
    pub async fn entries(&self, thread: &ThreadId) -> Result<Vec<ThreadEntry>, CoreError> {
        Ok(self.storage.list_thread_entries(thread).await?)
    }

    /// A thread's operations — its turns — newest first, each as it now stands.
    pub async fn operations(&self, thread: &ThreadId) -> Result<Vec<Operation>, CoreError> {
        Ok(self.storage.list_operations_for_thread(thread).await?)
    }
}
