//! One job: whether a task's `needs` links have been met.
use crate::{Runs, RunsError};
use core_plans::{LinkKind, Plan};

impl Runs {
    /// The tasks of this version that `task` needs and that nobody has marked
    /// done, ascending. A `needs` link to another plan's task is not waited on
    /// here: no mark of another plan is read.
    pub(crate) async fn waiting_on(&self, plan: &Plan, task: u32) -> Result<Vec<u32>, RunsError> {
        let done = self.inner.store.done_tasks(&plan.id, plan.version).await?;
        let mut waiting: Vec<u32> = plan
            .links
            .iter()
            .filter(|link| link.task == task && link.kind == LinkKind::Needs)
            .filter_map(|link| link.after.local())
            .filter(|after| !done.contains(after))
            .collect();
        waiting.sort_unstable();
        waiting.dedup();
        Ok(waiting)
    }
}
