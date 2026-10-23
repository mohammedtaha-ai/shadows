//! Ordered ACP notifications translated to harness events.
use crate::events::{HarnessEvent, SlashCommand, ToolMeta};
use agent_client_protocol::schema::{
    MaybeUndefined,
    v1::{AvailableCommandInput, ContentBlock, SessionUpdate},
};
use serde_json::Value;

pub(super) struct Mapping;

impl Mapping {
    pub(super) fn forward(events: &impl Fn(HarnessEvent), update: SessionUpdate) {
        let item = match update {
            SessionUpdate::AgentMessageChunk(c) => match c.content {
                ContentBlock::Text(t) => Some(HarnessEvent::Chunk {
                    message_id: c.message_id.map(|id| id.to_string()),
                    text: t.text,
                }),
                _ => None,
            },
            SessionUpdate::ToolCall(t) => {
                let m = ToolMeta::from_update(t.meta.as_ref(), t.raw_input.as_ref());
                Some(HarnessEvent::ToolCall {
                    id: t.tool_call_id.to_string(),
                    title: Some(t.title),
                    status: Some(format!("{:?}", t.status).to_lowercase()),
                    tool: m.tool.or(t.name),
                    parent: m.parent,
                    agent: m.agent,
                })
            }
            SessionUpdate::ToolCallUpdate(t) => {
                let m = ToolMeta::from_update(t.meta.as_ref(), t.fields.raw_input.as_ref());
                Some(HarnessEvent::ToolCall {
                    id: t.tool_call_id.to_string(),
                    title: t.fields.title,
                    status: t.fields.status.map(|s| format!("{s:?}").to_lowercase()),
                    tool: m.tool.or(t.fields.name),
                    parent: m.parent,
                    agent: m.agent,
                })
            }
            SessionUpdate::UsageUpdate(u) => {
                let meta = serde_json::to_value(u.meta).unwrap_or(Value::Null);
                Some(HarnessEvent::Usage {
                    cost: serde_json::to_value(u.cost)
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok()),
                    used: u.used,
                    size: u.size,
                    model: meta
                        .get("_claude/model")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    rate_limit: meta.get("_claude/rateLimit").cloned(),
                })
            }
            SessionUpdate::ConfigOptionUpdate(o) => Some(HarnessEvent::Options(
                serde_json::to_value(o.config_options).unwrap_or(Value::Null),
            )),
            // A title, or only `_meta` (a goal, a file-change report): the rest is
            // not Shadows'. A `null` title clears the harness's own; Shadows keeps
            // its thread's.
            SessionUpdate::SessionInfoUpdate(i) => match i.title {
                MaybeUndefined::Value(title) => Some(HarnessEvent::SessionTitle { title }),
                _ => None,
            },
            SessionUpdate::AvailableCommandsUpdate(u) => Some(HarnessEvent::Commands(
                u.available_commands
                    .into_iter()
                    .map(|c| SlashCommand {
                        hint: match c.input {
                            Some(AvailableCommandInput::Unstructured(i)) => Some(i.hint),
                            _ => None,
                        },
                        name: c.name,
                        description: c.description,
                    })
                    .collect(),
            )),
            other => {
                tracing::trace!(?other, "ignored ACP update");
                None
            }
        };
        if let Some(item) = item {
            events(item);
        }
    }
}
