//! Roadmap proposal values and service entry points (§23.5).
use super::{Design, DesignChange, OutcomeContent, OutcomeId, PartId, ProposalStatus};
use crate::db::{Anchor, CommandContext, derived_id, fingerprint};
use crate::grants::Writer;
use crate::{CoreError, ErrorCode, Grant, ProjectId, ThreadId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RoadmapAction {
    First,
    Add,
    Change,
    Remove,
}

/// One outcome of a proposal, with the parts it references.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, schemars::JsonSchema)]
pub struct ProposedOutcome {
    /// A slug, unique in the proposal.
    pub key: String,
    /// Another key of this proposal, for a first roadmap.
    pub parent_key: Option<String>,
    /// An existing outcome: an add's parent, or a change's resulting parent
    /// (absent for the top level).
    #[schemars(with = "Option<String>")]
    pub parent: Option<OutcomeId>,
    pub content: OutcomeContent,
    #[schemars(with = "Vec<String>")]
    pub parts: Vec<PartId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RoadmapPropose {
    pub action: RoadmapAction,
    pub outcomes: Vec<ProposedOutcome>,
    #[schemars(with = "Option<String>")]
    pub outcome_id: Option<OutcomeId>,
    pub replaces: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoadmapProposal {
    #[serde(default)]
    pub approved_by: Option<crate::grants::ApprovedBy>,
    pub id: String,
    pub thread_id: ThreadId,
    pub action: RoadmapAction,
    pub outcomes: Vec<ProposedOutcome>,
    pub outcome_id: Option<OutcomeId>,
    pub old: Option<OutcomeContent>,
    /// The named outcome's parent when proposed; a change whose parent differs moves it.
    #[serde(default)]
    pub old_parent: Option<OutcomeId>,
    pub status: ProposalStatus,
    pub outdated: bool,
    pub created_at: String,
}

impl Design {
    pub async fn propose_roadmap(
        &self,
        grant: &Grant,
        mut args: RoadmapPropose,
    ) -> Result<RoadmapProposal, CoreError> {
        let writer = super::agreement_scope::writer(grant)?;
        let Writer::Planner { thread, .. } = &writer else {
            return Err(CoreError::Refused {
                code: ErrorCode::GrantScope,
                message: "roadmap_propose needs a Planner's conversation".into(),
            });
        };
        let turn = self
            .handles
            .running_for(thread)
            .await
            .ok_or_else(|| CoreError::Refused {
                code: ErrorCode::InvalidCommand,
                message: "no turn is running".into(),
            })?;
        for outcome in &mut args.outcomes {
            outcome.content.title = outcome.content.title.trim().to_owned();
        }
        let fp = fingerprint(
            "RoadmapPropose",
            &serde_json::to_value(&args).map_err(crate::StorageError::from)?,
        );
        let (kind, id) = writer.principal();
        let ctx = CommandContext {
            agent: None,
            principal_kind: kind.into(),
            principal_id: id,
            command_id: derived_id(Anchor::Operation(turn.as_str()), &fp),
            command_kind: "RoadmapPropose".into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        };
        self.storage
            .propose_roadmap_in(&ctx, &writer, &grant.project_id, &turn, args)
            .await
            .map_err(Self::core_error)
    }

    pub async fn roadmap_proposals(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<RoadmapProposal>, CoreError> {
        Ok(self.storage.list_roadmap_proposals(project).await?)
    }

    pub async fn approve_roadmap_proposal(
        &self,
        command_id: String,
        project: &ProjectId,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let ctx = CommandContext::user(
            command_id,
            "RoadmapProposalApprove",
            serde_json::json!({"project":project,"proposal":proposal}),
        );
        self.storage
            .approve_roadmap_proposal_in(&ctx, project, proposal, &crate::grants::Approver::Person)
            .await
            .map_err(Self::core_error)
    }

    pub async fn approve_roadmap_proposal_as_planner(
        &self,
        grant: &Grant,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let approver = self.planner_approver(grant).await?;
        let ctx = approver.command(
            "RoadmapProposalApprove",
            serde_json::json!({"project":grant.project_id,"proposal":proposal}),
        );
        self.storage
            .approve_roadmap_proposal_in(&ctx, &grant.project_id, proposal, &approver)
            .await
            .map_err(Self::core_error)
    }
}
