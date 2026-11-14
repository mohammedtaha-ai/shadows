//! One job: settling the attempts a stopped daemon left running.
use crate::{RecoverReport, Runs, RunsError};

impl Runs {
    /// Every `Running` attempt found at startup becomes `Interrupted`, none
    /// restarted: with no accepted report it has no commit Shadows recorded.
    /// Grants are revoked and every scope still held is freed. A contract
    /// conflict stays on its attempt; each conflict notice still pending is
    /// then sent once more, in one pass off startup that waits for no reply.
    pub async fn recover(&self) -> Result<RecoverReport, RunsError> {
        let interrupted = self.inner.store.settle_running().await?;
        let pending = self.inner.store.pending_conflicts().await?;
        {
            let runs = self.clone();
            tokio::spawn(async move {
                for id in pending {
                    let sent = match runs.inner.store.get(&id).await {
                        Ok(attempt) => runs.retry_conflict_notice(&attempt).await,
                        Err(error) => Err(error),
                    };
                    if let Err(error) = sent {
                        tracing::warn!(attempt = %id, %error, "runs.conflict_notice_pending");
                    }
                }
                match runs.inner.store.unsent_committed().await {
                    Ok(attempts) => {
                        for id in attempts {
                            match runs.inner.store.get(&id).await {
                                Ok(attempt) => {
                                    if let Err(error) = runs.inner.store.clear_unfinished(&id).await
                                    {
                                        tracing::warn!(%id,%error,"runs.recover_clear_failed");
                                        continue;
                                    }
                                    runs.confirm(&attempt).await;
                                }
                                Err(error) => {
                                    tracing::warn!(%id,%error,"runs.recover_check_failed")
                                }
                            }
                        }
                    }
                    Err(error) => tracing::warn!(%error,"runs.recover_checks_failed"),
                }
            });
        }
        Ok(RecoverReport { interrupted })
    }
}
