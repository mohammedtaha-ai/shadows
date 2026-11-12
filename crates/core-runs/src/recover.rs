//! One job: settling the attempts a stopped daemon left running.
use crate::{RecoverReport, Runs, RunsError};

impl Runs {
    /// Every `Running` attempt found at startup becomes `Interrupted`, none
    /// restarted: with no accepted report it has no commit Shadows recorded.
    /// Grants are revoked and every scope still held is freed.
    pub async fn recover(&self) -> Result<RecoverReport, RunsError> {
        let interrupted = self.inner.store.settle_running().await?;
        Ok(RecoverReport { interrupted })
    }
}
