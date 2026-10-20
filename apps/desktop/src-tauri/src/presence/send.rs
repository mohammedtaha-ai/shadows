// One explicit companion submission through the existing Core admission API.
use super::{binding, BindingView, Presence, RequestContext};
use crate::core_connection::ReadyConnection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::Read, time::Duration};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Submission {
    pub text: String,
    pub command_id: String,
    pub context: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    pub thread_id: String,
    pub operation_id: String,
}
#[derive(Clone)]
pub(super) struct Captured {
    pub command_id: String,
    pub scope: String,
    pub thread_id: String,
    pub body: Value,
    ready: ReadyConnection,
}
struct Setup {
    command_id: String,
    scope: String,
    thread_id: String,
    default_model: Option<String>,
}
pub(super) struct State {
    pub scope: Option<String>,
    pub context: Option<String>,
    pub captured: Option<Captured>,
    active: Option<(String, String)>,
    setup: Option<Setup>,
    pub create_timeout: Duration,
    pub session_timeout: Duration,
    pub turn_timeout: Duration,
}
impl State {
    pub fn new(view: &BindingView) -> Self {
        Self {
            scope: view.agent.as_ref().map(|_| Uuid::new_v4().to_string()),
            context: view.agent.as_ref().map(|_| Uuid::new_v4().to_string()),
            captured: None,
            active: None,
            setup: None,
            create_timeout: Duration::from_secs(10),
            session_timeout: Duration::from_secs(30),
            turn_timeout: Duration::from_secs(30),
        }
    }
    pub fn changed(&mut self, before: &BindingView, after: &BindingView, own_bind: bool) {
        let different = before.agent != after.agent
            || before.configuration_id != after.configuration_id
            || (!own_bind && before.target != after.target);
        if different {
            self.scope = after.agent.as_ref().map(|_| Uuid::new_v4().to_string());
            self.captured = None;
            self.setup = None;
            self.active = None;
        }
        if before.generation != after.generation {
            self.context = after.agent.as_ref().map(|_| Uuid::new_v4().to_string());
        }
    }
}
struct Admitted {
    request: RequestContext,
    view: BindingView,
    scope: String,
    retry: Option<Captured>,
}
impl Presence {
    pub fn context(&self) -> (Option<String>, Option<String>) {
        let inner = self.inner.lock().unwrap();
        (inner.send.scope.clone(), inner.send.context.clone())
    }
    fn admit(&self, s: &Submission) -> Result<Admitted, &'static str> {
        let ready = self
            .connection
            .ready()
            .ok_or("companion_core_unavailable")?;
        let mut inner = self.inner.lock().unwrap();
        if inner.error.is_some() {
            return Err("companion_send_failed");
        }
        let scope = inner
            .send
            .scope
            .clone()
            .ok_or("companion_agent_unavailable")?;
        let retry = inner
            .send
            .captured
            .as_ref()
            .filter(|c| c.command_id == s.command_id)
            .cloned();
        if let Some(c) = &retry {
            if c.body["prompt"] != s.text {
                return Err("companion_send_invalid");
            }
            if c.scope != scope || c.ready != ready {
                return Err("companion_send_stale");
            }
        } else if inner.send.context.as_deref() != Some(&s.context) {
            return Err("companion_send_stale");
        }
        if inner.view.configuration_id.as_deref() != Some(&ready.configuration_id) {
            return Err("companion_send_stale");
        }
        if inner.send.active.is_some() {
            return Err("companion_send_busy");
        }
        if inner
            .send
            .captured
            .as_ref()
            .is_some_and(|c| c.command_id != s.command_id)
        {
            return Err("companion_send_busy");
        }
        inner.ticket += 1;
        let admitted = Admitted {
            request: RequestContext {
                ticket: inner.ticket,
                generation: inner.view.generation,
                ready,
            },
            view: inner.view.clone(),
            scope: scope.clone(),
            retry,
        };
        inner.send.active = Some((scope, s.command_id.clone()));
        Ok(admitted)
    }
    pub fn send(&self, s: Submission) -> Result<Sent, &'static str> {
        if s.text.trim().is_empty()
            || s.text.len() > 32768
            || Uuid::parse_str(&s.command_id).is_err()
        {
            return Err("companion_send_invalid");
        }
        let admitted = self.admit(&s)?;
        let scope = admitted.scope.clone();
        let result = self.run_send(&s, admitted);
        let mut inner = self.inner.lock().unwrap();
        if inner.send.active.as_ref() == Some(&(scope.clone(), s.command_id.clone())) {
            inner.send.active = None;
        }
        if inner.send.scope.as_deref() == Some(&scope)
            && inner
                .send
                .captured
                .as_ref()
                .is_some_and(|c| c.command_id == s.command_id)
        {
            if result.is_ok()
                || !matches!(
                    result,
                    Err("companion_send_uncertain"
                        | "companion_send_busy"
                        | "companion_core_unavailable")
                )
            {
                inner.send.captured = None;
            }
        }
        if result.is_ok()
            && inner
                .send
                .setup
                .as_ref()
                .is_some_and(|x| x.command_id == s.command_id)
        {
            inner.send.setup = None;
        }
        result
    }
    fn run_send(&self, s: &Submission, a: Admitted) -> Result<Sent, &'static str> {
        let timeouts = {
            let inner = self.inner.lock().unwrap();
            (
                inner.send.create_timeout,
                inner.send.session_timeout,
                inner.send.turn_timeout,
            )
        };
        if let Some(c) = a.retry {
            return post_turn(&c, timeouts.2);
        }
        let ready = a.request.ready.clone();
        let workspace = binding::workspace(&ready)?;
        let agent_slug = a
            .view
            .agent
            .as_deref()
            .ok_or("companion_agent_unavailable")?;
        binding::live_agent(&workspace, agent_slug)?;
        let agent = workspace
            .agents
            .iter()
            .find(|a| a.slug == agent_slug)
            .ok_or("companion_agent_unavailable")?;
        let (target, new_thread) = if let Some(t) = a.view.target.as_ref() {
            (
                binding::target(&ready, &workspace, agent_slug, &t.thread_id)?,
                false,
            )
        } else {
            let create_id = Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("companion-thread:{}", s.command_id).as_bytes(),
            );
            let created = call(
                &ready,
                reqwest::Method::POST,
                &format!("/api/projects/{}/threads", workspace.project_id),
                Some(
                    json!({"command_id":create_id,"title":"New conversation","harness":"aylul","profile":agent_slug}),
                ),
                timeouts.0,
            )?;
            let id = created["id"].as_str().ok_or("companion_send_uncertain")?;
            let target = binding::target(&ready, &workspace, agent_slug, id)?;
            self.commit_with_origin(a.request, |v| v.target = Some(target.clone()), true)
                .map_err(|_| "companion_send_stale")?;
            (target, true)
        };
        let default_model = {
            let mut inner = self.inner.lock().unwrap();
            if new_thread && inner.send.scope.as_deref() == Some(&a.scope) {
                inner.send.setup = Some(Setup {
                    command_id: s.command_id.clone(),
                    scope: a.scope.clone(),
                    thread_id: target.thread_id.clone(),
                    default_model: agent.default_model.clone(),
                });
            }
            inner
                .send
                .setup
                .as_ref()
                .filter(|x| {
                    x.command_id == s.command_id
                        && x.scope == a.scope
                        && x.thread_id == target.thread_id
                })
                .and_then(|x| x.default_model.clone())
        };
        let base = format!("/api/threads/{}", target.thread_id);
        let mut session = call(
            &ready,
            reqwest::Method::POST,
            &format!("{base}/session"),
            None,
            timeouts.1,
        )?;
        if let Some(default) = default_model {
            if session["current"]["model"].as_str() != Some(&default) {
                session = call(
                    &ready,
                    reqwest::Method::PUT,
                    &format!("{base}/session/model"),
                    Some(json!({"model":default})),
                    timeouts.1,
                )?;
            }
        }
        let current = &session["current"];
        if !current["model"].is_string()
            || !current["mode"].is_string()
            || !(current["effort"].is_null() || current["effort"].is_string())
        {
            return Err("companion_send_failed");
        }
        let c = Captured {
            command_id: s.command_id.clone(),
            scope: a.scope.clone(),
            thread_id: target.thread_id,
            body: json!({"command_id":s.command_id,"prompt":s.text,"model":current["model"],"mode":current["mode"],"effort":current["effort"],"focus":null,"client_tab":null}),
            ready,
        };
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.send.scope.as_deref() == Some(&a.scope)
                && inner
                    .send
                    .captured
                    .as_ref()
                    .is_none_or(|old| old.command_id == s.command_id)
            {
                inner.send.captured = Some(c.clone());
            }
        }
        post_turn(&c, timeouts.2)
    }
}
fn post_turn(c: &Captured, timeout: Duration) -> Result<Sent, &'static str> {
    let response = call(
        &c.ready,
        reqwest::Method::POST,
        &format!("/api/threads/{}/turns", c.thread_id),
        Some(c.body.clone()),
        timeout,
    )?;
    let operation_id = response["operation_id"]
        .as_str()
        .ok_or("companion_send_uncertain")?;
    Ok(Sent {
        thread_id: c.thread_id.clone(),
        operation_id: operation_id.into(),
    })
}
fn call(
    ready: &ReadyConnection,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
    timeout: Duration,
) -> Result<Value, &'static str> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(|_| "companion_core_unavailable")?;
    let mut request = client.request(method, format!("{}{path}", ready.origin));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().map_err(|e| {
        if e.is_connect() {
            "companion_core_unavailable"
        } else {
            "companion_send_uncertain"
        }
    })?;
    let status = response.status();
    let mut bytes = Vec::new();
    response
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "companion_send_uncertain")?;
    if bytes.len() > 1_048_576 {
        return Err("companion_send_uncertain");
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "companion_send_uncertain")?;
    if status.is_success() {
        return Ok(value);
    }
    Err(match value["code"].as_str() {
        Some("THREAD_BUSY") => "companion_send_busy",
        Some(
            "AYLUL_KEY_MISSING"
            | "AYLUL_NOT_CONFIGURED"
            | "AYLUL_PROFILE_NOT_FOUND"
            | "AYLUL_PROVIDER_NOT_FOUND"
            | "RUNTIME_NOT_CONFIGURED",
        ) => "companion_send_setup",
        Some("MODE_NOT_ALLOWED" | "SETTING_NOT_OFFERED") => "companion_send_settings",
        _ => "companion_send_failed",
    })
}
