//! One job: reading the brief an attempt received.
//!
//! The context is saved inside the write that admits the attempt; here it is
//! only read back. An attempt recorded before contexts were saved has none,
//! and none is rebuilt from today's layout.
use crate::model::{Attempt, AttemptId};
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};
use core_plans::TaskContext;

/// What an attempt was given: the attempt itself, the task context saved when
/// it was admitted, and when that was. The attempt stays the source of its
/// identity, model, base commit and addition.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AttemptBrief {
    pub attempt: Attempt,
    pub context: TaskContext,
    pub captured_at: String,
}

impl Runs {
    /// The brief of `attempt`: unknown is `NotFound`, an attempt with no saved
    /// context is `ContextNotRecorded`.
    pub async fn brief(&self, attempt: &AttemptId) -> Result<AttemptBrief, RunsError> {
        let attempt = self.inner.store.get(attempt).await?;
        self.brief_of(attempt).await
    }

    /// The brief of the attempt `grant`, an executor's, belongs to: never one
    /// the caller names.
    pub async fn brief_for(&self, grant: &Grant) -> Result<AttemptBrief, RunsError> {
        let (Some(thread), Some(scope), GrantKind::Executor) =
            (&grant.thread_id, &grant.scope, grant.kind)
        else {
            return Err(RunsError::NotFound);
        };
        let attempt = self.inner.store.executor_attempt(thread, scope).await?;
        self.brief_of(attempt).await
    }

    async fn brief_of(&self, attempt: Attempt) -> Result<AttemptBrief, RunsError> {
        let (context, captured_at) = self
            .inner
            .store
            .context(&attempt.id)
            .await?
            .ok_or(RunsError::ContextNotRecorded)?;
        Ok(AttemptBrief {
            attempt,
            context,
            captured_at,
        })
    }
}
