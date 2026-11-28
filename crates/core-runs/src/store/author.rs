//! Persistence of the reports kept for a plan's external author.
use super::RunsStore;
use crate::RunsError;
use crate::model::{AttemptId, ConflictDelivery};
use core_db::{Actor, Storage};

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

    /// Keeps `report` once and, in the same write, settles what it reports: an ended
    /// attempt's notice is sent, a pending conflict's delivery becomes delivered. Answers whether
    /// this write kept a new row.
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
                        if moved > 0 {
                            super::attempt_progress_in(conn, &report.attempt, Actor::system(), &at)
                                .await?;
                        }
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
                    Ok(kept)
                })
            })
            .await
    }
}
