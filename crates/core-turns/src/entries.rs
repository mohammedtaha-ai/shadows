//! One job: turn ACP updates into complete durable conversation entries.

use std::path::PathBuf;

use shadows_agent::events::{HarnessEvent, SubagentCard, ToolCallCard};

use crate::tools::Tools;

use super::subagents::{Call, Subagents};

#[derive(Debug, PartialEq)]
pub enum Durable {
    Message(String),
    Tool {
        title: String,
        card: Box<ToolCallCard>,
    },
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
    tools: Tools,
    subagents: Subagents,
}

impl Collector {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            tools: Tools::new(root),
            ..Self::default()
        }
    }

    fn flush_tools(&mut self, out: &mut Vec<Durable>) {
        out.extend(
            self.tools
                .ended()
                .into_iter()
                .map(|(title, card)| Durable::Tool {
                    title,
                    card: Box::new(card),
                }),
        );
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
                self.flush_tools(&mut out);
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
                facts,
            } => {
                if self.tools.is_written(id) {
                    return out;
                }
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
                    let ready = self.subagents.ready();
                    if !ready.is_empty() {
                        self.flush_tools(&mut out);
                    }
                    out.extend(ready.into_iter().map(card));
                    return out;
                }
                self.tools.take(
                    id,
                    title.as_deref(),
                    status.as_deref(),
                    tool.as_deref(),
                    facts,
                    &core_db::Storage::now(),
                );
            }
            HarnessEvent::PermissionRefused { title } => {
                self.flush_message(&mut out);
                self.flush_tools(&mut out);
                if !title.is_empty() {
                    out.push(Durable::PermissionRefused(title.clone()));
                }
            }
            HarnessEvent::Usage { .. }
            | HarnessEvent::Options(_)
            | HarnessEvent::Commands(_)
            | HarnessEvent::SessionTitle { .. }
            | HarnessEvent::Tool(_)
            | HarnessEvent::Todos(_)
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
        self.flush_tools(&mut out);
        self.message_id = None;
        out
    }

    pub fn finish(&mut self) -> Vec<Durable> {
        let mut out = Vec::new();
        self.flush_message(&mut out);
        self.flush_tools(&mut out);
        out.extend(self.subagents.all().into_iter().map(card));
        out.extend(
            self.tools
                .stop_all(&core_db::Storage::now())
                .into_iter()
                .map(|(title, card)| Durable::Tool {
                    title,
                    card: Box::new(card),
                }),
        );
        out
    }
}
