//! One job: gathering a subagent's tool calls into its card (spec §22.2).

use shadows_agent::events::{AgentFacts, SubagentCard};

/// The tool that hands a subagent's report back: the report, not a step.
const HANDBACK: &str = "SubagentHandback";

struct Open {
    card: SubagentCard,
    /// Its call has `completed` or `failed`.
    ended: bool,
    /// Its input's `description` has come.
    described: bool,
    /// Its own tool calls not yet ended: id and latest title.
    steps: Vec<(String, String)>,
}

/// The turn's open subagents, in the order their calls began.
#[derive(Default)]
pub(crate) struct Subagents {
    open: Vec<Open>,
    /// The cards already given out: a late update to one, or to one of its
    /// steps, is swallowed rather than opening a second card or a tool line.
    given: Vec<String>,
    /// Each card as it was after a change, oldest first, until taken.
    changed: Vec<SubagentCard>,
}

/// One tool call's update, as the collector was given it.
pub(crate) struct Call<'a> {
    pub id: &'a str,
    pub title: Option<&'a str>,
    pub status: Option<&'a str>,
    pub tool: Option<&'a str>,
    pub parent: Option<&'a str>,
    pub agent: Option<&'a AgentFacts>,
}

fn ends(status: Option<&str>) -> bool {
    matches!(status, Some("completed" | "failed"))
}

impl Subagents {
    /// Takes `call` when it is a subagent's or one of its steps; `false`
    /// leaves it to the collector as an ordinary tool.
    pub(crate) fn take(&mut self, call: Call<'_>) -> bool {
        if let Some(at) = self.open.iter().position(|o| o.card.id == call.id) {
            self.update(at, &call);
            return true;
        }
        let given = |id: &str| self.given.iter().any(|g| g == id);
        if given(call.id) || call.parent.is_some_and(given) {
            return true;
        }
        if call.agent.is_some() {
            self.open.push(Open {
                card: SubagentCard {
                    id: call.id.to_owned(),
                    title: call.title.unwrap_or_default().to_owned(),
                    status: "running".into(),
                    ..SubagentCard::default()
                },
                ended: false,
                described: false,
                steps: Vec::new(),
            });
            self.update(self.open.len() - 1, &call);
            return true;
        }
        let owner = match call.parent {
            Some(parent) => self.open.iter().position(|o| o.card.id == parent),
            None => self
                .open
                .iter()
                .position(|o| o.steps.iter().any(|(id, _)| id == call.id)),
        };
        let Some(at) = owner else { return false };
        self.step(at, &call);
        true
    }

    fn update(&mut self, at: usize, call: &Call<'_>) {
        let open = &mut self.open[at];
        let card = &mut open.card;
        // The description is the task; a title only stands in until it comes.
        if let Some(title) = call.title
            && !open.described
        {
            card.title = title.to_owned();
        }
        if let Some(f) = call.agent {
            if let Some(d) = &f.description {
                card.title = d.clone();
                open.described = true;
            }
            card.agent_type = f.agent_type.clone().or(card.agent_type.take());
            card.prompt = f.prompt.clone().or(card.prompt.take());
            if f.resolved_model.is_some() {
                card.model = f.resolved_model.clone();
            } else if card.model.is_none() {
                card.model = f.model.clone();
            }
            card.report = f.report.clone().or(card.report.take());
            card.duration_ms = f.duration_ms.or(card.duration_ms);
            card.tokens = f.tokens.or(card.tokens);
            card.tool_count = f.tool_count.or(card.tool_count);
        }
        if ends(call.status) {
            card.status = call.status.unwrap_or_default().to_owned();
            open.ended = true;
        }
        self.changed.push(self.open[at].card.clone());
    }

    fn step(&mut self, at: usize, call: &Call<'_>) {
        let open = &mut self.open[at];
        let i = match open.steps.iter().position(|(id, _)| id == call.id) {
            Some(i) => i,
            None => {
                open.steps.push((call.id.to_owned(), String::new()));
                open.steps.len() - 1
            }
        };
        if call.tool == Some(HANDBACK) {
            open.steps.remove(i);
            return;
        }
        if let Some(title) = call.title {
            open.steps[i].1 = title.to_owned();
        }
        if ends(call.status) {
            let (_, title) = open.steps.remove(i);
            if !title.is_empty() {
                open.card.steps.push(title);
                self.changed.push(open.card.clone());
            }
        }
    }

    /// The cards whose call has ended and whose numbers have come, removed.
    pub(crate) fn ready(&mut self) -> Vec<SubagentCard> {
        self.remove(|o| o.ended && (o.card.tokens.is_some() || o.card.report.is_some()))
    }

    /// Every card whose call has ended, removed: the parent wrote on, so a
    /// failed call's numbers are not coming.
    pub(crate) fn ended(&mut self) -> Vec<SubagentCard> {
        self.remove(|o| o.ended)
    }

    /// Every card, removed; one still running is `stopped`: its turn is over.
    pub(crate) fn all(&mut self) -> Vec<SubagentCard> {
        let mut cards = self.remove(|_| true);
        for card in cards.iter_mut().filter(|c| c.status == "running") {
            card.status = "stopped".into();
        }
        cards
    }

    fn remove(&mut self, done: impl Fn(&Open) -> bool) -> Vec<SubagentCard> {
        let (gone, kept): (Vec<Open>, Vec<Open>) =
            std::mem::take(&mut self.open).into_iter().partition(done);
        self.open = kept;
        self.given.extend(gone.iter().map(|o| o.card.id.clone()));
        gone.into_iter().map(|o| o.card).collect()
    }

    /// Each card changed since the last call, as it is now.
    pub(crate) fn take_changed(&mut self) -> Vec<SubagentCard> {
        let mut changed = std::mem::take(&mut self.changed);
        let mut seen = std::collections::HashSet::new();
        changed.reverse();
        changed.retain(|c| seen.insert(c.id.clone()));
        changed.reverse();
        changed
    }
}
