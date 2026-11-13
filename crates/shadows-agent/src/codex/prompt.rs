//! Bind streamed events to the acknowledged native turn.
use std::collections::HashSet;
use std::sync::Mutex;

use serde_json::{Value, json};
use tokio::sync::{oneshot, watch};

use super::{Connection, mapping};
use crate::connection::{
    ConnectionError, PromptBlock, PromptCapabilities, PromptResult, Steer, TurnEnd,
};
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
    model: String,
    turn: Option<String>,
    buffered: Vec<(String, Value)>,
    streamed: HashSet<String>,
    tokens: mapping::TurnTokens,
    phase: watch::Sender<Phase>,
    end: Option<oneshot::Sender<Ending>>,
}

impl Active {
    fn finish(&mut self, ending: Ending) {
        self.phase.send_replace(Phase::Ended);
        let usage = self.tokens.take();
        if let Some(reply) = self.end.take() {
            let _ = reply.send(ending.map(|result| PromptResult { usage, ..result }));
        }
    }

    fn dispatch(
        &mut self,
        method: &str,
        params: &Value,
        limits: Option<&Value>,
        emit: &impl Fn(HarnessEvent),
    ) {
        let id = if method == "turn/completed" {
            params.pointer("/turn/id")
        } else {
            params.get("turnId")
        }
        .and_then(Value::as_str);
        if self.end.is_none() || id != self.turn.as_deref() {
            return;
        }
        let outcome = match method {
            "turn/completed" => {
                let end = match params.pointer("/turn/status").and_then(Value::as_str) {
                    Some("completed") => Ok(TurnEnd::Ended),
                    Some("interrupted") => Ok(TurnEnd::Cancelled),
                    Some("failed") => {
                        let reason = params
                            .pointer("/turn/error/message")
                            .and_then(Value::as_str)
                            .map(mapping::public_message)
                            .filter(|reason| !reason.is_empty());
                        Ok(TurnEnd::Refused(
                            reason.unwrap_or_else(|| "Codex turn failed".into()),
                        ))
                    }
                    _ => Err(ConnectionError::Protocol("invalid Codex terminal status")),
                };
                self.finish(end.map(|end| PromptResult { end, usage: None }));
                return;
            }
            "thread/tokenUsage/updated" => self.tokens.update(params).map(|()| {
                if let Some(report) = self.tokens.report(&self.model, limits) {
                    emit(report);
                }
            }),
            _ => mapping::event(method, params, &mut self.streamed, emit),
        };
        if let Err(error) = outcome {
            self.finish(Err(error));
        }
    }
}

#[derive(Default)]
struct State {
    active: Option<Active>,
    closed: Option<ConnectionError>,
    /// The account's latest windows, carried by every usage report.
    limits: Option<Value>,
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
        model: &str,
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
            model: model.into(),
            turn: None,
            buffered: Vec::new(),
            streamed: HashSet::new(),
            tokens: mapping::TurnTokens::default(),
            phase: watch::channel(Phase::Starting).0,
            end: Some(send),
        });
        Ok((Lease(self), receive))
    }

    fn bind(&self, id: String) {
        let mut state = self.state.lock().unwrap();
        let State {
            active,
            closed,
            limits,
        } = &mut *state;
        let Some(active) = active.as_mut() else {
            return;
        };
        active.turn = Some(id.clone());
        active.phase.send_replace(Phase::Running(id));
        for (method, params) in std::mem::take(&mut active.buffered) {
            active.dispatch(&method, &params, limits.as_ref(), &self.emit);
        }
        if let Some(error) = closed.clone() {
            active.finish(Err(error));
        }
    }

    /// Keeps the account's windows when Codex reports readable ones; a turn
    /// with a context figure reports them at once.
    pub(super) fn record_limits(&self, snapshot: &Value) {
        let Some(limits) = mapping::limits(snapshot) else {
            return;
        };
        let mut state = self.state.lock().unwrap();
        let report = state
            .active
            .as_ref()
            .filter(|active| active.turn.is_some() && active.end.is_some())
            .and_then(|active| active.tokens.report(&active.model, Some(&limits)));
        state.limits = Some(limits);
        if let Some(report) = report {
            (self.emit)(report);
        }
    }

    /// The opening's read, which can answer after a newer update: it only
    /// fills windows that are still unknown.
    pub(super) fn first_limits(&self, snapshot: &Value) {
        if self.state.lock().unwrap().limits.is_none() {
            self.record_limits(snapshot);
        }
    }

    pub(super) fn notification(&self, method: &str, params: Value) {
        match method {
            "shadows/permissionRefused" => {
                (self.emit)(HarnessEvent::PermissionRefused {
                    title: "Codex permission request".into(),
                    request: None,
                    reason: Some("Refused by Shadows".into()),
                });
                return;
            }
            "account/rateLimits/updated" => return self.record_limits(&params["rateLimits"]),
            "item/started"
            | "item/completed"
            | "item/agentMessage/delta"
            | "thread/tokenUsage/updated"
            | "turn/completed" => {}
            _ => return,
        }
        let mut state = self.state.lock().unwrap();
        let State { active, limits, .. } = &mut *state;
        let Some(active) = active.as_mut() else {
            return;
        };
        if params["threadId"].as_str() != Some(active.thread.as_str()) {
            return;
        }
        if active.turn.is_some() {
            active.dispatch(method, &params, limits.as_ref(), &self.emit);
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

/// The acknowledged turn behind `phase`, once its start has been answered.
async fn acknowledged(mut phase: watch::Receiver<Phase>) -> Option<String> {
    let phase = phase
        .wait_for(|phase| !matches!(phase, Phase::Starting))
        .await
        .ok()?;
    match &*phase {
        Phase::Running(id) => Some(id.clone()),
        _ => None,
    }
}

impl Connection {
    pub fn prompt_capabilities(&self) -> PromptCapabilities {
        PromptCapabilities {
            image: true,
            image_reason: None,
        }
    }

    fn input(blocks: &[PromptBlock]) -> Result<Vec<Value>, ConnectionError> {
        use base64::Engine as _;
        blocks
            .iter()
            .map(|block| match block {
                PromptBlock::Text(text) => {
                    Ok(json!({"type":"text","text":text,"text_elements":[]}))
                }
                PromptBlock::Image(image) => {
                    let data = base64::engine::general_purpose::STANDARD.encode(&image.bytes);
                    let url = format!("data:{};base64,{data}", image.media_type);
                    Ok(json!({"type":"image","url":url}))
                }
            })
            .collect()
    }

    pub async fn prompt_blocks(
        &self,
        thread: &str,
        blocks: &[PromptBlock],
    ) -> Result<PromptResult, ConnectionError> {
        let settings = self.settings(thread)?;
        let input = Self::input(blocks)?;
        let (_lease, end) = self.prompts.begin(thread, &settings.model)?;
        let started: Value = self.rpc.request("turn/start", json!({
            "threadId":thread, "input":input, "model":settings.model, "effort":settings.effort
        })).await?;
        let id = started.pointer("/turn/id").and_then(Value::as_str).ok_or(
            ConnectionError::Protocol("Codex did not acknowledge a turn ID"),
        )?;
        self.prompts.bind(id.into());
        let answer = end.await.map_err(|_| ConnectionError::Closed)?;
        // A request still held belongs to the turn that ended: it is refused, not left.
        {
            let _guard = self.permissions.settlement.lock().await;
            let reason = if answer.is_err() { "closed" } else { "ended" };
            self.permissions.refuse_session(thread, reason);
        }
        answer
    }

    /// Adds the person's message to the running turn, which then answers for
    /// it. With no turn running nothing is started.
    pub async fn steer_blocks(
        &self,
        thread: &str,
        blocks: &[PromptBlock],
    ) -> Result<Steer, ConnectionError> {
        let input = Self::input(blocks)?;
        let Some(phase) = self.prompts.running(thread) else {
            return Ok(Steer::PromptRequired);
        };
        let Some(turn) = acknowledged(phase).await else {
            return Ok(Steer::PromptRequired);
        };
        let _: Value = self
            .rpc
            .request(
                "turn/steer",
                json!({"threadId": thread, "expectedTurnId": turn, "input": input}),
            )
            .await?;
        Ok(Steer::Injected)
    }

    pub fn cancel(&self, thread: &str) {
        let Some(phase) = self.prompts.running(thread) else {
            return;
        };
        let rpc = self.rpc.clone();
        let thread = thread.to_owned();
        tokio::spawn(async move {
            let Some(id) = acknowledged(phase).await else {
                return;
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
