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
use shadows_core::{Grant, TaskQuestion, WorkflowId};

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
struct TaskReadyArgs {
    /// The approved plan version.
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
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
        description = "Read your attempt's brief before you work: the approved task, the \
                       contracts it uses, the parts and owners of its paths, and the map and \
                       standards versions, as Shadows saved them when this attempt began. It \
                       does not change when the project's map or standards change later. An \
                       attempt begun before briefs were saved answers context_not_recorded."
    )]
    async fn task_brief(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.core
                .runs()
                .brief_for(&grant)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Ask about your own task's contract while your attempt runs. selector \
                       picks the whole task (own), one symbol (symbol_id), one output \
                       (output_id) or one source your task uses (use). planning answers \
                       what this_frozen_version of the plan promises: the selected contract, \
                       its producers and the planned consumers of its outputs, a page at a \
                       time (after, limit 1 to 50, next). A planned consumer is a task that \
                       plans to use it, not code that calls it; tasks of other versions or \
                       plans are not listed, so an empty list is not proof that nothing uses \
                       it. producer_status says whether each producer is marked done and its \
                       latest commit now. code asks the project's language server as code_lsp \
                       does, inside or outside your task's reads; code_error says the server \
                       could not answer, which is never evidence that a symbol is absent. A \
                       too large answer is question_too_large: ask a narrower selector."
    )]
    async fn task_question(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(question): Parameters<TaskQuestion>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .question(&grant, question)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Report your task's work after you committed it yourself with git. \
                       Shadows accepts the commit only if it is on the checked-out branch, \
                       made after your task started, changes only paths inside your task's \
                       writes, changes at least one file and adds no binary file; a refusal \
                       says what to fix, then report again. One accepted report per attempt."
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
                       one: git init and a first commit, made in Shadows' name, of the files \
                       the folder already holds (none in an empty folder). It leaves out what \
                       git ignores and the secret and dependency files project_files lists as \
                       left out; those stay in the folder, untracked. Refused when the folder \
                       is already inside a repository. It writes no project file; starter \
                       files are a task's work."
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
        description = "List the tasks of an approved plan version you may start together now: \
                       each never ran, is not done, has its needs done, and its writes meet \
                       no running or unjudged task nor another task of the list, in task \
                       order with its suggested model. It reserves nothing: each task_start \
                       checks again, and a refused one is reported, not retried from this \
                       list; ask task_ready again for what is ready now. A version that cannot \
                       start work is refused with its reason."
    )]
    async fn task_ready(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskReadyArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .runs()
                .ready_for(&grant, &args.workflow_id)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Start an executor on one task of an approved plan version: one the \
                       person asked you to run, or a task of the group task_ready or task_done \
                       named. Shadows checks the task again as the Start button does (an approved \
                       version, its needs links done, a git folder, \
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
                       done true it answers next: the group task_ready would answer, as a \
                       list (empty when nothing can start, or when this version has a newer \
                       one); start each with task_start. Marking it done also closes the \
                       executor's session."
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
