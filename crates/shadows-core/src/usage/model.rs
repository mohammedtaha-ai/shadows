//! Usage report shapes, and the measurements one invocation records.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UsageReport {
    pub by_provider: Vec<ProviderUsage>,
    pub by_model: Vec<ModelUsage>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProviderUsage {
    pub provider_slug: String,
    pub provider_name: String,
    pub label: Option<String>,
    pub requests: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ModelUsage {
    pub model: String,
    pub provider_slug: String,
    pub requests: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost: Option<f64>,
    pub currency: Option<String>,
}

/// One finished invocation's counters, with the provider metadata it ran under.
pub(crate) struct UsageRecord {
    pub(crate) operation_id: String,
    pub(crate) thread_id: String,
    pub(crate) agent_slug: String,
    pub(crate) provider_slug: String,
    pub(crate) provider_name: String,
    pub(crate) label: Option<String>,
    pub(crate) preset: Option<String>,
    pub(crate) model: String,
    pub(crate) input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) cached_read_tokens: Option<u64>,
    pub(crate) cached_write_tokens: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
    /// The reported amount and its currency, before the pricing rule.
    pub(crate) cost: Option<(f64, String)>,
}

/// One stored measurement, as a report reads it.
pub(super) struct UsageRow {
    pub(super) recorded_at: String,
    pub(super) operation_id: String,
    pub(super) provider_slug: String,
    pub(super) provider_name: String,
    pub(super) label: Option<String>,
    pub(super) preset: Option<String>,
    pub(super) model: String,
    pub(super) input_tokens: Option<i64>,
    pub(super) output_tokens: Option<i64>,
    pub(super) cached_read_tokens: Option<i64>,
    pub(super) cached_write_tokens: Option<i64>,
    pub(super) cost: Option<f64>,
    pub(super) currency: Option<String>,
}
