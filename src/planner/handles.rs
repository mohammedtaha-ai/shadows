//! One job: register live turns for watcher/stop arbitration.
use crate::operation::OperationId;
use crate::thread::ThreadId;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};
use tokio::sync::Mutex;

pub(crate) struct LiveTurn {
    pub(crate) thread_id: ThreadId,
    pub(crate) turn_end_seen: Arc<AtomicBool>,
    pub(crate) cancel_requested: Arc<AtomicBool>,
    pub(crate) span: tracing::Span,
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
    pub async fn is_closed(&self) -> bool {
        self.0.lock().await.closed
    }
    #[cfg(feature = "test-support")]
    pub async fn contains(&self, op: &OperationId) -> bool {
        self.0.lock().await.turns.contains_key(op)
    }
}
