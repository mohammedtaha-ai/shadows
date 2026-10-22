//! The tools each grant kind permits.
use super::{GrantKind, Grants};
/// Only the Planner may show a plan in a person's conversation.
const THREAD_TOOLS: [&str; 19] = [
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
    "draft_start",
    "plan_edit",
    "plan_show",
    "vision_propose",
    "workspace_get",
];

/// External agents can edit project plans and read reachable project code.
const PROJECT_TOOLS: [&str; 15] = [
    "agreement_list",
    "agreement_get",
    "agreement_start",
    "agreement_edit",
    "agreement_review",
    "workflow_list",
    "workflow_get",
    "task_get",
    "draft_prepare",
    "draft_start",
    "plan_edit",
    "where_is",
    "who_uses",
    "outline",
    "workspace_get",
];

impl Grants {
    pub fn tools_for(kind: GrantKind) -> &'static [&'static str] {
        match kind {
            GrantKind::Thread => &THREAD_TOOLS,
            GrantKind::Project => &PROJECT_TOOLS,
        }
    }
}
