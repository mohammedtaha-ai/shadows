//! Atomically record one terminal AYLUL invocation's measured counters.
use crate::usage::{Usage, UsageRecord};
use crate::{OperationId, db::StorageError};
use shadows_agent::events::TurnUsage;
use sqlx::{Row, SqliteConnection};

pub(crate) async fn record_usage_in(
    conn: &mut SqliteConnection,
    op: &OperationId,
    usage: Option<&TurnUsage>,
    ts: &str,
) -> Result<(), StorageError> {
    let row = sqlx::query(
        "SELECT i.*, o.thread_id FROM agent_invocation i
        JOIN operation o ON o.id=i.operation_id WHERE i.operation_id=?
        AND i.harness_kind='aylul' AND i.usage_provider_slug IS NOT NULL
        AND o.started_at IS NOT NULL",
    )
    .bind(op.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else {
        return Ok(());
    };
    let record = UsageRecord {
        operation_id: op.as_str().to_owned(),
        thread_id: row.get("thread_id"),
        agent_slug: row.get("usage_agent_slug"),
        provider_slug: row.get("usage_provider_slug"),
        provider_name: row.get("usage_provider_name"),
        label: row.get("usage_label"),
        preset: row.get("usage_preset"),
        model: row.get("requested_model"),
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
