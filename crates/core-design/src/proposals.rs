//! Vision proposal values and the person/Planner entry points.
use super::{Design, DesignChange, VisionField};
use crate::{DesignError, Grant, ProjectId, ThreadId};
use core_db::CommandContext;
use core_grants::Writer;
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
    pub approved_by: Option<core_grants::ApprovedBy>,
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
    pub async fn planner_approver(
        &self,
        grant: &Grant,
    ) -> Result<core_grants::Approver, DesignError> {
        let writer = super::agreement_scope::AgreementScope::writer(grant)?;
        let Writer::Planner { thread, .. } = &writer else {
            return Err(DesignError::GrantScope(
                "approval needs a Planner's conversation".into(),
            ));
        };
        let turn = self
            .operations
            .running_for(thread)
            .await?
            .ok_or_else(|| DesignError::InvalidCommand("no turn is running".into()))?;
        let harness = core_threads::Threads::turn_context_in(
            &mut *self
                .store
                .storage
                .reader()
                .begin()
                .await
                .map_err(core_db::StorageError::from)?,
            thread,
        )
        .await?
        .harness;
        Ok(core_grants::Approver::Planner {
            writer,
            turn: turn.to_string(),
            harness,
        })
    }

    pub async fn approve_vision_proposal_as_planner(
        &self,
        grant: &Grant,
        proposal: &str,
    ) -> Result<DesignChange, DesignError> {
        let approver = self.planner_approver(grant).await?;
        let ctx = approver.command(
            "VisionProposalApprove",
            serde_json::json!({"project":grant.project_id,"proposal":proposal}),
        );
        self.store
            .approve_vision_proposal_in(&ctx, &grant.project_id, proposal, &approver)
            .await
            .map_err(DesignError::invalid_constraint)
    }

    pub async fn propose_vision(
        &self,
        grant: &Grant,
        args: VisionPropose,
    ) -> Result<VisionProposal, DesignError> {
        let (writer, op) = self
            .proposal_writer(
                grant,
                "vision_propose needs a Planner's conversation",
                "no turn is running for this conversation",
            )
            .await?;
        let ctx = Self::proposal_command(&writer, &op, "VisionPropose", &args)?;
        self.store
            .propose_vision_in(&ctx, &writer, &grant.project_id, &op, args)
            .await
            .map_err(DesignError::invalid_constraint)
    }

    pub async fn vision_proposals(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<VisionProposal>, DesignError> {
        Ok(self.store.list_vision_proposals(project).await?)
    }

    pub async fn approve_vision_proposal(
        &self,
        command_id: String,
        project: &ProjectId,
        proposal: &str,
    ) -> Result<DesignChange, DesignError> {
        let ctx = CommandContext::user(
            command_id,
            "VisionProposalApprove",
            serde_json::json!({"project":project,"proposal":proposal}),
        );
        self.store
            .approve_vision_proposal_in(&ctx, project, proposal, &core_grants::Approver::Person)
            .await
            .map_err(DesignError::invalid_constraint)
    }
}
