//! SQL for measured usage rows.
use super::model::{UsageRecord, UsageRow};
use crate::db::{Storage, StorageError};
use sqlx::{Row, SqliteConnection};
use std::sync::Arc;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// The service's SQL, over the shared storage handle.
pub(super) struct UsageStore {
    storage: Arc<Storage>,
}

impl UsageStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// The rows recorded within a second of the period, in recorded order. SQLite
    /// narrows the candidates; the report keeps exact RFC3339 fractional endpoints.
    pub(super) async fn candidates(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Vec<UsageRow>, StorageError> {
        let lower = (from.checked_sub(time::Duration::seconds(1)).unwrap_or(from))
            .format(&Rfc3339)
            .map_err(|_| StorageError::Unavailable("usage period".into()))?;
        let upper = (to.checked_add(time::Duration::seconds(1)).unwrap_or(to))
            .format(&Rfc3339)
            .map_err(|_| StorageError::Unavailable("usage period".into()))?;
        let rows = sqlx::query(
            "SELECT * FROM aylul_usage
            WHERE julianday(recorded_at)>=julianday(?) AND julianday(recorded_at)<=julianday(?)
            ORDER BY recorded_at,operation_id",
        )
        .bind(lower)
        .bind(upper)
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| UsageRow {
                recorded_at: row.get("recorded_at"),
                operation_id: row.get("operation_id"),
                provider_slug: row.get("provider_slug"),
                provider_name: row.get("provider_name"),
                label: row.get("label"),
                preset: row.get("preset"),
                model: row.get("model"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                cached_read_tokens: row.get("cached_read_tokens"),
                cached_write_tokens: row.get("cached_write_tokens"),
                cost: row.get("cost"),
                currency: row.get("currency"),
            })
            .collect())
    }

    pub(super) async fn insert_in(
        conn: &mut SqliteConnection,
        record: &UsageRecord,
        ts: &str,
    ) -> Result<(), StorageError> {
        let number = |value: Option<u64>| -> Result<Option<i64>, StorageError> {
            value.map(i64::try_from).transpose().map_err(|_| {
                StorageError::Unavailable("usage count exceeds storage capacity".into())
            })
        };
        sqlx::query(
            "INSERT INTO aylul_usage
            (operation_id,thread_id,agent_slug,provider_slug,provider_name,label,preset,model,
             input_tokens,output_tokens,cached_read_tokens,cached_write_tokens,total_tokens,
             cost,currency,recorded_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&record.operation_id)
        .bind(&record.thread_id)
        .bind(&record.agent_slug)
        .bind(&record.provider_slug)
        .bind(&record.provider_name)
        .bind(&record.label)
        .bind(&record.preset)
        .bind(&record.model)
        .bind(number(record.input_tokens)?)
        .bind(number(record.output_tokens)?)
        .bind(number(record.cached_read_tokens)?)
        .bind(number(record.cached_write_tokens)?)
        .bind(number(record.total_tokens)?)
        .bind(record.cost.as_ref().map(|c| c.0))
        .bind(record.cost.as_ref().map(|c| c.1.as_str()))
        .bind(ts)
        .execute(conn)
        .await?;
        Ok(())
    }
}
