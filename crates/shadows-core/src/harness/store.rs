//! One job: per-harness remembered settings and limits (spec §12.4, §12.8).
//!
//! Neither is journal data: one row per harness (and per model, for an
//! effort), the latest wins, and nothing replays them. The turn command writes
//! the remembered settings inside its own transaction; the limits are written
//! as a harness reports them.

use std::collections::HashMap;

use sqlx::SqliteConnection;

use crate::db::{Storage, StorageError};
use shadows_agent::TurnSettings;
use shadows_agent::events::{AccountLimits, LimitWindow};

/// Remembers the model of the turn being started for its harness, and its
/// effort for that model; a turn without an effort leaves the model's
/// remembered effort as it was. The mode is never remembered (§12.4).
pub(crate) async fn remember_settings(
    conn: &mut SqliteConnection,
    kind: &str,
    settings: &TurnSettings,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO harness_preference (harness_kind, model, updated_at)
         VALUES (?,?,?)
         ON CONFLICT (harness_kind)
         DO UPDATE SET model = excluded.model, updated_at = excluded.updated_at",
    )
    .bind(kind)
    .bind(&settings.model)
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    if let Some(effort) = &settings.effort {
        sqlx::query(
            "INSERT INTO harness_model_effort (harness_kind, model, effort, updated_at)
             VALUES (?,?,?,?)
             ON CONFLICT (harness_kind, model)
             DO UPDATE SET effort = excluded.effort, updated_at = excluded.updated_at",
        )
        .bind(kind)
        .bind(&settings.model)
        .bind(effort)
        .bind(ts)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// What a harness remembers (§12.4): the model of the last turn started on
/// it, and per model the effort of the last turn started on that model.
#[derive(Debug, Default)]
pub(super) struct Remembered {
    pub(super) model: Option<String>,
    pub(super) efforts: HashMap<String, String>,
}

type LimitRow = (Option<f64>, Option<i64>, Option<f64>, Option<i64>, String);

fn window(utilization: Option<f64>, resets_at: Option<i64>) -> Option<LimitWindow> {
    Some(LimitWindow {
        utilization: utilization?,
        resets_at: resets_at?,
    })
}

impl Storage {
    /// The model of the last turn started on `kind`, if any, with that
    /// model's remembered effort.
    pub async fn remembered_settings(
        &self,
        kind: &str,
    ) -> Result<Option<(String, Option<String>)>, StorageError> {
        let mut r = self.remembered(kind).await?;
        Ok(r.model.map(|model| {
            let effort = r.efforts.remove(&model);
            (model, effort)
        }))
    }

    /// Everything `kind` remembers: its model, and every model's effort.
    pub(super) async fn remembered(&self, kind: &str) -> Result<Remembered, StorageError> {
        let model: Option<String> =
            sqlx::query_scalar("SELECT model FROM harness_preference WHERE harness_kind = ?")
                .bind(kind)
                .fetch_optional(self.reader())
                .await?;
        let efforts: Vec<(String, String)> =
            sqlx::query_as("SELECT model, effort FROM harness_model_effort WHERE harness_kind = ?")
                .bind(kind)
                .fetch_all(self.reader())
                .await?;
        Ok(Remembered {
            model,
            efforts: efforts.into_iter().collect(),
        })
    }

    /// Test-support: remember settings without starting a turn.
    #[cfg(feature = "test-support")]
    pub async fn remember_for_test(&self, kind: &str, model: &str, effort: Option<&str>) {
        let (kind, ts) = (kind.to_string(), crate::db::now());
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
