//! One job: the projects and the folders a person picks them from (spec §4,
//! §11.1, §12.5, §14.4) — the `Projects` service.
//!
//! A project's directory is resolved (absolute, canonical, a directory)
//! before the fingerprint is taken, so two spellings of one folder are one
//! request and a bad path is refused before any write. A mode change is
//! checked against Shadows' policy for each harness, and each list is a set.
//!
//! `model` holds the types, `directory` the check that makes a
//! `ProjectDirectory`, `browse` the disk a person picks from, and `store` the
//! queries.

mod browse;
mod directory;
mod model;
mod store;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use shadows_agent::policy;

pub use browse::{DirectoryEntry, DirectoryListing};
pub use directory::{DirectoryError, ProjectDirectory};
pub use model::{Project, ProjectId};

use crate::app::user_command;
use crate::error::CoreError;
use crate::storage::Storage;
use crate::threads::known_harness;

/// Projects: what storage holds.
pub struct Projects {
    storage: Arc<Storage>,
}

impl Projects {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Every project, oldest first.
    pub async fn list(&self) -> Result<Vec<Project>, CoreError> {
        Ok(self.storage.list_projects().await?)
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
        let c = user_command(command_id, "project.create", params);
        Ok(self
            .storage
            .create_project(&c, slug, name, &directory, &policy::default_modes())
            .await?)
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
            known_harness(&harness)?;
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
        let c = user_command(command_id, "project.modes", params);
        Ok(self.storage.set_project_modes(&c, project, &modes).await?)
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
