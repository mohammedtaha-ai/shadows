pub mod acp;
pub mod choices;
pub mod claude;
pub mod events;
pub mod policy;
pub mod profile;

/// The settings a turn runs with. `effort` is `None` exactly
/// when the model offers none.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TurnSettings {
    pub model: String,
    pub mode: String,
    #[schema(required)]
    pub effort: Option<String>,
}

mod usage;
