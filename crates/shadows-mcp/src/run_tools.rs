//! Runs service MCP translation.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use serde::Deserialize;
use serde_json::json;
use shadows_core::Grant;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskCommitArgs {
    /// What the commit does and why, in a few lines. Shadows adds the task's
    /// number and title above it.
    message: String,
}

#[tool_router(router = run_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Commit your task's work once its checks pass. Shadows commits only the \
                       changed files inside your task's writes; any other changed file stays \
                       uncommitted and is listed in `left`. One commit per attempt."
    )]
    async fn task_commit(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskCommitArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .commit(&grant, args.message)
                .await
                .map(|done| {
                    json!({ "commit": done.attempt.commit, "files": done.files, "left": done.left })
                })
                .map_err(Refusal::from),
        )
    }
}
