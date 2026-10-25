//! One job: turn ACP updates into complete durable conversation entries.

use shadows_agent::events::{HarnessEvent, SubagentCard};

use super::subagents::{Call, Subagents};

#[derive(Debug, PartialEq)]
pub enum Durable {
    Message(String),
    Tool(String),
    /// A subagent's card; its steps are no entries of their own.
    Subagent(Box<SubagentCard>),
    PermissionRefused(String),
}

fn card(card: SubagentCard) -> Durable {
    Durable::Subagent(Box::new(card))
}

#[derive(Default)]
pub struct Collector {
    message_id: Option<String>,
    message: String,
    /// Open tool calls, id and latest title, in the order they were first seen.
    tools: Vec<(String, String)>,
    subagents: Subagents,
}

impl Collector {
    pub fn new() -> Self {
        Self::default()
    }

    fn flush_message(&mut self, out: &mut Vec<Durable>) {
        if !self.message.is_empty() {
            out.push(Durable::Message(std::mem::take(&mut self.message)));
        }
    }

    pub fn push(&mut self, event: &HarnessEvent) -> Vec<Durable> {
        let mut out = Vec::new();
        match event {
            HarnessEvent::Chunk { message_id, text } => {
                // A card whose numbers did not come goes before this text.
                let ended = self.subagents.ended();
                if (self.message_id != *message_id || !ended.is_empty()) && !self.message.is_empty()
                {
                    self.flush_message(&mut out);
                }
                out.extend(ended.into_iter().map(card));
                self.message_id = message_id.clone();
                self.message.push_str(text);
            }
            HarnessEvent::ToolCall {
                id,
                title,
                status,
                tool,
                parent,
                agent,
            } => {
                self.flush_message(&mut out);
                let call = Call {
                    id,
                    title: title.as_deref(),
                    status: status.as_deref(),
                    tool: tool.as_deref(),
                    parent: parent.as_deref(),
                    agent: agent.as_deref(),
                };
                if self.subagents.take(call) {
                    out.extend(self.subagents.ready().into_iter().map(card));
                    return out;
                }
                let at = match self.tools.iter().position(|(open, _)| open == id) {
                    Some(at) => at,
                    None => {
                        self.tools.push((id.clone(), String::new()));
                        self.tools.len() - 1
                    }
                };
                if let Some(title) = title {
                    self.tools[at].1 = title.clone();
                }
                if matches!(status.as_deref(), Some("completed" | "failed")) {
                    let (_, title) = self.tools.remove(at);
                    if !title.is_empty() {
                        out.push(Durable::Tool(title));
                    }
                }
            }
            HarnessEvent::PermissionRefused { title } => {
                self.flush_message(&mut out);
                if !title.is_empty() {
                    out.push(Durable::PermissionRefused(title.clone()));
                }
            }
            HarnessEvent::Usage { .. }
            | HarnessEvent::Options(_)
            | HarnessEvent::Commands(_)
            | HarnessEvent::SessionTitle { .. }
            | HarnessEvent::Subagent(_)
            | HarnessEvent::TurnEnd { .. } => {}
        }
        out
    }

    /// Each subagent card changed since the last call, to publish.
    pub fn changed_cards(&mut self) -> Vec<SubagentCard> {
        self.subagents.take_changed()
    }

    /// The message being streamed, written now: a steered message
    /// goes after the text the person already saw, and the reply continues
    /// as a new message.
    pub fn cut(&mut self) -> Vec<Durable> {
        let mut out = Vec::new();
        self.flush_message(&mut out);
        self.message_id = None;
        out
    }

    pub fn finish(&mut self) -> Vec<Durable> {
        let mut out = Vec::new();
        self.flush_message(&mut out);
        out.extend(self.subagents.all().into_iter().map(card));
        for (_, title) in self.tools.drain(..) {
            if !title.is_empty() {
                out.push(Durable::Tool(title));
            }
        }
        out
    }
}
