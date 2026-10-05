//! A fake session's state and offer.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicBool},
};

use agent_client_protocol::schema::v1::{McpServer, Meta, SessionConfigOption};
use serde_json::{Value, json};
use tokio::sync::watch;

#[derive(Clone)]
pub(crate) struct Session {
    pub(crate) cwd: PathBuf,
    pub(crate) how: &'static str,
    pub(crate) model: String,
    pub(crate) effort: Option<String>,
    pub(crate) mode: String,
    pub(crate) cancel: watch::Sender<bool>,
    /// The text of a `_session/steering` while `steerable` waits (`steer.rs`).
    pub(crate) steer: Arc<watch::Sender<Option<String>>>,
    /// Whether a `steerable` prompt is waiting to be steered.
    pub(crate) steerable: Arc<AtomicBool>,
    pub(crate) setup: Setup,
}

/// What a session opened with: Shadows' MCP server, the appended prompt and
/// the pre-approved tools. The bearer is kept to call `/mcp` and reported only
/// as its hash, because a reply is stored as an entry.
#[derive(Clone, Default)]
pub(crate) struct Setup {
    pub(crate) mcp: Value,
    pub(crate) url: Option<String>,
    pub(crate) bearer: Option<String>,
    pub(crate) append: Option<String>,
    pub(crate) allowed: Value,
}

pub(crate) fn setup_of(servers: &[McpServer], meta: Option<&Meta>) -> Setup {
    let http = servers.iter().find_map(|s| match s {
        McpServer::Http(h) => Some(h),
        _ => None,
    });
    let auth = http.and_then(|h| h.headers.iter().find(|x| x.name == "Authorization"));
    let meta = meta
        .map(|m| Value::Object(m.clone()))
        .unwrap_or(Value::Null);
    Setup {
        mcp: http
            .map(|h| json!({"name":h.name,"url":h.url}))
            .unwrap_or(Value::Null),
        url: http.map(|h| h.url.clone()),
        bearer: auth
            .and_then(|x| x.value.strip_prefix("Bearer "))
            .map(str::to_owned),
        append: meta
            .pointer("/systemPrompt/append")
            .and_then(Value::as_str)
            .map(str::to_owned),
        allowed: meta
            .pointer("/claudeCode/options/allowedTools")
            .cloned()
            .unwrap_or(Value::Null),
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) next: usize,
    pub(crate) sessions: HashMap<String, Session>,
    pub(crate) context_seen: bool,
    /// `session/resume` requests this process received.
    pub(crate) resumes: usize,
    /// The client capabilities' `_meta` `initialize` carried.
    pub(crate) client_meta: Value,
}
pub(crate) type Shared = Arc<Mutex<State>>;

/// The efforts a fake model offers; `fake-tiny` offers none, as Haiku 4.5
/// does (ACP_PROBE §1).
pub(crate) fn efforts(model: &str) -> &'static [&'static str] {
    match model {
        "fake-large" => &["low", "high", "max"],
        "fake-small" => &["low", "high"],
        _ => &[],
    }
}

/// The options as the real adapter shapes them (ACP_PROBE §1): ids that are
/// not their categories (`effort` is `thought_level`), the effort option
/// absent for a model without one, and a `model_config` option to ignore.
pub(crate) fn options(s: &Session) -> Vec<SessionConfigOption> {
    let efforts = efforts(&s.model);
    let mut values = vec![
        json!({
            "id": "mode",
            "name": "Mode",
            "category": "mode",
            "type": "select",
            "currentValue": s.mode,
            "options": (
                ["default", "acceptEdits", "plan", "auto", "bypassPermissions"]
                    .iter()
                    .map(|v| json!({"value": v, "name": v}))
                    .collect::<Vec<_>>()
            ),
        }),
        json!({
            "id": "model",
            "name": "Model",
            "category": "model",
            "type": "select",
            "currentValue": s.model,
            "options": [
                {"value": "fake-large", "name": "Fake Large", "description": "The biggest fake"},
                {"value": "fake-small", "name": "Fake Small"},
                {"value": "fake-tiny", "name": "Fake Tiny"},
                {"value": "fake-locked", "name": "Fake Locked"},
            ],
        }),
    ];
    if let Some(effort) = &s.effort {
        values.push(json!({
            "id": "effort",
            "name": "Effort",
            "category": "thought_level",
            "type": "select",
            "currentValue": effort,
            "options": efforts
                .iter()
                .map(|v| json!({"value": v, "name": v}))
                .collect::<Vec<_>>(),
        }));
    }
    values.push(json!({
        "id": "fast",
        "name": "Fast",
        "category": "model_config",
        "type": "select",
        "currentValue": "off",
        "options": [{"value": "on", "name": "On"}, {"value": "off", "name": "Off"}],
    }));
    values
        .into_iter()
        .map(|v| serde_json::from_value(v).expect("fake option schema"))
        .collect()
}

pub(crate) fn make_session(cwd: PathBuf, how: &'static str, setup: Setup) -> Session {
    let (cancel, _) = watch::channel(false);
    Session {
        cwd,
        how,
        model: "fake-large".into(),
        effort: Some("high".into()),
        mode: "auto".into(),
        cancel,
        steer: Arc::new(watch::channel(None).0),
        steerable: Arc::new(AtomicBool::new(false)),
        setup,
    }
}
