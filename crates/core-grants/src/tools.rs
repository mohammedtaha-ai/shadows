//! The tools each grant kind permits.
use super::{GrantKind, Grants};
/// Only the Planner may show a plan in a person's conversation.
const THREAD_TOOLS: [&str; 48] = [
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
    "decision_propose",
    "decision_list",
    "design_element_put",
    "design_element_delete",
    "design_elements",
    "project_state",
    "code_where",
    "workspace_get",
    "project_files",
    "project_refresh",
    "code_lsp",
    "project_git_init",
    "task_ready",
    "task_runs",
    "task_start",
    "task_start_status",
    "task_executor_set",
    "executor_catalog",
    "task_stop",
    "task_done",
    "task_amend",
    "task_lessons",
    "task_diff",
    "task_file",
    "task_checks",
    "project_recheck",
    "message_agent",
    "person_ask",
    "project_change_note",
];

/// External agents work a project as the Planner does, less the person's
/// approvals and the tools bound to a person's conversation.
const PROJECT_TOOLS: [&str; 42] = [
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
    "vision_propose",
    "map_propose",
    "roadmap_propose",
    "decision_propose",
    "decision_list",
    "design_element_put",
    "design_element_delete",
    "design_elements",
    "project_state",
    "code_where",
    "project_files",
    "project_refresh",
    "task_ready",
    "task_runs",
    "task_start",
    "task_start_status",
    "task_executor_set",
    "executor_catalog",
    "task_stop",
    "task_done",
    "task_amend",
    "task_lessons",
    "task_diff",
    "task_file",
    "task_checks",
    "project_recheck",
    "project_git_init",
    "message_agent",
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
