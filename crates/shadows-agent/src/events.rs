mod limits;
mod subagent;
mod tool;
mod usage;

pub use limits::{AccountLimits, LimitWindow};
pub use subagent::{AgentFacts, SubagentCard, ToolMeta};
pub use tool::{
    CallKind, CallStatus, DiffHunk, DiffLine, DiffOp, TodoItem, TodoListCard, TodoStatus,
    ToolCallCard, ToolDiff, ToolFacts, ToolLive, ToolLocation,
};
pub use usage::{TurnObservation, TurnUsage, UsageCost};

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
        /// Claude's name for the tool (`_meta.claudeCode.toolName`): `Read`,
        /// `Agent`, `SubagentHandback` …
        tool: Option<String>,
        /// The Agent call this call is a step of (`parentToolUseId`).
        parent: Option<String>,
        /// What an Agent call told of its subagent; `None` for any
        /// other tool.
        agent: Option<Box<AgentFacts>>,
        facts: ToolFacts,
    },
    /// A subagent's card changed while its turn runs. Sent by the
    /// Planner's watcher, not the connection, as `TurnEnd` is.
    Subagent(SubagentCard),
    Tool(ToolLive),
    Todos(Vec<TodoItem>),
    PermissionAsked {
        request: String,
        title: String,
        options: Vec<agent_client_protocol::schema::v1::PermissionOption>,
        asked_at: u128,
    },
    PermissionAnswered {
        request: String,
        option: String,
        kind: String,
        name: String,
    },
    PermissionRefused {
        title: String,
        request: Option<String>,
        reason: Option<String>,
    },
    Usage {
        used: u64,
        size: u64,
        model: Option<String>,
        rate_limit: Option<Value>,
        cost: Option<UsageCost>,
    },
    Options(Value),
    /// The harness's complete `/` list, sent after the session opens
    /// and whenever it changes, between turns too.
    Commands(Vec<SlashCommand>),
    /// The harness named its session: a `session_info_update` carrying a
    /// `title`. Sent between turns too, when nobody reads the
    /// thread's events: the adapter generates it after the turn has answered.
    SessionTitle {
        title: String,
    },
    /// The prompt has answered and the turn's entries are durable. Sent by the
    /// Planner's watcher, not the connection: ACP ends a turn with the prompt's
    /// response, which no notification carries.
    TurnEnd {
        subtype: &'static str,
        stop_reason: Option<String>,
    },
}

/// One entry of the harness's `/` list: a skill, a plugin
/// command or a built-in command, with no field saying which.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    /// What the command takes after its name, e.g. `[topic]`.
    pub hint: Option<String>,
}
