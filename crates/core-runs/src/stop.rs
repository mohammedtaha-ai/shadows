//! One job: stopping an attempt and waiting for its turn's confirmed end.
use crate::model::AttemptState;
use crate::{Attempt, AttemptId, Runs, RunsError};
use core_turns::TurnsError;

impl Runs {
    /// Stops the attempt's turn, as the Planner's Stop does, and answers once
    /// the end is confirmed. An attempt that already left `Running` keeps its
    /// state; its session is closed all the same.
    pub async fn stop(&self, attempt: &AttemptId) -> Result<Attempt, RunsError> {
        let current = self.inner.store.get(attempt).await?;
        if let Some(operation) = &current.operation_id {
            match self.inner.turns.stop(operation).await {
                Ok(_) => {}
                Err(TurnsError::TerminationFailed) => {
                    return Err(RunsError::Unavailable(
                        "the session could not be terminated".into(),
                    ));
                }
                Err(error) => tracing::info!(%attempt, %error, "runs.stop_not_live"),
            }
        }
        self.inner
            .store
            .finish(attempt, AttemptState::Stopped, None)
            .await?;
        self.release(attempt, &current.thread_id).await;
        self.inner.store.get(attempt).await
    }
}
