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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskDiffArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    /// The task's number: 4 for T4.
    task_number: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskFileArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    /// The task's number: 4 for T4.
    task_number: u32,
    /// A path inside the task's writes, as task_diff names it.
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskAmendArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    /// The task's number: 4 for T4.
    task_number: u32,
    /// What is missing or must be extended, as the executor should act on it.
    addition: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskLessonsArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    /// The task's number: 4 for T4.
    task_number: u32,
    /// What kept the work from being done, in a few sentences: the cause, not the
    /// additions again.
    stopped_by: String,
    /// What the task should have said so the executor would not have needed an
    /// addition: its acceptance, contract, setup or reads.
    plan_should_have_said: String,
}

#[tool_router(router = run_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Read what a task's latest committed attempt changed: each file with its \
                       added and removed line counts and its patch. Read it before you judge \
                       an attempt; \
                       never ask the executor for file contents. After an addition it shows only \
                       that attempt's change. Patches past 2,000 lines in all are not shown: read \
                       those files with task_file."
    )]
    async fn task_diff(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskDiffArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .diff_for(&grant, &args.workflow_id, args.task_number)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Read one file of a task as its latest committed attempt left it, whole: \
                       any path inside the task's writes, changed by that attempt or not. text \
                       is null for a deleted or binary file; cut is true when Shadows shortened \
                       a long file."
    )]
    async fn task_file(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskFileArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .file_for(&grant, &args.workflow_id, args.task_number, &args.path)
                .await
                .map_err(Refusal::from),
        )
    }

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
        description = "Start an executor on one task of an approved plan version: one the \
                       person asked you to run, or the task task_done named as next. Shadows \
                       checks the task as the Start \
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
                       task_start. Marking it done also closes the executor's session."
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

    #[tool(
        description = "Send the executor of a task you judged what is missing or must be \
                       extended. The task's committed attempt must be waiting for your \
                       judgment (not marked done). The executor carries on in its own session \
                       as a new attempt, and the addition is recorded on the task; the plan \
                       version does not change and needs no approval. Shadows tells you again \
                       when it ends."
    )]
    async fn task_amend(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskAmendArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .amend(&grant, &args.workflow_id, args.task_number, args.addition)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "After a task was stopped on its three additions, record what stopped \
                       the work and what the plan should have said. Shadows keeps it with the \
                       task's additions and the commands the executor ran, to improve how \
                       plans are written."
    )]
    async fn task_lessons(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskLessonsArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .write_lesson(
                    &grant,
                    &args.workflow_id,
                    args.task_number,
                    args.stopped_by,
                    args.plan_should_have_said,
                )
                .await
                .map_err(Refusal::from),
        )
    }
}
