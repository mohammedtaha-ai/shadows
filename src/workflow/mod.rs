//! One job: a plan's content under the rules of spec §13.
//!
//! Pure: no storage, no I/O. A plan is changed only by [`apply`]ing a batch of
//! [`PlanOp`]s, and judged only by [`edit_problems`] and [`approval_problems`],
//! so every writer — the Planner's MCP tools, the HTTP routes — and every reader
//! that lists what blocks approval share one set of rules (§13.4).

use std::collections::BTreeMap;

pub mod check;
pub mod ops;

pub use check::{Problem, approval_problems, edit_problems};
pub use ops::{Applied, PlanOp, apply};

use crate::id::newtype_id;

newtype_id! {
    /// Spec §13.2. One version of a plan.
    WorkflowId
}

newtype_id! {
    /// Spec §13.3. Storage identity only: tools and people name a task by its
    /// `number` within the version, never by this id.
    TaskId
}

/// Spec §13.2. A `Draft` is edited; a `Frozen` version is approved and never
/// changes again. The web client shows `Frozen` as "Approved".
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
pub enum WorkflowState {
    Draft,
    Frozen,
}

/// Spec §13.3. How a link's `task` waits for its `after`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// The task does not start until `after` is complete.
    Needs,
    /// The task may finish now, but the acceptance items the link names only
    /// hold once `after` is complete.
    CompletesAfter,
}

impl LinkKind {
    /// The wire name, as serde writes it.
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkKind::Needs => "needs",
            LinkKind::CompletesAfter => "completes_after",
        }
    }
}

/// How a refusal names one link: `the needs link T2 → T1`, the arrow pointing
/// from the task that waits to the task it waits for.
fn link_name(task: u32, after: u32, kind: LinkKind) -> String {
    format!("the {} link T{task} → T{after}", kind.as_str())
}

/// Spec §13.3. One sentence someone can check, numbered within its task.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AcceptanceItem {
    pub number: u32,
    pub text: String,
}

/// Spec §13.3. A task as the plan holds it, shown as `T{number}`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct TaskContent {
    pub number: u32,
    pub title: String,
    pub goal: String,
    /// Paths the task reads (its declared scope).
    pub reads: Vec<String>,
    /// Paths the task may change (its declared scope).
    pub writes: Vec<String>,
    pub acceptance: Vec<AcceptanceItem>,
}

/// Spec §13.3. `task` waits for `after`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Link {
    pub task: u32,
    pub after: u32,
    pub kind: LinkKind,
    /// A few words saying what passes from `after` to `task`.
    pub label: String,
    /// The acceptance items of `task` that wait. Empty for `needs`.
    #[serde(default)]
    pub waiting_items: Vec<u32>,
}

/// One plan version's content. Tasks are keyed by their number; links keep
/// the order they were stored in, which is the order problems are reported in.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PlanContent {
    pub title: String,
    pub goal: String,
    pub tasks: BTreeMap<u32, TaskContent>,
    pub links: Vec<Link>,
}
