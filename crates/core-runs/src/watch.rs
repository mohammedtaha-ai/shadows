//! One job: noticing that an attempt's turn ended, and freeing its scope.
use std::time::Duration;

use crate::Runs;
use crate::model::{AttemptId, AttemptState};
use core_operations::OperationId;
use core_threads::ThreadId;

/// How often the turn's operation is looked at while it runs.
const POLL: Duration = Duration::from_millis(25);

impl Runs {
    /// Follows the turn in the background until it ends.
    pub(crate) fn watch(&self, attempt: AttemptId, thread: ThreadId, operation: OperationId) {
        let runs = self.clone();
        tokio::spawn(async move { runs.observe(attempt, thread, operation).await });
    }

    async fn observe(&self, attempt: AttemptId, thread: ThreadId, operation: OperationId) {
        let ended = loop {
            let Ok(operations) = self.inner.turns.operations(&thread).await else {
                return;
            };
            match operations
                .into_iter()
                .find(|op| op.id.as_str() == operation.as_str())
            {
                Some(op) if op.finished_at.is_some() => break op,
                _ => tokio::time::sleep(POLL).await,
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
        if let Some(reason) = outcome
            && let Err(error) = self
                .inner
                .store
                .finish(&attempt, AttemptState::Failed, Some(reason))
                .await
        {
            tracing::error!(%attempt, %error, "runs.finish_failed");
        }
        self.release(&attempt, &thread).await;
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
