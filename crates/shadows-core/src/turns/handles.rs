//! One job: register live turns for watcher/stop arbitration.
use super::{model::OperationId, steer::SteerRequest};
use crate::harness::OpenSession;
use crate::threads::ThreadId;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};
use tokio::sync::{Mutex, mpsc};

pub(crate) struct LiveTurn {
    pub(crate) thread_id: ThreadId,
    /// The session the prompt runs on: Stop cancels on it, and terminates
    /// its adapter — never one that replaced it.
    pub(crate) session: OpenSession,
    /// The tab that sent the turn (§13.9): transport state, kept here only,
    /// never stored.
    pub(crate) client_tab: Option<String>,
    pub(crate) turn_end_seen: Arc<AtomicBool>,
    pub(crate) cancel_requested: Arc<AtomicBool>,
    /// Where Send now hands the watcher a message the adapter took (§20.4).
    pub(crate) steer: mpsc::UnboundedSender<SteerRequest>,
    pub(crate) span: tracing::Span,
}

/// What Send now needs of the running turn of a thread (§20.4).
pub(crate) struct SteerTarget {
    pub(crate) cancel_requested: Arc<AtomicBool>,
    pub(crate) steer: mpsc::UnboundedSender<SteerRequest>,
}
#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) turns: HashMap<OperationId, LiveTurn>,
    closed: bool,
}
#[derive(Default)]
pub struct LiveHandles(pub(crate) Mutex<Registry>);
impl LiveHandles {
    pub(crate) async fn register(
        &self,
        op: OperationId,
        turn: LiveTurn,
    ) -> Result<(), Box<LiveTurn>> {
        let mut r = self.0.lock().await;
        if r.closed {
            return Err(Box::new(turn));
        }
        r.turns.insert(op, turn);
        Ok(())
    }
    pub(crate) async fn claim(&self, op: &OperationId) -> Option<LiveTurn> {
        self.0.lock().await.turns.remove(op)
    }
    pub(crate) async fn contains_internal(&self, op: &OperationId) -> bool {
        self.0.lock().await.turns.contains_key(op)
    }
    pub(crate) async fn restore(&self, op: OperationId, turn: LiveTurn) {
        self.0.lock().await.turns.insert(op, turn);
    }
    pub(crate) async fn close(&self) -> Vec<OperationId> {
        let mut r = self.0.lock().await;
        r.closed = true;
        r.turns.keys().cloned().collect()
    }
    /// The turn running on `thread`, if one is: what the Planner's
    /// `draft_start` anchors its derived command id to (spec §13.5).
    pub async fn running_for(&self, thread: &ThreadId) -> Option<OperationId> {
        self.running_turn(thread).await.map(|(op, _)| op)
    }
    /// As `running_for`, with the tab that sent the turn: whom `plan_show`
    /// signals (spec §13.9).
    pub async fn running_turn(&self, thread: &ThreadId) -> Option<(OperationId, Option<String>)> {
        let r = self.0.lock().await;
        r.turns
            .iter()
            .find(|(_, turn)| &turn.thread_id == thread)
            .map(|(op, turn)| (op.clone(), turn.client_tab.clone()))
    }
    /// The running turn of `thread`, as Send now reaches it (§20.4).
    pub(crate) async fn steer_target(&self, thread: &ThreadId) -> Option<SteerTarget> {
        let r = self.0.lock().await;
        r.turns
            .iter()
            .find(|(_, t)| &t.thread_id == thread)
            .map(|(_, t)| SteerTarget {
                cancel_requested: t.cancel_requested.clone(),
                steer: t.steer.clone(),
            })
    }
    pub async fn is_closed(&self) -> bool {
        self.0.lock().await.closed
    }
    #[cfg(feature = "test-support")]
    pub async fn contains(&self, op: &OperationId) -> bool {
        self.0.lock().await.turns.contains_key(op)
    }
    /// What shutdown does first (§8.5): closes the registry to new turns.
    #[cfg(feature = "test-support")]
    pub async fn close_for_test(&self) {
        self.close().await;
    }
}
