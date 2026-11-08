//! Bind streamed events to the acknowledged native turn.
use std::collections::HashSet;
use std::sync::Mutex;

use serde_json::{Value, json};
use tokio::sync::{oneshot, watch};

use super::{Connection, mapping};
use crate::connection::{ConnectionError, PromptBlock, PromptCapabilities, PromptResult, TurnEnd};
use crate::events::HarnessEvent;

type Ending = Result<PromptResult, ConnectionError>;

#[derive(Clone)]
enum Phase {
    Starting,
    Running(String),
    Ended,
}

struct Active {
    thread: String,
    turn: Option<String>,
    buffered: Vec<(String, Value)>,
    streamed: HashSet<String>,
    phase: watch::Sender<Phase>,
    end: Option<oneshot::Sender<Ending>>,
}

impl Active {
    fn finish(&mut self, ending: Ending) {
        self.phase.send_replace(Phase::Ended);
        if let Some(reply) = self.end.take() {
            let _ = reply.send(ending);
        }
    }

    fn dispatch(&mut self, method: &str, params: &Value, emit: &impl Fn(HarnessEvent)) {
        let id = if method == "turn/completed" {
            params.pointer("/turn/id")
        } else {
            params.get("turnId")
        }
        .and_then(Value::as_str);
        if self.end.is_none() || id != self.turn.as_deref() {
            return;
        }
        if method == "turn/completed" {
            let end = match params.pointer("/turn/status").and_then(Value::as_str) {
                Some("completed") => Ok(TurnEnd::Ended),
                Some("interrupted") => Ok(TurnEnd::Cancelled),
                Some("failed") => Ok(TurnEnd::Refused("Codex turn failed".into())),
                _ => Err(ConnectionError::Protocol("invalid Codex terminal status")),
            };
            self.finish(end.map(|end| PromptResult { end, usage: None }));
        } else if let Err(error) = mapping::event(method, params, &mut self.streamed, emit) {
            self.finish(Err(error));
        }
    }
}

#[derive(Default)]
struct State {
    active: Option<Active>,
    closed: Option<ConnectionError>,
}

pub(super) struct Prompts {
    state: Mutex<State>,
    emit: Box<dyn Fn(HarnessEvent) + Send + Sync>,
}

struct Lease<'a>(&'a Prompts);

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        if let Some(mut active) = self.0.state.lock().unwrap().active.take() {
            active.finish(Err(ConnectionError::Closed));
        }
    }
}

impl Prompts {
    pub(super) fn new(emit: impl Fn(HarnessEvent) + Send + Sync + 'static) -> Self {
        Self {
            state: Mutex::default(),
            emit: Box::new(emit),
        }
    }

    fn begin(
        &self,
        thread: &str,
    ) -> Result<(Lease<'_>, oneshot::Receiver<Ending>), ConnectionError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = &state.closed {
            return Err(error.clone());
        }
        if state.active.is_some() {
            return Err(ConnectionError::Rpc(
                "Codex already has an active turn".into(),
            ));
        }
        let (send, receive) = oneshot::channel();
        state.active = Some(Active {
            thread: thread.into(),
            turn: None,
            buffered: Vec::new(),
            streamed: HashSet::new(),
            phase: watch::channel(Phase::Starting).0,
            end: Some(send),
        });
        Ok((Lease(self), receive))
    }

    fn bind(&self, id: String) {
        let mut state = self.state.lock().unwrap();
        let closed = state.closed.clone();
        let Some(active) = state.active.as_mut() else {
            return;
        };
        active.turn = Some(id.clone());
        active.phase.send_replace(Phase::Running(id));
        for (method, params) in std::mem::take(&mut active.buffered) {
            active.dispatch(&method, &params, &self.emit);
        }
        if let Some(error) = closed {
            active.finish(Err(error));
        }
    }

    pub(super) fn notification(&self, method: &str, params: Value) {
        if method == "shadows/permissionRefused" {
            (self.emit)(HarnessEvent::PermissionRefused {
                title: "Codex permission request".into(),
                request: None,
                reason: Some("Refused by Shadows".into()),
            });
            return;
        }
        if !matches!(
            method,
            "item/started" | "item/completed" | "item/agentMessage/delta" | "turn/completed"
        ) {
            return;
        }
        let mut state = self.state.lock().unwrap();
        let Some(active) = state.active.as_mut() else {
            return;
        };
        if params["threadId"].as_str() != Some(active.thread.as_str()) {
            return;
        }
        if active.turn.is_some() {
            active.dispatch(method, &params, &self.emit);
        } else {
            active.buffered.push((method.into(), params));
        }
    }

    pub(super) fn close(&self, error: ConnectionError) {
        let mut state = self.state.lock().unwrap();
        state.closed = Some(error.clone());
        if let Some(active) = state.active.as_mut()
            && active.turn.is_some()
        {
            active.finish(Err(error));
        }
    }

    fn running(&self, thread: &str) -> Option<watch::Receiver<Phase>> {
        self.state
            .lock()
            .unwrap()
            .active
            .as_ref()
            .filter(|active| active.thread == thread)
            .map(|active| active.phase.subscribe())
    }
}

impl Connection {
    pub fn prompt_capabilities(&self) -> PromptCapabilities {
        PromptCapabilities {
            image: false,
            image_reason: Some("Codex does not support image prompts in Shadows".into()),
        }
    }

    pub async fn prompt_blocks(
        &self,
        thread: &str,
        blocks: &[PromptBlock],
    ) -> Result<PromptResult, ConnectionError> {
        let settings = self.settings(thread)?;
        let input = blocks
            .iter()
            .map(|block| match block {
                PromptBlock::Text(text) => {
                    Ok(json!({"type":"text","text":text,"text_elements":[]}))
                }
                PromptBlock::Image(_) => {
                    Err(ConnectionError::Rpc("Codex images are unavailable".into()))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (_lease, end) = self.prompts.begin(thread)?;
        let started: Value = self.rpc.request("turn/start", json!({
            "threadId":thread, "input":input, "model":settings.model, "effort":settings.effort
        })).await?;
        let id = started.pointer("/turn/id").and_then(Value::as_str).ok_or(
            ConnectionError::Protocol("Codex did not acknowledge a turn ID"),
        )?;
        self.prompts.bind(id.into());
        end.await.map_err(|_| ConnectionError::Closed)?
    }

    pub fn cancel(&self, thread: &str) {
        let Some(mut phase) = self.prompts.running(thread) else {
            return;
        };
        let rpc = self.rpc.clone();
        let thread = thread.to_owned();
        tokio::spawn(async move {
            let id = match phase
                .wait_for(|phase| !matches!(phase, Phase::Starting))
                .await
            {
                Ok(phase) => match &*phase {
                    Phase::Running(id) => id.clone(),
                    _ => return,
                },
                Err(_) => return,
            };
            let _ = rpc
                .request::<Value>(
                    "turn/interrupt",
                    json!({
                        "threadId":thread, "turnId":id
                    }),
                )
                .await;
        });
    }
}
