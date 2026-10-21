//! Period validation and usage report shapes.
use super::{Profiles, usage_store};
use crate::CoreError;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

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
impl Profiles {
    pub async fn usage(&self, from: &str, to: &str) -> Result<UsageReport, CoreError> {
        let invalid = || CoreError::ProfileInvalid("invalid usage period".into());
        let from = OffsetDateTime::parse(from, &Rfc3339).map_err(|_| invalid())?;
        let to = OffsetDateTime::parse(to, &Rfc3339).map_err(|_| invalid())?;
        if from >= to {
            return Err(invalid());
        }
        Ok(usage_store::report(&self.storage, from, to).await?)
    }
}
