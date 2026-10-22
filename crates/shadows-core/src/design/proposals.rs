//! Vision proposal values and the person/Planner entry points (§23.5).
use super::{Design, DesignChange, VisionField};
use crate::db::{Anchor, CommandContext, derived_id, fingerprint};
use crate::{CoreError, ErrorCode, Grant, ProjectId, ThreadId, grants::Writer};
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
    pub approved_by: Option<crate::grants::ApprovedBy>,
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

impl Design {
    pub(super) fn core_error(e: impl Into<CoreError>) -> CoreError {
        match e.into() {
            CoreError::Storage(crate::StorageError::Constraint(message)) => CoreError::Refused {
                code: ErrorCode::InvalidCommand,
                message,
            },
            other => other,
        }
    }
}

impl Design {
    pub(super) async fn planner_approver(
        &self,
        grant: &Grant,
    ) -> Result<crate::grants::Approver, CoreError> {
        let writer = super::agreement_scope::writer(grant)?;
        let Writer::Planner { thread, .. } = &writer else {
            return Err(CoreError::Refused {
                code: ErrorCode::GrantScope,
                message: "approval needs a Planner's conversation".into(),
            });
        };
        let turn =
            self.operations
                .running_for(thread)
                .await?
                .ok_or_else(|| CoreError::Refused {
                    code: ErrorCode::InvalidCommand,
                    message: "no turn is running".into(),
                })?;
        let harness = crate::threads::Threads::turn_context_in(
            &mut *self
                .storage
                .reader()
                .begin()
                .await
                .map_err(crate::db::StorageError::from)?,
            thread,
        )
        .await?
        .harness;
        Ok(crate::grants::Approver::Planner {
            writer,
            turn: turn.to_string(),
            harness,
        })
    }

    pub async fn approve_vision_proposal_as_planner(
        &self,
        grant: &Grant,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let approver = self.planner_approver(grant).await?;
        let ctx = approver.command(
            "VisionProposalApprove",
            serde_json::json!({"project":grant.project_id,"proposal":proposal}),
        );
        self.storage
            .approve_vision_proposal_in(&ctx, &grant.project_id, proposal, &approver)
            .await
            .map_err(Self::core_error)
    }

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
            .operations
            .running_for(thread)
            .await?
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
            command_id: derived_id(Anchor::Operation(op.as_str()), &fp),
            command_kind: "VisionPropose".into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        };
        self.storage
            .propose_vision_in(&ctx, &writer, &grant.project_id, &op, args)
            .await
            .map_err(Self::core_error)
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
        let ctx = CommandContext::user(
            command_id,
            "VisionProposalApprove",
            serde_json::json!({"project":project,"proposal":proposal}),
        );
        self.storage
            .approve_vision_proposal_in(&ctx, project, proposal, &crate::grants::Approver::Person)
            .await
            .map_err(Self::core_error)
    }
}
