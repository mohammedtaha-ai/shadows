//! One job: starting and stopping a Planner turn —
//! the `Turns` service.
//!
//! `send` follows one order: a replay is
//! answered first; the harness is checked, then the thread's busyness, before
//! the session is touched; the session is opened, then its events leased,
//! before any setting changes; the one transaction checks busyness again,
//! atomically; only then does the turn run. `stop` is Stop as the local user.
//!
//! `record` is the checks and the one transaction, `queue` the waiting
//! messages. `spawn` registers a committed turn and commits `Running`, `turn` records
//! its ending and arbitrates it with Stop, `handles` is the registry of live
//! turns, `entries` turns harness events into entries, `subagents` gathers a
//! subagent's calls into its card, `shutdown` stops every
//! turn when the runtime stops, `model` holds the operation types and
//! `store` the turn queries. All are private: a caller starts or stops a turn
//! through `Turns` only.

mod entries;
mod error;
mod handles;
mod model;
mod plan_show;
mod prompt;
mod queue;
mod record;
mod remove;
mod shutdown;
mod spawn;
mod stage;
mod steer;
mod store;
mod subagents;
#[cfg(feature = "test-support")]
mod testing;
mod turn;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::TurnSettings;

pub use handles::LiveHandles;
pub use model::{
    InvocationView, Operation, OperationId, Queued, QueuedMessage, QueuedMessageId, SentNow,
};
pub use spawn::{OnCompleted, PlannerTurnRequest, StartError};
pub use store::{NewQueued, QueueAnswer};

use prompt::Prompt;
use record::Turn;
use store::Dequeue;

pub use turn::PlannerTurn;
pub use turn::StopOutcome;

/// A running turn's live harness events, for every live subscriber.
pub type Bus =
    tokio::sync::broadcast::Sender<(ThreadId, OperationId, shadows_agent::events::HarnessEvent)>;
use core_code::Code;
use core_db::{Actor, CommandContext, Storage, StorageError};
use core_harness::{Harness, LeaseError};
use core_instructions::Instructions;
use core_plans::{Focus, PlanId};
use core_projects::Projects;
use core_runtime::Runtime;
use core_runtime::StopKind;
use core_standards::Standards;
use core_threads::ThreadId;
pub use error::TurnsError;

/// What the turn machinery needs for tests that drive it below `Turns`:
/// `shadows_core::testing` re-exports these, and nothing outside the crate
/// reaches them otherwise.
#[cfg(feature = "test-support")]
pub mod for_tests {
    pub use super::model::FailureStage;
    pub use super::spawn::PlannerTurnRequest;
    pub use super::store::{Dequeue, InvocationVersions, NewTurn, StartedTurn};
    pub use super::turn::{PlannerTurn, StopOutcome};
}

/// Turns: what storage holds, the runtime that owns their operations, the
/// sessions they prompt on, the registry of live turns, the bus their live
/// events go out on, and the code index a turn's project is touched in.
#[derive(Clone)]
pub struct Turns {
    storage: Arc<Storage>,
    store: store::TurnsStore,
    operations: core_operations::Operations,
    runtime: Arc<Runtime>,
    sessions: Arc<Harness>,
    handles: Arc<LiveHandles>,
    bus: Bus,
    code: Code,
    instructions: Instructions,
    standards: Standards,
    projects: Projects,
    plans: core_plans::Plans,
    ui: tokio::sync::broadcast::Sender<core_plans::UiSignal>,
}

/// A person's turn, as the route received it.
#[derive(Clone)]
pub struct SendTurn {
    /// The idempotency key.
    pub command_id: String,
    pub prompt: String,
    pub attachments: Vec<core_threads::AttachmentId>,
    pub model: String,
    pub mode: String,
    pub effort: Option<String>,
    /// The task the person points at; part of the command.
    pub focus: Option<Focus>,
    /// The plan the person chose to continue; part of the command.
    pub plan: Option<PlanId>,
    /// The sending tab, kept in memory for the turn only; not part of the
    /// command, never stored.
    pub client_tab: Option<String>,
}

/// `turn.start`'s command for `turn` on `thread`: `send` asks its
/// replay first, and `queue` asks the replay of the start it derived.
fn start_command(command_id: String, thread: &ThreadId, turn: &SendTurn) -> CommandContext {
    let mut params = serde_json::json!({
        "thread_id": thread,
        "prompt": turn.prompt,
        "model": turn.model,
        "mode": turn.mode,
        "effort": turn.effort,
    });
    // Omit absent focus from the fingerprint so plain-turn retries keep their identity.
    if let Some(focus) = &turn.focus {
        params["focus"] = serde_json::json!(focus);
    }
    if let Some(plan) = &turn.plan {
        params["plan"] = serde_json::json!(plan);
    }
    if !turn.attachments.is_empty() {
        params["attachments"] = serde_json::json!(turn.attachments);
    }
    CommandContext::user(command_id, "turn.start", params)
}

impl Turns {
    // One argument for each service it calls: the constructor is its dependency list.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<Storage>,
        operations: core_operations::Operations,
        runtime: Arc<Runtime>,
        sessions: Arc<Harness>,
        handles: Arc<LiveHandles>,
        bus: Bus,
        code: Code,
        instructions: Instructions,
        standards: Standards,
        projects: Projects,
        plans: core_plans::Plans,
        ui: tokio::sync::broadcast::Sender<core_plans::UiSignal>,
    ) -> Self {
        Self {
            store: store::TurnsStore::with_storage(storage.clone()),
            storage,
            operations,
            runtime,
            sessions,
            handles,
            bus,
            code,
            instructions,
            standards,
            projects,
            plans,
            ui,
        }
    }

    /// Starts a Planner turn: "turn.start", params
    /// { "thread_id", "prompt", "model", "mode", "effort" }, and "focus" when
    /// one is given. Order: a replay first, answered before anything
    /// else; then every check, before any write; then the one transaction;
    /// then the run.
    pub async fn send(
        &self,
        thread_id: ThreadId,
        turn: SendTurn,
    ) -> Result<OperationId, TurnsError> {
        self.send_with(thread_id, turn, None).await
    }

    /// `send`, and when `dequeue` names a waiting message, the turn that takes
    /// it from the queue in its own transaction.
    pub async fn send_with(
        &self,
        thread_id: ThreadId,
        turn: SendTurn,
        dequeue: Option<(QueuedMessageId, Option<CommandContext>)>,
    ) -> Result<OperationId, TurnsError> {
        let command = start_command(turn.command_id.clone(), &thread_id, &turn);
        let SendTurn {
            command_id: _,
            prompt,
            attachments,
            model,
            mode,
            effort,
            focus,
            plan,
            client_tab,
        } = turn;
        if let Some(replay) = self.store.replayed_turn(&command, &thread_id).await? {
            return Ok(replay.operation_id);
        }
        if self.handles.is_closed().await {
            return Err(TurnsError::RuntimeStopping);
        }
        let images = self.images(&thread_id, &attachments).await?;
        let settings = TurnSettings {
            model,
            mode,
            effort,
        };
        let context = core_threads::Threads::turn_context_in(
            &mut *self
                .storage
                .reader()
                .begin()
                .await
                .map_err(core_db::StorageError::from)?,
            &thread_id,
        )
        .await?;
        let continue_plan = match &plan {
            Some(plan_id) => Some(
                self.plans
                    .list(&context.project_id, true)
                    .await?
                    .into_iter()
                    .find(|plan| &plan.plan_id == plan_id)
                    .map(|plan| Prompt::continue_plan_block(&plan))
                    .ok_or_else(|| {
                        crate::TurnsError::InvalidCommand(
                            "the plan is not in this conversation's project".into(),
                        )
                    })?,
            ),
            None => None,
        };
        self.sessions.check_idle(&thread_id, &context).await?;
        // The checks passed: a turn starts in this project, which is used.
        self.code.touch(&context.project_id).await;
        let opened = self
            .sessions
            .open_for_images(&thread_id, !attachments.is_empty())
            .await?;
        self.sessions
            .require_images(&opened, !attachments.is_empty())?;
        let (opened, offered, previous) = self
            .sessions
            .offer_for_model(&thread_id, &opened, &settings.model)
            .await?;
        self.sessions
            .require_images(&opened, !attachments.is_empty())?;
        // The turn holds the session from here: a second start, or a `/context`
        // read, cannot change or prompt it until this turn gives it back.
        let events =
            (self.sessions.lease_events(&thread_id, &opened).await).map_err(|e| match e {
                LeaseError::Busy => TurnsError::ThreadBusy,
                LeaseError::Closed => TurnsError::HarnessStartFailed(e.to_string()),
            })?;
        // A concurrent pick may finish between model settlement and this lease.
        if self.sessions.offered(&thread_id).await != Some(offered.clone()) {
            self.sessions
                .give_back_events(&thread_id, &opened, events)
                .await;
            return Err(TurnsError::ThreadBusy);
        }
        let turn = Turn {
            command: &command,
            attachments: &attachments,
            prompt: &prompt,
            settings: &settings,
            focus: focus.as_ref(),
            dequeue: dequeue.as_ref().map(|(id, also)| Dequeue {
                id,
                also: also.as_ref(),
            }),
        };
        let started = match self
            .record(&thread_id, &opened, &context, turn, &offered)
            .await
        {
            Ok(started) if !started.replayed => started,
            other => {
                (self.sessions)
                    .give_back_events(&thread_id, &opened, events)
                    .await;
                if let Some(previous) = previous {
                    self.sessions
                        .set_model_back(&thread_id, &opened, &offered, &previous)
                        .await;
                }
                return other.map(|replay| replay.operation_id);
            }
        };
        Ok(PlannerTurn::start(
            self.runtime.clone(),
            self.storage.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            opened,
            PlannerTurnRequest {
                thread_id,
                harness: context.harness,
                operation_id: started.operation_id,
                images,
                prompt,
                settings,
                focus: focus
                    .zip(started.focus_task)
                    .map(|(focus, (number, title))| Prompt::focus_block(&focus, number, &title)),
                continue_plan,
                client_tab,
                events,
                on_completed: Some(self.on_completed()),
            },
            self.bus.clone(),
        )
        .await?)
    }

    /// Stops a turn as the local user, then answers the
    /// operation as it now stands. A tree that could not be terminated is
    /// `TerminationFailed`, and the operation is not `Cancelled`.
    pub async fn stop(&self, op: &OperationId) -> Result<Operation, TurnsError> {
        let outcome = PlannerTurn::stop(
            self.storage.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            op,
            Actor::user("local"),
        )
        .await?;
        if outcome == StopOutcome::TerminationFailed {
            return Err(TurnsError::TerminationFailed);
        }
        Ok(self.store.get_operation(op).await?)
    }

    /// Stops every running turn and closes every adapter before recording shutdown.
    /// Answers the stop kind recorded.
    pub async fn shut_down(
        &self,
        bound: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> Result<StopKind, StorageError> {
        PlannerTurn::shut_down(
            self.runtime.clone(),
            self.storage.clone(),
            self.operations.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            bound,
            second_signal,
        )
        .await
    }
}

impl Turns {
    pub async fn operations(&self, thread: &ThreadId) -> Result<Vec<Operation>, TurnsError> {
        Ok(self.store.list_operations_for_thread(thread).await?)
    }
}

impl Turns {
    pub async fn stop_running(&self, thread: &ThreadId) -> Result<StopOutcome, StorageError> {
        // A turn commits Pending before it registers a live handle. Removal
        // must also stop that gap, and wait for the durable terminal write
        // when a live watcher is still finishing after Stop answers.
        let mut conn = self.storage.reader().acquire().await?;
        let Some(op) =
            core_operations::Operations::pending_or_running_in(&mut conn, thread).await?
        else {
            return Ok(StopOutcome::NotLive);
        };
        drop(conn);
        let mut committed = self.storage.watch_committed();
        let outcome = PlannerTurn::stop(
            self.storage.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            &op,
            Actor::user("local"),
        )
        .await?;
        if outcome == StopOutcome::TerminationFailed {
            return Ok(outcome);
        }
        while self.store.get_operation(&op).await?.finished_at.is_none() {
            committed.changed().await.map_err(|_| {
                StorageError::Unavailable("storage closed while stopping a turn".into())
            })?;
        }
        Ok(outcome)
    }
    pub fn subscribe_live(
        &self,
    ) -> tokio::sync::broadcast::Receiver<(
        ThreadId,
        OperationId,
        shadows_agent::events::HarnessEvent,
    )> {
        self.bus.subscribe()
    }
}

impl Turns {
    pub(crate) async fn images(
        &self,
        thread: &ThreadId,
        ids: &[core_threads::AttachmentId],
    ) -> Result<Vec<shadows_agent::acp::ImageBlock>, TurnsError> {
        Self::images_in_storage(&self.storage, thread, ids).await
    }

    pub(crate) async fn images_in_storage(
        storage: &Arc<Storage>,
        thread: &ThreadId,
        ids: &[core_threads::AttachmentId],
    ) -> Result<Vec<shadows_agent::acp::ImageBlock>, TurnsError> {
        store::TurnsStore::with_storage(storage.clone())
            .images(thread, ids)
            .await
    }
}

use core_grants::Grant;

#[cfg(feature = "test-support")]
pub use entries::{Collector as TestCollector, Durable as TestDurable};
#[cfg(feature = "test-support")]
pub use prompt::{Prompt as TestPrompt, PromptParts as TestPromptParts};

use core_harness::AylulUsageSnapshot;
