//! Approval provenance derived by the core from the authenticated call path.
use super::Writer;
use crate::{Actor, OperationId, ThreadEntryId, ThreadId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approver {
    Person,
    Planner {
        writer: Writer,
        turn: OperationId,
        harness: String,
    },
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

impl Approver {
    pub(crate) fn command(
        &self,
        kind: &str,
        params: serde_json::Value,
    ) -> crate::db::CommandContext {
        let Self::Planner { writer, turn, .. } = self else {
            unreachable!("Planner command construction is core-only")
        };
        let fp = crate::db::fingerprint(kind, &params);
        let (principal_kind, principal_id) = writer.principal();
        crate::db::CommandContext {
            agent: Some(self.agent()),
            principal_kind: principal_kind.into(),
            principal_id,
            command_id: crate::db::derived_id(crate::db::Anchor::Operation(turn.as_str()), &fp),
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        }
    }

    pub fn agent(&self) -> String {
        match self {
            Self::Person => "human".into(),
            Self::Planner { harness, .. } => format!("planner:{harness}"),
        }
    }
    pub fn actor(&self) -> Actor {
        match self {
            Self::Person => Actor::user("local"),
            Self::Planner { writer, .. } => writer.actor(),
        }
    }
}
