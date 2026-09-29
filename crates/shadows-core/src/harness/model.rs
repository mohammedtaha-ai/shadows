//! One job: the harness shapes a caller meets (spec §12.4, §12.8, §14.5).
//!
//! Moved from `shadows-http` with the same names, fields and derives, so
//! `api/openapi.json` does not change. `Harness` builds them; an adapter only
//! serializes them.

use shadows_agent::breakdown::Category;
use shadows_agent::events::AccountLimits;
use shadows_agent::policy;

/// The model and effort last chosen for this harness (spec §12.4).
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct RememberedSettings {
    pub(super) model: String,
    #[schema(required)]
    pub(super) effort: Option<String>,
}

/// A CLI a conversation can run on (spec §12.1). `kind` is `claude-code` or
/// `codex`.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HarnessInfo {
    pub(super) kind: String,
    pub(super) label: String,
    pub(super) available: bool,
    /// Why it cannot run, when `available` is false.
    #[schema(required)]
    pub(super) reason: Option<String>,
    #[schema(required)]
    pub(super) remembered: Option<RememberedSettings>,
    #[schema(required)]
    pub(super) limits: Option<AccountLimits>,
}

pub(super) fn label(kind: &str) -> &'static str {
    match kind {
        policy::CLAUDE_CODE => "Claude Code",
        policy::CODEX => "Codex",
        _ => "Unknown",
    }
}

/// The context breakdown read on demand (spec §12.8): the categories, or none
/// with the reason.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ContextBreakdown {
    #[schema(required)]
    pub(super) categories: Option<Vec<Category>>,
    #[schema(required)]
    pub(super) reason: Option<String>,
}
