pub mod acp;
pub mod breakdown;
pub mod choices;
pub mod claude;
pub mod events;
pub mod policy;

/// The settings a turn runs with (spec §12.7). `effort` is `None` exactly
/// when the model offers none (§12.4).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TurnSettings {
    pub model: String,
    pub mode: String,
    #[schema(required)]
    pub effort: Option<String>,
}
