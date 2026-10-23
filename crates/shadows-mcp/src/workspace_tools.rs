//! Workspace reads and Planner proposal translation (§23.5).
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::{Grant, VisionAction, VisionField, VisionPropose};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct VisionProposeArgs {
    action: VisionAction,
    field: Option<VisionField>,
    item_id: Option<String>,
    text: Option<String>,
    replaces: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct ProposalApproveArgs {
    proposal_id: String,
}

#[tool_router(router = workspace_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(description = "Approve a vision proposal at the person's request, recorded as Planner.")]
    async fn vision_approve(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ProposalApproveArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .approve_vision_proposal_as_planner(&grant, &args.proposal_id)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Approve a map at the person's request. First-map waivers need their button."
    )]
    async fn map_approve(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ProposalApproveArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .approve_map_proposal_as_planner(&grant, &args.proposal_id)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Propose a project map. first proposes all parts with slug keys
        and parent_key;
        add/change proposes one part, change names part_id. Include served vision ids, folders,
        starter files and contract rules. First-map api/frontend waivers need written reasons.
        One call draws one card. Approval is separate; this tool writes no approved parts."
    )]
    async fn map_propose(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<shadows_core::MapPropose>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .propose_map(&grant, args)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Approve a roadmap proposal at the person's request, recorded as \
        Planner."
    )]
    async fn roadmap_approve(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ProposalApproveArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .approve_roadmap_proposal_as_planner(&grant, &args.proposal_id)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Propose the roadmap once the structure is written. first proposes \
        every outcome with slug keys and parent_key; after it, add, change or remove one \
        outcome (change and remove name outcome_id; add may name an existing parent; a \
        change replaces content, parts and parent, so give the current parent to keep it \
        and omit it to move the outcome to the top). Each \
        outcome has a title, intended result, acceptance items and the part ids it \
        references (at least one). One call draws one card. Approval is separate; this tool \
        writes no outcome."
    )]
    async fn roadmap_propose(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<shadows_core::RoadmapPropose>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .propose_roadmap(&grant, args)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Propose one change to the vision: add an item to a field, or edit or \
        remove an item by its id. One call is one card; several calls in a reply are several \
        cards. Nothing changes until the button or Planner approval tool approves."
    )]
    async fn vision_propose(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<VisionProposeArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .propose_vision(
                    &grant,
                    VisionPropose {
                        action: args.action,
                        field: args.field,
                        item_id: args.item_id,
                        text: args.text,
                        replaces: args.replaces,
                    },
                )
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Read this project's workspace: the vision a person wrote, and its \
            top-level parts and roadmap outcomes. Read it before planning new work. It is \
            read only."
    )]
    async fn workspace_get(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.core
                .design()
                .workspace_for(&grant)
                .await
                .map_err(Refusal::from),
        )
    }
}
