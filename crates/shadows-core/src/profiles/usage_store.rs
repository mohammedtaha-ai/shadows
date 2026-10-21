//! Historical usage aggregation without mutable-provider joins.
use super::usage::{ModelUsage, ProviderUsage, UsageReport};
use crate::db::{Storage, StorageError};
use sqlx::Row;
use std::collections::BTreeMap;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Default)]
struct Totals {
    requests: u64,
    input: u64,
    output: u64,
    cost: Option<f64>,
    complete: bool,
}
impl Totals {
    fn add(&mut self, input: u64, output: u64, cost: Option<f64>) -> Result<(), StorageError> {
        let overflow = || StorageError::Unavailable("usage total exceeds report capacity".into());
        if self.requests == 0 {
            self.complete = true;
        }
        self.requests = self.requests.checked_add(1).ok_or_else(overflow)?;
        self.input = self.input.checked_add(input).ok_or_else(overflow)?;
        self.output = self.output.checked_add(output).ok_or_else(overflow)?;
        self.complete &= cost.is_some();
        self.cost = if self.complete {
            Some(self.cost.unwrap_or(0.0) + cost.unwrap_or(0.0))
        } else {
            None
        };
        if self.cost.is_some_and(|v| !v.is_finite()) {
            self.cost = None;
            self.complete = false;
        }
        Ok(())
    }
}

pub(super) async fn report(
    storage: &Storage,
    from: OffsetDateTime,
    to: OffsetDateTime,
) -> Result<UsageReport, StorageError> {
    // SQLite narrows candidates; Rust keeps exact RFC3339 fractional endpoint semantics.
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
    .fetch_all(storage.reader())
    .await?;
    let mut providers: BTreeMap<String, (Totals, String, Option<String>, OffsetDateTime, String)> =
        BTreeMap::new();
    let mut models: BTreeMap<(String, String), Totals> = BTreeMap::new();
    for row in rows {
        let when: String = row.get("recorded_at");
        let when = OffsetDateTime::parse(&when, &Rfc3339)
            .map_err(|_| StorageError::Unavailable("invalid usage timestamp".into()))?;
        if when < from || when >= to {
            continue;
        }
        let number = |column| -> Result<u64, StorageError> {
            let value: Option<i64> = row.get(column);
            u64::try_from(value.unwrap_or(0))
                .map_err(|_| StorageError::Unavailable("invalid usage count".into()))
        };
        let input = number("input_tokens")?
            .checked_add(number("cached_read_tokens")?)
            .and_then(|n| n.checked_add(number("cached_write_tokens").ok()?))
            .ok_or_else(|| StorageError::Unavailable("usage count overflow".into()))?;
        let output = number("output_tokens")?;
        let preset: Option<String> = row.get("preset");
        let currency: Option<String> = row.get("currency");
        let cost: Option<f64> = row.get("cost");
        let cost = cost.filter(|v| {
            preset.as_deref() == Some("anthropic")
                && currency.as_deref() == Some("USD")
                && v.is_finite()
                && *v >= 0.0
        });
        let slug: String = row.get("provider_slug");
        let op: String = row.get("operation_id");
        let name: String = row.get("provider_name");
        let label: Option<String> = row.get("label");
        let entry = providers.entry(slug.clone()).or_insert_with(|| {
            (
                Totals::default(),
                name.clone(),
                label.clone(),
                when,
                op.clone(),
            )
        });
        entry.0.add(input, output, cost)?;
        if (when, &op) > (entry.3, &entry.4) {
            *entry = (std::mem::take(&mut entry.0), name, label, when, op);
        }
        let model: String = row.get("model");
        models
            .entry((slug, model))
            .or_default()
            .add(input, output, cost)?;
    }
    let mut by_provider: Vec<_> = providers
        .into_iter()
        .map(|(slug, (t, name, label, _, _))| ProviderUsage {
            provider_slug: slug,
            provider_name: name,
            label,
            requests: t.requests,
            tokens_in: t.input,
            tokens_out: t.output,
            cost: t.cost,
            currency: t.cost.map(|_| "USD".into()),
        })
        .collect();
    let mut by_model: Vec<_> = models
        .into_iter()
        .map(|((slug, model), t)| ModelUsage {
            provider_slug: slug,
            model,
            requests: t.requests,
            tokens_in: t.input,
            tokens_out: t.output,
            cost: t.cost,
            currency: t.cost.map(|_| "USD".into()),
        })
        .collect();
    by_provider.sort_by(|a, b| {
        (u128::from(b.tokens_in) + u128::from(b.tokens_out))
            .cmp(&(u128::from(a.tokens_in) + u128::from(a.tokens_out)))
            .then_with(|| a.provider_slug.cmp(&b.provider_slug))
    });
    by_model.sort_by(|a, b| {
        (u128::from(b.tokens_in) + u128::from(b.tokens_out))
            .cmp(&(u128::from(a.tokens_in) + u128::from(a.tokens_out)))
            .then_with(|| (&a.provider_slug, &a.model).cmp(&(&b.provider_slug, &b.model)))
    });
    Ok(UsageReport {
        by_provider,
        by_model,
    })
}
