//! Project lifecycle effects that span the registration and indexing owners.
mod error;
mod store;

use core_code::Code;
use core_db::{CommandContext, Storage};
use core_projects::{Project, ProjectId, Projects};
pub use error::ProjectLifecycleError;
use std::sync::Arc;
use store::ProjectLifecycleStore;

#[derive(Clone)]
pub struct ProjectLifecycle {
    store: ProjectLifecycleStore,
    projects: Projects,
    code: Code,
}

impl ProjectLifecycle {
    pub fn new(storage: Arc<Storage>, projects: Projects, code: Code) -> Self {
        Self {
            store: ProjectLifecycleStore::with_storage(storage),
            projects,
            code,
        }
    }

    /// A successful creation, including a replay, makes the project active in the index.
    pub async fn create(
        &self,
        command_id: String,
        slug: &str,
        name: &str,
        directory: &str,
    ) -> Result<Project, ProjectLifecycleError> {
        let project = self
            .projects
            .create(command_id, slug, name, directory)
            .await?;
        self.code.touch(&project.id).await;
        Ok(project)
    }

    /// Commit removal before stopping its index worker; a replay returns the retained row.
    pub async fn remove(
        &self,
        command_id: String,
        project: &ProjectId,
    ) -> Result<Project, ProjectLifecycleError> {
        let ctx = CommandContext::user(
            command_id,
            "project.remove",
            serde_json::json!({ "project": project }),
        );
        let removed = self.store.remove(&ctx, project).await?;
        let code = self.code.clone();
        let gone = project.clone();
        // A client disconnect after commit must not leave the worker running.
        let _ = tokio::spawn(async move { code.forget(&gone).await }).await;
        Ok(removed)
    }
}
