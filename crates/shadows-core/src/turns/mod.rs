//! One job: starting and stopping a Planner turn (spec §12.3, §12.7, §14.4) —
//! the `Turns` service.
//!
//! `send` is §12.7's order, and the order is the rule (§14.9): a replay is
//! answered first; the harness is checked, then the thread's busyness, before
//! the session is touched; the session is opened, then its events leased,
//! before any setting changes; the one transaction checks busyness again,
//! atomically; only then does the turn run. `stop` is Stop as the local user.
//!
//! `spawn` registers a committed turn and commits `Running`, `turn` records
//! its ending and arbitrates it with Stop, `handles` is the registry of live
//! turns, `entries` turns harness events into entries, `shutdown` stops every
//! turn when the runtime stops (§8.5), `model` holds the operation types and
//! `store` the queries. All are private: a caller starts or stops a turn
//! through `Turns` only.

mod entries;
mod handles;
mod model;
mod shutdown;
mod spawn;
mod store;
mod turn;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::TurnSettings;
use shadows_agent::acp::AcpError;
use shadows_agent::choices::{Offered, refusal};
use shadows_agent::policy;

pub use handles::LiveHandles;
pub use model::{InvocationView, Operation, OperationId};
pub use spawn::StartError;
// What another write calls inside its own transaction (spec §14.6):
// forking checks for an open turn, recovery records its transitions.
pub(crate) use store::{existed, has_open_operation, read_before, record};

use spawn::{PlannerTurnRequest, continue_plan_block, focus_block};
use store::{NewTurn, StartedTurn};
use turn::PlannerTurn;
pub(crate) use turn::StopOutcome;

use crate::app::{Bus, user_command};
use crate::code::Code;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError};
use crate::error::CoreError;
use crate::events::Actor;
use crate::harness::{LeaseError, OpenSession, Sessions, prompt_version};
use crate::plans::{Focus, PlanId};
use crate::runtime::Runtime;
use crate::runtime::StopKind;
use crate::threads::{ThreadId, TurnContext};

/// What the turn machinery needs for tests that drive it below `Turns`:
/// `shadows_core::testing` re-exports these, and nothing outside the crate
/// reaches them otherwise.
#[cfg(feature = "test-support")]
pub(crate) mod for_tests {
    pub use super::model::FailureStage;
    pub use super::shutdown::shut_down;
    pub use super::spawn::PlannerTurnRequest;
    pub use super::store::{NewTurn, StartedTurn};
    pub use super::turn::{PlannerTurn, StopOutcome};
}

/// Turns: what storage holds, the runtime that owns their operations, the
/// sessions they prompt on, the registry of live turns, the bus their live
/// events go out on, and the code index a turn's project is touched in.
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

/// What the person asked for: the command and what it records.
struct Turn<'a> {
    command: &'a CommandContext,
    prompt: &'a str,
    settings: &'a TurnSettings,
    focus: Option<&'a Focus>,
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
        let SendTurn {
            command_id,
            prompt,
            model,
            mode,
            effort,
            focus,
            plan,
            client_tab,
        } = turn;
        let mut params = serde_json::json!({
            "thread_id": thread_id, "prompt": prompt, "model": model, "mode": mode, "effort": effort,
        });
        // Absent without a focus, so a turn recorded before §13.9 replays as it did.
        if let Some(focus) = &focus {
            params["focus"] = serde_json::json!(focus);
        }
        if let Some(plan) = &plan {
            params["plan"] = serde_json::json!(plan);
        }
        let command = user_command(command_id, "turn.start", params);
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

    /// The rest of §12.7's checks on the leased session, then the one
    /// transaction. A model set for the turn is set back when the turn is not
    /// started, so a refused turn leaves the session as it found it.
    async fn record(
        &self,
        thread_id: &ThreadId,
        opened: &OpenSession,
        context: &TurnContext,
        turn: Turn<'_>,
    ) -> Result<StartedTurn, CoreError> {
        let (offered, previous) = self
            .offer_for_model(thread_id, opened, &turn.settings.model)
            .await?;
        let started = self.start(thread_id, context, turn, &offered).await;
        if let Some(previous) = previous
            && !matches!(&started, Ok(s) if !s.replayed)
        {
            self.set_model_back(thread_id, opened, &offered, &previous)
                .await;
        }
        started
    }

    /// The offer's checks on the turn's settings, then `start_turn`.
    async fn start(
        &self,
        thread_id: &ThreadId,
        context: &TurnContext,
        turn: Turn<'_>,
        offered: &Offered,
    ) -> Result<StartedTurn, CoreError> {
        let Turn {
            command,
            prompt,
            settings,
            focus,
        } = turn;
        if let Some((what, id)) = refusal(offered, &context.harness, settings) {
            return Err(CoreError::SettingNotOffered {
                what: what.to_string(),
                id,
                detail: None,
            });
        }
        let project = self.storage.get_project(&context.project_id).await?;
        let allowed = project.allowed_modes.get(&context.harness);
        if !allowed.is_some_and(|modes| modes.contains(&settings.mode)) {
            return Err(CoreError::ModeNotAllowed(settings.mode.clone()));
        }

        let adapter = self.sessions.adapter();
        let (harness_path, agent_path) = (
            adapter.adapter.to_string_lossy().into_owned(),
            adapter.agent.to_string_lossy().into_owned(),
        );
        // §13.8: recorded so a later turn tells the session only what changed.
        let instructions = (self.storage)
            .current_planner_instructions(&context.project_id)
            .await?;
        let started = self
            .storage
            .start_turn(
                command,
                NewTurn {
                    thread_id,
                    runtime: &self.runtime.instance_id,
                    prompt,
                    role: "Planner",
                    harness_kind: &context.harness,
                    harness_path: &harness_path,
                    harness_version: &adapter.adapter_version,
                    agent_path: &agent_path,
                    agent_version: &adapter.agent_version,
                    settings,
                    prompt_version: Some(prompt_version()),
                    instructions_version: instructions.as_ref().map(|v| v.id.as_str()),
                    focus,
                },
            )
            .await;
        match started {
            Ok(started) => Ok(started),
            Err(StorageError::TransitionConflict { .. }) if self.handles.is_closed().await => {
                Err(CoreError::RuntimeStopping)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// The session's offer for `model`, with the model it held when this set
    /// another. Efforts belong to a model, so a turn naming another model than
    /// the session holds sets it first (§12.7): the only session change made
    /// before the transaction, since it records nothing. The harness refusing
    /// it is `SettingNotOffered` in its words.
    async fn offer_for_model(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        model: &str,
    ) -> Result<(Offered, Option<String>), CoreError> {
        let closed = || CoreError::HarnessStartFailed("the harness session closed".into());
        let offered = self.sessions.offered(thread).await.ok_or_else(closed)?;
        if offered.current.model == model || !offered.offers_model(model) {
            return Ok((offered, None));
        }
        let id = offered.ids.model.clone();
        match self.sessions.set_option(thread, opened, &id, model).await {
            Ok(next) => Ok((next, Some(offered.current.model))),
            Err(AcpError::Rpc(message)) => Err(CoreError::SettingNotOffered {
                what: "model".into(),
                id: model.into(),
                detail: Some(message),
            }),
            Err(AcpError::Closed) => Err(closed()),
        }
    }

    /// Sets back the model `offer_for_model` replaced. The turn's refusal is
    /// the answer either way, so a harness that will not take the model back
    /// is logged, not returned.
    async fn set_model_back(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        offered: &Offered,
        previous: &str,
    ) {
        let id = &offered.ids.model;
        if let Err(e) = self.sessions.set_option(thread, opened, id, previous).await {
            tracing::warn!(thread_id = %thread, model = previous, error = %e, "turn.model_not_set_back");
        }
    }
}
