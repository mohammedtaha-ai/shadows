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
use shadows_core::{Grant, OutcomeId, PartId};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectStateArgs {
    #[schemars(with = "Option<String>")]
    part_id: Option<PartId>,
    #[schemars(with = "Option<String>")]
    outcome_id: Option<OutcomeId>,
}

#[tool_router(router = state_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Read what Shadows knows of this project before you plan: its stage, each \
            part with its plans and their tasks (done, started, not started), its tables and \
            screens by state (planned, built, gap, unchecked), the gaps, each roadmap outcome \
            with its plans' task counts, and the project's task totals. Give part_id for one \
            part in full: the tasks that read or write it and its elements; or outcome_id for \
            one outcome in full: its plans and their tasks. Never both. Read only; no language \
            server is asked."
    )]
    async fn project_state(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ProjectStateArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .project_state()
                .state_for(
                    &grant.project_id,
                    args.part_id.as_ref(),
                    args.outcome_id.as_ref(),
                )
                .await
                .map_err(Refusal::from),
        )
    }
}
