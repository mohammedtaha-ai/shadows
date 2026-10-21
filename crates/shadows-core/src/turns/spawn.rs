//! One job: register a Planner prompt before committing Running.
//!
//! The operation already exists: the turn command (§12.7,
//! `Storage::start_turn`) committed it `Pending` with the user's entry. This
//! file sets the session to the turn's settings, registers the prompt, and
//! commits `Running`; `turn.rs` takes it from there.
use super::{
    handles::{LiveHandles, LiveTurn},
    model::{FailureStage, OperationId},
    prompt::{Prompt, PromptParts},
    turn::{PlannerTurn, TurnWatch, watch_turn},
};
use crate::{
    db::StorageError,
    events::Actor,
    harness::{OpenSession, Sessions},
    runtime::Runtime,
    threads::ThreadId,
};
use shadows_agent::{TurnSettings, events::HarnessEvent};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("the runtime is stopping and accepts no new turns")]
    RuntimeStopping,
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Called once a turn's `Completed` is recorded (§20.3): sends the thread's
/// next waiting message.
pub type OnCompleted = Arc<dyn Fn(ThreadId) + Send + Sync>;

/// A turn whose operation `Storage::start_turn` has committed `Pending`.
pub struct PlannerTurnRequest {
    pub thread_id: ThreadId,
    /// The thread's harness as the turn command validated it, so nothing is
    /// read between the commit and the run that could strand it `Pending`.
    pub harness: String,
    pub operation_id: OperationId,
    pub prompt: String,
    pub settings: TurnSettings,
    /// The focus block (§13.9, `focus_block`), sent after the instructions
    /// block so the person's text stays first.
    pub focus: Option<String>,
    /// The selected plan block, after any focus block.
    pub continue_plan: Option<String>,
    /// The tab that sent the turn, kept in memory only (§13.9).
    pub client_tab: Option<String>,
    /// The session's events, leased before validation
    /// (`Sessions::lease_events`): the turn holds the session from its
    /// checks to its ending, and gives them back.
    pub events: mpsc::UnboundedReceiver<HarnessEvent>,
    /// Sends the thread's next waiting message once this turn completes.
    pub on_completed: Option<OnCompleted>,
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
            focus,
            continue_plan,
            client_tab,
            events,
            on_completed,
        } = request;
        let span = tracing::info_span!(
            parent: None,
            "planner.turn",
            operation_id = %op_id,
            thread_id = %thread_id
        );
        // §13.8: what changed since the session last heard, read while this
        // turn is still `Pending` and so not yet its thread's latest.
        let facts = sessions.setups().context_facts(&thread_id, &op_id).await;
        let prepared = match facts {
            Ok(facts) => (sessions.prepare_turn(&thread_id, &opened, &settings).await).map(|()| {
                Prompt::compose(PromptParts {
                    text: prompt,
                    changes: facts.changes,
                    stage: facts.stage,
                    focus,
                    continue_plan,
                })
            }),
            Err(error) => Err(error.to_string()),
        };
        let Prompt {
            text: prompt,
            context,
            withheld,
        } = match prepared {
            Ok(prompt) => prompt,
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
        // A command skipped the changes: the next normal turn carries them.
        sessions.setups().withhold(&thread_id, withheld);
        let turn_end_seen = Arc::new(AtomicBool::new(false));
        let cancel_requested = Arc::new(AtomicBool::new(false));
        let (steer, steers) = mpsc::unbounded_channel();
        if let Err(_turn) = handles
            .register(
                op_id.clone(),
                LiveTurn {
                    thread_id: thread_id.clone(),
                    session: opened.clone(),
                    client_tab,
                    turn_end_seen: turn_end_seen.clone(),
                    cancel_requested: cancel_requested.clone(),
                    steer,
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
                on_completed,
                span,
            },
            events,
            steers,
            bus,
        );
        Ok(op_id)
    }
}
