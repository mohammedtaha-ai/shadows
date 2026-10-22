//! Per-harness model and per-model effort persistence.
use super::HarnessStore;
use crate::db::StorageError;
use shadows_agent::TurnSettings;
use sqlx::SqliteConnection;
use std::collections::HashMap;
/// Remembers the model of the turn being started for its harness, and its
/// effort for that model; a turn without an effort leaves the model's
/// remembered effort as it was. The mode is never remembered.
impl HarnessStore {
    pub(in crate::harness) async fn remember_settings(
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
}

/// What a harness remembers: the model of the last turn started on
/// it, and per model the effort of the last turn started on that model.
#[derive(Debug, Default)]
pub(in crate::harness) struct Remembered {
    pub(in crate::harness) model: Option<String>,
    pub(in crate::harness) efforts: HashMap<String, String>,
}

impl HarnessStore {
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
    pub(in crate::harness) async fn remembered(
        &self,
        kind: &str,
    ) -> Result<Remembered, StorageError> {
        let model: Option<String> =
            sqlx::query_scalar("SELECT model FROM harness_preference WHERE harness_kind = ?")
                .bind(kind)
                .fetch_optional(self.storage.reader())
                .await?;
        let efforts: Vec<(String, String)> =
            sqlx::query_as("SELECT model, effort FROM harness_model_effort WHERE harness_kind = ?")
                .bind(kind)
                .fetch_all(self.storage.reader())
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
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move { Self::remember_settings(conn, &kind, &settings, &ts).await })
            })
            .await
            .expect("remember_for_test");
    }
}
