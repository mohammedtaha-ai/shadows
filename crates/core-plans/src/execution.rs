//! One job: whether a plan version may start work now — its state read in
//! one snapshot, for Runs' preflight and inside its admission write.
use crate::{Plan, PlanState, Plans, PlansError, WorkflowId, WorkflowState};
use core_db::{StorageError, Tx};

/// Why a version admits no new attempt now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionRefusal {
    Archived,
    NotFrozen,
    /// A newer version, Draft or Frozen, follows this one.
    Superseded,
}

/// A version with its linked tasks, and why it admits no new attempt, if it
/// does not.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub plan: Plan,
    pub refusal: Option<ExecutionRefusal>,
}

impl Plans {
    /// `execution_plan_in`, in a read snapshot of its own.
    pub async fn execution_plan(
        &self,
        workflow: &WorkflowId,
        version: i64,
    ) -> Result<ExecutionPlan, PlansError> {
        let mut snapshot =
            (self.store.storage.reader().begin().await).map_err(StorageError::from)?;
        let plan = Self::execution_plan_in(&mut snapshot, workflow, version).await?;
        snapshot.commit().await.map_err(StorageError::from)?;
        Ok(plan)
    }

    /// Version `version` of `workflow` as the caller's transaction sees it. Another
    /// version number is not found; the refusal is checked archived, then not
    /// frozen, then superseded.
    pub async fn execution_plan_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
    ) -> Result<ExecutionPlan, PlansError> {
        let plan = Self::version_in(tx, workflow).await?;
        if plan.version != version {
            return Err(StorageError::NotFound("workflow").into());
        }
        let refusal = if plan.plan_state == PlanState::Archived {
            Some(ExecutionRefusal::Archived)
        } else if plan.state != WorkflowState::Frozen {
            Some(ExecutionRefusal::NotFrozen)
        } else if plan.next.is_some() {
            Some(ExecutionRefusal::Superseded)
        } else {
            None
        };
        Ok(ExecutionPlan { plan, refusal })
    }
}
