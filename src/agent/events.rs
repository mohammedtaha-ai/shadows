use serde_json::Value;

/// Transient reports from one ACP connection.
#[derive(Debug, Clone)]
pub enum HarnessEvent {
    Chunk {
        message_id: Option<String>,
        text: String,
    },
    ToolCall {
        id: String,
        title: Option<String>,
        status: Option<String>,
    },
    PermissionRefused {
        title: String,
    },
    Usage {
        used: u64,
        size: u64,
        model: Option<String>,
        rate_limit: Option<Value>,
    },
    Options(Value),
    /// The prompt has answered and the turn's entries are durable. Sent by the
    /// Planner's watcher, not the connection: ACP ends a turn with the prompt's
    /// response, which no notification carries.
    TurnEnd {
        subtype: &'static str,
        stop_reason: Option<String>,
    },
}

/// One account limit window as the harness reported it: `utilization` from 0
/// to 1, `resets_at` in Unix seconds.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LimitWindow {
    pub utilization: f64,
    pub resets_at: i64,
}

/// The account's limits as last reported by the harness (spec §12.8). A
/// window the harness did not report is `None`, never estimated.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AccountLimits {
    pub five_hour: Option<LimitWindow>,
    pub seven_day: Option<LimitWindow>,
    /// When the daemon received the report (RFC 3339).
    pub observed_at: String,
}
