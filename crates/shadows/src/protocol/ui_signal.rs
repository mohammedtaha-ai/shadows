//! One job: the live-only signal that moves a person's screen (spec §13.9).
//!
//! `plan_show` sends one after its card commits. It is transport state
//! (§2.10): never stored, never journaled, never replayed. The thread's
//! stream (`sse.rs`) turns it into a `plan-show` frame for every live
//! subscriber; only the tab it names opens the panel or the page.

use shadows_core::thread::ThreadId;
use shadows_core::workflow::{Place, WorkflowId};

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
