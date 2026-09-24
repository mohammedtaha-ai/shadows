//! One job: register a Planner prompt before committing Running.
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::broadcast;
use crate::{agent::events::HarnessEvent, events::Actor, operation::{OperationId, FailureStage}, runtime::Runtime, storage::StorageError, thread::ThreadId};
use super::{LiveHandles, LiveTurn, OpenSession, Sessions, turn::{PlannerTurn, TurnWatch, watch_turn}};

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("the runtime is stopping and accepts no new turns")]
    RuntimeStopping,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
#[derive(Debug, Clone)]
pub struct PlannerTurnRequest { pub thread_id: ThreadId, pub prompt: String }
impl PlannerTurn {
    pub async fn start(runtime: Arc<Runtime>, handles: Arc<LiveHandles>, sessions: Arc<Sessions>, opened: OpenSession, request: PlannerTurnRequest, bus: broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>) -> Result<OperationId, StartError> {
        let PlannerTurnRequest { thread_id, prompt } = request;
        if handles.is_closed().await { return Err(StartError::RuntimeStopping); }
        runtime.storage.turn_context(&thread_id).await?;
        let op_id = match runtime.storage.create_pending_operation(&thread_id, &runtime.instance_id).await {
            Ok(id) => id,
            Err(_) if handles.is_closed().await => return Err(StartError::RuntimeStopping),
            Err(e) => return Err(e.into()),
        };
        let span = tracing::info_span!(parent: None, "planner.turn", operation_id = %op_id, thread_id = %thread_id);
        let turn_end_seen = Arc::new(AtomicBool::new(false));
        let cancel_requested = Arc::new(AtomicBool::new(false));
        if let Err(_turn) = handles.register(op_id.clone(), LiveTurn { thread_id: thread_id.clone(), turn_end_seen: turn_end_seen.clone(), cancel_requested: cancel_requested.clone(), span: span.clone() }).await {
            runtime.storage.request_cancellation(&op_id, Actor::system()).await?;
            runtime.storage.mark_operation_cancelled(&op_id).await?;
            return Err(StartError::RuntimeStopping);
        }
        if let Err(e) = runtime.storage.mark_operation_started(&op_id, &runtime.instance_id).await {
            handles.claim(&op_id).await;
            runtime.storage.mark_operation_failed(&op_id, FailureStage::Prepare, &e.to_string()).await?;
            return Err(e.into());
        }
        let events = sessions.take_events(&thread_id).await;
        watch_turn(TurnWatch { op_id: op_id.clone(), runtime, handles, sessions, opened, thread_id, prompt, events, turn_end_seen, cancel_requested, span }, bus);
        Ok(op_id)
    }
}
