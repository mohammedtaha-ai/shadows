//! Map proposal values and service entry points.
use super::{Design, DesignChange, PartContent, PartId, ProposalStatus};
use core_db::CommandContext;

use crate::{CoreError, ErrorCode, Grant, ProjectId, ThreadId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MapAction {
    First,
    Add,
    Change,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, schemars::JsonSchema)]
pub struct ProposedPart {
    pub key: String,
    pub parent_key: Option<String>,
    #[schemars(with = "Option<String>")]
    pub parent: Option<PartId>,
    pub content: PartContent,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, schemars::JsonSchema)]
pub struct ProposedWaiver {
    pub part: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MapPropose {
    pub action: MapAction,
    pub parts: Vec<ProposedPart>,
    #[schemars(with = "Option<String>")]
    pub part_id: Option<PartId>,
    #[serde(default)]
    pub waivers: Vec<ProposedWaiver>,
    pub replaces: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MapProposal {
    #[serde(default)]
    pub approved_by: Option<crate::grants::ApprovedBy>,
    pub id: String,
    pub thread_id: ThreadId,
    pub action: MapAction,
    pub parts: Vec<ProposedPart>,
    pub part_id: Option<PartId>,
    pub old: Option<PartContent>,
    pub waivers: Vec<ProposedWaiver>,
    pub status: ProposalStatus,
    pub outdated: bool,
    pub created_at: String,
}
impl Design {
    pub async fn approve_map_proposal_as_planner(
        &self,
        grant: &Grant,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let approver = self.planner_approver(grant).await?;
        let ctx = approver.command(
            "MapProposalApprove",
            serde_json::json!({"project":grant.project_id,"proposal":proposal}),
        );
        // The store performs waiver rejection after project scoping and authority checks.
        self.store
            .approve_map_proposal_in(&ctx, &grant.project_id, proposal, &approver)
            .await
            .map_err(CoreError::invalid_constraint)
    }

    pub async fn propose_map(
        &self,
        grant: &Grant,
        mut args: MapPropose,
    ) -> Result<MapProposal, CoreError> {
        let (writer, turn) = self
            .proposal_writer(
                grant,
                "map_propose needs a Planner's conversation",
                "no turn is running",
            )
            .await?;
        for part in &mut args.parts {
            super::map_rules::MapRules::normalise(&mut part.content).map_err(|message| {
                CoreError::Refused {
                    code: ErrorCode::InvalidCommand,
                    message,
                }
            })?;
        }
        let ctx = Self::proposal_command(&writer, &turn, "MapPropose", &args)?;
        self.store
            .propose_map_in(&ctx, &writer, &grant.project_id, &turn, args)
            .await
            .map_err(CoreError::invalid_constraint)
    }
    pub async fn map_proposals(&self, project: &ProjectId) -> Result<Vec<MapProposal>, CoreError> {
        Ok(self.store.list_map_proposals(project).await?)
    }
    pub async fn approve_map_proposal(
        &self,
        command_id: String,
        project: &ProjectId,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let ctx = CommandContext::user(
            command_id,
            "MapProposalApprove",
            serde_json::json!({"project":project,"proposal":proposal}),
        );
        self.store
            .approve_map_proposal_in(&ctx, project, proposal, &crate::grants::Approver::Person)
            .await
            .map_err(CoreError::invalid_constraint)
    }
}
