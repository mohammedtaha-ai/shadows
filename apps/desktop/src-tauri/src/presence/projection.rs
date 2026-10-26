use super::preview::reply_preview;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub generation: u64,
    pub configuration_id: String,
    pub runtime_id: String,
    pub thread_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Operation {
    pub id: String,
    pub kind: String,
    pub status_kind: String,
    pub thread_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    pub id: String,
    pub thread_id: String,
    pub operation_id: Option<String>,
    pub ordinal: i64,
    pub kind: String,
    pub body: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub text: String,
    pub expires_at_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceProjection {
    pub expression: String,
    pub status: String,
    pub preview: Option<Preview>,
}

impl PresenceProjection {
    pub fn into_surface(self) -> crate::windows::companion::SurfacePresence {
        crate::windows::companion::SurfacePresence {
            expression: self.expression,
            status: self.status,
            preview: self
                .preview
                .map(|preview| crate::windows::companion::SurfacePreview {
                    id: preview.id,
                    text: preview.text,
                    expires_at_ms: preview.expires_at_ms,
                }),
        }
    }
    pub fn unbound() -> Self {
        Self {
            expression: "idle".into(),
            status: "Choose an agent".into(),
            preview: None,
        }
    }
    pub fn unavailable(status: &str) -> Self {
        Self {
            expression: "blocked".into(),
            status: status.into(),
            preview: None,
        }
    }
}
pub enum Observation {
    Ready,
    CaughtUp,
    Disconnected,
    Unavailable(&'static str),
    Tick,
    Delta(String),
    Tool(String),
    Snapshot {
        cursor: i64,
        operations: Vec<Operation>,
        entries: Vec<Entry>,
        live_completed: Option<String>,
    },
}
pub struct PresenceInput {
    pub scope: Scope,
    pub observation: Observation,
}
pub struct Projection {
    scope: Scope,
    bound: bool,
    previews: bool,
    live: bool,
    fault: Option<&'static str>,
    cursor: i64,
    operation: Option<Operation>,
    delta_at: Option<u64>,
    tool: bool,
    completed: Option<(String, u64)>,
    pending_until: Option<u64>,
    preview: Option<Preview>,
    output: PresenceProjection,
}
fn terminal(status: &str) -> bool {
    matches!(status, "Completed" | "Failed" | "Interrupted" | "Cancelled")
}
impl Projection {
    pub fn new(scope: Scope, bound: bool, previews: bool) -> Self {
        let output = if bound {
            PresenceProjection::unavailable("Connecting")
        } else {
            PresenceProjection::unbound()
        };
        Self {
            scope,
            bound,
            previews,
            live: false,
            fault: Some("Connecting"),
            cursor: 0,
            operation: None,
            delta_at: None,
            tool: false,
            completed: None,
            pending_until: None,
            preview: None,
            output,
        }
    }
    pub fn current_operation(&self) -> Option<&str> {
        self.operation
            .as_ref()
            .map(|operation| operation.id.as_str())
    }
    pub fn needs_entry(&self, now: u64) -> bool {
        self.previews && self.pending_until.is_some_and(|until| now < until)
    }
    pub fn reconciled(&self) -> bool {
        self.live && self.fault.is_none()
    }
    pub fn value(&self) -> PresenceProjection {
        self.output.clone()
    }
    pub fn apply(&mut self, input: PresenceInput, now: u64) -> PresenceProjection {
        if input.scope != self.scope {
            return self.output.clone();
        }
        match input.observation {
            Observation::Ready => self.fault = None,
            Observation::CaughtUp => self.live = true,
            Observation::Disconnected => {
                self.live = false;
                self.fault = Some("Reconnecting");
                self.clear_completion();
                self.delta_at = None;
                self.tool = false;
            }
            Observation::Unavailable(status) => {
                self.live = false;
                self.fault = Some(status);
                self.clear_completion();
                self.delta_at = None;
                self.tool = false;
            }
            Observation::Tick => {}
            Observation::Delta(id) => {
                if self.live && self.active(&id) {
                    self.delta_at = Some(now)
                }
            }
            Observation::Tool(id) => {
                if self.live && self.active(&id) {
                    self.tool = true
                }
            }
            Observation::Snapshot {
                cursor,
                operations,
                entries,
                live_completed,
            } => {
                if cursor < self.cursor {
                    return self.output.clone();
                }
                self.cursor = cursor;
                let latest = operations.into_iter().find(|operation| {
                    operation.kind == "PlannerTurn" && operation.thread_id == self.scope.thread_id
                });
                let changed = self.operation.as_ref().map(|operation| &operation.id)
                    != latest.as_ref().map(|operation| &operation.id);
                if changed {
                    self.clear_completion();
                    self.delta_at = None;
                    self.tool = false;
                }
                let latest = match (&self.operation, latest) {
                    (Some(current), Some(next))
                        if current.id == next.id && terminal(&current.status_kind) =>
                    {
                        Some(current.clone())
                    }
                    (_, latest) => latest,
                };
                self.operation = latest;
                if let Some(operation) = self.operation.as_ref() {
                    let id = operation.id.clone();
                    if self.live
                        && operation.status_kind == "Completed"
                        && live_completed.as_deref() == Some(&id)
                        && self.completed.as_ref().map(|(done, _)| done) != Some(&id)
                    {
                        self.completed = Some((id.clone(), now));
                        self.pending_until = Some(now.saturating_add(10_000));
                    }
                    if self.active(&id)
                        && entries.iter().any(|entry| {
                            self.entry_matches(entry, &id)
                                && matches!(entry.kind.as_str(), "ToolCall" | "Subagent")
                        })
                    {
                        self.tool = true;
                    }
                    if self.needs_entry(now) && operation.status_kind == "Completed" {
                        if let Some(entry) = entries
                            .iter()
                            .filter(|entry| {
                                self.entry_matches(entry, &id) && entry.kind == "AgentMessage"
                            })
                            .max_by_key(|entry| entry.ordinal)
                        {
                            self.pending_until = None;
                            if let Some(text) = reply_preview(&entry.body) {
                                self.preview = Some(Preview {
                                    id: format!(
                                        "{}:{}:{}:{}",
                                        self.scope.generation,
                                        self.scope.thread_id.as_deref().unwrap_or(""),
                                        id,
                                        entry.id
                                    ),
                                    text,
                                    expires_at_ms: now.saturating_add(8_000),
                                });
                            }
                        }
                    }
                }
            }
        }
        if self
            .preview
            .as_ref()
            .is_some_and(|preview| now >= preview.expires_at_ms)
        {
            self.preview = None;
        }
        if self.pending_until.is_some_and(|until| now >= until) {
            self.pending_until = None;
        }
        self.output = self.render(now);
        self.output.clone()
    }
    fn active(&self, id: &str) -> bool {
        self.operation.as_ref().is_some_and(|operation| {
            operation.id == id && matches!(operation.status_kind.as_str(), "Pending" | "Running")
        })
    }
    fn entry_matches(&self, entry: &Entry, id: &str) -> bool {
        Some(entry.thread_id.as_str()) == self.scope.thread_id.as_deref()
            && entry.operation_id.as_deref() == Some(id)
    }
    fn clear_completion(&mut self) {
        self.completed = None;
        self.pending_until = None;
        self.preview = None;
    }
    fn render(&self, now: u64) -> PresenceProjection {
        if !self.bound {
            return PresenceProjection::unbound();
        }
        if let Some(fault) = self.fault {
            return PresenceProjection::unavailable(fault);
        }
        if !self.live {
            return PresenceProjection::unavailable("Connecting");
        }
        let (expression, status) = match self
            .operation
            .as_ref()
            .map(|operation| operation.status_kind.as_str())
        {
            Some("Pending" | "Running") => {
                if self
                    .delta_at
                    .is_some_and(|at| now.saturating_sub(at) < 1000)
                {
                    ("speaking", "Replying")
                } else if self.tool {
                    ("working", "Working")
                } else {
                    ("thinking", "Thinking")
                }
            }
            Some("Completed")
                if self
                    .completed
                    .as_ref()
                    .is_some_and(|(_, at)| now.saturating_sub(*at) < 2000) =>
            {
                ("success", "Reply ready")
            }
            Some("Completed") | None => ("idle", "Ready"),
            Some("Failed") => ("blocked", "Turn failed. Open the conversation."),
            Some("Interrupted") => ("blocked", "Turn interrupted. Open the conversation."),
            Some("Cancelled") => ("idle", "Stopped"),
            Some(_) => ("blocked", "Unavailable. Retry local Core in Settings."),
        };
        PresenceProjection {
            expression: expression.into(),
            status: status.into(),
            preview: self.preview.clone(),
        }
    }
}

#[cfg(test)]
mod surface_tests {
    use super::*;

    #[test]
    fn surface_mapping_preserves_projection_bytes() {
        for projection in [
            PresenceProjection::unbound(),
            PresenceProjection::unavailable("Unavailable"),
            PresenceProjection {
                expression: "success".into(),
                status: "Reply ready".into(),
                preview: Some(Preview {
                    id: "reply-1".into(),
                    text: "مرحبا \"world\"\n".into(),
                    expires_at_ms: u64::MAX,
                }),
            },
        ] {
            assert_eq!(
                serde_json::to_vec(&projection).unwrap(),
                serde_json::to_vec(&projection.into_surface()).unwrap(),
                "surface mapping preserves every serialized field"
            );
        }
    }
}
