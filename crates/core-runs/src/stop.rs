//! One job: stopping an attempt and waiting for its turn's confirmed end,
//! for the person or a project agent.
use crate::model::AttemptState;
use crate::{Attempt, AttemptId, Runs, RunsError};
use core_db::CommandContext;
use core_grants::{Grant, GrantKind};
use core_turns::TurnsError;

impl Runs {
    /// Stops the attempt's turn, as the Planner's Stop does, and answers once
    /// the end is confirmed. An attempt that already left `Running` keeps its
    /// state; its session is closed all the same. A session that could not be
    /// terminated or closed keeps its scope held, and the Stop is refused.
    pub async fn stop(&self, attempt: &AttemptId) -> Result<Attempt, RunsError> {
        self.stop_by(attempt, Stopper::Person).await
    }

    async fn stop_by(&self, attempt: &AttemptId, by: Stopper) -> Result<Attempt, RunsError> {
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
        let owed = matches!(by, Stopper::Person) && self.inner.store.finish_owed(attempt).await?;
        if !owed {
            let moved = (self.inner.store)
                .finish(attempt, AttemptState::Stopped, None)
                .await?;
            // An agent's own Stop is answered to that agent, so its notice is settled with
            // nothing to send, or the author's wait would count the attempt pending.
            if moved && !matches!(by, Stopper::Person) {
                self.inner.store.mark_notice_sent(attempt).await?;
            }
        }
        self.release(attempt, &current.thread_id).await?;
        let now = self.inner.store.get(attempt).await?;
        if owed && let Err(error) = self.notify(&now).await {
            tracing::warn!(%attempt, %error, "runs.notify_due_failed");
        }
        if let Err(error) = self.retry_conflict_notice(&now).await {
            tracing::warn!(%attempt, %error, "runs.conflict_notice_pending");
        }
        self.pump(&current.thread_id).await;
        Ok(now)
    }

    /// A project agent's Stop of `attempt`, a task of its own project's plan: the
    /// same confirmed Stop. A replay of `command_id` answers the attempt as
    /// it stands; the same id for another request is a conflict.
    pub async fn stop_for(
        &self,
        grant: &Grant,
        command_id: String,
        attempt: &AttemptId,
    ) -> Result<Attempt, RunsError> {
        let current = self.inner.store.get(attempt).await?;
        self.planner_plan(grant, &current.workflow_id).await?;
        let (principal_kind, principal_id, stopper) = match grant.kind {
            GrantKind::Thread => {
                let thread = grant.thread_id.as_ref().ok_or(RunsError::NotFound)?;
                ("Thread", thread.to_string(), Stopper::Planner)
            }
            GrantKind::Project => ("Grant", grant.id.to_string(), Stopper::External),
            GrantKind::Executor => return Err(RunsError::NotFound),
        };
        let command = CommandContext {
            agent: None,
            principal_kind: principal_kind.into(),
            principal_id,
            command_id,
            command_kind: "run.stop".into(),
            command_schema_ver: 1,
            request_fingerprint: CommandContext::fingerprint(
                "run.stop",
                &serde_json::json!({ "attempt": attempt }),
            ),
        };
        if self.inner.store.stop_replayed(&command, attempt).await? {
            return self.inner.store.get(attempt).await;
        }
        let stopped = self.stop_by(attempt, stopper).await?;
        self.inner.store.record_stop(command, attempt).await?;
        Ok(stopped)
    }
}

enum Stopper {
    Person,
    Planner,
    External,
}
