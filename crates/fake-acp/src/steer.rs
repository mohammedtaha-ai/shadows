//! One job: steering a `steerable` prompt that waits for `_session/steering` (spec §20.1).

use std::sync::atomic::Ordering;

use serde_json::{Value, json};

use crate::session::Session;

/// The answer to a `_session/steering` for `s`: `injected` while a
/// `steerable` prompt waits (it then streams the text), else `promptRequired`.
pub(crate) fn answer(s: &Session, params: &Value) -> Value {
    if !s.steerable.load(Ordering::SeqCst) {
        return json!({ "outcome": "promptRequired", "reason": "noRunningTurn" });
    }
    let text = params
        .pointer("/prompt/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    s.steerable.store(false, Ordering::SeqCst);
    let _ = s.steer.send(Some(text));
    json!({ "outcome": "injected" })
}

/// The `steerable` prompt: marks the session steerable, waits, and answers
/// with the steered text as a second message.
pub(crate) async fn wait(s: &Session) -> String {
    s.steer.send_replace(None);
    let mut steered = s.steer.subscribe();
    s.steerable.store(true, Ordering::SeqCst);
    loop {
        if let Some(text) = steered.borrow_and_update().clone() {
            return text;
        }
        if steered.changed().await.is_err() {
            return String::new();
        }
    }
}
