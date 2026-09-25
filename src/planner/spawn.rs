//! One job: register a Planner prompt before committing Running.
//!
//! The operation already exists: the turn command (§12.7,
//! `Storage::start_turn`) committed it `Pending` with the user's entry. This
//! file sets the session to the turn's settings, registers the prompt, and
//! commits `Running`; `turn.rs` takes it from there.
use super::{
    LiveHandles, LiveTurn, OpenSession, Sessions,
    turn::{PlannerTurn, TurnWatch, watch_turn},
};
use crate::{
    agent::{TurnSettings, events::HarnessEvent},
    events::Actor,
    operation::{FailureStage, OperationId},
    runtime::Runtime,
    storage::StorageError,
    thread::ThreadId,
};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("the runtime is stopping and accepts no new turns")]
    RuntimeStopping,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
/// A turn whose operation `Storage::start_turn` has committed `Pending`.
#[derive(Debug)]
pub struct PlannerTurnRequest {
    pub thread_id: ThreadId,
    /// The thread's harness as the turn command validated it, so nothing is
    /// read between the commit and the run that could strand it `Pending`.
    pub harness: String,
    pub operation_id: OperationId,
    pub prompt: String,
    pub settings: TurnSettings,
    /// The session's events, leased before validation
    /// (`Sessions::lease_events`): the turn holds the session from its
    /// checks to its ending, and gives them back.
    pub events: mpsc::UnboundedReceiver<HarnessEvent>,
}

impl PlannerTurn {
    pub async fn start(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        sessions: Arc<Sessions>,
        opened: OpenSession,
        request: PlannerTurnRequest,
        bus: broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
    ) -> Result<OperationId, StartError> {
        let PlannerTurnRequest {
            thread_id,
            harness,
            operation_id: op_id,
            prompt,
            settings,
            events,
        } = request;
        let span = tracing::info_span!(parent: None, "planner.turn", operation_id = %op_id, thread_id = %thread_id);
        // §13.8: what changed since the session last heard, read while this
        // turn is still `Pending` and so not yet its thread's latest.
        let context = sessions.setups().context_before_turn(&thread_id).await;
        let prepared = match context {
            Ok(context) => (sessions.prepare_turn(&thread_id, &opened, &settings).await)
                .map(|()| context.into_iter().collect::<Vec<String>>()),
            Err(error) => Err(error.to_string()),
        };
        let context = match prepared {
            Ok(context) => context,
            Err(reason) => {
                tracing::info!(parent: &span, %reason, "planner.prepare_refused");
                sessions.give_back_events(&thread_id, &opened, events).await;
                runtime
                    .storage
                    .mark_operation_failed(&op_id, FailureStage::Prepare, &reason)
                    .await?;
                return Ok(op_id);
            }
        };
        let turn_end_seen = Arc::new(AtomicBool::new(false));
        let cancel_requested = Arc::new(AtomicBool::new(false));
        if let Err(_turn) = handles
            .register(
                op_id.clone(),
                LiveTurn {
                    thread_id: thread_id.clone(),
                    session: opened.clone(),
                    turn_end_seen: turn_end_seen.clone(),
                    cancel_requested: cancel_requested.clone(),
                    span: span.clone(),
                },
            )
            .await
        {
            sessions.give_back_events(&thread_id, &opened, events).await;
            runtime
                .storage
                .request_cancellation(&op_id, Actor::system())
                .await?;
            runtime.storage.mark_operation_cancelled(&op_id).await?;
            return Err(StartError::RuntimeStopping);
        }
        // A Stop that came while the turn was `Pending` recorded its request
        // and found nothing registered to cancel. Read after registering, so
        // either that Stop saw the registration or this read sees its request.
        let asked = runtime
            .storage
            .get_operation(&op_id)
            .await
            .map(|op| op.cancel_requested_at.is_some());
        if !matches!(asked, Ok(false)) {
            let mine = handles.claim(&op_id).await.is_some();
            sessions.give_back_events(&thread_id, &opened, events).await;
            if !mine {
                // A Stop claimed it and records its ending.
                return Ok(op_id);
            }
            return match asked {
                Ok(_) => {
                    tracing::info!(parent: &span, "planner.stopped_before_prompt");
                    runtime.storage.mark_operation_cancelled(&op_id).await?;
                    Ok(op_id)
                }
                Err(e) => {
                    runtime
                        .storage
                        .mark_operation_failed(&op_id, FailureStage::Prepare, &e.to_string())
                        .await?;
                    Err(e.into())
                }
            };
        }
        if let Err(e) = runtime
            .storage
            .mark_operation_started(&op_id, &runtime.instance_id)
            .await
        {
            handles.claim(&op_id).await;
            sessions.give_back_events(&thread_id, &opened, events).await;
            runtime
                .storage
                .mark_operation_failed(&op_id, FailureStage::Prepare, &e.to_string())
                .await?;
            return Err(e.into());
        }
        watch_turn(
            TurnWatch {
                op_id: op_id.clone(),
                runtime,
                handles,
                sessions,
                opened,
                thread_id,
                harness,
                prompt,
                context,
                turn_end_seen,
                cancel_requested,
                span,
            },
            events,
            bus,
        );
        Ok(op_id)
    }
}
