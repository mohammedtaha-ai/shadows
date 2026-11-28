//! Persistence of the reports kept for a plan's external author.
use super::RunsStore;
use crate::RunsError;
use crate::model::{AttemptId, AttemptState, AuthorReport, ConflictDelivery};
use core_db::{Actor, Storage};
use core_grants::{Grant, Grants};
use core_plans::WorkflowId;

/// One report as first kept: the attempt's end, or its executor's conflict.
pub(crate) struct NewAuthorReport {
    pub(crate) attempt: AttemptId,
    /// `ended` or `conflict`.
    pub(crate) kind: &'static str,
    pub(crate) project_id: String,
    pub(crate) grant_id: String,
    pub(crate) workflow_id: String,
    pub(crate) task: u32,
    pub(crate) body: String,
}

impl NewAuthorReport {
    /// Stable across every answer of one report.
    pub(crate) fn report_id(attempt: &AttemptId, kind: &str) -> String {
        format!("{}:{kind}", attempt.as_str())
    }
}

impl RunsStore {
    /// Whether report `report_id` is kept already.
    pub async fn author_report_kept(&self, report_id: &str) -> Result<bool, RunsError> {
        let found: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM run_author_report WHERE report_id = ?")
                .bind(report_id)
                .fetch_optional(self.storage.reader())
                .await?;
        Ok(found.is_some())
    }

    /// Whether `attempt` has a report its author has not yet acknowledged.
    pub async fn author_report_open(&self, attempt: &AttemptId) -> Result<bool, RunsError> {
        let found: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM run_author_report WHERE attempt_id = ? AND acked_at IS NULL LIMIT 1",
        )
        .bind(attempt.as_str())
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(found.is_some())
    }

    /// Keeps `report` once and, in the same write, settles what it reports and journals the
    /// task's progress: an ended attempt's notice is sent, a pending conflict's delivery becomes
    /// delivered. Answers whether this write kept a new row.
    pub async fn keep_author_report(&self, report: NewAuthorReport) -> Result<bool, RunsError> {
        let at = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let report_id = NewAuthorReport::report_id(&report.attempt, report.kind);
                    let kept = sqlx::query(
                        "INSERT OR IGNORE INTO run_author_report
                           (report_id, attempt_id, kind, project_id, grant_id, workflow_id,
                            task, body, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&report_id)
                    .bind(report.attempt.as_str())
                    .bind(report.kind)
                    .bind(&report.project_id)
                    .bind(&report.grant_id)
                    .bind(&report.workflow_id)
                    .bind(i64::from(report.task))
                    .bind(&report.body)
                    .bind(&at)
                    .execute(&mut *conn)
                    .await?
                    .rows_affected()
                        > 0;
                    let mut changed = kept;
                    if report.kind == "conflict" {
                        let moved = sqlx::query(
                            "UPDATE run_attempts SET conflict_delivery = ?
                             WHERE id = ? AND conflict_delivery = 'pending'",
                        )
                        .bind(ConflictDelivery::Delivered.as_str())
                        .bind(report.attempt.as_str())
                        .execute(&mut *conn)
                        .await?
                        .rows_affected();
                        changed |= moved > 0;
                    } else {
                        sqlx::query(
                            "UPDATE run_attempts SET notice_sent_at = COALESCE(notice_sent_at, ?)
                             WHERE id = ?",
                        )
                        .bind(&at)
                        .bind(report.attempt.as_str())
                        .execute(&mut *conn)
                        .await?;
                    }
                    if changed {
                        super::attempt_progress_in(conn, &report.attempt, Actor::system(), &at)
                            .await?;
                    }
                    Ok(kept)
                })
            })
            .await
    }

    /// One write, after `grant` is checked live: acknowledges the rows in `ack` that are
    /// `grant`'s, then claims every row of `grant` open or with a lapsed claim, for `lease`
    /// seconds. Answers the claimed rows, oldest first, and, when none, the seconds until the
    /// earliest claim still held on `grant`'s rows lapses.
    pub async fn claim_author_reports(
        &self,
        grant: &Grant,
        ack: &[String],
        lease: u32,
    ) -> Result<(Vec<AuthorReport>, Option<f64>), RunsError> {
        let (grant, ack) = (grant.clone(), ack.to_vec());
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    Grants::check_live_in(tx, &grant).await?;
                    let now = Storage::now();
                    for report in &ack {
                        let acked: Option<String> = sqlx::query_scalar(
                            "UPDATE run_author_report SET acked_at = ?
                             WHERE report_id = ? AND grant_id = ? AND acked_at IS NULL
                             RETURNING attempt_id",
                        )
                        .bind(&now)
                        .bind(report)
                        .bind(grant.id.as_str())
                        .fetch_optional(&mut *tx)
                        .await?;
                        if let Some(attempt) = acked {
                            let attempt = AttemptId::from_recorded(attempt);
                            super::attempt_progress_in(tx, &attempt, Actor::system(), &now).await?;
                        }
                    }
                    type Row = (String, String, String, i64, String, String);
                    let rows: Vec<Row> = sqlx::query_as(
                        "SELECT report_id, attempt_id, workflow_id, task, kind, body
                         FROM run_author_report
                         WHERE grant_id = ? AND acked_at IS NULL
                           AND (claimed_until IS NULL OR claimed_until <= ?)
                         ORDER BY created_at, report_id",
                    )
                    .bind(grant.id.as_str())
                    .bind(&now)
                    .fetch_all(&mut *tx)
                    .await?;
                    // Spelled as `Storage::now` spells a time: nine fraction digits and `Z`.
                    let modifier = format!("+{lease} seconds");
                    for row in &rows {
                        sqlx::query(
                            "UPDATE run_author_report
                             SET claimed_until =
                                 strftime('%Y-%m-%dT%H:%M:%f', 'now', ?) || '000000Z'
                             WHERE report_id = ?",
                        )
                        .bind(&modifier)
                        .bind(&row.0)
                        .execute(&mut *tx)
                        .await?;
                    }
                    let lapse = if rows.is_empty() {
                        sqlx::query_scalar(
                            "SELECT (julianday(MIN(claimed_until)) - julianday('now')) * 86400.0
                             FROM run_author_report
                             WHERE grant_id = ? AND acked_at IS NULL AND claimed_until IS NOT NULL",
                        )
                        .bind(grant.id.as_str())
                        .fetch_one(&mut *tx)
                        .await?
                    } else {
                        None
                    };
                    let reports = rows
                        .into_iter()
                        .map(
                            |(report_id, attempt, workflow, task, kind, body)| AuthorReport {
                                report_id,
                                attempt_id: AttemptId::from_recorded(attempt),
                                workflow_id: WorkflowId::from_recorded(workflow),
                                task: u32::try_from(task).unwrap_or(0),
                                kind,
                                report: body,
                            },
                        )
                        .collect();
                    Ok((reports, lapse))
                })
            })
            .await
    }

    /// The plans of `project` with an attempt still pending: running, or ended with its
    /// report not yet settled (a committed one's checks not yet run, or an ended one's row
    /// not yet kept) and not continued by a later attempt of its task.
    pub async fn pending_workflows(&self, project: &str) -> Result<Vec<WorkflowId>, RunsError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT a.workflow_id FROM run_attempts a
             WHERE a.project_id = ?
               AND (a.state = ?
                    OR (a.notice_sent_at IS NULL
                        AND COALESCE(a.conflict_delivery, '') <> ?
                        AND NOT EXISTS (SELECT 1 FROM run_attempts n
                                        WHERE n.workflow_id = a.workflow_id
                                          AND n.version = a.version AND n.task = a.task
                                          AND n.number > a.number)))
             ORDER BY a.workflow_id",
        )
        .bind(project)
        .bind(AttemptState::Running.as_str())
        .bind(ConflictDelivery::Delivered.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows.into_iter().map(WorkflowId::from_recorded).collect())
    }
}
