//! Roadmap approval, which refuses to remove an outcome a plan links to.
use super::Plans;
use crate::design::{Design, DesignChange};
use crate::grants::Approver;
use crate::{CoreError, Grant, ProjectId};
use core_db::CommandContext;

impl Plans {
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
        self.approve_roadmap_with(ctx, project, proposal, Approver::Person)
            .await
    }

    pub async fn approve_roadmap_proposal_as_planner(
        &self,
        grant: &Grant,
        proposal: &str,
    ) -> Result<DesignChange, CoreError> {
        let approver = self.design.planner_approver(grant).await?;
        let ctx = approver.command(
            "RoadmapProposalApprove",
            serde_json::json!({"project":grant.project_id,"proposal":proposal}),
        );
        self.approve_roadmap_with(ctx, &grant.project_id, proposal, approver)
            .await
    }

    async fn approve_roadmap_with(
        &self,
        ctx: CommandContext,
        project: &ProjectId,
        proposal: &str,
        approver: Approver,
    ) -> Result<DesignChange, CoreError> {
        let (project, proposal) = (project.clone(), proposal.to_owned());
        self.store
            .storage
            .write_txn_as::<_, _, CoreError>(move |conn| {
                Box::pin(async move {
                    let planned = Self::outcomes_with_plans_in(conn, &project).await?;
                    Design::approve_roadmap_proposal_in(
                        conn, &ctx, &project, &proposal, &approver, &planned,
                    )
                    .await
                })
            })
            .await
            .map_err(CoreError::invalid_constraint)
    }
}
