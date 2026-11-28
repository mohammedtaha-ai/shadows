//! One job: keeping, for a plan's external author, how an attempt ended or that its
//! executor reported a contract conflict.
//!
//! Shadows sends the report nowhere: it is kept as one row, addressed to the grant that
//! wrote the attempt's own version, and the author waits for it. The body is the one the
//! Planner's notice carries.
use crate::model::Attempt;
use crate::store::author::NewAuthorReport;
use crate::{Runs, RunsError};
use core_grants::GrantId;
use core_plans::{Plan, WrittenBy};

impl Runs {
    /// The attempt's own version and the grant that wrote it, when an external agent did.
    pub(crate) async fn external_author(
        &self,
        attempt: &Attempt,
    ) -> Result<Option<(Plan, GrantId)>, RunsError> {
        let plan = match self.read_plan(&attempt.workflow_id).await {
            Ok(plan) => plan,
            Err(RunsError::NotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let WrittenBy::External { grant_id } = &plan.written_by else {
            return Ok(None);
        };
        let grant = grant_id.clone();
        Ok(Some((plan, grant)))
    }

    /// Keeps the attempt's `conflict` or `ended` report for `grant`, once, in the write that
    /// settles its notice; a repeat keeps the first row and settles again.
    pub(crate) async fn keep_for_author(
        &self,
        attempt: &Attempt,
        plan: &Plan,
        grant: &GrantId,
        conflict: bool,
    ) -> Result<(), RunsError> {
        let kind = if conflict { "conflict" } else { "ended" };
        let report_id = NewAuthorReport::report_id(&attempt.id, kind);
        let body = if self.inner.store.author_report_kept(&report_id).await? {
            String::new()
        } else if conflict {
            let conflict = (self.conflict(&attempt.id).await?).ok_or(RunsError::NotFound)?;
            Self::conflict_body(attempt, &Self::title(plan, attempt), &conflict)
        } else {
            self.ended_notice(attempt, plan).await?.0
        };
        let kept = (self.inner.store)
            .keep_author_report(NewAuthorReport {
                attempt: attempt.id.clone(),
                kind,
                project_id: attempt.project_id.as_str().to_string(),
                grant_id: grant.as_str().to_string(),
                workflow_id: attempt.workflow_id.as_str().to_string(),
                task: attempt.task,
                body,
            })
            .await?;
        if kept {
            self.inner.author_reports.notify_waiters();
        }
        Ok(())
    }
}
