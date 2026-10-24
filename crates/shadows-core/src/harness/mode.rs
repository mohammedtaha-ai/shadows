//! Durable conversation mode commands and idle session application.
use super::{Harness, LeaseError, refusal};
use crate::CoreError;
use core_db::{CommandContext, StorageError};
use core_threads::{ThreadId, Threads};
use shadows_agent::choices::SessionChoices;

impl Harness {
    /// Remembers a permission choice immediately; a leased turn keeps its own mode.
    pub async fn change_mode(
        &self,
        command_id: String,
        thread: &ThreadId,
        mode: &str,
    ) -> Result<SessionChoices, CoreError> {
        let command = CommandContext::user(
            command_id,
            "thread.mode",
            serde_json::json!({ "thread_id": thread, "mode": mode }),
        );
        let mut conn = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        let replay = CommandContext::classify(&mut conn, &command, "Thread", thread.as_str())
            .await?
            .is_some();
        let context = Threads::turn_context_in(&mut conn, thread).await?;
        if !replay {
            Self::check_mode_in(&mut conn, thread, mode).await?;
        }
        drop(conn);
        self.check_available(&context).await?;
        self.sessions.open(thread).await?;
        let slot = self.sessions.slot(thread).await;
        let mut live = slot.lock().await;
        let item = live
            .as_mut()
            .ok_or_else(|| refusal("mode", mode, LeaseError::Closed.into()))?;
        let offered =
            self.sessions.offered(thread).await.ok_or_else(|| {
                CoreError::HarnessStartFailed("the harness session closed".into())
            })?;
        if !replay && !offered.offers_mode(mode) {
            return Err(CoreError::SettingNotOffered {
                what: "mode".into(),
                id: mode.into(),
                detail: None,
            });
        }
        let busy = self
            .store
            .change_mode(command, thread.clone(), mode.into())
            .await?;
        if !busy && item.events.is_some() {
            let selected = self.selected_mode(thread).await?;
            self.sessions
                .set_mode(thread, &item.opened, offered, &selected)
                .await
                .map_err(|r| refusal("mode", mode, r))?;
        }
        drop(live);
        let offered =
            self.sessions.offered(thread).await.ok_or_else(|| {
                CoreError::HarnessStartFailed("the harness session closed".into())
            })?;
        self.choices(thread, &offered).await
    }

    pub(super) async fn selected_mode(&self, thread: &ThreadId) -> Result<String, CoreError> {
        let mut conn = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        Self::selected_mode_in(&mut conn, thread).await
    }

    pub(super) async fn apply_idle_mode(&self, thread: &ThreadId) -> Result<(), CoreError> {
        let slot = self.sessions.slot(thread).await;
        let mut live = slot.lock().await;
        let Some(item) = live.as_mut() else {
            return Ok(());
        };
        if item.events.is_none() || self.operations.thread_is_busy(thread).await? {
            return Ok(());
        }
        let mode = self.selected_mode(thread).await?;
        let offered =
            self.sessions.offered(thread).await.ok_or_else(|| {
                CoreError::HarnessStartFailed("the harness session closed".into())
            })?;
        self.sessions
            .set_mode(thread, &item.opened, offered, &mode)
            .await
            .map_err(|r| refusal("mode", &mode, r))?;
        Ok(())
    }
}
