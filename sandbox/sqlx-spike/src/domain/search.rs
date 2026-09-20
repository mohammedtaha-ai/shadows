//! Search capability — domain-facing.
//!
//! Note: FTS5 / tsvector syntax NEVER appears here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchScope {
    Research,
    Operation,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub scope: SearchScope,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    /// Stable identifier (e.g. ResearchId, OperationId, EventId).
    pub ref_id: String,
    pub scope: SearchScope,
    pub snippet: String,
}
