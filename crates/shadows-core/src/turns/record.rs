//! One job: the checks and the one transaction that commit a turn
//! — the offer's checks on the turn's settings, then `TurnsStore::start_turn`.

use shadows_agent::TurnSettings;
use shadows_agent::choices::{Offered, refusal};

use super::Turns;
use super::store::{Dequeue, NewTurn, StartedTurn};
use crate::db::{CommandContext, StorageError};
use crate::error::CoreError;
use crate::harness::OpenSession;
use crate::plans::Focus;
use crate::threads::{ThreadId, TurnContext};

/// What the person asked for: the command and what it records.
pub(super) struct Turn<'a> {
    pub(super) command: &'a CommandContext,
    pub(super) prompt: &'a str,
    pub(super) attachments: &'a [crate::threads::AttachmentId],
    pub(super) settings: &'a TurnSettings,
    pub(super) focus: Option<&'a Focus>,
    pub(super) dequeue: Option<Dequeue<'a>>,
}

impl Turns {
    /// Checks the leased session, then commits the turn in one
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
            .sessions
            .offer_for_model(thread_id, opened, &turn.settings.model)
            .await?;
        let started = self
            .start(
                thread_id,
                context,
                turn,
                &offered,
                opened.aylul_usage.as_ref(),
            )
            .await;
        if let Some(previous) = previous
            && !matches!(&started, Ok(s) if !s.replayed)
        {
            self.sessions
                .set_model_back(thread_id, opened, &offered, &previous)
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
        snapshot: Option<&crate::AylulUsageSnapshot>,
    ) -> Result<StartedTurn, CoreError> {
        let Turn {
            command,
            attachments,
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
        let allowed = self
            .projects
            .allowed_modes(&context.project_id, &context.harness)
            .await?;
        if !allowed.contains(&settings.mode) {
            return Err(CoreError::ModeNotAllowed(settings.mode.clone()));
        }

        let adapter = self.sessions.adapter();
        let (harness_path, agent_path) = (
            adapter.adapter.to_string_lossy().into_owned(),
            adapter.agent.to_string_lossy().into_owned(),
        );
        // recorded so a later turn tells the session only what changed.
        let instructions = self.instructions.current(&context.project_id).await?;
        let additions = self.standards.current(&context.project_id).await?;
        let started = self
            .store
            .start_turn(
                command,
                NewTurn {
                    aylul_usage: snapshot,
                    thread_id,
                    runtime: &self.runtime.instance_id,
                    prompt,
                    attachments,
                    role: "Planner",
                    harness_kind: &context.harness,
                    harness_path: &harness_path,
                    harness_version: &adapter.adapter_version,
                    agent_path: &agent_path,
                    agent_version: &adapter.agent_version,
                    settings,
                    prompt_version: Some(crate::harness::Harness::prompt_version()),
                    instructions_version: instructions.as_ref().map(|v| v.id.as_str()),
                    standards_version: Some(crate::standards::Standards::base().version),
                    standards_additions_version: additions.as_ref().map(|v| v.id.as_str()),
                    focus,
                    dequeue,
                },
            )
            .await;
        match started {
            Ok(started) => Ok(started),
            Err(CoreError::Storage(StorageError::TransitionConflict { .. }))
                if self.handles.is_closed().await =>
            {
                Err(CoreError::RuntimeStopping)
            }
            Err(e) => Err(e),
        }
    }
}
