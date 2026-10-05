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
        /// The Agent call this call is a step of (`parentToolUseId`, §22.1).
        parent: Option<String>,
        /// What an Agent call told of its subagent (§22.1); `None` for any
        /// other tool.
        agent: Option<Box<AgentFacts>>,
    },
    /// A subagent's card changed while its turn runs (§22.2). Sent by the
    /// Planner's watcher, not the connection, as `TurnEnd` is.
    Subagent(SubagentCard),
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
    /// The harness's complete `/` list (§21), sent after the session opens
    /// and whenever it changes, between turns too.
    Commands(Vec<SlashCommand>),
    /// The harness named its session: a `session_info_update` carrying a
    /// `title` (§12.3). Sent between turns too, when nobody reads the
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

/// One entry of the harness's `/` list (spec §21.1): a skill, a plugin
/// command or a built-in command, with no field saying which.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    /// What the command takes after its name, e.g. `[topic]`.
    pub hint: Option<String>,
}

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

/// Reads a tool call's `_meta` and `rawInput`. An Agent (or `Task`) call, or
/// one marked `subagent`, carries [`AgentFacts`]; any other carries none.
pub fn tool_meta(
    meta: Option<&serde_json::Map<String, Value>>,
    raw_input: Option<&Value>,
) -> ToolMeta {
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

/// One subagent as the conversation shows it (spec §22.2): written once as
/// the body of an entry, `[subagent: <this as JSON>]`, and sent whole on
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
    #[schema(required)]
    pub five_hour: Option<LimitWindow>,
    #[schema(required)]
    pub seven_day: Option<LimitWindow>,
    /// When the daemon received the report (RFC 3339).
    pub observed_at: String,
}

/// Reads Claude's rate-limit report (`_meta["_claude/rateLimit"]` on a
/// `usage_update`): `unifiedWindows.five_hour` and `.seven_day`, each
/// `{ utilization, resetsAt }`. `None` when neither window is present, so a
/// report without figures is no limits rather than zero.
pub fn limits_from(rate_limit: &Value, observed_at: &str) -> Option<AccountLimits> {
    let windows = rate_limit.get("unifiedWindows")?;
    let window = |name: &str| {
        let w = windows.get(name)?;
        Some(LimitWindow {
            utilization: w.get("utilization")?.as_f64()?,
            resets_at: w.get("resetsAt")?.as_i64()?,
        })
    };
    let (five_hour, seven_day) = (window("five_hour"), window("seven_day"));
    if five_hour.is_none() && seven_day.is_none() {
        return None;
    }
    Some(AccountLimits {
        five_hour,
        seven_day,
        observed_at: observed_at.to_string(),
    })
}

/// What the harness reported about a turn, from its last usage report
/// (§12.8: only the last is trusted — the first `size` is a guess). Nothing
/// it did not report is estimated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnObservation {
    /// The harness session the turn ran in (§12.7 `native_session_id`).
    pub native_session_id: Option<String>,
    pub observed_model: Option<String>,
    pub context_used: Option<u64>,
    pub context_window: Option<u64>,
}

impl TurnObservation {
    /// From the turn's last `Usage`, or nothing when it reported none. A
    /// `size` of 0 is no usable window.
    pub fn from_usage(last: Option<&HarnessEvent>) -> Self {
        match last {
            Some(HarnessEvent::Usage {
                used, size, model, ..
            }) => Self {
                native_session_id: None,
                observed_model: model.clone(),
                context_used: Some(*used),
                context_window: (*size > 0).then_some(*size),
            },
            _ => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn limits_read_both_windows_with_reset_times() {
        let v = json!({ "unifiedWindows": {
            "five_hour": { "utilization": 0.75, "resetsAt": 1790212200 },
            "seven_day": { "utilization": 0.91, "resetsAt": 1790542800 } } });
        let l = limits_from(&v, "2026-09-24T10:00:00Z").unwrap();
        assert_eq!(l.five_hour.unwrap().resets_at, 1790212200);
        assert!((l.seven_day.unwrap().utilization - 0.91).abs() < 1e-9);
        assert_eq!(l.observed_at, "2026-09-24T10:00:00Z");
    }

    #[test]
    fn an_agent_call_carries_its_input_and_numbers_and_a_step_its_parent() {
        let meta = json!({ "claudeCode": { "toolName": "Agent", "toolResponse": {
            "resolvedModel": "claude-sonnet-5-5", "totalDurationMs": 13665,
            "totalTokens": 43119, "totalToolUseCount": 4,
            "handbackReport": { "text": "two files" } } } });
        let input = json!({ "description": "List files", "subagent_type": "general-purpose",
            "model": "sonnet", "prompt": "List them" });
        let m = tool_meta(meta.as_object(), Some(&input));
        let a = m.agent.unwrap();
        assert_eq!(m.tool.as_deref(), Some("Agent"));
        assert_eq!(a.agent_type.as_deref(), Some("general-purpose"));
        assert_eq!((a.tokens, a.duration_ms), (Some(43119), Some(13665)));
        assert_eq!(a.report.as_deref(), Some("two files"));

        let step = json!({ "claudeCode": { "toolName": "Read", "parentToolUseId": "t1" } });
        let s = tool_meta(step.as_object(), Some(&json!({ "description": "x" })));
        assert_eq!((s.parent.as_deref(), s.agent), (Some("t1"), None));
    }

    #[test]
    fn a_report_without_windows_is_no_limits_rather_than_zero() {
        assert!(limits_from(&json!({ "status": "allowed" }), "t").is_none());
        assert!(limits_from(&json!({ "unifiedWindows": {} }), "t").is_none());
    }

    #[test]
    fn an_observation_without_a_size_reports_no_window() {
        let u = HarnessEvent::Usage {
            used: 10,
            size: 0,
            model: None,
            rate_limit: None,
        };
        let o = TurnObservation::from_usage(Some(&u));
        assert_eq!((o.context_used, o.context_window), (Some(10), None));
        assert_eq!(TurnObservation::from_usage(None).context_used, None);
    }
}
