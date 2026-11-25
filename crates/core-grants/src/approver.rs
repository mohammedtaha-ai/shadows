//! Approval provenance derived by the core from the authenticated call path.
use super::Writer;
use core_db::Actor;
use core_db::{Anchor, CommandContext};
use core_threads::{ThreadEntryId, ThreadId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approver {
    Person,
    Planner(PlannerApprover),
}

/// A Planner approving through its own turn: the only approver whose command id is
/// derived by the core, so only it builds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerApprover {
    pub writer: Writer,
    pub turn: String,
    pub harness: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApprovalRequest {
    pub thread_id: ThreadId,
    pub entry_id: ThreadEntryId,
    pub ordinal: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApprovedBy {
    pub principal: String,
    pub agent: String,
    pub request: Option<ApprovalRequest>,
}

impl PlannerApprover {
    pub fn command(&self, kind: &str, params: serde_json::Value) -> CommandContext {
        let (writer, turn) = (&self.writer, &self.turn);
        let fp = CommandContext::fingerprint(kind, &params);
        let (principal_kind, principal_id) = writer.principal();
        CommandContext {
            agent: Some(format!("planner:{}", self.harness)),
            principal_kind: principal_kind.into(),
            principal_id,
            command_id: Anchor::derived_id(Anchor::Operation(turn.as_str()), &fp),
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        }
    }
}

impl Approver {
    pub fn agent(&self) -> String {
        match self {
            Self::Person => "human".into(),
            Self::Planner(planner) => format!("planner:{}", planner.harness),
        }
    }
    pub fn actor(&self) -> Actor {
        match self {
            Self::Person => Actor::user("local"),
            Self::Planner(planner) => planner.writer.actor(),
        }
    }
}
