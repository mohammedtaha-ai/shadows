//! One job: the planning threads and what they recorded — the `Threads` service.
//!
//! A thread's harness is one Shadows knows, `claude-code` when none is named;
//! it changes only before the thread's first operation, never on a fork, and
//! Harness closes the open session after changing it. A fork is
//! taken only from the source's last entry, written by a completed turn. A
//! thread is titled after its first message, then by its harness, never over
//! a name a person gave it.
//!
//! `model` holds the types, `title` what a title taken from the conversation reads, and `store` the
//! queries.

mod attachment;
mod error;
mod model;
mod store;
#[cfg(feature = "test-support")]
mod testing;
mod title;

use std::sync::Arc;

use shadows_agent::policy;

pub use attachment::{Attachment, AttachmentId, AttachmentRef, MAX_IMAGE_BYTES};
pub use model::{
    EntryRef, NewThreadEntry, OperationRef, PlanningThread, TaskRef, ThreadEntry, ThreadEntryId,
    ThreadEntryKind, ThreadId, TurnContext, WorkflowRef,
};

use core_code::Code;
use core_db::{CommandContext, Storage};
use core_projects::{ProjectId, Projects};
pub use error::ThreadsError;
use store::ThreadsStore;

/// Thread persistence and the project code activity a listing touches.
pub struct Threads {
    store: ThreadsStore,
    code: Code,
    projects: Projects,
}

impl Threads {
    pub fn new(storage: Arc<Storage>, code: Code, projects: Projects) -> Self {
        Self {
            store: ThreadsStore::with_storage(storage),
            code,
            projects,
        }
    }

    /// A project's planning threads, oldest first. An unknown project has none.
    /// Opening a project lists its threads, so the project is touched in Code.
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanningThread>, ThreadsError> {
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
    ) -> Result<PlanningThread, ThreadsError> {
        let harness = harness.unwrap_or(policy::CLAUDE_CODE);
        Projects::known_harness(harness)?;
        self.projects.harness_fits(project, harness).await?;
        if harness != policy::AYLUL && profile.is_some() {
            return Err(ThreadsError::InvalidProfile(
                "Planner threads cannot name an agent".into(),
            ));
        }
        if harness == policy::AYLUL && profile.is_none() {
            return Err(ThreadsError::Agent(core_agents::AgentsError::NotFound(
                "agent required".into(),
            )));
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

    pub async fn list_for_profile(&self, slug: &str) -> Result<Vec<PlanningThread>, ThreadsError> {
        Ok(self.store.list_threads_for_profile(slug).await?)
    }

    /// A thread's entries in ordinal order.
    pub async fn entries(&self, thread: &ThreadId) -> Result<Vec<ThreadEntry>, ThreadsError> {
        Ok(self.store.list_thread_entries(thread).await?)
    }

    /// One thread, including a removed one, for historical plan links.
    pub async fn get(&self, thread: &ThreadId) -> Result<PlanningThread, ThreadsError> {
        Ok(self.store.get_thread(thread).await?)
    }
    /// Records the session once; a racing first completion cannot replace it.
    pub async fn record_harness_session(
        &self,
        thread: &ThreadId,
        session: &str,
    ) -> Result<bool, ThreadsError> {
        Ok(self.store.record_harness_session(thread, session).await?)
    }
}
