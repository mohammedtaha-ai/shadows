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
    agent::{TurnSettings, acp::AcpError, events::HarnessEvent},
    events::Actor,
    operation::{FailureStage, OperationId},
    runtime::Runtime,
    storage::StorageError,
    thread::ThreadId,
};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("the runtime is stopping and accepts no new turns")]
    RuntimeStopping,
    #[error(transparent)]
    Storage(#[from] StorageError),
}
/// A turn whose operation `Storage::start_turn` has committed `Pending`.
#[derive(Debug, Clone)]
pub struct PlannerTurnRequest {
    pub thread_id: ThreadId,
    pub operation_id: OperationId,
    pub prompt: String,
    pub settings: TurnSettings,
}

/// §12.7: before the prompt, the session is set to the turn's model, effort
/// and mode, one call per value that differs from what the session holds.
/// `Err` names the setting the harness refused, in its words; nothing has
/// been sent to the model.
async fn prepare_settings(
    sessions: &Sessions,
    thread: &ThreadId,
    opened: &OpenSession,
    settings: &TurnSettings,
) -> Result<(), String> {
    let mut offered = sessions
        .offered(thread)
        .await
        .ok_or("the harness session closed before the turn")?;
    let refused = |what: &str, value: &str, e: AcpError| format!("{what} {value}: {e}");
    if offered.current.model != settings.model {
        let id = offered.ids.model.clone();
        offered = sessions
            .set_option(thread, opened, &id, &settings.model)
            .await
            .map_err(|e| refused("model", &settings.model, e))?;
    }
    if let Some(effort) = &settings.effort
        && offered.current.effort.as_ref() != Some(effort)
    {
        let id = offered
            .ids
            .effort
            .clone()
            .ok_or_else(|| format!("effort {effort}: the model offers no effort"))?;
        offered = sessions
            .set_option(thread, opened, &id, effort)
            .await
            .map_err(|e| refused("effort", effort, e))?;
    }
    if offered.current.mode != settings.mode {
        let id = offered.ids.mode.clone();
        sessions
            .set_option(thread, opened, &id, &settings.mode)
            .await
            .map_err(|e| refused("mode", &settings.mode, e))?;
    }
    Ok(())
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
            operation_id: op_id,
            prompt,
            settings,
        } = request;
        let span = tracing::info_span!(parent: None, "planner.turn", operation_id = %op_id, thread_id = %thread_id);
        if let Err(reason) = prepare_settings(&sessions, &thread_id, &opened, &settings).await {
            tracing::info!(parent: &span, %reason, "planner.prepare_refused");
            runtime
                .storage
                .mark_operation_failed(&op_id, FailureStage::Prepare, &reason)
                .await?;
            return Ok(op_id);
        }
        let turn_end_seen = Arc::new(AtomicBool::new(false));
        let cancel_requested = Arc::new(AtomicBool::new(false));
        if let Err(_turn) = handles
            .register(
                op_id.clone(),
                LiveTurn {
                    thread_id: thread_id.clone(),
                    turn_end_seen: turn_end_seen.clone(),
                    cancel_requested: cancel_requested.clone(),
                    span: span.clone(),
                },
            )
            .await
        {
            runtime
                .storage
                .request_cancellation(&op_id, Actor::system())
                .await?;
            runtime.storage.mark_operation_cancelled(&op_id).await?;
            return Err(StartError::RuntimeStopping);
        }
        if let Err(e) = runtime
            .storage
            .mark_operation_started(&op_id, &runtime.instance_id)
            .await
        {
            handles.claim(&op_id).await;
            runtime
                .storage
                .mark_operation_failed(&op_id, FailureStage::Prepare, &e.to_string())
                .await?;
            return Err(e.into());
        }
        let events = sessions.take_events(&thread_id).await;
        watch_turn(
            TurnWatch {
                op_id: op_id.clone(),
                runtime,
                handles,
                sessions,
                opened,
                thread_id,
                prompt,
                events,
                turn_end_seen,
                cancel_requested,
                span,
            },
            bus,
        );
        Ok(op_id)
    }
}
