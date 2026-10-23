//! Subagent tool metadata and conversation cards.
use serde_json::Value;

/// What an Agent call's updates told of its subagent (spec §22.1), each
/// `None` until an update carries it: its input, then the numbers Claude's
/// `PostToolUse` hook reports after the call has ended.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentFacts {
    pub description: Option<String>,
    pub agent_type: Option<String>,
    /// The model the call asked for, e.g. `sonnet`.
    pub model: Option<String>,
    pub prompt: Option<String>,
    /// The model it ran on, e.g. `claude-sonnet-5-5`.
    pub resolved_model: Option<String>,
    pub duration_ms: Option<u64>,
    pub tokens: Option<u64>,
    pub tool_count: Option<u64>,
    pub report: Option<String>,
}

/// What `_meta.claudeCode` and `rawInput` say about one tool call (§22.1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolMeta {
    pub tool: Option<String>,
    pub parent: Option<String>,
    pub agent: Option<Box<AgentFacts>>,
}

impl ToolMeta {
    /// Reads a tool call's `_meta` and `rawInput`. An Agent (or `Task`) call, or
    /// one marked `subagent`, carries [`AgentFacts`]; any other carries none.
    pub fn from_update(
        meta: Option<&serde_json::Map<String, Value>>,
        raw_input: Option<&Value>,
    ) -> Self {
        let claude = meta.and_then(|m| m.get("claudeCode"));
        let text = |v: Option<&Value>, key: &str| {
            v.and_then(|v| v.get(key))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let tool = text(claude, "toolName");
        let is_agent = matches!(tool.as_deref(), Some("Agent" | "Task"))
            || claude
                .and_then(|c| c.get("subagent"))
                .and_then(Value::as_bool)
                == Some(true);
        let agent = is_agent.then(|| {
            let response = claude.and_then(|c| c.get("toolResponse"));
            let number = |key: &str| response.and_then(|r| r.get(key)).and_then(Value::as_u64);
            Box::new(AgentFacts {
                description: text(raw_input, "description"),
                agent_type: text(raw_input, "subagent_type"),
                model: text(raw_input, "model"),
                prompt: text(raw_input, "prompt"),
                resolved_model: text(response, "resolvedModel"),
                duration_ms: number("totalDurationMs"),
                tokens: number("totalTokens"),
                tool_count: number("totalToolUseCount"),
                report: text(response.and_then(|r| r.get("handbackReport")), "text"),
            })
        });
        ToolMeta {
            tool,
            parent: text(claude, "parentToolUseId"),
            agent,
        }
    }
}

/// One subagent as the conversation shows it (spec §22.2): written once as
/// a `Subagent` entry carrying this as its `card` (§23.8), and sent whole on
/// every change while it runs.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubagentCard {
    /// The Agent call's id.
    pub id: String,
    pub title: String,
    pub agent_type: Option<String>,
    /// The model it ran on, or else the one it asked for.
    pub model: Option<String>,
    /// `running`, `completed`, `failed` or `stopped`.
    pub status: String,
    pub prompt: Option<String>,
    /// The titles of its own tool calls, in the order they ended.
    pub steps: Vec<String>,
    pub report: Option<String>,
    pub duration_ms: Option<u64>,
    pub tokens: Option<u64>,
    pub tool_count: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn an_agent_call_carries_its_input_and_numbers_and_a_step_its_parent() {
        let meta = json!({ "claudeCode": { "toolName": "Agent", "toolResponse": {
            "resolvedModel": "claude-sonnet-5-5", "totalDurationMs": 13665,
            "totalTokens": 43119, "totalToolUseCount": 4,
            "handbackReport": { "text": "two files" } } } });
        let input = json!({ "description": "List files", "subagent_type": "general-purpose",
            "model": "sonnet", "prompt": "List them" });
        let m = ToolMeta::from_update(meta.as_object(), Some(&input));
        let a = m.agent.unwrap();
        assert_eq!(m.tool.as_deref(), Some("Agent"));
        assert_eq!(a.agent_type.as_deref(), Some("general-purpose"));
        assert_eq!((a.tokens, a.duration_ms), (Some(43119), Some(13665)));
        assert_eq!(a.report.as_deref(), Some("two files"));

        let step = json!({ "claudeCode": { "toolName": "Read", "parentToolUseId": "t1" } });
        let s = ToolMeta::from_update(step.as_object(), Some(&json!({ "description": "x" })));
        assert_eq!((s.parent.as_deref(), s.agent), (Some("t1"), None));
    }
}
