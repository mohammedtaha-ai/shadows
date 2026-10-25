//! Public agent identity metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Agent {
    pub slug: String,
    pub name: String,
    pub provider: String,
    pub default_model: String,
    #[schema(pattern = "^[a-z][a-z0-9-]{0,31}$")]
    pub character_color: String,
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgentInput {
    pub name: String,
    pub provider: String,
    pub default_model: String,
    #[schema(pattern = "^[a-z][a-z0-9-]{0,31}$")]
    pub character_color: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AgentWorkspace {
    pub project_id: core_projects::ProjectId,
    pub agents: Vec<Agent>,
}
