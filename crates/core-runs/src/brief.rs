//! One job: reading the brief an attempt received.
//!
//! The context is saved inside the write that admits the attempt; here it is
//! only read back. An attempt recorded before contexts were saved has none,
//! and none is rebuilt from today's layout. The planned consumers beside it
//! are read from the attempt's own Frozen version, never a later one.
use crate::model::{Attempt, AttemptId};
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantKind};
use core_plans::{ConsumerRef, ContractSelector, TaskContext};

/// The most planned consumers a brief lists; `task_question` pages the rest.
const BRIEF_CONSUMERS: u16 = 50;

/// What an attempt was given: the attempt itself, the task context saved when
/// it was admitted, and when that was. The attempt stays the source of its
/// identity, model, base commit and addition.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AttemptBrief {
    pub attempt: Attempt,
    pub context: TaskContext,
    pub captured_at: String,
    /// The tasks of the attempt's own version that plan to use this task's
    /// outputs, by task then use: the first 50. Plans in other versions or
    /// other plans are not listed, so an empty list is not proof of no use.
    pub consumers: Vec<ConsumerRef>,
    /// More planned consumers than `consumers` lists.
    pub consumers_more: bool,
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
        let facts = self
            .inner
            .plans
            .contract_facts(
                &attempt.workflow_id,
                attempt.task,
                ContractSelector::Own,
                None,
                BRIEF_CONSUMERS,
            )
            .await
            .map_err(RunsError::from_plans)?;
        Ok(AttemptBrief {
            attempt,
            context,
            captured_at,
            consumers: facts.consumers,
            consumers_more: facts.next.is_some(),
        })
    }
}
