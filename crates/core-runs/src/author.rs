//! One job: a plan's external author's reports: how an attempt ended or that its executor
//! reported a contract conflict, kept, then answered to the author's wait.
//!
//! Shadows sends the report nowhere: it is kept as one row, addressed to the grant that
//! wrote the attempt's own version, and the author waits for it. The body is the one the
//! Planner's notice carries.
//! A report is delivered once its author acknowledges it by id on a later wait; an answer
//! that never reached the session is answered again once its claim lapses.
use crate::model::{Attempt, AuthorWait, WaitOutcome};
use crate::store::author::NewAuthorReport;
use crate::{Runs, RunsError};
use core_grants::{Grant, GrantId, GrantKind};
use core_plans::{Plan, WrittenBy};
use std::time::Duration;

/// How long a wait's answer holds its reports before a later wait may answer them again.
const LEASE_SECS: u32 = 120;

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

    /// Waits, up to `bound`, for reports kept for `grant`'s plans, after acknowledging those
    /// in `ack`. Answers at once with reports already open, `NoneRunning` when nothing of
    /// this author's is pending or held, else waits for a new report or a lapsed claim.
    pub async fn author_wait(
        &self,
        grant: &Grant,
        ack: &[String],
        bound: Duration,
    ) -> Result<AuthorWait, RunsError> {
        if grant.kind != GrantKind::Project {
            return Err(RunsError::NotFound);
        }
        let deadline = tokio::time::Instant::now() + bound;
        let mut ack = ack;
        loop {
            let notified = self.inner.author_reports.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let (reports, lapse) = (self.inner.store)
                .claim_author_reports(grant, ack, LEASE_SECS)
                .await?;
            ack = &[];
            if !reports.is_empty() {
                return Ok(AuthorWait {
                    outcome: WaitOutcome::Ended,
                    reports,
                });
            }
            if lapse.is_none() && !self.author_pending(grant).await? {
                return Ok(AuthorWait {
                    outcome: WaitOutcome::NoneRunning,
                    reports: vec![],
                });
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return Ok(AuthorWait {
                    outcome: WaitOutcome::NothingYet,
                    reports: vec![],
                });
            }
            let until = lapse
                .map(|secs| now + Duration::from_secs_f64(secs.max(0.0) + 0.01))
                .map_or(deadline, |at| at.min(deadline));
            tokio::select! {
                () = &mut notified => {}
                () = tokio::time::sleep_until(until) => {}
            }
        }
    }

    /// Whether an attempt of a version `grant` wrote is still pending.
    async fn author_pending(&self, grant: &Grant) -> Result<bool, RunsError> {
        let project = grant.project_id.as_str();
        for workflow in self.inner.store.pending_workflows(project).await? {
            let plan = match self.read_plan(&workflow).await {
                Ok(plan) => plan,
                Err(RunsError::NotFound) => continue,
                Err(error) => return Err(error),
            };
            if matches!(&plan.written_by, WrittenBy::External { grant_id } if *grant_id == grant.id)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
