//! Roadmap proposal values and service entry points.
use super::{Design, DesignChange, OutcomeContent, OutcomeId, PartId, ProposalStatus};
use core_db::CommandContext;

use crate::{DesignError, Grant, ProjectId, ThreadId};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;
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
    pub approved_by: Option<core_grants::ApprovedBy>,
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
    ) -> Result<RoadmapProposal, DesignError> {
        let (writer, turn) = self
            .proposal_writer(
                grant,
                "roadmap_propose needs a Planner's conversation",
                "no turn is running",
            )
            .await?;
        for outcome in &mut args.outcomes {
            outcome.content.title = outcome.content.title.trim().to_owned();
        }
        let ctx = Self::proposal_command(&writer, &turn, "RoadmapPropose", &args)?;
        self.store
            .propose_roadmap_in(&ctx, &writer, &grant.project_id, &turn, args)
            .await
            .map_err(DesignError::invalid_constraint)
    }

    pub async fn roadmap_proposals(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<RoadmapProposal>, DesignError> {
        Ok(self.store.list_roadmap_proposals(project).await?)
    }

    /// Approves a roadmap proposal in the caller's transaction. `planned` is
    /// the set of outcomes a plan links to: a removal of one of them is refused.
    pub async fn approve_roadmap_proposal_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
        proposal: &str,
        approver: &core_grants::Approver,
        planned: &BTreeSet<String>,
    ) -> Result<DesignChange, DesignError> {
        super::store::DesignStore::approve_roadmap_in(
            conn, ctx, project, proposal, approver, planned,
        )
        .await
    }
}
