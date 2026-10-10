//! One job: starting and stopping a Planner turn (spec §12.3, §12.7, §14.4) —
//! the `Turns` service.
//!
//! `send` is §12.7's order, and the order is the rule (§14.9): a replay is
//! answered first; the harness is checked, then the thread's busyness, before
//! the session is touched; the session is opened, then its events leased,
//! before any setting changes; the one transaction checks busyness again,
//! atomically; only then does the turn run. `stop` is Stop as the local user.
//!
//! `record` is the checks and the one transaction, `queue` the waiting
//! messages. `spawn` registers a committed turn and commits `Running`, `turn` records
//! its ending and arbitrates it with Stop, `handles` is the registry of live
//! turns, `entries` turns harness events into entries, `subagents` gathers a
//! subagent's calls into its card (§22), `shutdown` stops every
//! turn when the runtime stops (§8.5), `model` holds the operation types and
//! `store` the queries. All are private: a caller starts or stops a turn
//! through `Turns` only.

mod entries;
mod handles;
mod model;
mod queue;
mod record;
mod shutdown;
mod spawn;
mod steer;
mod store;
mod subagents;
mod turn;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::TurnSettings;
use shadows_agent::policy;

pub use handles::LiveHandles;
pub use model::{
    InvocationView, Operation, OperationId, Queued, QueuedMessage, QueuedMessageId, SentNow,
};
pub use spawn::StartError;
// What another write calls inside its own transaction (spec §14.6):
// forking checks for an open turn, recovery records its transitions.
pub use store::{NewQueued, QueueAnswer};
pub(crate) use store::{existed, has_open_operation, read_before, record};

use record::Turn;
use spawn::{PlannerTurnRequest, continue_plan_block, focus_block};
use store::Dequeue;

use turn::PlannerTurn;
pub(crate) use turn::StopOutcome;

use crate::app::{Bus, user_command};
use crate::code::Code;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError};
use crate::error::CoreError;
use crate::events::Actor;
use crate::harness::{LeaseError, Sessions};
use crate::plans::{Focus, PlanId};
use crate::runtime::Runtime;
use crate::runtime::StopKind;
use crate::threads::ThreadId;

/// What the turn machinery needs for tests that drive it below `Turns`:
/// `shadows_core::testing` re-exports these, and nothing outside the crate
/// reaches them otherwise.
#[cfg(feature = "test-support")]
pub(crate) mod for_tests {
    pub use super::model::FailureStage;
    pub use super::shutdown::shut_down;
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
    runtime: Arc<Runtime>,
    sessions: Arc<Sessions>,
    handles: Arc<LiveHandles>,
    bus: Bus,
    code: Code,
}

/// The narrow turn control that thread removal needs. It owns no session
/// slot; Stop may take that slot while the turn records its ending.
#[derive(Clone)]
pub(crate) struct ThreadStopper {
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    sessions: Arc<Sessions>,
}

impl ThreadStopper {
    pub(crate) fn new(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        sessions: Arc<Sessions>,
    ) -> Self {
        Self {
            runtime,
            handles,
            sessions,
        }
    }

    pub(crate) async fn stop_running(
        &self,
        thread: &ThreadId,
    ) -> Result<StopOutcome, StorageError> {
        // A turn commits Pending before it registers a live handle. Removal
        // must also stop that gap, and wait for the durable terminal write
        // when a live watcher is still finishing after Stop answers.
        let Some(op) = self
            .runtime
            .storage
            .list_operations_for_thread(thread)
            .await?
            .into_iter()
            .find(|op| matches!(op.status_kind.as_str(), "Pending" | "Running"))
        else {
            return Ok(StopOutcome::NotLive);
        };
        let mut committed = self.runtime.storage.watch_committed();
        let outcome = PlannerTurn::stop(
            self.runtime.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            &op.id,
            Actor::user("local"),
        )
        .await?;
        if outcome == StopOutcome::TerminationFailed {
            return Ok(outcome);
        }
        while self
            .runtime
            .storage
            .get_operation(&op.id)
            .await?
            .finished_at
            .is_none()
        {
            committed.changed().await.map_err(|_| {
                StorageError::Unavailable("storage closed while stopping a turn".into())
            })?;
        }
        Ok(outcome)
    }
}

/// A person's turn, as the route received it (§12.7, §13.9).
#[derive(Clone)]
pub struct SendTurn {
    /// The idempotency key (spec §3.2).
    pub command_id: String,
    pub prompt: String,
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

/// `turn.start`'s command for `turn` on `thread` (§12.7): `send` asks its
/// replay first, and `queue` asks the replay of the start it derived.
fn start_command(command_id: String, thread: &ThreadId, turn: &SendTurn) -> CommandContext {
    let mut params = serde_json::json!({
        "thread_id": thread,
        "prompt": turn.prompt,
        "model": turn.model,
        "mode": turn.mode,
        "effort": turn.effort,
    });
    // Absent without a focus, so a turn recorded before §13.9 replays as it did.
    if let Some(focus) = &turn.focus {
        params["focus"] = serde_json::json!(focus);
    }
    if let Some(plan) = &turn.plan {
        params["plan"] = serde_json::json!(plan);
    }
    user_command(command_id, "turn.start", params)
}

impl Turns {
    pub(crate) fn new(
        storage: Arc<Storage>,
        runtime: Arc<Runtime>,
        sessions: Arc<Sessions>,
        handles: Arc<LiveHandles>,
        bus: Bus,
        code: Code,
    ) -> Self {
        Self {
            storage,
            runtime,
            sessions,
            handles,
            bus,
            code,
        }
    }

    /// Starts a Planner turn (§12.3, §12.7, §13.9): "turn.start", params
    /// { "thread_id", "prompt", "model", "mode", "effort" }, and "focus" when
    /// one is given. §12.7's order: a replay first, answered before anything
    /// else; then every check, before any write; then the one transaction;
    /// then the run.
    pub async fn send(
        &self,
        thread_id: ThreadId,
        turn: SendTurn,
    ) -> Result<OperationId, CoreError> {
        self.send_with(thread_id, turn, None).await
    }

    /// `send`, and when `dequeue` names a waiting message, the turn that takes
    /// it from the queue in its own transaction (§20.3).
    pub(crate) async fn send_with(
        &self,
        thread_id: ThreadId,
        turn: SendTurn,
        dequeue: Option<(QueuedMessageId, Option<CommandContext>)>,
    ) -> Result<OperationId, CoreError> {
        let command = start_command(turn.command_id.clone(), &thread_id, &turn);
        let SendTurn {
            command_id: _,
            prompt,
            model,
            mode,
            effort,
            focus,
            plan,
            client_tab,
        } = turn;
        if let Some(replay) = self.storage.replayed_turn(&command, &thread_id).await? {
            return Ok(replay.operation_id);
        }
        if self.handles.is_closed().await {
            return Err(CoreError::RuntimeStopping);
        }
        let settings = TurnSettings {
            model,
            mode,
            effort,
        };
        let context = self.storage.turn_context(&thread_id).await?;
        let continue_plan = match &plan {
            Some(plan_id) => Some(
                self.storage
                    .list_plans(&context.project_id, true)
                    .await?
                    .into_iter()
                    .find(|plan| &plan.plan_id == plan_id)
                    .map(|plan| continue_plan_block(&plan))
                    .ok_or_else(|| crate::error::CoreError::Refused {
                        code: crate::error::ErrorCode::InvalidCommand,
                        message: "the plan is not in this conversation's project".into(),
                    })?,
            ),
            None => None,
        };
        if !policy::is_available(&context.harness) {
            return Err(CoreError::HarnessUnavailable(context.harness));
        }
        // Checked before the session is touched: setting a model below would
        // change the session a running turn is using.
        if self.storage.thread_is_busy(&thread_id).await? {
            return Err(StorageError::ThreadBusy.into());
        }
        // The checks passed: a turn starts in this project, which is used (§15.6).
        self.code.touch(&context.project_id).await;
        let opened = self.sessions.open(&thread_id).await?;
        // The turn holds the session from here: a second start, or a `/context`
        // read, cannot change or prompt it until this turn gives it back.
        let events =
            (self.sessions.lease_events(&thread_id, &opened).await).map_err(|e| match e {
                LeaseError::Busy => CoreError::from(StorageError::ThreadBusy),
                LeaseError::Closed => CoreError::HarnessStartFailed(e.to_string()),
            })?;
        let turn = Turn {
            command: &command,
            prompt: &prompt,
            settings: &settings,
            focus: focus.as_ref(),
            dequeue: dequeue.as_ref().map(|(id, also)| Dequeue {
                id,
                also: also.as_ref(),
            }),
        };
        let started = match self.record(&thread_id, &opened, &context, turn).await {
            Ok(started) if !started.replayed => started,
            other => {
                (self.sessions)
                    .give_back_events(&thread_id, &opened, events)
                    .await;
                return other.map(|replay| replay.operation_id);
            }
        };
        Ok(PlannerTurn::start(
            self.runtime.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            opened,
            PlannerTurnRequest {
                thread_id,
                harness: context.harness,
                operation_id: started.operation_id,
                prompt,
                settings,
                focus: focus
                    .zip(started.focus_task)
                    .map(|(focus, (number, title))| focus_block(&focus, number, &title)),
                continue_plan,
                client_tab,
                events,
                on_completed: Some(self.on_completed()),
            },
            self.bus.clone(),
        )
        .await?)
    }

    /// Stops a turn as the local user (§2.3, §12.3), then answers the
    /// operation as it now stands. A tree that could not be terminated is
    /// `TerminationFailed`, and the operation is not `Cancelled`.
    pub async fn stop(&self, op: &OperationId) -> Result<Operation, CoreError> {
        let outcome = PlannerTurn::stop(
            self.runtime.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            op,
            Actor::user("local"),
        )
        .await?;
        if outcome == StopOutcome::TerminationFailed {
            return Err(CoreError::TerminationFailed);
        }
        Ok(self.storage.get_operation(op).await?)
    }

    /// §8.5, unchanged: every running turn is stopped and every adapter
    /// closed; answers the stop kind recorded.
    pub(crate) async fn shut_down(
        &self,
        bound: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> Result<StopKind, StorageError> {
        shutdown::shut_down(
            self.runtime.clone(),
            self.handles.clone(),
            self.sessions.clone(),
            bound,
            second_signal,
        )
        .await
    }
}
