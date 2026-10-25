//! One job: the project types callers meet.

use std::collections::BTreeMap;

use core_db::newtype_id;

newtype_id! {
    /// A project's identity is a UUID, never its directory path:
    /// Selecting a local directory must not make that path the
    /// project's identity, and the slug is the stable human key.
    ProjectId
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Project {
    pub id: ProjectId,
    pub slug: String,
    pub name: String,
    /// The directory this project's turns run in, as
    /// [`ProjectDirectory`] resolved it. `None` when no directory is recorded;
    /// a turn on such a project fails at Prepare rather than running in the
    /// daemon's own working directory.
    pub directory: Option<String>,
    pub created_at: String,
    /// Per harness kind, the modes this project's turns may use.
    /// Each list is a set: its order carries no meaning. Every known harness
    /// is present; an empty list means no turn can start on it.
    pub allowed_modes: BTreeMap<String, Vec<String>>,
    /// True only for the project `Projects::ensure_assistant` created: the
    /// home of AYLUL's conversations, which clients list apart from projects.
    pub assistant: bool,
}

impl ProjectId {
    /// Reconstitutes an identity already recorded by a service store.
    pub fn from_recorded(value: String) -> Self {
        Self::from_stored(value)
    }
}
