//! One job: reading a project's links. Writing one is a command Plans owns,
//! through `put_link_in` and `remove_link_in`.

use super::Codebase;
use super::model::ProjectLink;
use crate::CodebaseError;
use core_projects::ProjectId;

impl Codebase {
    /// The projects `project` reads, by their slug.
    pub async fn links(&self, project: &ProjectId) -> Result<Vec<ProjectLink>, CodebaseError> {
        Ok(self.inner.store.code_links(project).await?)
    }
}
