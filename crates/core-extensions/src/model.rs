//! Public extension library metadata contains no local paths.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionKind {
    Plugin,
    Skill,
}
impl ExtensionKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Skill => "skill",
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgentItem {
    #[serde(flatten)]
    pub item: Item,
    pub enabled: bool,
}
#[derive(Debug, Default, serde::Serialize, utoipa::ToSchema)]
pub struct ExtensionLibrary {
    pub plugins: Vec<Item>,
    pub skills: Vec<Item>,
}
#[derive(Debug, Default, serde::Serialize, utoipa::ToSchema)]
pub struct AgentExtensions {
    pub plugins: Vec<AgentItem>,
    pub skills: Vec<AgentItem>,
}
