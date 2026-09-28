//! One job: the plan in the conversation (spec §13.9) — the task a person
//! points at, and where a shown plan goes.

use super::model::{TaskId, WorkflowId};
use crate::thread::ThreadEntryId;

/// The task a person points at when they send a turn: the task's id, in the
/// plan version and at the revision they were looking at. It is checked to
/// belong to that version and kept with their message.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Focus {
    pub workflow_id: WorkflowId,
    pub task_id: TaskId,
    pub revision: i64,
}

/// Where `plan_show` puts a plan: a card in the conversation, a panel beside
/// it, or its own page. Every shown plan is a card; `side` and `page` also act
/// in the tab that sent the turn, live only.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    Inline,
    Side,
    Page,
}

/// What a `plan_show` recorded, fixed when it committed: the `PlanShown`
/// event's payload. It never names a tab (§2.10: a tab is transport state).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlanShown {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task_number: Option<u32>,
    pub place: Place,
    /// The `PlanView` entry the card is.
    pub entry_id: ThreadEntryId,
    /// Whether this answers a call that had already happened, which writes
    /// and signals nothing. Never recorded.
    #[serde(skip)]
    pub replayed: bool,
}
