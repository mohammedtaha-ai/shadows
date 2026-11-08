//! Events belonging to the active native turn.
use std::collections::HashSet;

use serde_json::Value;

use crate::connection::ConnectionError;
use crate::events::{CallKind, HarnessEvent, ToolFacts};

pub(super) fn event(
    method: &str,
    params: &Value,
    streamed: &mut HashSet<String>,
    emit: &impl Fn(HarnessEvent),
) -> Result<(), ConnectionError> {
    match method {
        "item/agentMessage/delta" => {
            let id = text(params, "itemId")?;
            let delta = text(params, "delta")?;
            streamed.insert(id.to_owned());
            emit(HarnessEvent::Chunk {
                message_id: Some(id.into()),
                text: delta.into(),
            });
        }
        "item/started" | "item/completed" => {
            let item = &params["item"];
            let id = text(item, "id")?;
            let completed = method == "item/completed";
            match text(item, "type")? {
                "agentMessage" if completed && streamed.insert(id.into()) => {
                    let text = text(item, "text")?;
                    emit(HarnessEvent::Chunk {
                        message_id: Some(id.into()),
                        text: text.into(),
                    });
                }
                "mcpToolCall" | "commandExecution" | "fileChange" => {
                    tool_event(item, id, completed, emit)?;
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

fn tool_event(
    item: &Value,
    id: &str,
    completed: bool,
    emit: &impl Fn(HarnessEvent),
) -> Result<(), ConnectionError> {
    let (title, kind) = match text(item, "type")? {
        "mcpToolCall" => (text(item, "tool")?, CallKind::Other),
        "commandExecution" => ("Command", CallKind::Execute),
        _ => ("File change", CallKind::Edit),
    };
    let status = match (completed, item["status"].as_str()) {
        (false, _) => "in_progress",
        (_, Some("completed")) => "completed",
        (_, Some("declined" | "failed")) => "failed",
        (_, Some("interrupted")) => "stopped",
        _ => return Err(ConnectionError::Protocol("invalid Codex tool status")),
    };
    let approval_required = item
        .pointer("/error/message")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("requires approval"));
    if completed && approval_required {
        emit(HarnessEvent::PermissionRefused {
            title: title.into(),
            request: None,
            reason: Some("Codex approval is disabled".into()),
        });
    }
    let texts = item
        .pointer("/result/content")
        .and_then(Value::as_array)
        .map(|content| {
            content
                .iter()
                .filter_map(|entry| entry["text"].as_str().map(str::to_owned))
                .collect()
        });
    emit(HarnessEvent::ToolCall {
        id: id.into(),
        title: Some(title.into()),
        status: Some(status.into()),
        tool: Some(title.into()),
        parent: None,
        agent: None,
        facts: ToolFacts {
            kind: Some(kind),
            texts,
            input: item.get("arguments").cloned(),
            ..ToolFacts::default()
        },
    });
    Ok(())
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, ConnectionError> {
    value[key]
        .as_str()
        .ok_or(ConnectionError::Protocol("missing Codex string field"))
}
