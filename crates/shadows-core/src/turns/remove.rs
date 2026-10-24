//! Thread removal commits before live cancellation, so a new turn cannot race it.
use super::{StopOutcome, Turns};
use crate::CoreError;
use crate::threads::{PlanningThread, ThreadId};
use core_db::CommandContext;
impl Turns {
    /// Remove first, then stop the turn, then close its adapter.
    pub async fn remove_thread(
        &self,
        command_id: String,
        thread: &ThreadId,
    ) -> Result<PlanningThread, CoreError> {
        let c = CommandContext::user(
            command_id,
            "thread.remove",
            serde_json::json!({ "thread_id": thread }),
        );
        let removed = self.store.remove_thread(&c, thread).await?;
        match self.stop_running(thread).await {
            Ok(StopOutcome::TerminationFailed) => {
                tracing::error!(thread_id = %thread, "thread.remove_stop_failed");
            }
            Err(error) => {
                tracing::error!(%error, thread_id = %thread, "thread.remove_stop_failed");
            }
            Ok(_) => {}
        }
        if let Err(error) = self.sessions.close_session(thread).await {
            tracing::error!(%error, thread_id = %thread, "thread.remove_close_failed");
        }
        Ok(removed)
    }
}
