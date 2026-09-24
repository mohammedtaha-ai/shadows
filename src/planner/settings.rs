//! One job: setting an open session's options (spec §12.4, §12.7).
//!
//! Every `session/set_config_option` Shadows sends goes through here: the
//! opening's default mode and remembered model and effort, and a turn's own
//! settings before its prompt. The harness answers each with the complete
//! set, which becomes the thread's latest offer (`offers.rs`).

use serde_json::Value;

use super::{OpenSession, Sessions};
use crate::{
    agent::{TurnSettings, acp::AcpError, choices::Offered},
    thread::ThreadId,
};

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
    pub(super) async fn prepare_turn(
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
