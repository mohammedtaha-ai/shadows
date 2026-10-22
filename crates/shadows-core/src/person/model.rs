//! The person's public record.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Person {
    pub name: Option<String>,
    pub aylul_calls_me: Option<String>,
    pub language: Option<String>,
    pub theme: Theme,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PersonInput {
    pub name: Option<String>,
    pub aylul_calls_me: Option<String>,
    pub language: Option<String>,
    pub theme: Theme,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    #[default]
    System,
}
