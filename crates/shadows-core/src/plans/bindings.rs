//! Exact shared-agreement version pins owned by a plan version.
use crate::design::{AgreementId, AgreementRole, PartId};
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
pub struct AgreementBinding {
    pub task: u32,
    #[schemars(with = "String")]
    pub agreement_id: AgreementId,
    pub version: i64,
    #[schemars(with = "String")]
    pub part_id: PartId,
    pub role: AgreementRole,
    pub operations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct BindingParticipant {
    pub plan_id: super::PlanId,
    pub workflow_id: super::WorkflowId,
    pub title: String,
    pub plan_version: i64,
    pub revision: i64,
    pub plan_state: super::PlanState,
    pub workflow_state: super::WorkflowState,
    pub current: bool,
    pub binding: AgreementBinding,
}
