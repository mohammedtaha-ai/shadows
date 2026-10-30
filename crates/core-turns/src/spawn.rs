//! One job: register a Planner prompt before committing Running.
//!
//! The turn command committed the operation `Pending` with the user's entry. This
//! file sets the session to the turn's settings, registers the prompt, and
//! commits `Running`; `turn.rs` takes it from there.
use super::{
    handles::{LiveHandles, LiveTurn},
    model::{FailureStage, OperationId},
    prompt::{Prompt, PromptParts},
    turn::{PlannerTurn, TurnWatch},
};
use core_db::{Actor, StorageError};
use core_harness::{Harness, OpenSession};
use core_runtime::Runtime;
use core_threads::ThreadId;
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

/// Called once a turn's `Completed` is recorded: sends the thread's
/// next waiting message.
pub type OnCompleted = Arc<dyn Fn(ThreadId) + Send + Sync>;

/// A turn whose operation `TurnsStore::start_turn` has committed `Pending`.
pub struct PlannerTurnRequest {
    pub thread_id: ThreadId,
    pub root: Option<std::path::PathBuf>,
    /// The thread's harness as the turn command validated it, so nothing is
    /// read between the commit and the run that could strand it `Pending`.
    pub harness: String,
    pub operation_id: OperationId,
    pub prompt: String,
    pub images: Vec<shadows_agent::acp::ImageBlock>,
    pub settings: TurnSettings,
    /// The focus block, sent after the instructions
    /// block so the person's text stays first.
    pub focus: Option<String>,
    /// The selected plan block, after any focus block.
    pub continue_plan: Option<String>,
    /// The tab that sent the turn, kept in memory only.
    pub client_tab: Option<String>,
    /// The session's events, leased before validation
    /// (`Harness::lease_events`): the turn holds the session from its
    /// checks to its ending, and gives them back.
    pub events: mpsc::UnboundedReceiver<HarnessEvent>,
    /// Sends the thread's next waiting message once this turn completes.
    pub on_completed: Option<OnCompleted>,
}

impl PlannerTurn {
    pub async fn start(
        runtime: Arc<Runtime>,
        storage: Arc<core_db::Storage>,
        handles: Arc<LiveHandles>,
        sessions: Arc<Harness>,
        opened: OpenSession,
        request: PlannerTurnRequest,
        bus: broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
    ) -> Result<OperationId, StartError> {
        let PlannerTurnRequest {
            thread_id,
            root,
            harness,
            operation_id: op_id,
            prompt,
            images,
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
        // what changed since the session last heard, read while this
        // turn is still `Pending` and so not yet its thread's latest.
        let facts = match super::Turns::stage_line(&storage, &thread_id).await {
            Ok(stage) => {
                let invocation = if harness == shadows_agent::policy::AYLUL {
                    None
                } else {
                    Some(super::Turns::invocation_context(&storage, &thread_id, &op_id).await)
                };
                sessions.context_facts(&thread_id, invocation, stage).await
            }
            Err(error) => Err(error.into()),
        };
        let prepared = match facts {
            Ok(facts) => (sessions.prepare_turn(&thread_id, &opened, &settings).await).map(|()| {
                Prompt::compose(PromptParts {
                    text: prompt,
                    images,
                    changes: facts.changes,
                    stage: facts.stage,
                    focus,
                    continue_plan,
                })
            }),
            Err(error) => Err(error.to_string()),
        };
        let Prompt { blocks, withholds } = match prepared {
            Ok(prompt) => prompt,
            Err(reason) => {
                tracing::info!(parent: &span, %reason, "planner.prepare_refused");
                sessions.give_back_events(&thread_id, &opened, events).await;
                (super::store::TurnsStore {
                    storage: storage.clone(),
                })
                .mark_operation_failed(&op_id, FailureStage::Prepare, &reason)
                .await?;
                return Ok(op_id);
            }
        };
        let activity = Arc::new(std::sync::Mutex::new((Vec::new(), None)));
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
                    activity: activity.clone(),
                },
            )
            .await
        {
            sessions.give_back_events(&thread_id, &opened, events).await;
            (super::store::TurnsStore {
                storage: storage.clone(),
            })
            .request_cancellation(&op_id, Actor::system())
            .await?;
            (super::store::TurnsStore {
                storage: storage.clone(),
            })
            .mark_operation_cancelled(&op_id)
            .await?;
            return Err(StartError::RuntimeStopping);
        }
        // A Stop that came while the turn was `Pending` recorded its request
        // and found nothing registered to cancel. Read after registering, so
        // either that Stop saw the registration or this read sees its request.
        let asked = (super::store::TurnsStore {
            storage: storage.clone(),
        })
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
                    (super::store::TurnsStore {
                        storage: storage.clone(),
                    })
                    .mark_operation_cancelled(&op_id)
                    .await?;
                    Ok(op_id)
                }
                Err(e) => {
                    (super::store::TurnsStore {
                        storage: storage.clone(),
                    })
                    .mark_operation_failed(&op_id, FailureStage::Prepare, &e.to_string())
                    .await?;
                    Err(e.into())
                }
            };
        }
        if let Err(e) = (super::store::TurnsStore {
            storage: storage.clone(),
        })
        .mark_operation_started(&op_id, runtime.instance_id())
        .await
        {
            handles.claim(&op_id).await;
            sessions.give_back_events(&thread_id, &opened, events).await;
            (super::store::TurnsStore {
                storage: storage.clone(),
            })
            .mark_operation_failed(&op_id, FailureStage::Prepare, &e.to_string())
            .await?;
            return Err(e.into());
        }
        // Only a started turn counts as heard, so the harness learns here
        // whether this one carried its changes or a command left them waiting.
        sessions.withhold(&thread_id, withholds);
        Self::watch_turn(
            TurnWatch {
                activity,
                root,
                op_id: op_id.clone(),
                storage,
                handles,
                sessions,
                opened,
                thread_id,
                harness,
                blocks,
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
