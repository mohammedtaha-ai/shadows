//! ProjectState's translation for the Planner: the project's state.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::{Grant, PartId};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectStateArgs {
    #[schemars(with = "Option<String>")]
    part_id: Option<PartId>,
}

#[tool_router(router = state_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Read what Shadows knows of this project before you plan: its stage, each \
            part with its plans and their tasks (done, started, not started), its tables and \
            screens by state (planned, built, gap, unchecked), and the gaps. Give part_id for one \
            part in full: its plans' tasks and its elements. Read only; no language server is \
            asked."
    )]
    async fn project_state(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ProjectStateArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .project_state()
                .state(&grant.project_id, args.part_id.as_ref())
                .await
                .map_err(Refusal::from),
        )
    }
}
