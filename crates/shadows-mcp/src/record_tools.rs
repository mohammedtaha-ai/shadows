//! CodeRecord's translation for the Planner: where a topic is in the project.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::Grant;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct CodeWhereArgs {
    /// A topic (login), a path (backend/auth), a symbol, or an element id or name.
    topic: String,
}

#[tool_router(router = record_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Ask where something is in this project before you name a file, a table or \
            a screen: a topic (login), a path (backend/auth), a symbol, or an element id or name. \
            Shadows answers from its record first: each file with the plans and tasks that made \
            it, its symbols and their contracts, the elements and their states. Then the \
            language servers confirm: each symbol's line now and who uses it, by part and layer. \
            Within 30 asks and 20 seconds; what they did not reach is marked unchecked. Files no \
            contract names are listed as not in the record."
    )]
    async fn code_where(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<CodeWhereArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .code_record()
                .code_where(&grant.project_id, &args.topic)
                .await
                .map_err(Refusal::from),
        )
    }
}
