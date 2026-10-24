use super::{Place, WorkflowId};
use core_threads::ThreadId;

/// The live-only signal that moves a person's screen.
///
/// `plan_show` sends one after its card commits. It is transport state
/// that is never stored, never journaled, never replayed. `Events` delivers
/// it to every live subscriber of its thread as `Delivery::PlanShow`, which
/// the stream (`sse.rs`) frames as `plan-show`; only the tab it names opens
/// the panel or the page.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UiSignal {
    pub thread_id: ThreadId,
    /// The tab that sent the turn, as it sent it; `None` when it named none.
    pub target_tab: Option<String>,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task_number: Option<u32>,
    pub place: Place,
}
