//! One job: per-harness remembered settings and limits (spec §12.4, §12.8).
//!
//! Neither is journal data: one row per harness, the latest wins, and nothing
//! replays them. The turn command writes the remembered settings inside its
//! own transaction; the limits are written as a harness reports them.

use sqlx::SqliteConnection;

use super::{Storage, StorageError};
use shadows_agent::TurnSettings;
use shadows_agent::events::{AccountLimits, LimitWindow};

/// Remembers the model and effort of the turn being started for its harness.
/// The mode is never remembered (§12.4).
pub(in crate::storage) async fn remember_settings(
    conn: &mut SqliteConnection,
    kind: &str,
    settings: &TurnSettings,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO harness_preference (harness_kind, model, effort, updated_at)
         VALUES (?,?,?,?)
         ON CONFLICT (harness_kind)
         DO UPDATE SET model = excluded.model, effort = excluded.effort,
                       updated_at = excluded.updated_at",
    )
    .bind(kind)
    .bind(&settings.model)
    .bind(&settings.effort)
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

type LimitRow = (Option<f64>, Option<i64>, Option<f64>, Option<i64>, String);

fn window(utilization: Option<f64>, resets_at: Option<i64>) -> Option<LimitWindow> {
    Some(LimitWindow {
        utilization: utilization?,
        resets_at: resets_at?,
    })
}

impl Storage {
    /// The model and effort of the last turn started on `kind`, if any.
    pub async fn remembered_settings(
        &self,
        kind: &str,
    ) -> Result<Option<(String, Option<String>)>, StorageError> {
        Ok(
            sqlx::query_as("SELECT model, effort FROM harness_preference WHERE harness_kind = ?")
                .bind(kind)
                .fetch_optional(self.reader())
                .await?,
        )
    }

    /// Test-support: remember settings without starting a turn.
    #[cfg(feature = "test-support")]
    pub async fn remember_for_test(&self, kind: &str, model: &str, effort: Option<&str>) {
        let (kind, ts) = (kind.to_string(), super::now());
        let settings = TurnSettings {
            model: model.into(),
            mode: String::new(),
            effort: effort.map(str::to_owned),
        };
        self.write_txn(move |conn| {
            Box::pin(async move { remember_settings(conn, &kind, &settings, &ts).await })
        })
        .await
        .expect("remember_for_test");
    }

    /// The latest limits `kind` reported, with when they were observed.
    pub async fn latest_limits(&self, kind: &str) -> Result<Option<AccountLimits>, StorageError> {
        let row: Option<LimitRow> = sqlx::query_as(
            "SELECT five_hour_utilization, five_hour_resets_at,
                    seven_day_utilization, seven_day_resets_at, observed_at
               FROM harness_limit WHERE harness_kind = ?",
        )
        .bind(kind)
        .fetch_optional(self.reader())
        .await?;
        Ok(row.map(|r| AccountLimits {
            five_hour: window(r.0, r.1),
            seven_day: window(r.2, r.3),
            observed_at: r.4,
        }))
    }

    /// Records the limits `kind` just reported; the latest wins (§12.8).
    pub async fn record_limits(
        &self,
        kind: &str,
        limits: &AccountLimits,
    ) -> Result<(), StorageError> {
        let (kind, l) = (kind.to_string(), limits.clone());
        self.write_txn(move |conn| {
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
