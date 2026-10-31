//! Ordered ACP notifications translated to harness events.
use crate::events::{
    CallKind, HarnessEvent, SlashCommand, TodoItem, TodoStatus, ToolDiff, ToolFacts, ToolLocation,
    ToolMeta,
};
use agent_client_protocol::schema::{
    MaybeUndefined,
    v1::{
        AvailableCommandInput, ContentBlock, PlanEntryStatus, SessionUpdate, ToolCallContent,
        ToolCallLocation, ToolCallStatus, ToolKind,
    },
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
                    status: Some(call_status(t.status).into()),
                    tool: m.tool.or(t.name),
                    parent: m.parent,
                    agent: m.agent,
                    facts: ToolFacts {
                        kind: Some(call_kind(t.kind)),
                        locations: Some(locations(t.locations)),
                        input: t.raw_input,
                        diffs: Some(diffs(&t.content)),
                        texts: Some(texts(t.content)),
                        output: t.raw_output,
                    },
                })
            }
            SessionUpdate::ToolCallUpdate(t) => {
                let m = ToolMeta::from_update(t.meta.as_ref(), t.fields.raw_input.as_ref());
                Some(HarnessEvent::ToolCall {
                    id: t.tool_call_id.to_string(),
                    title: t.fields.title,
                    status: t.fields.status.map(|s| call_status(s).into()),
                    tool: m.tool.or(t.fields.name),
                    parent: m.parent,
                    agent: m.agent,
                    facts: ToolFacts {
                        kind: t.fields.kind.map(call_kind),
                        locations: t.fields.locations.map(locations),
                        input: t.fields.raw_input,
                        diffs: t.fields.content.as_ref().map(|content| diffs(content)),
                        texts: t.fields.content.map(texts),
                        output: t.fields.raw_output,
                    },
                })
            }
            SessionUpdate::Plan(plan) => Some(HarnessEvent::Todos(
                plan.entries
                    .into_iter()
                    .map(|entry| TodoItem {
                        content: entry.content,
                        status: match entry.status {
                            PlanEntryStatus::Pending => TodoStatus::Pending,
                            PlanEntryStatus::InProgress => TodoStatus::InProgress,
                            PlanEntryStatus::Completed => TodoStatus::Completed,
                            _ => TodoStatus::Pending,
                        },
                    })
                    .collect(),
            )),
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

/// The ACP wire name of a call's status, which the collectors match on.
fn call_status(status: ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::InProgress => "in_progress",
        ToolCallStatus::Completed => "completed",
        ToolCallStatus::Failed => "failed",
        _ => "pending",
    }
}

fn call_kind(kind: ToolKind) -> CallKind {
    match kind {
        ToolKind::Read => CallKind::Read,
        ToolKind::Edit => CallKind::Edit,
        ToolKind::Delete => CallKind::Delete,
        ToolKind::Move => CallKind::Move,
        ToolKind::Search => CallKind::Search,
        ToolKind::Execute => CallKind::Execute,
        ToolKind::Think => CallKind::Think,
        ToolKind::Fetch => CallKind::Fetch,
        ToolKind::SwitchMode => CallKind::SwitchMode,
        _ => CallKind::Other,
    }
}

fn locations(items: Vec<ToolCallLocation>) -> Vec<ToolLocation> {
    items
        .into_iter()
        .map(|item| ToolLocation {
            path: item.path.display().to_string(),
            line: item.line,
        })
        .collect()
}

fn diffs(items: &[ToolCallContent]) -> Vec<ToolDiff> {
    items
        .iter()
        .filter_map(|item| match item {
            ToolCallContent::Diff(diff) => Some(ToolDiff {
                path: diff.path.display().to_string(),
                old_text: diff.old_text.clone(),
                new_text: diff.new_text.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn texts(items: Vec<ToolCallContent>) -> Vec<String> {
    items
        .into_iter()
        .filter_map(|item| match item {
            ToolCallContent::Content(content) => match content.content {
                ContentBlock::Text(text) => Some(text.text),
                _ => None,
            },
            _ => None,
        })
        .collect()
}
