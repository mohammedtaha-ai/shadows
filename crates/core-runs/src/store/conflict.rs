//! Each attempt's contract conflict and its Planner notice, kept on the
//! attempt's own row: no table of their own and no state machine beside the
//! attempt's state.
use super::RunsStore;
use crate::RunsError;
use crate::model::{Attempt, AttemptId, ConflictDelivery};
use core_db::{CommandContext, Storage, StorageError, Tx};
use core_grants::{Grant, Grants};

/// The command record's scope of a conflict and of the Planner's Stop: the
/// attempt they name.
const ATTEMPT_SCOPE: &str = "Attempt";

/// An attempt's conflict as one read saw it with the attempt.
pub(crate) struct ConflictRow {
    pub(crate) attempt: Attempt,
    pub(crate) conflict_json: String,
    pub(crate) at: String,
    pub(crate) delivery: ConflictDelivery,
    /// The attempt has not released its scope.
    pub(crate) held: bool,
}

impl RunsStore {
    /// One write: `grant` is still live, a replay of `command` is answered
    /// before any state is looked at, and only then is `conflict` recorded on
    /// the running attempt with its delivery pending. Answers whether this
    /// call recorded it; a replay records nothing.
    pub async fn record_conflict(
        &self,
        grant: &Grant,
        attempt: &AttemptId,
        command: CommandContext,
        conflict: String,
    ) -> Result<bool, RunsError> {
        let (grant, attempt, ts) = (grant.clone(), attempt.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    Grants::check_live_in(conn, &grant).await?;
                    if Self::classify(conn, &command, &attempt).await? {
                        return Ok(false);
                    }
                    let row: Option<(String, bool)> = sqlx::query_as(
                        "SELECT state, conflict_json IS NOT NULL FROM run_attempts WHERE id = ?",
                    )
                    .bind(attempt.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    let (state, conflicted) = row.ok_or(RunsError::NotFound)?;
                    if conflicted {
                        return Err(RunsError::ContractConflict);
                    }
                    match state.as_str() {
                        "running" => {}
                        "committed" => return Err(RunsError::AlreadyCommitted),
                        _ => return Err(RunsError::NotFound),
                    }
                    sqlx::query(
                        "UPDATE run_attempts
                         SET conflict_json = ?, conflict_at = ?, conflict_delivery = 'pending'
                         WHERE id = ? AND state = 'running' AND conflict_json IS NULL",
                    )
                    .bind(&conflict)
                    .bind(&ts)
                    .bind(attempt.as_str())
                    .execute(&mut *conn)
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &command,
                        ATTEMPT_SCOPE,
                        attempt.as_str(),
                        "run_attempt",
                        attempt.as_str(),
                        &ts,
                    )
                    .await?;
                    Ok(true)
                })
            })
            .await
    }

    /// Whether `command` on `attempt` was already accepted; the same id with
    /// other input is a conflict.
    async fn classify(
        conn: &mut Tx,
        command: &CommandContext,
        attempt: &AttemptId,
    ) -> Result<bool, RunsError> {
        let seen = CommandContext::classify(conn, command, ATTEMPT_SCOPE, attempt.as_str())
            .await
            .map_err(|error| match error {
                StorageError::CommandConflict => RunsError::CommandConflict,
                other => other.into(),
            })?;
        Ok(seen.is_some())
    }

    /// The attempt's conflict with the attempt as the same read sees it.
    /// `NotFound` for an unknown attempt, `None` for one without a conflict.
    pub async fn conflict(&self, attempt: &AttemptId) -> Result<Option<ConflictRow>, RunsError> {
        let mut read = (self.storage.reader().begin().await).map_err(StorageError::from)?;
        let loaded = Self::load(&mut read, attempt).await?;
        type Row = (Option<String>, Option<String>, Option<String>, bool);
        let (conflict, at, delivery, held): Row = sqlx::query_as(
            "SELECT conflict_json, conflict_at, conflict_delivery, released_at IS NULL
             FROM run_attempts WHERE id = ?",
        )
        .bind(attempt.as_str())
        .fetch_one(&mut *read)
        .await?;
        read.commit().await.map_err(StorageError::from)?;
        let (Some(conflict_json), Some(at), Some(delivery)) = (conflict, at, delivery) else {
            return Ok(None);
        };
        let delivery = ConflictDelivery::parse(&delivery).ok_or_else(|| {
            StorageError::Constraint(format!("run_attempts.conflict_delivery {delivery}"))
        })?;
        Ok(Some(ConflictRow {
            attempt: loaded,
            conflict_json,
            at,
            delivery,
            held,
        }))
    }

    /// The conflict notice's delivery and its saved payload, when the attempt
    /// has a conflict.
    pub async fn notice_intent(
        &self,
        attempt: &AttemptId,
    ) -> Result<Option<(ConflictDelivery, Option<String>)>, RunsError> {
        let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT conflict_delivery, conflict_notice_json FROM run_attempts WHERE id = ?",
        )
        .bind(attempt.as_str())
        .fetch_optional(self.storage.reader())
        .await?;
        let Some((Some(delivery), intent)) = row else {
            return Ok(None);
        };
        let delivery = ConflictDelivery::parse(&delivery).ok_or_else(|| {
            StorageError::Constraint(format!("run_attempts.conflict_delivery {delivery}"))
        })?;
        Ok(Some((delivery, intent)))
    }

    /// Saves `intent` as the conflict notice's payload unless one is saved,
    /// and answers the saved one: the first resolved payload is the only one.
    pub async fn keep_notice_intent(
        &self,
        attempt: &AttemptId,
        intent: String,
    ) -> Result<String, RunsError> {
        let attempt = attempt.clone();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_attempts SET conflict_notice_json = ?
                         WHERE id = ? AND conflict_json IS NOT NULL
                           AND conflict_notice_json IS NULL",
                    )
                    .bind(&intent)
                    .bind(attempt.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let saved: Option<Option<String>> = sqlx::query_scalar(
                        "SELECT conflict_notice_json FROM run_attempts WHERE id = ?",
                    )
                    .bind(attempt.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    saved.flatten().ok_or(RunsError::NotFound)
                })
            })
            .await
    }

    /// A pending conflict notice is now `delivery`; a settled one keeps its
    /// state.
    pub async fn settle_delivery(
        &self,
        attempt: &AttemptId,
        delivery: ConflictDelivery,
    ) -> Result<(), RunsError> {
        let attempt = attempt.clone();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_attempts SET conflict_delivery = ?
                         WHERE id = ? AND conflict_delivery = 'pending'",
                    )
                    .bind(delivery.as_str())
                    .bind(attempt.as_str())
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// Every attempt whose conflict notice is still pending, oldest first.
    pub async fn pending_conflicts(&self) -> Result<Vec<AttemptId>, RunsError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM run_attempts WHERE conflict_delivery = 'pending'
             ORDER BY conflict_at, id",
        )
        .fetch_all(self.storage.reader())
        .await?;
        Ok(ids.into_iter().map(AttemptId::from_recorded).collect())
    }

    /// The turn of a running attempt ended: one write decides its state. With
    /// a conflict it is `Stopped` on that conflict, otherwise `Failed` with
    /// `reason`. `None` when it had already left `Running`; otherwise whether
    /// it had a conflict.
    pub async fn settle_ended(
        &self,
        attempt: &AttemptId,
        reason: String,
    ) -> Result<Option<bool>, RunsError> {
        let (attempt, ts) = (attempt.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let conflicted: Option<bool> = sqlx::query_scalar(
                        "UPDATE run_attempts
                         SET state = CASE WHEN conflict_json IS NULL
                                          THEN 'failed' ELSE 'stopped' END,
                             reason = CASE WHEN conflict_json IS NULL
                                           THEN ? ELSE 'contract conflict' END,
                             ended_at = ?
                         WHERE id = ? AND state = 'running'
                         RETURNING conflict_json IS NOT NULL",
                    )
                    .bind(&reason)
                    .bind(&ts)
                    .bind(attempt.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    Ok(conflicted)
                })
            })
            .await
    }

    /// Whether the Planner's Stop `command` of `attempt` was already
    /// confirmed; the same id with other input is a conflict.
    pub async fn stop_replayed(
        &self,
        command: &CommandContext,
        attempt: &AttemptId,
    ) -> Result<bool, RunsError> {
        let mut read = (self.storage.reader().acquire().await).map_err(StorageError::from)?;
        Self::classify(&mut read, command, attempt).await
    }

    /// Records the Planner's confirmed Stop `command` of `attempt`, once.
    pub async fn record_stop(
        &self,
        command: CommandContext,
        attempt: &AttemptId,
    ) -> Result<(), RunsError> {
        let (attempt, ts) = (attempt.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if Self::classify(conn, &command, &attempt).await? {
                        return Ok(());
                    }
                    CommandContext::record_command(
                        conn,
                        &command,
                        ATTEMPT_SCOPE,
                        attempt.as_str(),
                        "run_attempt",
                        attempt.as_str(),
                        &ts,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}
