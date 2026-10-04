//! Shared HTTP agreement value shapes (§18.4).
use super::PartId;

crate::id::newtype_id!(AgreementId);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AgreementRole {
    Provides,
    Uses,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct AgreementParty {
    #[schemars(with = "String")]
    pub part_id: PartId,
    pub role: AgreementRole,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct AgreementContent {
    pub capability: String,
    pub purpose: String,
    pub behavior: String,
    pub acceptance: Vec<String>,
    pub parties: Vec<AgreementParty>,
    pub openapi: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum AgreementState {
    Draft,
    Agreed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgreementIssue {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgreementVersion {
    pub agreement_id: AgreementId,
    pub project_id: crate::ProjectId,
    pub version: i64,
    pub revision: i64,
    pub state: AgreementState,
    pub reason: Option<String>,
    pub writer: AgreementWriter,
    pub created_at: String,
    pub agreed_at: Option<String>,
    pub content: AgreementContent,
    pub issues: Vec<AgreementIssue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgreementWriter {
    pub kind: String,
    pub id: String,
    pub operation_id: Option<crate::OperationId>,
}
