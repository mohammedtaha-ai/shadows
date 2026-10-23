//! Atomically record one terminal AYLUL invocation's measured counters.
use crate::usage::{Usage, UsageRecord};
use crate::{OperationId, db::StorageError};
use shadows_agent::events::TurnUsage;
use sqlx::SqliteConnection;

impl super::TurnsStore {
    pub(crate) async fn record_usage_in(
        conn: &mut SqliteConnection,
        op: &OperationId,
        usage: Option<&TurnUsage>,
        ts: &str,
    ) -> Result<(), StorageError> {
        let Some(thread_id) = crate::operations::Operations::started_thread_in(conn, op).await?
        else {
            return Ok(());
        };
        let row = crate::harness::Harness::invocation_usage_in(conn, op.as_str()).await?;
        let Some(row) = row else {
            return Ok(());
        };
        let record = UsageRecord {
            operation_id: op.as_str().to_owned(),
            thread_id,
            agent_slug: row.agent_slug,
            provider_slug: row.provider_slug,
            provider_name: row.provider_name,
            label: row.label,
            preset: row.preset,
            model: row.model,
            input_tokens: usage.map(|u| u.input_tokens),
            output_tokens: usage.map(|u| u.output_tokens),
            cached_read_tokens: usage.map(|u| u.cached_read_tokens),
            cached_write_tokens: usage.map(|u| u.cached_write_tokens),
            total_tokens: usage.map(|u| u.total_tokens),
            cost: usage
                .and_then(|u| u.cost.as_ref())
                .map(|c| (c.amount, c.currency.clone())),
        };
        Usage::record_in(conn, record, ts).await
    }
}
