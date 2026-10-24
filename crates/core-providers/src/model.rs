//! Public provider configuration metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Provider {
    pub slug: String,
    pub name: String,
    pub base_url: String,
    pub preset: Option<String>,
    pub label: Option<String>,
    pub models: Vec<ProviderModel>,
    pub key_set: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ProviderInput {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub preset: Option<String>,
    pub label: Option<String>,
    pub models: Vec<ProviderModelInput>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ProviderModel {
    pub id: String,
    pub context_window: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ProviderModelInput {
    pub id: String,
    pub context_window: Option<u32>,
}
