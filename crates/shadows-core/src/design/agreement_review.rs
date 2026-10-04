//! Values returned by a shared agreement impact review.
use super::{AgreementId, AgreementParty};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgreementPartyReview {
    pub party: AgreementParty,
    pub revision: i64,
    pub title: String,
    pub change: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgreementParticipantImpact {
    pub participant: crate::plans::BindingParticipant,
    pub affected: bool,
    pub changes: serde_json::Value,
    pub next_action: String,
    pub execution: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgreementReview {
    pub agreement_id: AgreementId,
    pub version: i64,
    pub revision: i64,
    pub review_id: String,
    pub base_version: Option<i64>,
    pub compatibility: String,
    pub changes: serde_json::Value,
    pub parties: Vec<AgreementPartyReview>,
    pub participants: Vec<AgreementParticipantImpact>,
    pub limits: Vec<String>,
}
