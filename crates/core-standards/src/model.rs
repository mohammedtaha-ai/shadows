//! The standards values exposed to callers.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MandatoryPart {
    pub name: String,
    pub owns: String,
    pub waivable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StandardRule {
    pub id: String,
    pub text: String,
    pub parts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ContractTemplate {
    pub rules: Vec<String>,
    pub shape: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BaseStandards {
    pub version: i64,
    pub parts: Vec<MandatoryPart>,
    pub rules: Vec<StandardRule>,
    pub contract_template: ContractTemplate,
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
    pub fn mandatory_parts(&self) -> Vec<&str> {
        self.base
            .parts
            .iter()
            .map(|p| p.name.as_str())
            .chain(
                self.additions
                    .iter()
                    .flat_map(|a| &a.content.parts)
                    .map(|p| p.name.as_str()),
            )
            .collect()
    }
}
