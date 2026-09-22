use crate::id::newtype_id;

newtype_id! {
    /// Spec §4.1. A project's identity is a UUID, never its directory path:
    /// §11.1 requires selecting a local directory without that path becoming the
    /// project's identity, and the slug is the stable human key.
    ProjectId
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Project {
    pub id: ProjectId,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}
