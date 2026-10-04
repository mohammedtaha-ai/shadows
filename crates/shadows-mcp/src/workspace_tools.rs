//! The project workspace, read only, for a grant's agent (§18.10).
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{handler::server::tool::Extension, model::CallToolResult, tool, tool_router};
use shadows_core::Grant;

#[tool_router(router = workspace_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Read this project's workspace: the vision a person wrote, and its top-level parts and roadmap outcomes. Read it before planning new work. It is read only."
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
