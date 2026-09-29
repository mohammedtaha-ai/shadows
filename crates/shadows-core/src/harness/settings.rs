//! One job: setting an open session's options (spec §12.4, §12.7).
//!
//! Every `session/set_config_option` Shadows sends goes through here: the
//! opening's default mode and remembered model and effort, the model a person
//! picks, and a turn's own settings before its prompt. The harness answers
//! each with the complete set, which becomes the thread's latest offer
//! (`offers.rs`).

use serde_json::Value;

use super::{LeaseError, OpenSession, Sessions};
use crate::threads::ThreadId;
use shadows_agent::{TurnSettings, acp::AcpError, choices::Offered};

/// Why the session's model was not changed (spec §12.7).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ModelRefused {
    /// The session's model list does not hold it.
    #[error("the session does not offer this model")]
    NotOffered,
    /// The harness refused it, in its own words.
    #[error("{0}")]
    Harness(String),
    /// The session could not be taken: a turn holds it, or it closed.
    #[error(transparent)]
    Lease(#[from] LeaseError),
}

impl Sessions {
    /// Sets one of the session's options. The harness answers the complete
    /// set, which becomes the thread's latest offer and is published.
    pub async fn set_option(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        config_id: &str,
        value: &str,
    ) -> Result<Offered, AcpError> {
        let answer: Value = opened
            .connection()
            .set_option(&opened.session_id, config_id, value)
            .await?;
        self.offers.record(thread, &answer).map_err(AcpError::Rpc)
    }

    /// Sets the session to `model` as soon as a person picks it (§12.7), so
    /// the efforts it answers are that model's before any turn. The session
    /// is held the way a turn holds it, so neither a turn nor a `/context`
    /// read runs while it changes; it is given back whatever the outcome.
    /// Nothing durable is written: a turn records its own model.
    pub(crate) async fn change_model(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        model: &str,
    ) -> Result<Offered, ModelRefused> {
        let events = self.lease_events(thread, opened).await?;
        let changed = self.set_model(thread, opened, model).await;
        self.give_back_events(thread, opened, events).await;
        changed
    }

    async fn set_model(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        model: &str,
    ) -> Result<Offered, ModelRefused> {
        let offered = self
            .offered(thread)
            .await
            .ok_or(ModelRefused::Lease(LeaseError::Closed))?;
        if !offered.offers_model(model) {
            return Err(ModelRefused::NotOffered);
        }
        if offered.current.model == model {
            return Ok(offered);
        }
        let id = offered.ids.model.clone();
        match self.set_option(thread, opened, &id, model).await {
            Ok(next) => Ok(next),
            Err(AcpError::Rpc(message)) => Err(ModelRefused::Harness(message)),
            Err(AcpError::Closed) => Err(ModelRefused::Lease(LeaseError::Closed)),
        }
    }

    /// §12.2: every opening sets the policy's default mode, since a new or
    /// resumed session starts at the person's own Claude defaults. Then the
    /// harness's remembered model and effort (§12.4), each skipped when the
    /// session does not offer it or the harness refuses it; a model change
    /// can move the mode, so the default is set again after.
    pub(super) async fn apply_opening_settings(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        mut offered: Offered,
        default_mode: &str,
        remembered: Option<(String, Option<String>)>,
    ) -> Result<(), AcpError> {
        if offered.current.mode != default_mode {
            let id = offered.ids.mode.clone();
            offered = self.set_option(thread, opened, &id, default_mode).await?;
        }
        let Some((model, effort)) = remembered else {
            return Ok(());
        };
        if offered.current.model != model {
            let id = offered.ids.model.clone();
            offered = match self
                .try_remembered(thread, opened, &id, &model, offered.offers_model(&model))
                .await?
            {
                Some(next) => next,
                None => offered,
            };
        }
        if let (Some(effort), Some(id)) = (effort, offered.ids.effort.clone())
            && offered.current.model == model
            && offered.current.effort.as_deref() != Some(effort.as_str())
        {
            let offers = offered.offers_effort(&effort);
            if let Some(next) = self
                .try_remembered(thread, opened, &id, &effort, offers)
                .await?
            {
                offered = next;
            }
        }
        if offered.current.mode != default_mode {
            let id = offered.ids.mode.clone();
            self.set_option(thread, opened, &id, default_mode).await?;
        }
        Ok(())
    }

    /// Sets one remembered value, or skips it with a line saying why: it is
    /// not on offer, or the harness refused it (an account that cannot use the
    /// model). Only a connection failure is an error.
    async fn try_remembered(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        config_id: &str,
        value: &str,
        offered: bool,
    ) -> Result<Option<Offered>, AcpError> {
        if !offered {
            tracing::info!(config_id, value, "sessions.remembered_not_offered: dropped");
            return Ok(None);
        }
        match self.set_option(thread, opened, config_id, value).await {
            Ok(next) => Ok(Some(next)),
            Err(AcpError::Rpc(message)) => {
                tracing::info!(config_id, value, %message, "sessions.remembered_refused: dropped");
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// §12.7: before the prompt, the session is set to the turn's model,
    /// effort and mode, one call per value that differs from what the session
    /// holds. `Err` names the setting the harness refused, in its words;
    /// nothing has been sent to the model.
    pub(crate) async fn prepare_turn(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        settings: &TurnSettings,
    ) -> Result<(), String> {
        let mut offered = self
            .offered(thread)
            .await
            .ok_or("the harness session closed before the turn")?;
        let refused = |what: &str, value: &str, e: AcpError| format!("{what} {value}: {e}");
        if offered.current.model != settings.model {
            let id = offered.ids.model.clone();
            offered = self
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
            offered = self
                .set_option(thread, opened, &id, effort)
                .await
                .map_err(|e| refused("effort", effort, e))?;
        }
        if offered.current.mode != settings.mode {
            let id = offered.ids.mode.clone();
            self.set_option(thread, opened, &id, &settings.mode)
                .await
                .map_err(|e| refused("mode", &settings.mode, e))?;
        }
        Ok(())
    }
}
