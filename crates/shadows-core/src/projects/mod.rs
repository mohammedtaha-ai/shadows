//! One job: the projects and the folders a person picks them from (spec §4,
//! §11.1, §12.5, §14.4) — the `Projects` service.
//!
//! A project's directory is resolved (absolute, canonical, a directory)
//! before the fingerprint is taken, so two spellings of one folder are one
//! request and a bad path is refused before any write. A mode change is
//! checked against Shadows' policy for each harness, and each list is a set.
//!
//! `model` holds the types, `directory` the check that makes a
//! `ProjectDirectory`, `harness_policy` which harness a thread or a project's
//! modes may name, `browse` the disk a person picks from, and `store` the
//! queries.

mod browse;
mod directory;
mod harness_policy;
mod model;
mod store;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use shadows_agent::policy;

pub use browse::{DirectoryEntry, DirectoryListing};
pub use directory::{DirectoryError, ProjectDirectory};
pub use model::{Project, ProjectId};

use crate::code::Code;
use crate::db::{CommandContext, Storage, StorageError};
use crate::error::CoreError;
use store::ProjectsStore;

/// Projects: what storage holds, and the code index a new project is
/// touched in. Cheap to clone. The copy the readers hold has no code
/// index (`new`); only the one `AppCore` serves does (`with_code`).
#[derive(Clone)]
pub struct Projects {
    store: ProjectsStore,
    code: Option<Code>,
}

impl Projects {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self {
            store: ProjectsStore::new(storage),
            code: None,
        }
    }

    /// This service with the code index that `create` touches and `remove`
    /// forgets in. Code itself reads projects, so it is built over a copy made
    /// by `new`.
    pub(crate) fn with_code(mut self, code: Code) -> Self {
        self.code = Some(code);
        self
    }

    /// Every project, oldest first.
    pub async fn list(&self) -> Result<Vec<Project>, CoreError> {
        Ok(self.store.list().await?)
    }

    /// The project, `NotFound` once removed.
    pub async fn get(&self, id: &ProjectId) -> Result<Project, CoreError> {
        Ok(self.store.get(id).await?)
    }

    /// Live projects, most recently used first: by the newest
    /// operation among their threads, or by their creation for one with none.
    /// Computed from the journal, so no caller joins another service's table.
    pub async fn by_recent_activity(&self) -> Result<Vec<Project>, CoreError> {
        Ok(self.store.by_recent_activity().await?)
    }

    /// AYLUL's project: the live project `slug`, created over `directory`
    /// when absent and marked `assistant`. Idempotent: a second call returns
    /// the same project. A project that already holds the slug and is not
    /// AYLUL's is returned unchanged, never claimed.
    pub async fn ensure_assistant(
        &self,
        slug: &str,
        name: &str,
        directory: &Path,
    ) -> Result<Project, CoreError> {
        if let Some(found) = (self.store.list().await?)
            .into_iter()
            .find(|p| p.slug == slug)
        {
            if !found.assistant {
                tracing::warn!(slug, "projects.assistant_slug_taken");
            }
            return Ok(found);
        }
        // A removed AYLUL project keeps its slug (UNIQUE) and a replay of the
        // creation command would answer that dead row: bring the row back.
        if let Some(restored) = self.store.restore_assistant(slug).await? {
            return Ok(restored);
        }
        let directory = ProjectDirectory::resolve(directory)?;
        let params = serde_json::json!({
            "slug": slug, "name": name, "directory": directory.as_str(),
        });
        let c = CommandContext::user(
            format!("assistant-project-{slug}"),
            "project.create",
            params,
        );
        self.store
            .create(&c, slug, name, &directory, &policy::default_modes())
            .await?;
        let marked = self.store.mark_assistant(slug).await?;
        marked.ok_or_else(|| StorageError::NotFound("project").into())
    }

    /// Creates a project owning an existing directory: "project.create",
    /// params { "slug", "name", "directory" }, the directory as resolved.
    pub async fn create(
        &self,
        command_id: String,
        slug: &str,
        name: &str,
        directory: &str,
    ) -> Result<Project, CoreError> {
        // Resolved before the fingerprint is taken, so two spellings of one
        // folder are one request, and a bad path is refused before any write.
        let directory = ProjectDirectory::resolve(Path::new(directory))?;
        let params = serde_json::json!({
            "slug": slug, "name": name, "directory": directory.as_str(),
        });
        let c = CommandContext::user(command_id, "project.create", params);
        let project = self
            .store
            .create(&c, slug, name, &directory, &policy::default_modes())
            .await?;
        // A created project is a used one (§15.6); a replay touches it again.
        if let Some(code) = &self.code {
            code.touch(&project.id).await;
        }
        Ok(project)
    }

    /// Sets the modes the project allows, per harness (spec §12.5): every
    /// harness known and every mode in its policy, each list deduplicated and
    /// sorted. "project.modes", params { "project", "allowed_modes" }.
    pub async fn set_modes(
        &self,
        command_id: String,
        project: &ProjectId,
        allowed_modes: BTreeMap<String, Vec<String>>,
    ) -> Result<Project, CoreError> {
        let mut modes = BTreeMap::new();
        for (harness, list) in allowed_modes {
            Self::known_harness(&harness)?;
            let policy = policy::allowed_modes(&harness);
            let mut set: Vec<String> = Vec::new();
            for mode in list {
                if !policy.contains(&mode.as_str()) {
                    return Err(CoreError::SettingNotOffered {
                        what: "mode".into(),
                        id: mode,
                        detail: None,
                    });
                }
                if !set.contains(&mode) {
                    set.push(mode);
                }
            }
            // A set: the order it was sent in is not part of the request.
            set.sort();
            modes.insert(harness, set);
        }
        let params = serde_json::json!({ "project": project, "allowed_modes": modes });
        let c = CommandContext::user(command_id, "project.modes", params);
        Ok(self.store.set_modes(&c, project, &modes).await?)
    }

    /// Removes a project that holds no planning thread (§4.2, §15.4):
    /// "project.remove", params { "project" }. Its index, its links both ways
    /// and its live project grants go in the same write; then the code index
    /// forgets it, which never fails the removal. A replay answers the
    /// removed project and changes nothing.
    pub async fn remove(
        &self,
        command_id: String,
        project: &ProjectId,
    ) -> Result<Project, CoreError> {
        let params = serde_json::json!({ "project": project });
        let c = CommandContext::user(command_id, "project.remove", params);
        let removed = self.store.remove(&c, project).await?;
        if let Some(code) = self.code.clone() {
            // On its own task: a caller dropped after the commit (a client that
            // hung up) must not leave the removed project's worker running.
            let gone = project.clone();
            let _ = tokio::spawn(async move { code.forget(&gone).await }).await;
        }
        Ok(removed)
    }

    /// A directory's immediate subdirectories, or the roots when `path` is
    /// absent or empty. Read on a blocking thread.
    pub async fn list_dirs(&self, path: Option<String>) -> Result<DirectoryListing, CoreError> {
        let path = path.filter(|p| !p.is_empty());
        Ok(browse::blocking(move || browse::list(path.as_deref().map(Path::new))).await?)
    }

    /// Creates `name` inside `parent`, one new directory to choose. Made on a
    /// blocking thread.
    pub async fn create_dir(
        &self,
        parent: String,
        name: String,
    ) -> Result<DirectoryEntry, CoreError> {
        Ok(
            browse::blocking(move || browse::create_subdirectory(Path::new(&parent), &name))
                .await?,
        )
    }
}

/// What the tests drive below the public methods: a write with the command
/// context the test built, which `create` and `set_modes` derive themselves.
#[cfg(feature = "test-support")]
impl Projects {
    pub(crate) async fn create_for_test(
        &self,
        ctx: &CommandContext,
        slug: &str,
        name: &str,
        directory: &ProjectDirectory,
        default_modes: &BTreeMap<String, Vec<String>>,
    ) -> Result<Project, StorageError> {
        self.store
            .create(ctx, slug, name, directory, default_modes)
            .await
    }

    pub(crate) async fn set_modes_for_test(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        modes: &BTreeMap<String, Vec<String>>,
    ) -> Result<Project, StorageError> {
        self.store.set_modes(ctx, project, modes).await
    }
}
