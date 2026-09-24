//! One job: turn ACP updates into complete durable conversation entries.

use crate::agent::events::HarnessEvent;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Durable {
    Message(String),
    Tool(String),
    PermissionRefused(String),
}

#[derive(Default)]
pub(crate) struct Collector {
    message_id: Option<String>,
    message: String,
    /// Open tool calls, id and latest title, in the order they were first seen.
    tools: Vec<(String, String)>,
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
                if self.message_id != *message_id && !self.message.is_empty() {
                    self.flush_message(&mut out);
                }
                self.message_id = message_id.clone();
                self.message.push_str(text);
            }
            HarnessEvent::ToolCall { id, title, status } => {
                self.flush_message(&mut out);
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
            | HarnessEvent::TurnEnd { .. } => {}
        }
        out
    }

    pub(crate) fn finish(&mut self) -> Vec<Durable> {
        let mut out = Vec::new();
        self.flush_message(&mut out);
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
        }
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
    fn a_changed_message_id_closes_the_previous_message() {
        let mut c = Collector::new();
        assert!(c.push(&chunk("m1", "a")).is_empty());
        assert_eq!(c.push(&chunk("m2", "b")), [Durable::Message("a".into())]);
        assert_eq!(c.finish(), [Durable::Message("b".into())]);
        assert!(c.finish().is_empty());
    }
}
