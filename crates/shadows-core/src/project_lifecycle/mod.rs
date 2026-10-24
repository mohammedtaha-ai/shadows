//! Project lifecycle effects that span the registration and indexing owners.
mod error;
mod store;

use crate::CoreError;
use core_code::Code;
use core_db::{CommandContext, Storage};
use core_projects::{Project, ProjectId, Projects};
use std::sync::Arc;
use store::ProjectLifecycleStore;

#[derive(Clone)]
pub struct ProjectLifecycle {
    store: ProjectLifecycleStore,
    projects: Projects,
    code: Code,
}

impl ProjectLifecycle {
    pub(crate) fn new(storage: Arc<Storage>, projects: Projects, code: Code) -> Self {
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
    ) -> Result<Project, CoreError> {
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
    ) -> Result<Project, CoreError> {
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

#[cfg(test)]
mod tests {
    use core_code::CodeConfig;

    #[tokio::test]
    async fn creation_and_replay_activate_the_project_index() {
        let tmp = tempfile::tempdir().unwrap();
        let core = crate::testing::app(tmp.path().join("core.db"))
            .await
            .unwrap();
        core.code().start(CodeConfig::default()).await.unwrap();
        let directory = tmp.path().join("project");
        std::fs::create_dir(&directory).unwrap();
        let create = || {
            core.project_lifecycle().create(
                "register-project".into(),
                "project",
                "Project",
                directory.to_str().unwrap(),
            )
        };
        let project = create().await.unwrap();
        assert!(core.code().active_for_test().await.contains(&project.id));
        core.code().forget(&project.id).await;
        assert!(!core.code().active_for_test().await.contains(&project.id));
        let replay = create().await.unwrap();
        assert_eq!(replay.id, project.id);
        assert!(core.code().active_for_test().await.contains(&project.id));
        core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
    }
}
