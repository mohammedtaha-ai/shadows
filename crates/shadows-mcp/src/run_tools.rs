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
struct TaskReportArgs {
    /// The full sha of the commit you made with `git commit` for this task.
    commit: String,
    /// What you did, what you ran or tested, and what passed or failed.
    report: String,
}

#[tool_router(router = run_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Report your task's work after you committed it yourself with git. \
                       Shadows accepts the commit only if it is on the checked-out branch, \
                       made after your task started, changes only paths inside your task's \
                       writes and adds no binary file; a refusal says what to fix, then \
                       report again. One accepted report per attempt."
    )]
    async fn task_report(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskReportArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .report(&grant, &args.commit, args.report)
                .await
                .map(|a| json!({ "attempt": a.id, "state": a.state, "commit": a.commit }))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Make this project's folder a git repository when Start says it is not \
                       one: git init and one empty first commit, made in Shadows' name. \
                       Refused when the folder is already inside a repository. It writes no \
                       file; starter files are a task's work."
    )]
    async fn project_git_init(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.core
                .runs()
                .init_git(&grant)
                .await
                .map(|commit| json!({ "commit": commit }))
                .map_err(Refusal::from),
        )
    }
}
