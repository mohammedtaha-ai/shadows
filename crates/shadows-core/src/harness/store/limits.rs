//! Latest reported account limit windows.
use super::HarnessStore;
use crate::db::StorageError;
use shadows_agent::events::{AccountLimits, LimitWindow};
type LimitRow = (Option<f64>, Option<i64>, Option<f64>, Option<i64>, String);

fn window(utilization: Option<f64>, resets_at: Option<i64>) -> Option<LimitWindow> {
    Some(LimitWindow {
        utilization: utilization?,
        resets_at: resets_at?,
    })
}

impl HarnessStore {
    /// The latest limits `kind` reported, with when they were observed.
    pub async fn latest_limits(&self, kind: &str) -> Result<Option<AccountLimits>, StorageError> {
        let row: Option<LimitRow> = sqlx::query_as(
            "SELECT five_hour_utilization, five_hour_resets_at,
                    seven_day_utilization, seven_day_resets_at, observed_at
               FROM harness_limit WHERE harness_kind = ?",
        )
        .bind(kind)
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(row.map(|r| AccountLimits {
            five_hour: window(r.0, r.1),
            seven_day: window(r.2, r.3),
            observed_at: r.4,
        }))
    }

    /// Records the limits `kind` just reported; the latest wins.
    pub async fn record_limits(
        &self,
        kind: &str,
        limits: &AccountLimits,
    ) -> Result<(), StorageError> {
        let (kind, l) = (kind.to_string(), limits.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let (five, seven) = (l.five_hour.as_ref(), l.seven_day.as_ref());
                    sqlx::query(
                        "INSERT INTO harness_limit
                       (harness_kind, five_hour_utilization, five_hour_resets_at,
                        seven_day_utilization, seven_day_resets_at, observed_at)
                     VALUES (?,?,?,?,?,?)
                     ON CONFLICT (harness_kind) DO UPDATE SET
                       five_hour_utilization = excluded.five_hour_utilization,
                       five_hour_resets_at = excluded.five_hour_resets_at,
                       seven_day_utilization = excluded.seven_day_utilization,
                       seven_day_resets_at = excluded.seven_day_resets_at,
                       observed_at = excluded.observed_at",
                    )
                    .bind(&kind)
                    .bind(five.map(|w| w.utilization))
                    .bind(five.map(|w| w.resets_at))
                    .bind(seven.map(|w| w.utilization))
                    .bind(seven.map(|w| w.resets_at))
                    .bind(&l.observed_at)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}
