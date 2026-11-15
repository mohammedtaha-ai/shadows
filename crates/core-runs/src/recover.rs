//! One job: settling the attempts a stopped daemon left running.
use crate::{RecoverReport, Runs, RunsError};

impl Runs {
    /// Every `Running` attempt found at startup becomes `Interrupted`, none
    /// restarted: with no accepted report it has no commit Shadows recorded.
    /// Grants are revoked and every scope still held is freed. A contract
    /// conflict stays on its attempt. Then, in one pass off startup that waits
    /// for no reply: each conflict notice still pending is sent once more; each
    /// question to a task's agent is answered from its ended turn, delivered or
    /// sent; and each task's latest committed attempt whose notice was not sent
    /// runs its unfinished checks again, keeping finished results, and goes on
    /// to its return or notice. Questions go first, so their replies do not wait
    /// behind a long check.
    pub async fn recover(&self) -> Result<RecoverReport, RunsError> {
        let interrupted = self.inner.store.settle_running().await?;
        let pending = self.inner.store.pending_conflicts().await?;
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
            match runs.inner.store.due_notices().await {
                Ok(ids) => {
                    for id in ids {
                        let sent = match runs.inner.store.get(&id).await {
                            Ok(attempt) => runs.notify(&attempt).await,
                            Err(error) => Err(error),
                        };
                        if let Err(error) = sent {
                            tracing::warn!(%id, %error, "runs.notify_due_failed");
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "runs.notify_due_failed"),
            }
            if let Err(error) = runs.recover_messages().await {
                tracing::warn!(%error, "runs.recover_messages_failed");
            }
            let attempts = match runs.inner.store.unsent_committed().await {
                Ok(attempts) => attempts,
                Err(error) => {
                    tracing::warn!(%error, "runs.recover_checks_failed");
                    return;
                }
            };
            for id in attempts {
                let attempt = match runs.inner.store.get(&id).await {
                    Ok(attempt) => attempt,
                    Err(error) => {
                        tracing::warn!(%id, %error, "runs.recover_check_failed");
                        continue;
                    }
                };
                if let Err(error) = runs.inner.store.clear_unfinished(&id).await {
                    tracing::warn!(%id, %error, "runs.recover_clear_failed");
                    continue;
                }
                runs.confirm(&attempt).await;
            }
        });
        Ok(RecoverReport { interrupted })
    }
}
