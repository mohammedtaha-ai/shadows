//! Measured AYLUL usage: what each finished invocation consumed, and reports over a period.
mod model;
mod report;
mod store;

pub(crate) use model::UsageRecord;
pub use model::{ModelUsage, ProviderUsage, UsageReport};

use crate::CoreError;
use crate::db::{Storage, StorageError};
use sqlx::SqliteConnection;
use std::sync::Arc;
use store::UsageStore;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub struct Usage {
    store: UsageStore,
}

impl Usage {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self {
            store: UsageStore::new(storage),
        }
    }

    /// Aggregates the measurements recorded in the half-open period `[from, to)`.
    pub async fn usage(&self, from: &str, to: &str) -> Result<UsageReport, CoreError> {
        let invalid = || CoreError::ProfileInvalid("invalid usage period".into());
        let from = OffsetDateTime::parse(from, &Rfc3339).map_err(|_| invalid())?;
        let to = OffsetDateTime::parse(to, &Rfc3339).map_err(|_| invalid())?;
        if from >= to {
            return Err(invalid());
        }
        let rows = self.store.candidates(from, to).await?;
        Ok(Self::aggregate(rows, from, to)?)
    }

    /// Records one finished invocation's counters inside the caller's write. Its cost is
    /// kept only when trusted.
    pub(crate) async fn record_in(
        conn: &mut SqliteConnection,
        mut record: UsageRecord,
        ts: &str,
    ) -> Result<(), StorageError> {
        record.cost = record.cost.filter(|(amount, currency)| {
            Self::trusted_cost(record.preset.as_deref(), Some(currency), *amount).is_some()
        });
        UsageStore::insert_in(conn, &record, ts).await
    }

    /// The one pricing rule: only Anthropic's finite, non-negative USD cost is trusted.
    fn trusted_cost(preset: Option<&str>, currency: Option<&str>, amount: f64) -> Option<f64> {
        (preset == Some("anthropic")
            && currency == Some("USD")
            && amount.is_finite()
            && amount >= 0.0)
            .then_some(amount)
    }
}
