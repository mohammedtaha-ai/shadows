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
use shadows_core::{Grant, WorkflowId};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskReportArgs {
    /// The full sha of the commit you made with `git commit` for this task.
    commit: String,
    /// What you did, what you ran or tested, and what passed or failed.
    report: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskStartArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    /// The task's number: 4 for T4.
    task_number: u32,
    /// A model the executor's harness (claude-code) offers; leave it out for the
    /// task's suggested model.
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskDoneArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    /// The task's number: 4 for T4.
    task_number: u32,
    /// true to mark the task done, false to take the mark back.
    done: bool,
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

    #[tool(
        description = "Start an executor on one task of an approved plan version, only when \
                       the person asked you to run it. Shadows checks the task as the Start \
                       button does (an approved version, its needs links done, a git folder, \
                       no overlapping task running) and answers the attempt, or the refusal \
                       with its code. The executor works on its own; Shadows tells you in \
                       this conversation when its attempt ends."
    )]
    async fn task_start(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskStartArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .start_for(&grant, &args.workflow_id, args.task_number, args.model)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Mark a task done (done true) only after you judged its committed work \
                       against every acceptance item, or not done (done false) to take a mark \
                       back. A done task frees the tasks whose needs links wait for it. With \
                       done true it answers next: the lowest-numbered task of this version \
                       that has never run and can start now, or null; start it with \
                       task_start."
    )]
    async fn task_done(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskDoneArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .mark_done_for(&grant, &args.workflow_id, args.task_number, args.done)
                .await
                .map_err(Refusal::from),
        )
    }
}
