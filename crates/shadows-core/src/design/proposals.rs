//! Vision proposal values and the person/Planner entry points (§23.5).
use super::{Design, DesignChange, VisionField};
use crate::{
    CoreError, ErrorCode, Grant, ProjectId, ThreadId,
    app::user_command,
    command::{
        CommandContext, Writer,
        derive::{Anchor, derived_id},
        fingerprint,
    },
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum VisionAction {
    Add,
    Edit,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct VisionProposal {
    #[serde(default)]
    pub approved_by: Option<crate::command::ApprovedBy>,
    pub id: String,
    pub thread_id: ThreadId,
    pub action: VisionAction,
    pub field: VisionField,
    pub item_id: Option<String>,
    pub text: Option<String>,
    pub old_text: Option<String>,
    pub expected_item_revision: Option<i64>,
    pub status: ProposalStatus,
    pub outdated: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionPropose {
    pub action: VisionAction,
    pub field: Option<VisionField>,
    pub item_id: Option<String>,
    pub text: Option<String>,
    pub replaces: Option<String>,
}

pub(super) fn core_error(e: crate::StorageError) -> CoreError {
    match e {
        crate::StorageError::Constraint(message) => CoreError::Refused {
            code: ErrorCode::InvalidCommand,
            message,
        },
        other => other.into(),
    }
}

impl Design {
    pub async fn propose_vision(
        &self,
        grant: &Grant,
        args: VisionPropose,
    ) -> Result<VisionProposal, CoreError> {
        let writer = super::agreement_scope::writer(grant)?;
        let Writer::Planner { thread, .. } = &writer else {
            return Err(CoreError::Refused {
                code: ErrorCode::GrantScope,
                message: "vision_propose needs a Planner's conversation".into(),
            });
        };
        let op = self
            .handles
            .running_for(thread)
            .await
            .ok_or_else(|| CoreError::Refused {
                code: ErrorCode::InvalidCommand,
                message: "no turn is running for this conversation".into(),
            })?;
        let fp = fingerprint(
            "VisionPropose",
            &serde_json::to_value(&args).map_err(crate::StorageError::from)?,
        );
        let (kind, id) = writer.principal();
        let ctx = CommandContext {
            agent: None,
            principal_kind: kind.into(),
            principal_id: id,
            command_id: derived_id(Anchor::Operation(&op), &fp),
            command_kind: "VisionPropose".into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        };
        self.storage
            .propose_vision_in(&ctx, &writer, &grant.project_id, &op, args)
            .await
            .map_err(core_error)
    }

    pub async fn vision_proposals(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<VisionProposal>, CoreError> {
        Ok(self.storage.list_vision_proposals(project).await?)
    }

    pub async fn approve_vision_proposal(
        &self,
        command_id: String,
        project: &ProjectId,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let ctx = user_command(
            command_id,
            "VisionProposalApprove",
            serde_json::json!({"project":project,"proposal":proposal}),
        );
        self.storage
            .approve_vision_proposal_in(&ctx, project, proposal, &crate::command::Approver::Person)
            .await
            .map_err(core_error)
    }
}
