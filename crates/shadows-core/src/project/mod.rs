use std::collections::BTreeMap;

pub mod browse;
pub mod directory;

pub use directory::{DirectoryError, ProjectDirectory};

use crate::id::newtype_id;

newtype_id! {
    /// Spec §4.1. A project's identity is a UUID, never its directory path:
    /// §11.1 requires selecting a local directory without that path becoming the
    /// project's identity, and the slug is the stable human key.
    ProjectId
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Project {
    pub id: ProjectId,
    pub slug: String,
    pub name: String,
    /// Spec §4.2: the directory this project's turns run in, as
    /// [`ProjectDirectory`] resolved it. `None` only for a project created
    /// before projects owned one (`migrations/0003_project_directory.sql`);
    /// a turn on such a project fails at Prepare rather than running in the
    /// daemon's own working directory.
    pub directory: Option<String>,
    pub created_at: String,
    /// Per harness kind, the modes this project's turns may use (spec §12.5).
    /// Each list is a set: its order carries no meaning. Every known harness
    /// is present; an empty list means no turn can start on it.
    pub allowed_modes: BTreeMap<String, Vec<String>>,
}
