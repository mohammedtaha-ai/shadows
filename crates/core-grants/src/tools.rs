//! The tools each grant kind permits.
use super::{GrantKind, Grants};
/// Only the Planner may show a plan in a person's conversation.
const THREAD_TOOLS: [&str; 34] = [
    "vision_approve",
    "map_approve",
    "roadmap_approve",
    "plan_approve",
    "map_propose",
    "roadmap_propose",
    "agreement_list",
    "agreement_get",
    "agreement_start",
    "agreement_edit",
    "agreement_review",
    "workflow_list",
    "workflow_get",
    "task_get",
    "task_context",
    "draft_start",
    "plan_edit",
    "plan_show",
    "vision_propose",
    "workspace_get",
    "project_files",
    "project_refresh",
    "code_lsp",
    "project_git_init",
    "task_ready",
    "task_start",
    "task_stop",
    "task_done",
    "task_amend",
    "task_lessons",
    "task_diff",
    "task_file",
    "person_ask",
    "project_change_note",
];

/// External agents can edit project plans and read reachable project code.
const PROJECT_TOOLS: [&str; 14] = [
    "agreement_list",
    "agreement_get",
    "agreement_start",
    "agreement_edit",
    "agreement_review",
    "workflow_list",
    "workflow_get",
    "task_get",
    "task_context",
    "draft_prepare",
    "draft_start",
    "plan_edit",
    "code_lsp",
    "workspace_get",
];

/// An executor reads the brief its attempt was given, asks about its task's
/// contract and the code it needs, and reports the commit it made or the
/// contract conflict that stops it. It has no fresh `task_context`, so it
/// never reads a layout its attempt was not given.
const EXECUTOR_TOOLS: [&str; 5] = [
    "task_brief",
    "task_question",
    "code_lsp",
    "task_report",
    "task_conflict",
];

impl Grants {
    pub fn tools_for(kind: GrantKind) -> &'static [&'static str] {
        match kind {
            GrantKind::Thread => &THREAD_TOOLS,
            GrantKind::Project => &PROJECT_TOOLS,
            GrantKind::Executor => &EXECUTOR_TOOLS,
        }
    }
}
