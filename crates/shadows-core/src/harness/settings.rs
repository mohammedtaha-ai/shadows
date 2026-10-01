//! One job: setting an open session's options (spec §12.4, §12.7).
//!
//! Every `session/set_config_option` Shadows sends goes through here: the
//! opening's default mode and remembered model and effort, the model or effort
//! a person picks (and the picked model's remembered effort), and a turn's own
//! settings before its prompt. The harness answers
//! each with the complete set, which becomes the thread's latest offer
//! (`offers.rs`).

use serde_json::Value;

use super::store::Remembered;
use super::{LeaseError, OpenSession, Sessions};
use crate::threads::ThreadId;
use shadows_agent::{TurnSettings, acp::AcpError, choices::Offered};

/// Why the session's model or effort was not changed (spec §12.7).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SettingRefused {
    /// The session does not offer it: a model not in its list, or an effort
    /// the model it holds does not offer.
    #[error("the session does not offer this value")]
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
    /// the efforts it answers are that model's before any turn; when it moves
    /// to the model, then to `effort`, that model's remembered effort, if it
    /// is still offered (§12.4). The session is held the way a turn holds it,
    /// so neither a turn nor a `/context` read runs while it changes; it is
    /// given back whatever the outcome. Nothing durable is written: a turn
    /// records its own model.
    pub(crate) async fn change_model(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        model: &str,
        effort: Option<&str>,
    ) -> Result<Offered, SettingRefused> {
        let events = self.lease_events(thread, opened).await?;
        let changed = self.set_model(thread, opened, model, effort).await;
        self.give_back_events(thread, opened, events).await;
        changed
    }

    /// Sets the session to `effort` as soon as a person picks it (§12.7),
    /// held as `change_model` holds it. Nothing durable is written.
    pub(crate) async fn change_effort(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        effort: &str,
    ) -> Result<Offered, SettingRefused> {
        let events = self.lease_events(thread, opened).await?;
        let changed = self.set_effort(thread, opened, effort).await;
        self.give_back_events(thread, opened, events).await;
        changed
    }

    async fn set_model(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        model: &str,
        effort: Option<&str>,
    ) -> Result<Offered, SettingRefused> {
        let offered = self.offered(thread).await.ok_or(LeaseError::Closed)?;
        if !offered.offers_model(model) {
            return Err(SettingRefused::NotOffered);
        }
        if offered.current.model == model {
            return Ok(offered);
        }
        let id = offered.ids.model.clone();
        let next = (self.set_option(thread, opened, &id, model).await).map_err(refused)?;
        (self.remembered_effort(thread, opened, next, effort).await).map_err(refused)
    }

    async fn set_effort(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        effort: &str,
    ) -> Result<Offered, SettingRefused> {
        let offered = self.offered(thread).await.ok_or(LeaseError::Closed)?;
        let Some(id) = offered
            .ids
            .effort
            .clone()
            .filter(|_| offered.offers_effort(effort))
        else {
            return Err(SettingRefused::NotOffered);
        };
        if offered.current.effort.as_deref() == Some(effort) {
            return Ok(offered);
        }
        (self.set_option(thread, opened, &id, effort).await).map_err(refused)
    }

    /// Sets the session to `effort`, the remembered effort of the model it
    /// holds, unless there is none, the model does not offer it or the harness
    /// refuses it: then the adapter's current effort stands (§12.4).
    async fn remembered_effort(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        offered: Offered,
        effort: Option<&str>,
    ) -> Result<Offered, AcpError> {
        let (Some(effort), Some(id)) = (effort, offered.ids.effort.clone()) else {
            return Ok(offered);
        };
        if offered.current.effort.as_deref() == Some(effort) {
            return Ok(offered);
        }
        let offers = offered.offers_effort(effort);
        Ok((self
            .try_remembered(thread, opened, &id, effort, offers)
            .await?)
            .unwrap_or(offered))
    }

    /// §12.2: every opening sets the policy's default mode, since a new or
    /// resumed session starts at the person's own Claude defaults. Then the
    /// harness's remembered model, then the remembered effort of the model
    /// the session then holds (§12.4), each skipped when the session does not
    /// offer it or the harness refuses it; a model change can move the mode,
    /// so the default is set again after.
    pub(super) async fn apply_opening_settings(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        mut offered: Offered,
        default_mode: &str,
        remembered: Remembered,
    ) -> Result<(), AcpError> {
        if offered.current.mode != default_mode {
            let id = offered.ids.mode.clone();
            offered = self.set_option(thread, opened, &id, default_mode).await?;
        }
        if let Some(model) = remembered.model
            && offered.current.model != model
        {
            let id = offered.ids.model.clone();
            let offers = offered.offers_model(&model);
            if let Some(next) = self
                .try_remembered(thread, opened, &id, &model, offers)
                .await?
            {
                offered = next;
            }
        }
        let effort = remembered
            .efforts
            .get(&offered.current.model)
            .map(String::as_str);
        offered = self
            .remembered_effort(thread, opened, offered, effort)
            .await?;
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

/// A refused `set_option` while a person changes a setting: the harness's own
/// words, or the session gone.
fn refused(error: AcpError) -> SettingRefused {
    match error {
        AcpError::Rpc(message) => SettingRefused::Harness(message),
        AcpError::Closed => SettingRefused::Lease(LeaseError::Closed),
    }
}
