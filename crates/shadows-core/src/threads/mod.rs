//! One job: the planning threads and what they recorded — the `Threads` service.
//!
//! A thread's harness is one Shadows knows, `claude-code` when none is named;
//! it changes only before the thread's first operation, never on a fork, and
//! a change closes the thread's open session through `Harness`. A fork is
//! taken only from the source's last entry, written by a completed turn. A
//! thread is titled after its first message, then by its harness, never over
//! a name a person gave it.
//!
//! `model` holds the types, `title` what a title taken from the conversation reads, and `store` the
//! queries.

mod error;
mod model;
mod store;
#[cfg(feature = "test-support")]
mod testing;
mod title;

use std::sync::Arc;

use shadows_agent::policy;

pub use model::{
    EntryRef, NewThreadEntry, PlanningThread, ThreadEntry, ThreadEntryId, ThreadEntryKind,
    ThreadId, TurnContext,
};

use crate::code::Code;
use crate::db::{CommandContext, Storage};
use crate::error::CoreError;
use crate::harness::Harness;
use crate::projects::{ProjectId, Projects};
use crate::turns::Operation;
use crate::turns::{StopOutcome, ThreadStopper};
pub(crate) use error::ThreadsError;
use store::ThreadsStore;

/// Threads: what storage holds, the harness whose session a harness change
/// closes, and the code index a listed project is touched in.
pub struct Threads {
    store: ThreadsStore,
    harness: Arc<Harness>,
    code: Code,
    projects: Projects,
    stopper: ThreadStopper,
}

impl Threads {
    pub(crate) fn new(
        storage: Arc<Storage>,
        harness: Arc<Harness>,
        code: Code,
        projects: Projects,
        stopper: ThreadStopper,
    ) -> Self {
        Self {
            store: ThreadsStore::with_storage(storage),
            harness,
            code,
            projects,
            stopper,
        }
    }

    /// A project's planning threads, oldest first. An unknown project has none.
    /// Opening a project lists its threads, so the project is touched in Code.
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanningThread>, CoreError> {
        let threads = self.store.list_threads_for_project(project).await?;
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
        profile: Option<&str>,
    ) -> Result<PlanningThread, CoreError> {
        let harness = harness.unwrap_or(policy::CLAUDE_CODE);
        Projects::known_harness(harness)?;
        self.projects.harness_fits(project, harness).await?;
        if harness != policy::AYLUL && profile.is_some() {
            return Err(CoreError::ProfileInvalid(
                "Planner threads cannot name an agent".into(),
            ));
        }
        if harness == policy::AYLUL && profile.is_none() {
            return Err(CoreError::AylulProfileNotFound("agent required".into()));
        }
        let mut params = serde_json::json!({
            "project": project, "title": title, "harness": harness,
        });
        // Only an agent's thread names one; Planner fingerprints omit the profile.
        if let Some(profile) = profile {
            params["profile"] = profile.into();
        }
        let c = CommandContext::user(command_id, "thread.create", params);
        self.store
            .create_planning_thread(&c, project, title, harness, profile)
            .await
    }

    pub async fn list_for_profile(&self, slug: &str) -> Result<Vec<PlanningThread>, CoreError> {
        Ok(self.store.list_threads_for_profile(slug).await?)
    }

    /// Changes the thread's harness before its first turn:
    /// "thread.harness", params { "thread_id", "harness" }. When the harness
    /// changed, the thread's session is closed; a failure to close is
    /// logged and the change still answers.
    pub async fn set_harness(
        &self,
        command_id: String,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<PlanningThread, CoreError> {
        Projects::known_harness(harness)?;
        let context = self.store.turn_context(thread).await?;
        self.projects
            .harness_fits(&context.project_id, harness)
            .await?;
        let before = context.harness;
        let params = serde_json::json!({ "thread_id": thread, "harness": harness });
        let c = CommandContext::user(command_id, "thread.harness", params);
        let updated = self.store.set_thread_harness(&c, thread, harness).await?;
        if updated.harness != before
            && let Err(error) = self.harness.close_session(thread).await
        {
            tracing::error!(%error, thread_id = %thread, "thread.harness_change_close_failed");
        }
        Ok(updated)
    }

    /// Forks the thread from its last completed entry:
    /// "thread.fork", params { "thread_id", "at_entry_id" }.
    pub async fn fork(
        &self,
        command_id: String,
        thread: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<PlanningThread, CoreError> {
        let params = serde_json::json!({ "thread_id": thread, "at_entry_id": at });
        let c = CommandContext::user(command_id, "thread.fork", params);
        Ok(self.store.fork_thread(&c, thread, at).await?)
    }

    /// A thread's entries in ordinal order.
    pub async fn entries(&self, thread: &ThreadId) -> Result<Vec<ThreadEntry>, CoreError> {
        Ok(self.store.list_thread_entries(thread).await?)
    }

    /// A thread's operations — its turns — newest first, each as it now stands.
    pub async fn operations(&self, thread: &ThreadId) -> Result<Vec<Operation>, CoreError> {
        Ok(self
            .store
            .storage
            .list_operations_for_thread(thread)
            .await?)
    }

    /// Remove first, then stop the turn, then close its adapter.
    pub async fn remove(
        &self,
        command_id: String,
        thread: &ThreadId,
    ) -> Result<PlanningThread, CoreError> {
        let c = CommandContext::user(
            command_id,
            "thread.remove",
            serde_json::json!({ "thread_id": thread }),
        );
        let removed = self.store.remove_thread(&c, thread).await?;
        match self.stopper.stop_running(thread).await {
            Ok(StopOutcome::TerminationFailed) => {
                tracing::error!(thread_id = %thread, "thread.remove_stop_failed");
            }
            Err(error) => {
                tracing::error!(%error, thread_id = %thread, "thread.remove_stop_failed");
            }
            Ok(_) => {}
        }
        if let Err(error) = self.harness.close_session(thread).await {
            tracing::error!(%error, thread_id = %thread, "thread.remove_close_failed");
        }
        Ok(removed)
    }

    /// One thread, including a removed one, for historical plan links.
    pub async fn get(&self, thread: &ThreadId) -> Result<PlanningThread, CoreError> {
        Ok(self.store.get_thread(thread).await?)
    }
    /// Records the session once; a racing first completion cannot replace it.
    pub async fn record_harness_session(
        &self,
        thread: &ThreadId,
        session: &str,
    ) -> Result<bool, CoreError> {
        Ok(self.store.record_harness_session(thread, session).await?)
    }
}
