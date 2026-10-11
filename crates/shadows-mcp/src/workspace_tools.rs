//! Workspace reads and Planner vision proposal translation (§23.5).
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

#[tool_router(router = workspace_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Propose one change to the vision: add an item to a field, or edit or \
        remove an item by its id. One call is one card; several calls in a reply are several \
        cards. Nothing changes until the person approves the card's button."
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
