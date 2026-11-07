//! The standards values exposed to callers.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// How a part kind's presence is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Required,
    Waivable,
    Optional,
}

/// A layer another layer may depend on. No kind means the same part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LayerRef {
    pub part_kind: Option<String>,
    pub layer: String,
}

/// One role inside a part kind, from the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LayerSpec {
    pub id: String,
    pub role: String,
    pub required: bool,
    pub may_use: Vec<LayerRef>,
}

/// One part kind, the layers it owns and how it may be absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PartKind {
    pub name: String,
    pub owns: String,
    pub presence: Presence,
    pub layers: Vec<LayerSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StandardRule {
    pub id: String,
    pub text: String,
    pub parts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BaseStandards {
    pub version: i64,
    pub parts: Vec<PartKind>,
    pub rules: Vec<StandardRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct AdditionalPart {
    pub name: String,
    pub owns: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct StandardsAdditions {
    pub rules: Vec<StandardRule>,
    pub parts: Vec<AdditionalPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StandardsAdditionsVersion {
    #[serde(skip)]
    pub id: String,
    pub number: i64,
    pub content: StandardsAdditions,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EffectiveStandards {
    pub base: BaseStandards,
    pub additions: Option<StandardsAdditionsVersion>,
}

impl EffectiveStandards {
    /// Base parts followed by added parts, in declared order.
    /// An optional kind is not mandatory.
    pub fn mandatory_parts(&self) -> Vec<&str> {
        self.base
            .parts
            .iter()
            .filter(|p| p.presence != Presence::Optional)
            .map(|p| p.name.as_str())
            .chain(
                self.additions
                    .iter()
                    .flat_map(|a| &a.content.parts)
                    .map(|p| p.name.as_str()),
            )
            .collect()
    }

    /// The catalog layers of one kind. An added or unknown kind has none.
    pub fn layers(&self, kind: &str) -> &[LayerSpec] {
        match self.base.parts.iter().find(|p| p.name == kind) {
            Some(part) => &part.layers,
            None => &[],
        }
    }
}
