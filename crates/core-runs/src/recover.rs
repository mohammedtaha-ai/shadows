//! One job: settling the attempts a stopped daemon left running.
use crate::{RecoverReport, Runs, RunsError};

impl Runs {
    /// Every `Running` attempt found at startup becomes `Interrupted`, with its
    /// grant revoked; none is restarted, and every scope still held is freed.
    pub async fn recover(&self) -> Result<RecoverReport, RunsError> {
        let interrupted = self.inner.store.interrupt_running().await?;
        Ok(RecoverReport {
            committed: 0,
            interrupted,
        })
    }
}
