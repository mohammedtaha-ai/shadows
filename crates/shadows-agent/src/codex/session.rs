//! Opening a Planner thread and reading its native model choices.
use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Connection, GRANT_ENV, MODE, planner_server};

/// The Planner's shell writes inside the project, and Codex asks the person
/// (through Shadows' held request) when it judges a command needs it.
const SANDBOX: &str = "workspace-write";
const APPROVAL: &str = "on-request";
use crate::TurnSettings;
use crate::choices::{Choice, Offered, OptionIds};
use crate::connection::{ConnectionError, Opened, SessionSetup, SessionStart};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Effort {
    reasoning_effort: String,
    description: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Model {
    model: String,
    display_name: String,
    description: String,
    default_reasoning_effort: String,
    supported_reasoning_efforts: Vec<Effort>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPage {
    data: Vec<Model>,
    next_cursor: Option<String>,
}

pub(super) struct Selection {
    thread: String,
    catalog: Vec<Model>,
    current: TurnSettings,
}

impl Selection {
    fn model(&self, id: &str) -> Option<&Model> {
        self.catalog.iter().find(|m| m.model == id)
    }

    fn offered(&self) -> Result<Offered, ConnectionError> {
        let current = self
            .model(&self.current.model)
            .ok_or(ConnectionError::Protocol(
                "Codex selected a model outside its catalog",
            ))?;
        Ok(Offered {
            models: self
                .catalog
                .iter()
                .map(|m| choice(&m.model, &m.display_name, Some(m.description.clone())))
                .collect(),
            efforts: current
                .supported_reasoning_efforts
                .iter()
                .map(|e| {
                    choice(
                        &e.reasoning_effort,
                        &e.reasoning_effort,
                        Some(e.description.clone()),
                    )
                })
                .collect(),
            modes: crate::policy::modes(crate::policy::CODEX)
                .iter()
                .map(|m| choice(m.id, m.label, Some(m.description.into())))
                .collect(),
            current: self.current.clone(),
            ids: OptionIds {
                model: "model".into(),
                effort: Some("effort".into()),
                mode: "mode".into(),
            },
        })
    }
}

fn choice(id: &str, label: &str, description: Option<String>) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        description,
        enabled: true,
        reason: None,
    }
}

impl Connection {
    pub async fn start_session(
        &self,
        cwd: &Path,
        how: SessionStart,
        setup: &SessionSetup,
        permitted_tools: &[&str],
    ) -> Result<Opened, ConnectionError> {
        let (url, _) = planner_server(setup)?;
        let preflight: Value = self
            .rpc
            .request(
                "config/read",
                json!({
                    "cwd": cwd, "includeLayers": false
                }),
            )
            .await?;
        match preflight.pointer("/config/mcp_servers") {
            Some(Value::Object(servers)) if servers.is_empty() => {}
            None if preflight["config"].is_object() => {}
            _ => {
                return Err(ConnectionError::Rpc(
                    "The dedicated Codex home contains unexpected MCP configuration".into(),
                ));
            }
        }
        let account: Value = self
            .rpc
            .request("account/read", json!({"refreshToken": false}))
            .await?;
        if account.pointer("/account/type").and_then(Value::as_str) != Some("chatgpt") {
            return Err(ConnectionError::Rpc(
                "Sign in to Codex in the dedicated Shadows home".into(),
            ));
        }
        let approvals: serde_json::Map<String, Value> = permitted_tools
            .iter()
            .filter(|name| !name.ends_with("_approve"))
            .map(|name| (name.to_string(), json!({"approval_mode":"approve"})))
            .collect();
        let mut params = json!({
            "cwd": cwd,
            "sandbox": SANDBOX,
            "approvalPolicy": APPROVAL,
            "developerInstructions": setup.append,
            "config": {
                "mcp_servers.shadows": {
                    "url": url, "required": true, "enabled_tools": permitted_tools,
                    "bearer_token_env_var": GRANT_ENV,
                    "default_tools_approval_mode": "prompt",
                    "tools": approvals,
                }
            }
        });
        let method = match &how {
            SessionStart::New => "thread/start",
            SessionStart::Resume(id) => {
                params["threadId"] = json!(id);
                params["excludeTurns"] = json!(true);
                "thread/resume"
            }
            // The source's history is copied into a new native thread; the
            // source itself is never resumed or written by the fork.
            SessionStart::Fork(source) => {
                params["threadId"] = json!(source);
                params["excludeTurns"] = json!(true);
                "thread/fork"
            }
        };
        let opened: Value = self.rpc.request(method, params).await?;
        let thread = opened.pointer("/thread/id").and_then(Value::as_str).ok_or(
            ConnectionError::Protocol("Codex did not return a thread ID"),
        )?;
        match &how {
            SessionStart::Resume(expected) if expected != thread => {
                return Err(ConnectionError::Protocol(
                    "Codex resumed a different thread",
                ));
            }
            SessionStart::Fork(source) if source == thread => {
                return Err(ConnectionError::Protocol(
                    "Codex answered a fork with its source thread",
                ));
            }
            _ => {}
        }
        // Account windows are a display fact read from the provider, which
        // takes seconds: the opening neither waits for them nor fails without.
        let (rpc, prompts) = (self.rpc.clone(), self.prompts.clone());
        tokio::spawn(async move {
            if let Ok(limits) = rpc
                .request::<Value>("account/rateLimits/read", Value::Null)
                .await
            {
                prompts.first_limits(&limits["rateLimits"]);
            }
        });
        if opened.pointer("/sandbox/type").and_then(Value::as_str) != Some("workspaceWrite")
            || opened["approvalPolicy"].as_str() != Some(APPROVAL)
        {
            return Err(ConnectionError::Protocol(
                "Codex did not accept the Planner sandbox policy",
            ));
        }
        self.verify_tools(thread, permitted_tools).await?;
        let catalog = self.catalog().await?;
        let model = opened["model"].as_str().ok_or(ConnectionError::Protocol(
            "Codex did not report its selected model",
        ))?;
        // A thread opened without an effort runs at its model's default.
        let effort = opened["reasoningEffort"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| {
                catalog
                    .iter()
                    .find(|entry| entry.model == model)
                    .map(|entry| entry.default_reasoning_effort.clone())
            });
        let selection = Selection {
            thread: thread.into(),
            catalog,
            current: TurnSettings {
                model: model.into(),
                mode: MODE.into(),
                effort,
            },
        };
        let options = selection.offered()?;
        *self.selection.lock().unwrap() = Some(selection);
        Ok(Opened {
            session_id: thread.into(),
            options,
        })
    }

    async fn verify_tools(&self, thread: &str, permitted: &[&str]) -> Result<(), ConnectionError> {
        let inventory: Value = self
            .rpc
            .request(
                "mcpServerStatus/list",
                json!({
                    "threadId": thread, "detail": "full"
                }),
            )
            .await?;
        let statuses = inventory["data"]
            .as_array()
            .ok_or(ConnectionError::Protocol(
                "Codex did not return its MCP inventory",
            ))?;
        if !inventory["nextCursor"].is_null() {
            return Err(ConnectionError::Protocol(
                "Codex returned unexpected MCP inventory pages",
            ));
        }
        let mut shadows = false;
        for server in statuses {
            let tools = server["tools"]
                .as_object()
                .ok_or(ConnectionError::Protocol(
                    "Codex returned an invalid MCP tool list",
                ))?;
            let empty = |name: &str| server[name].as_array().is_some_and(Vec::is_empty);
            if !empty("resources") || !empty("resourceTemplates") {
                return Err(ConnectionError::Protocol(
                    "Codex exposed unexpected MCP resources",
                ));
            }
            match server["name"].as_str() {
                Some("shadows")
                    if !shadows
                        && tools.len() == permitted.len()
                        && permitted.iter().all(|name| tools.contains_key(*name)) =>
                {
                    shadows = true
                }
                _ => {
                    return Err(ConnectionError::Protocol(
                        "Codex exposed unexpected MCP tools",
                    ));
                }
            }
        }
        if !shadows {
            return Err(ConnectionError::Protocol(
                "Codex did not initialize the required Shadows server",
            ));
        }
        Ok(())
    }

    async fn catalog(&self) -> Result<Vec<Model>, ConnectionError> {
        let mut cursor = None;
        let mut catalog = Vec::new();
        let mut seen = std::collections::HashSet::new();
        loop {
            let page: ModelPage = self
                .rpc
                .request(
                    "model/list",
                    json!({
                        "cursor": cursor, "includeHidden": false
                    }),
                )
                .await?;
            catalog.extend(page.data);
            let Some(next) = page.next_cursor else {
                return Ok(catalog);
            };
            if !seen.insert(next.clone()) {
                return Err(ConnectionError::Protocol(
                    "Codex repeated a model catalog page",
                ));
            }
            cursor = Some(next);
        }
    }

    pub fn set_option(
        &self,
        thread: &str,
        id: &str,
        value: &str,
    ) -> Result<Offered, ConnectionError> {
        let mut selection = self.selection.lock().unwrap();
        let selection = selection
            .as_mut()
            .filter(|s| s.thread == thread)
            .ok_or(ConnectionError::Closed)?;
        let unavailable = || ConnectionError::Rpc("Codex setting is not offered".into());
        match id {
            "model" => {
                let model = selection.model(value).ok_or_else(unavailable)?;
                let effort = model
                    .supported_reasoning_efforts
                    .iter()
                    .find(|e| e.reasoning_effort == model.default_reasoning_effort)
                    .map(|e| e.reasoning_effort.clone());
                selection.current.model = value.into();
                selection.current.effort = effort;
            }
            "effort" => {
                let model = selection
                    .model(&selection.current.model)
                    .ok_or_else(unavailable)?;
                if !model
                    .supported_reasoning_efforts
                    .iter()
                    .any(|e| e.reasoning_effort == value)
                {
                    return Err(unavailable());
                }
                selection.current.effort = Some(value.into());
            }
            "mode" if value == MODE => {}
            _ => return Err(unavailable()),
        }
        selection.offered()
    }

    pub(super) fn settings(&self, thread: &str) -> Result<TurnSettings, ConnectionError> {
        self.selection
            .lock()
            .unwrap()
            .as_ref()
            .filter(|s| s.thread == thread)
            .map(|s| s.current.clone())
            .ok_or(ConnectionError::Closed)
    }
}
