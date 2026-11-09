//! Events belonging to the active native turn.
use std::collections::HashSet;

use serde_json::{Value, json};

use crate::connection::ConnectionError;
use crate::events::{CallKind, HarnessEvent, ToolFacts, TurnUsage};

/// One turn's token reports. Codex reports each model request as `last` and
/// the thread's running sum as `total`, so a report whose total did not grow
/// repeats one already counted.
#[derive(Default)]
pub(super) struct TurnTokens {
    total: Option<u64>,
    sum: Option<TurnUsage>,
    context: Option<(u64, u64)>,
}

impl TurnTokens {
    /// Reads one `thread/tokenUsage/updated`. The context figure is the last
    /// request alone: its input already holds the cached part, and `total` is
    /// a running sum, never occupancy. A missing or zero window stays 0, which
    /// a reader treats as unknown.
    pub(super) fn update(&mut self, params: &Value) -> Result<(), ConnectionError> {
        let usage = &params["tokenUsage"];
        let count = |value: &Value, key: &str| {
            value[key]
                .as_u64()
                .ok_or(ConnectionError::Protocol("invalid Codex token usage"))
        };
        let last = &usage["last"];
        let used = count(last, "totalTokens")?;
        let total = count(&usage["total"], "totalTokens")?;
        let size = usage["modelContextWindow"].as_u64().unwrap_or(0);
        if self.total.is_none_or(|seen| total > seen) {
            let (input, output) = (count(last, "inputTokens")?, count(last, "outputTokens")?);
            let cached = count(last, "cachedInputTokens")?;
            let written = last["cacheWriteInputTokens"].as_u64().unwrap_or(0);
            let sum = self.sum.get_or_insert(TurnUsage {
                input_tokens: 0,
                output_tokens: 0,
                cached_read_tokens: 0,
                cached_write_tokens: 0,
                total_tokens: 0,
                cost: None,
            });
            sum.input_tokens += input;
            sum.output_tokens += output;
            sum.cached_read_tokens += cached;
            sum.cached_write_tokens += written;
            sum.total_tokens += used;
            self.total = Some(total);
        }
        self.context = Some((used, size));
        Ok(())
    }

    /// The latest context figure as a usage report, once the turn has one.
    pub(super) fn report(&self, model: &str, limits: Option<&Value>) -> Option<HarnessEvent> {
        let (used, size) = self.context?;
        Some(HarnessEvent::Usage {
            used,
            size,
            model: Some(model.into()),
            rate_limit: limits.cloned(),
            cost: None,
        })
    }

    pub(super) fn take(&mut self) -> Option<TurnUsage> {
        self.sum.take()
    }
}

/// Codex's account windows in the report shape `AccountLimits::from_report`
/// reads: the five-hour and weekly windows found by their length, with
/// `usedPercent` as a fraction. A window without a reset time is left out
/// rather than guessed; no readable window is no report.
pub(super) fn limits(snapshot: &Value) -> Option<Value> {
    let mut windows = serde_json::Map::new();
    for slot in ["primary", "secondary"] {
        let window = &snapshot[slot];
        let name = match window["windowDurationMins"].as_u64() {
            Some(300) => "five_hour",
            Some(10080) => "seven_day",
            _ => continue,
        };
        let (Some(used), Some(resets)) =
            (window["usedPercent"].as_f64(), window["resetsAt"].as_i64())
        else {
            continue;
        };
        windows.insert(
            name.into(),
            json!({"utilization": used / 100.0, "resetsAt": resets}),
        );
    }
    (!windows.is_empty()).then(|| json!({ "unifiedWindows": windows }))
}

/// A provider's words as a person may read them: the first line, bounded, and
/// cut before any bearer value, so a credential or raw configuration never
/// reaches an entry or a log.
pub(super) fn public_message(message: &str) -> String {
    let line = message.lines().next().unwrap_or("").trim();
    let line = match line.find("Bearer ") {
        Some(at) => &line[..at],
        None => line,
    };
    line.chars().take(300).collect()
}

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
