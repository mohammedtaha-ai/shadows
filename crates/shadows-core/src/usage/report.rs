//! Grouping historical measurements by provider and model, without mutable-provider joins.
use super::Usage;
use super::model::{ModelUsage, ProviderUsage, UsageReport, UsageRow};
use crate::db::StorageError;
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

impl Usage {
    /// Keeps the rows inside `[from, to)` exactly and groups them; `rows` are in
    /// recorded order.
    pub(super) fn aggregate(
        rows: Vec<UsageRow>,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<UsageReport, StorageError> {
        let mut providers: BTreeMap<
            String,
            (Totals, String, Option<String>, OffsetDateTime, String),
        > = BTreeMap::new();
        let mut models: BTreeMap<(String, String), Totals> = BTreeMap::new();
        for row in rows {
            let when = OffsetDateTime::parse(&row.recorded_at, &Rfc3339)
                .map_err(|_| StorageError::Unavailable("invalid usage timestamp".into()))?;
            if when < from || when >= to {
                continue;
            }
            let number = |value: Option<i64>| -> Result<u64, StorageError> {
                u64::try_from(value.unwrap_or(0))
                    .map_err(|_| StorageError::Unavailable("invalid usage count".into()))
            };
            let input = number(row.input_tokens)?
                .checked_add(number(row.cached_read_tokens)?)
                .and_then(|n| n.checked_add(number(row.cached_write_tokens).ok()?))
                .ok_or_else(|| StorageError::Unavailable("usage count overflow".into()))?;
            let output = number(row.output_tokens)?;
            let cost = row.cost.and_then(|v| {
                Self::trusted_cost(row.preset.as_deref(), row.currency.as_deref(), v)
            });
            let UsageRow {
                provider_slug: slug,
                operation_id: op,
                provider_name: name,
                label,
                model,
                ..
            } = row;
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
}
