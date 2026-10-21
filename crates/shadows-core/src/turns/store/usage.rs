//! Atomically record one terminal AYLUL invocation's measured counters.
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
    let preset: Option<String> = row.get("usage_preset");
    let cost = usage.and_then(|u| u.cost.as_ref()).filter(|c| {
        preset.as_deref() == Some("anthropic")
            && c.currency == "USD"
            && c.amount.is_finite()
            && c.amount >= 0.0
    });
    let number = |value: Option<u64>| -> Result<Option<i64>, StorageError> {
        value
            .map(i64::try_from)
            .transpose()
            .map_err(|_| StorageError::Unavailable("usage count exceeds storage capacity".into()))
    };
    sqlx::query(
        "INSERT INTO aylul_usage
        (operation_id,thread_id,agent_slug,provider_slug,provider_name,label,preset,model,
         input_tokens,output_tokens,cached_read_tokens,cached_write_tokens,total_tokens,
         cost,currency,recorded_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(op.as_str())
    .bind(row.get::<String, _>("thread_id"))
    .bind(row.get::<String, _>("usage_agent_slug"))
    .bind(row.get::<String, _>("usage_provider_slug"))
    .bind(row.get::<String, _>("usage_provider_name"))
    .bind(row.get::<Option<String>, _>("usage_label"))
    .bind(preset)
    .bind(row.get::<String, _>("requested_model"))
    .bind(number(usage.map(|u| u.input_tokens))?)
    .bind(number(usage.map(|u| u.output_tokens))?)
    .bind(number(usage.map(|u| u.cached_read_tokens))?)
    .bind(number(usage.map(|u| u.cached_write_tokens))?)
    .bind(number(usage.map(|u| u.total_tokens))?)
    .bind(cost.map(|c| c.amount))
    .bind(cost.map(|c| c.currency.as_str()))
    .bind(ts)
    .execute(conn)
    .await?;
    Ok(())
}
