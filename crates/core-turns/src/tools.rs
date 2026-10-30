//! Gather tool calls until their final details are ready to persist.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use shadows_agent::events::{CallKind, CallStatus, ToolCallCard, ToolFacts};

use crate::hunks::Hunks;

struct OpenCall {
    card: ToolCallCard,
    title: String,
    facts: ToolFacts,
}

#[derive(Default)]
pub(crate) struct Tools {
    root: Option<PathBuf>,
    open: Vec<OpenCall>,
    ended: Vec<OpenCall>,
    written: HashSet<String>,
}

impl Tools {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            ..Self::default()
        }
    }

    pub fn is_written(&self, id: &str) -> bool {
        self.written.contains(id)
    }

    pub fn take(
        &mut self,
        id: &str,
        title: Option<&str>,
        status: Option<&str>,
        tool: Option<&str>,
        facts: &ToolFacts,
        now: &str,
    ) {
        if self.written.contains(id) {
            return;
        }
        let ended_at = self.ended.iter().position(|call| call.card.id == id);
        let open_at = self.open.iter().position(|call| call.card.id == id);
        if ended_at.is_none() && open_at.is_none() {
            self.open.push(OpenCall {
                title: String::new(),
                facts: ToolFacts::default(),
                card: ToolCallCard {
                    id: id.into(),
                    tool: None,
                    kind: CallKind::Other,
                    status: CallStatus::Pending,
                    target: None,
                    lines: None,
                    added: None,
                    removed: None,
                    diff: None,
                    error: None,
                    started_at: now.into(),
                    finished_at: String::new(),
                },
            });
        }
        let call = if let Some(at) = ended_at {
            &mut self.ended[at]
        } else {
            let at = open_at.unwrap_or(self.open.len() - 1);
            &mut self.open[at]
        };
        if let Some(title) = title {
            call.title = title.to_owned();
        }
        if let Some(tool) = tool {
            call.card.tool = Some(tool.to_owned());
        }
        if let Some(kind) = &facts.kind {
            call.facts.kind = Some(kind.clone());
        }
        if let Some(locations) = &facts.locations {
            call.facts.locations = Some(locations.clone());
        }
        if let Some(input) = &facts.input {
            call.facts.input = Some(input.clone());
        }
        if let Some(diffs) = &facts.diffs {
            call.facts.diffs = Some(diffs.clone());
        }
        if let Some(texts) = &facts.texts {
            call.facts.texts = Some(texts.clone());
        }
        if let Some(output) = &facts.output {
            call.facts.output = Some(output.clone());
        }
        if let Some(status) =
            status.filter(|status| ended_at.is_none() || matches!(*status, "completed" | "failed"))
        {
            call.card.status = match status {
                "in_progress" => CallStatus::InProgress,
                "completed" => CallStatus::Completed,
                "failed" => CallStatus::Failed,
                _ => CallStatus::Pending,
            };
        }
        if ended_at.is_none()
            && matches!(call.card.status, CallStatus::Completed | CallStatus::Failed)
        {
            call.card.finished_at = now.into();
            let at = self
                .open
                .iter()
                .position(|call| call.card.id == id)
                .unwrap();
            self.ended.push(self.open.remove(at));
        }
    }

    pub fn ended(&mut self) -> Vec<(String, ToolCallCard)> {
        let calls = std::mem::take(&mut self.ended);
        self.finish_calls(calls)
    }

    pub fn stop_all(&mut self, now: &str) -> Vec<(String, ToolCallCard)> {
        let mut calls = std::mem::take(&mut self.ended);
        for mut call in self.open.drain(..) {
            call.card.status = CallStatus::Stopped;
            call.card.finished_at = now.into();
            calls.push(call);
        }
        self.finish_calls(calls)
    }

    fn finish_calls(&mut self, calls: Vec<OpenCall>) -> Vec<(String, ToolCallCard)> {
        calls
            .into_iter()
            .filter_map(|mut call| {
                self.written.insert(call.card.id.clone());
                if call.title.is_empty() {
                    return None;
                }
                complete(&mut call, self.root.as_deref());
                Some((call.title, call.card))
            })
            .collect()
    }
}

fn complete(call: &mut OpenCall, root: Option<&Path>) {
    let facts = &call.facts;
    let card = &mut call.card;
    card.kind = facts.kind.clone().unwrap_or(CallKind::Other);
    let input = facts.input.as_ref();
    let mut is_path = false;
    let target = facts
        .locations
        .as_ref()
        .and_then(|v| v.first())
        .map(|v| {
            is_path = true;
            v.path.clone()
        })
        .or_else(|| {
            [
                "file_path",
                "notebook_path",
                "path",
                "command",
                "pattern",
                "url",
                "query",
            ]
            .into_iter()
            .find_map(|key| {
                let value = input?.get(key)?.as_str()?;
                is_path = matches!(key, "file_path" | "notebook_path" | "path");
                Some(value.to_owned())
            })
        });
    card.target = target.map(|value| {
        if is_path {
            let path = Path::new(&value);
            if let Some(relative) = root.and_then(|root| path.strip_prefix(root).ok()) {
                return relative.to_string_lossy().replace('\\', "/");
            }
        }
        value
    });
    if let Some(diffs) = facts.diffs.as_ref().filter(|diffs| !diffs.is_empty()) {
        let placed = Hunks::from_diffs(diffs, facts.locations.as_deref().unwrap_or_default());
        card.added = Some(placed.added);
        card.removed = Some(placed.removed);
        if card.kind == CallKind::Edit {
            let first = placed.hunks.first().and_then(|h| h.new_start);
            let last = placed.hunks.last().and_then(|h| {
                h.new_start.map(|start| {
                    let count = h
                        .lines
                        .iter()
                        .filter(|l| l.op != shadows_agent::events::DiffOp::Remove)
                        .count() as u32;
                    start.saturating_add(count.saturating_sub(1))
                })
            });
            card.lines = first.zip(last).map(|(from, to)| [from, to]);
        }
        if placed.hunks.iter().map(|h| h.lines.len()).sum::<usize>() <= 400 {
            card.diff = Some(placed.hunks);
        }
    }
    if card.kind == CallKind::Read {
        card.lines = input.and_then(|input| {
            let offset = u32::try_from(input.get("offset")?.as_u64()?).ok()?;
            let limit = u32::try_from(input.get("limit")?.as_u64()?).ok()?;
            if offset == 0 || limit == 0 {
                return None;
            }
            Some([offset, offset.checked_add(limit - 1)?])
        });
    }
    if card.status == CallStatus::Failed {
        let text = facts
            .texts
            .as_ref()
            .filter(|v| !v.is_empty())
            .map(|v| v.join("\n"))
            .or_else(|| {
                facts.output.as_ref().map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                })
            });
        card.error = text.and_then(|text| {
            let text = text.trim();
            if text.is_empty() {
                return None;
            }
            if text.chars().count() > 2000 {
                Some(format!("{}…", text.chars().take(1999).collect::<String>()))
            } else {
                Some(text.to_owned())
            }
        });
    }
}
