//! Blocking Core requests: binding and navigation reads, and submission writes.
use super::RuntimeIdentity;
use crate::{
    core_connection::ReadyConnection,
    presence::binding::{Thread, Workspace},
};
use serde_json::{json, Value};
use std::{io::Read, time::Duration};
use uuid::Uuid;

pub struct Blocking<'a> {
    ready: &'a ReadyConnection,
}
impl<'a> Blocking<'a> {
    pub fn new(ready: &'a ReadyConnection) -> Self {
        Self { ready }
    }
    fn read<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, &'static str> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .map_err(|_| "companion_core_unavailable")?;
        let response = client
            .get(format!("{}{path}", self.ready.origin))
            .send()
            .map_err(|_| "companion_core_unavailable")?;
        if !response.status().is_success() {
            return Err("companion_core_unavailable");
        }
        let mut bytes = Vec::new();
        response
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| "companion_core_unavailable")?;
        if bytes.len() > 1_048_576 {
            return Err("companion_response_invalid");
        }
        serde_json::from_slice(&bytes).map_err(|_| "companion_response_invalid")
    }
    pub fn runtime(&self) -> Result<RuntimeIdentity, &'static str> {
        self.read("/api/runtime")
    }
    pub fn workspace(&self) -> Result<Workspace, &'static str> {
        self.read("/api/aylul/agents")
    }
    pub fn thread(&self, id: &str) -> Result<Thread, &'static str> {
        self.read(&format!("/api/threads/{id}"))
    }
    pub fn create_thread(
        &self,
        project_id: &str,
        command_id: Uuid,
        profile: &str,
        timeout: Duration,
    ) -> Result<Value, &'static str> {
        self.call(
            reqwest::Method::POST,
            &format!("/api/projects/{project_id}/threads"),
            Some(
                json!({"command_id":command_id,"title":"New conversation","harness":"aylul","profile":profile}),
            ),
            timeout,
        )
    }
    pub fn open_session(&self, thread_id: &str, timeout: Duration) -> Result<Value, &'static str> {
        self.call(
            reqwest::Method::POST,
            &format!("/api/threads/{thread_id}/session"),
            None,
            timeout,
        )
    }
    pub fn set_model(
        &self,
        thread_id: &str,
        model: &str,
        timeout: Duration,
    ) -> Result<Value, &'static str> {
        self.call(
            reqwest::Method::PUT,
            &format!("/api/threads/{thread_id}/session/model"),
            Some(json!({"model":model})),
            timeout,
        )
    }
    /// The body is the captured request, so a retry sends exactly the same bytes.
    pub fn post_turn(
        &self,
        thread_id: &str,
        body: Value,
        timeout: Duration,
    ) -> Result<Value, &'static str> {
        self.call(
            reqwest::Method::POST,
            &format!("/api/threads/{thread_id}/turns"),
            Some(body),
            timeout,
        )
    }
    fn call(
        &self,
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
        let mut request = client.request(method, format!("{}{path}", self.ready.origin));
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
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| "companion_send_uncertain")?;
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
}
