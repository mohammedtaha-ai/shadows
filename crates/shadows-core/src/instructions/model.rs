//! One job: the instructions type callers meet.

/// One saved version of a project's instructions. `id` is what an
/// `agent_invocation` records (§13.15); a client sees the number instead.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct InstructionsVersion {
    #[serde(skip)]
    pub id: String,
    /// 1, 2, 3 … within the project.
    pub number: i64,
    pub body: String,
    pub created_at: String,
}
