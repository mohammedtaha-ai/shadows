/// Spec section 11.1 requires selecting a local directory without a path
/// becoming the project's identity. The id is a UUID and the slug is the
/// stable human key; the directory is configuration, not identity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}
