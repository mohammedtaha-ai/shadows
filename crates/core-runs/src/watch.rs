//! One job: settling an attempt when its turn ends. A committed attempt keeps
//! its scope until its task is judged.
use std::time::Duration;

use crate::Runs;
use crate::model::{AttemptId, AttemptState};
use core_operations::OperationId;
use core_threads::ThreadId;

/// The longest the turn's operation goes unread while it runs. A commit wakes
/// the watcher sooner; this bound covers a signal it did not see.
const RECHECK: Duration = Duration::from_secs(1);

impl Runs {
    /// Follows the turn in the background until it ends.
    pub(crate) fn watch(&self, attempt: AttemptId, thread: ThreadId, operation: OperationId) {
        let runs = self.clone();
        tokio::spawn(async move { runs.observe(attempt, thread, operation).await });
    }

    async fn observe(&self, attempt: AttemptId, thread: ThreadId, operation: OperationId) {
        let mut warned = false;
        // Taken before the first read: an ending committed between a read and
        // the wait is already a pending change.
        let mut committed = self.inner.store.committed();
        let ended = loop {
            // A failed read proves nothing about the turn: the scope stays held
            // and the turn is looked at again.
            let operations = match self.inner.turns.operations(&thread).await {
                Ok(operations) => operations,
                Err(error) => {
                    if !std::mem::replace(&mut warned, true) {
                        tracing::warn!(%attempt, %error, "runs.watch_read_failed");
                    }
                    tokio::time::sleep(RECHECK).await;
                    continue;
                }
            };
            match operations
                .into_iter()
                .find(|op| op.id.as_str() == operation.as_str())
            {
                Some(op) if op.finished_at.is_some() => break op,
                _ => {
                    // A closed signal answers at once: wait the bound instead.
                    if let Ok(Err(_)) = tokio::time::timeout(RECHECK, committed.changed()).await {
                        tokio::time::sleep(RECHECK).await;
                    }
                }
            }
        };
        // A stopped turn is Stop's to record; a daemon stop leaves the attempt
        // to recovery.
        let outcome = match ended.status_kind.as_str() {
            "Completed" => Some("ended without a commit".to_string()),
            "Failed" => Some(
                ended
                    .failure_reason
                    .unwrap_or_else(|| "the turn failed".to_string()),
            ),
            _ => None,
        };
        let told = outcome.is_some();
        let mut failed_here = false;
        if let Some(reason) = outcome {
            match self
                .inner
                .store
                .finish(&attempt, AttemptState::Failed, Some(reason))
                .await
            {
                Ok(moved) => failed_here = moved,
                Err(error) => tracing::error!(%attempt, %error, "runs.finish_failed"),
            }
        }
        let now = match self.inner.store.get(&attempt).await {
            Ok(now) => now,
            Err(error) => {
                tracing::error!(%attempt, %error, "runs.watch_read_failed");
                self.release(&attempt, &thread).await;
                return;
            }
        };
        match now.state {
            // A committed attempt waits, holding its scope, for the Planner's
            // judgment; a cancelled turn was Stop's, which released it.
            AttemptState::Committed => {
                if told {
                    self.notify(&now).await;
                }
            }
            AttemptState::Failed if failed_here => {
                self.release(&attempt, &thread).await;
                self.notify(&now).await;
            }
            _ => self.release(&attempt, &thread).await,
        }
    }

    /// The session cannot write any more: close it, and free the scope.
    pub(crate) async fn release(&self, attempt: &AttemptId, thread: &ThreadId) {
        if let Err(error) = self.inner.harness.release_executor(thread).await {
            tracing::warn!(%attempt, %error, "runs.session_close_failed");
        }
        if let Err(error) = self.inner.store.release(attempt).await {
            tracing::error!(%attempt, %error, "runs.release_failed");
        }
    }
}
