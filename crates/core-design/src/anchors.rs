//! Planner conversation anchor commands.
use crate::{Design, DesignAnchor, DesignError, ThreadId};

impl Design {
    pub async fn set_thread_anchor(
        &self,
        command_id: String,
        thread: &ThreadId,
        anchor: Option<DesignAnchor>,
    ) -> Result<Option<DesignAnchor>, DesignError> {
        self.store
            .set_thread_anchor(command_id, thread, anchor)
            .await
    }

    pub async fn thread_anchor(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<DesignAnchor>, DesignError> {
        Ok(self.store.thread_anchor(thread).await?)
    }
}
