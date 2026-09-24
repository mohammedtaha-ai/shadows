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
}
