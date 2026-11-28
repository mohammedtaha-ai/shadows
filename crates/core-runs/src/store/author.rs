//! Persistence of the reports kept for a plan's external author.
use super::RunsStore;
use crate::model::{AttemptId, AttemptState, AuthorReport, ConflictDelivery};
use crate::{Runs, RunsError};
use core_db::{Actor, Storage, Tx};
use core_grants::{Grant, Grants};
use core_plans::WorkflowId;

/// One report as first kept, of any kind.
pub(crate) struct AuthorRow {
    /// `ended`, `conflict`, `reply` or `recheck`.
    pub(crate) kind: &'static str,
    /// The attempt for `ended` and `conflict`, the message for `reply`, the batch for `recheck`.
    pub(crate) source_id: String,
    pub(crate) attempt: Option<AttemptId>,
    pub(crate) project_id: String,
    pub(crate) grant_id: String,
    pub(crate) workflow_id: Option<String>,
    pub(crate) task: Option<u32>,
    pub(crate) body: String,
}

impl AuthorRow {
    /// Stable across every answer of one report.
    pub(crate) fn report_id(kind: &str, source_id: &str) -> String {
        format!("{kind}:{source_id}")
    }
}

impl RunsStore {
    /// Inserts `row` unless its report is kept already; answers whether it inserted.
    pub(crate) async fn insert_author_row_in(
        tx: &mut Tx,
        row: &AuthorRow,
        at: &str,
    ) -> Result<bool, RunsError> {
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO run_author_report
               (report_id, kind, source_id, attempt_id, project_id, grant_id, workflow_id,
                task, body, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(AuthorRow::report_id(row.kind, &row.source_id))
        .bind(row.kind)
        .bind(&row.source_id)
        .bind(row.attempt.as_ref().map(AttemptId::as_str))
        .bind(&row.project_id)
        .bind(&row.grant_id)
        .bind(row.workflow_id.as_deref())
        .bind(row.task.map(i64::from))
        .bind(&row.body)
        .bind(at)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        Ok(inserted > 0)
    }

    /// Journals TaskProgressChanged for `attempt`.
    pub(crate) async fn author_progress_in(
        tx: &mut Tx,
        attempt: &AttemptId,
        at: &str,
    ) -> Result<(), RunsError> {
        super::attempt_progress_in(tx, attempt, Actor::system(), at).await
    }

    /// Whether report `report_id` is kept already.
    pub async fn author_report_kept(&self, report_id: &str) -> Result<bool, RunsError> {
        let found: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM run_author_report WHERE report_id = ?")
                .bind(report_id)
                .fetch_optional(self.storage.reader())
                .await?;
        Ok(found.is_some())
    }

    /// Whether `attempt` has an `ended` or `conflict` report its author has not yet
    /// acknowledged.
    pub async fn author_report_open(&self, attempt: &AttemptId) -> Result<bool, RunsError> {
        let found: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM run_author_report
             WHERE attempt_id = ? AND kind IN ('ended', 'conflict') AND acked_at IS NULL
             LIMIT 1",
        )
        .bind(attempt.as_str())
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(found.is_some())
    }

    /// Keeps an attempt's `ended` or `conflict` row once and, in the same write, settles what
    /// it reports and journals the task's progress: an ended attempt's notice is sent, a
    /// pending conflict's delivery becomes delivered. Answers whether this write kept a new row.
    pub async fn keep_attempt_report(&self, row: AuthorRow) -> Result<bool, RunsError> {
        let at = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let kept = Runs::keep_for_author_in(conn, &row, &at).await?;
                    let Some(attempt) = row.attempt.as_ref() else {
                        return Ok(kept);
                    };
                    if row.kind == "conflict" {
                        let moved = sqlx::query(
                            "UPDATE run_attempts SET conflict_delivery = ?
                             WHERE id = ? AND conflict_delivery = 'pending'",
                        )
                        .bind(ConflictDelivery::Delivered.as_str())
                        .bind(attempt.as_str())
                        .execute(&mut *conn)
                        .await?
                        .rows_affected();
                        if moved > 0 && !kept {
                            Self::author_progress_in(conn, attempt, &at).await?;
                        }
                    } else {
                        sqlx::query(
                            "UPDATE run_attempts SET notice_sent_at = COALESCE(notice_sent_at, ?)
                             WHERE id = ?",
                        )
                        .bind(&at)
                        .bind(attempt.as_str())
                        .execute(&mut *conn)
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
                        let acked: Option<(Option<String>, Option<i64>)> = sqlx::query_as(
                            "UPDATE run_author_report SET acked_at = ?
                             WHERE report_id = ? AND grant_id = ? AND acked_at IS NULL
                             RETURNING attempt_id, task",
                        )
                        .bind(&now)
                        .bind(report)
                        .bind(grant.id.as_str())
                        .fetch_optional(&mut *tx)
                        .await?;
                        if let Some((Some(attempt), Some(_))) = acked {
                            let attempt = AttemptId::from_recorded(attempt);
                            Self::author_progress_in(tx, &attempt, &now).await?;
                        }
                    }
                    type Row = (
                        String,
                        Option<String>,
                        Option<String>,
                        Option<i64>,
                        String,
                        String,
                    );
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
                                attempt_id: attempt.map(AttemptId::from_recorded),
                                workflow_id: workflow.map(WorkflowId::from_recorded),
                                task: task.and_then(|task| u32::try_from(task).ok()),
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

    /// Keeps a `reply` row once and, in the same write, marks message `message` delivered.
    /// Answers whether this write kept a new row.
    pub async fn keep_reply_report(
        &self,
        row: AuthorRow,
        message: &str,
    ) -> Result<bool, RunsError> {
        let (at, message) = (Storage::now(), message.to_string());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let kept = Runs::keep_for_author_in(conn, &row, &at).await?;
                    sqlx::query(
                        "UPDATE run_agent_messages SET delivered_at = COALESCE(delivered_at, ?),
                         state = CASE WHEN state = 'failed' THEN 'failed' ELSE 'delivered' END
                         WHERE id = ?",
                    )
                    .bind(&at)
                    .bind(&message)
                    .execute(&mut *conn)
                    .await?;
                    Ok(kept)
                })
            })
            .await
    }

    /// Keeps a `recheck` row once and, in the same write, ends batch `batch` with its notice
    /// settled: the row is the notice. Answers whether this write kept a new row.
    pub async fn keep_recheck_report(
        &self,
        row: AuthorRow,
        batch: &str,
    ) -> Result<bool, RunsError> {
        let (at, batch) = (Storage::now(), batch.to_string());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let kept = Runs::keep_for_author_in(conn, &row, &at).await?;
                    Self::finish_batch_in(conn, &batch, None).await?;
                    sqlx::query(
                        "UPDATE run_rechecks SET notice_sent_at=COALESCE(notice_sent_at,?)
                         WHERE id=?",
                    )
                    .bind(&at)
                    .bind(&batch)
                    .execute(&mut *conn)
                    .await?;
                    Ok(kept)
                })
            })
            .await
    }

    /// The plans of `project` with an attempt still pending: running, or ended with its
    /// report not yet settled (a committed one's checks not yet run, or an ended one's row
    /// not yet kept) and not continued by a later attempt of its task; or with a question to
    /// an executor that has neither a reply nor a no-reply reason yet.
    pub async fn pending_workflows(&self, project: &str) -> Result<Vec<WorkflowId>, RunsError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT a.workflow_id FROM run_attempts a
             WHERE a.project_id = ?
               AND (a.state = ?
                    OR EXISTS (SELECT 1 FROM run_agent_messages m
                               WHERE m.attempt_id = a.id AND m.state IN ('waiting', 'sent'))
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
