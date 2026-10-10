//! One job: the checks and the one transaction that commit a turn (spec §12.7)
//! — the offer's checks on the turn's settings, then `Storage::start_turn`.

use shadows_agent::TurnSettings;
use shadows_agent::acp::AcpError;
use shadows_agent::choices::{Offered, refusal};

use super::Turns;
use super::store::{Dequeue, NewTurn, StartedTurn};
use crate::command::CommandContext;
use crate::db::StorageError;
use crate::error::CoreError;
use crate::harness::{OpenSession, prompt_version};
use crate::plans::Focus;
use crate::threads::{ThreadId, TurnContext};

/// What the person asked for: the command and what it records.
pub(super) struct Turn<'a> {
    pub(super) command: &'a CommandContext,
    pub(super) prompt: &'a str,
    pub(super) settings: &'a TurnSettings,
    pub(super) focus: Option<&'a Focus>,
    pub(super) dequeue: Option<Dequeue<'a>>,
}

impl Turns {
    /// The rest of §12.7's checks on the leased session, then the one
    /// transaction. A model set for the turn is set back when the turn is not
    /// started, so a refused turn leaves the session as it found it.
    pub(super) async fn record(
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
            dequeue,
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
        let additions = self
            .storage
            .current_standards_additions(&context.project_id)
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
                    standards_version: Some(crate::design::base_standards().version),
                    standards_additions_version: additions.as_ref().map(|v| v.id.as_str()),
                    focus,
                    dequeue,
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
            tracing::warn!(
                thread_id = %thread,
                model = previous,
                error = %e,
                "turn.model_not_set_back"
            );
        }
    }
}
