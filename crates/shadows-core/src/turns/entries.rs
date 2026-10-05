//! One job: turn ACP updates into complete durable conversation entries.

use shadows_agent::events::{HarnessEvent, SubagentCard};

use super::subagents::{Call, Subagents};

#[derive(Debug, PartialEq)]
pub(crate) enum Durable {
    Message(String),
    Tool(String),
    /// A subagent's card (§22.2); its steps are no entries of their own.
    Subagent(Box<SubagentCard>),
    PermissionRefused(String),
}

fn card(card: SubagentCard) -> Durable {
    Durable::Subagent(Box::new(card))
}

#[derive(Default)]
pub(crate) struct Collector {
    message_id: Option<String>,
    message: String,
    /// Open tool calls, id and latest title, in the order they were first seen.
    tools: Vec<(String, String)>,
    subagents: Subagents,
}

impl Collector {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn flush_message(&mut self, out: &mut Vec<Durable>) {
        if !self.message.is_empty() {
            out.push(Durable::Message(std::mem::take(&mut self.message)));
        }
    }

    pub(crate) fn push(&mut self, event: &HarnessEvent) -> Vec<Durable> {
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

    /// Each subagent card changed since the last call (§22.2), to publish.
    pub(crate) fn changed_cards(&mut self) -> Vec<SubagentCard> {
        self.subagents.take_changed()
    }

    /// The message being streamed, written now (§20.4): a steered message
    /// goes after the text the person already saw, and the reply continues
    /// as a new message.
    pub(crate) fn cut(&mut self) -> Vec<Durable> {
        let mut out = Vec::new();
        self.flush_message(&mut out);
        self.message_id = None;
        out
    }

    pub(crate) fn finish(&mut self) -> Vec<Durable> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use shadows_agent::events::AgentFacts;
    fn chunk(id: &str, text: &str) -> HarnessEvent {
        HarnessEvent::Chunk {
            message_id: Some(id.into()),
            text: text.into(),
        }
    }
    fn tool(id: &str, title: Option<&str>, status: Option<&str>) -> HarnessEvent {
        HarnessEvent::ToolCall {
            id: id.into(),
            title: title.map(str::to_owned),
            status: status.map(str::to_owned),
            tool: None,
            parent: None,
            agent: None,
        }
    }
    fn agent(id: &str, status: Option<&str>, facts: AgentFacts) -> HarnessEvent {
        HarnessEvent::ToolCall {
            id: id.into(),
            title: Some("Task".into()),
            status: status.map(str::to_owned),
            tool: Some("Agent".into()),
            parent: None,
            agent: Some(Box::new(facts)),
        }
    }
    fn step(id: &str, tool: &str, title: &str, parent: Option<&str>, status: &str) -> HarnessEvent {
        HarnessEvent::ToolCall {
            id: id.into(),
            title: Some(title.into()),
            status: Some(status.into()),
            tool: Some(tool.into()),
            parent: parent.map(str::to_owned),
            agent: None,
        }
    }
    fn cards(out: &[Durable]) -> Vec<&SubagentCard> {
        out.iter()
            .filter_map(|d| match d {
                Durable::Subagent(c) => Some(&**c),
                _ => None,
            })
            .collect()
    }
    #[test]
    fn a_subagent_becomes_one_entry_with_its_steps_and_numbers() {
        let mut c = Collector::new();
        let mut out = Vec::new();
        let asked = AgentFacts {
            description: Some("List files".into()),
            model: Some("sonnet".into()),
            prompt: Some("List them".into()),
            ..AgentFacts::default()
        };
        let numbers = AgentFacts {
            resolved_model: Some("claude-sonnet-5-5".into()),
            tokens: Some(43119),
            report: Some("two files".into()),
            ..AgentFacts::default()
        };
        for e in [
            chunk("m1", "launching"),
            agent("a", Some("pending"), asked),
            step("r1", "Read", "Read alpha.txt", Some("a"), "pending"),
            step("r1", "Read", "Read alpha.txt", None, "completed"),
            step("r2", "Glob", "Find *", Some("a"), "completed"),
            step("h", "SubagentHandback", "Hand back", Some("a"), "completed"),
            agent("a", Some("completed"), AgentFacts::default()),
        ] {
            out.extend(c.push(&e));
        }
        assert!(cards(&out).is_empty(), "no numbers yet: {out:?}");
        out.extend(c.push(&agent("a", None, numbers)));
        out.extend(c.push(&chunk("m2", "done")));
        out.extend(c.finish());
        let card = cards(&out)[0].clone();
        assert_eq!(out.len(), 3, "an inner tool writes no entry: {out:?}");
        assert_eq!(out[0], Durable::Message("launching".into()));
        assert_eq!(out[2], Durable::Message("done".into()));
        assert_eq!(card.title, "List files");
        assert_eq!(card.steps, ["Read alpha.txt", "Find *"]);
        assert_eq!(card.model.as_deref(), Some("claude-sonnet-5-5"));
        assert_eq!(
            (card.status.as_str(), card.tokens),
            ("completed", Some(43119))
        );
        assert_eq!(card.report.as_deref(), Some("two files"));
        assert!(!c.changed_cards().is_empty());
    }
    #[test]
    fn a_card_without_numbers_goes_before_the_next_text_and_a_running_one_stops() {
        let mut c = Collector::new();
        c.push(&agent("a", Some("pending"), AgentFacts::default()));
        c.push(&agent("a", Some("failed"), AgentFacts::default()));
        let out = c.push(&chunk("m1", "it failed"));
        assert_eq!(cards(&out)[0].status, "failed");
        let out = c.push(&agent("b", Some("pending"), AgentFacts::default()));
        assert_eq!(out, [Durable::Message("it failed".into())]);
        assert_eq!(cards(&c.finish())[0].status, "stopped");
    }
    #[test]
    fn chunks_of_one_message_become_one_entry_and_a_tool_call_splits_messages() {
        let mut c = Collector::new();
        let mut out = Vec::new();
        for e in [
            chunk("m1", "hello "),
            chunk("m1", "there"),
            tool("t1", Some("Terminal"), Some("pending")),
            tool("t1", Some("Read notes.md"), None),
            tool("t1", None, Some("completed")),
            chunk("m2", "done"),
        ] {
            out.extend(c.push(&e));
        }
        out.extend(c.finish());
        assert_eq!(
            out,
            [
                Durable::Message("hello there".into()),
                Durable::Tool("Read notes.md".into()),
                Durable::Message("done".into())
            ]
        );
    }
    #[test]
    fn a_tool_call_that_never_finished_is_emitted_at_the_end_under_its_last_title() {
        let mut c = Collector::new();
        assert!(
            c.push(&tool("t9", Some("Terminal"), Some("pending")))
                .is_empty()
        );
        assert!(c.push(&tool("t9", Some("npm install"), None)).is_empty());
        assert_eq!(c.finish(), [Durable::Tool("npm install".into())]);
    }
    #[test]
    fn unfinished_tools_are_emitted_in_the_order_they_began() {
        let mut c = Collector::new();
        for id in ["a", "b", "c"] {
            assert!(c.push(&tool(id, Some(id), Some("pending"))).is_empty());
        }
        assert_eq!(c.finish(), ["a", "b", "c"].map(|t| Durable::Tool(t.into())));
    }
    #[test]
    fn a_cut_writes_the_streamed_text_and_the_reply_continues_as_a_new_message() {
        let mut c = Collector::new();
        assert!(c.push(&chunk("m1", "so far")).is_empty());
        assert_eq!(c.cut(), [Durable::Message("so far".into())]);
        assert!(c.push(&chunk("m1", " and on")).is_empty());
        assert_eq!(c.finish(), [Durable::Message(" and on".into())]);
    }
    #[test]
    fn a_changed_message_id_closes_the_previous_message() {
        let mut c = Collector::new();
        assert!(c.push(&chunk("m1", "a")).is_empty());
        assert_eq!(c.push(&chunk("m2", "b")), [Durable::Message("a".into())]);
        assert_eq!(c.finish(), [Durable::Message("b".into())]);
        assert!(c.finish().is_empty());
    }
}
