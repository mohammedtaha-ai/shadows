use crate::core_connection::ReadyConnection;
use serde::{Deserialize, Serialize};
use std::io::Read;
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConversationTarget {
    pub project_id: String,
    pub thread_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingView {
    pub revision: u64,
    pub generation: u64,
    pub configuration_id: Option<String>,
    pub agent: Option<String>,
    pub target: Option<ConversationTarget>,
    pub previews: bool,
}
pub struct RequestContext {
    pub ticket: u64,
    pub generation: u64,
    pub ready: ReadyConnection,
}
#[derive(Deserialize)]
pub struct Workspace {
    pub project_id: String,
    pub agents: Vec<Agent>,
}
#[derive(Deserialize)]
pub struct Agent {
    pub slug: String,
    pub deleted: bool,
}
#[derive(Deserialize)]
pub struct Thread {
    pub id: String,
    pub project_id: String,
    pub harness: String,
    pub profile: Option<String>,
    pub removed_at: Option<String>,
}
pub fn read<T: serde::de::DeserializeOwned>(
    ready: &ReadyConnection,
    path: &str,
) -> Result<T, &'static str> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|_| "companion_core_unavailable")?;
    let response = client
        .get(format!("{}{path}", ready.origin))
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
pub fn workspace(ready: &ReadyConnection) -> Result<Workspace, &'static str> {
    #[derive(Deserialize)]
    struct Identity {
        runtime_id: String,
        configuration_id: String,
    }
    let identity: Identity = read(ready, "/api/runtime")?;
    if identity.runtime_id != ready.runtime_id
        || identity.configuration_id != ready.configuration_id
    {
        return Err("companion_core_changed");
    }
    read(ready, "/api/aylul/agents")
}
pub fn live_agent(workspace: &Workspace, agent: &str) -> Result<(), &'static str> {
    if workspace
        .agents
        .iter()
        .any(|item| item.slug == agent && !item.deleted)
    {
        Ok(())
    } else {
        Err("companion_agent_unavailable")
    }
}
pub fn target(
    ready: &ReadyConnection,
    workspace: &Workspace,
    agent: &str,
    id: &str,
) -> Result<ConversationTarget, &'static str> {
    uuid::Uuid::parse_str(id).map_err(|_| "companion_thread_invalid")?;
    let thread: Thread = read(ready, &format!("/api/threads/{id}"))?;
    if thread.id != id
        || thread.project_id != workspace.project_id
        || thread.harness != "aylul"
        || thread.profile.as_deref() != Some(agent)
        || thread.removed_at.is_some()
    {
        return Err("companion_thread_invalid");
    }
    Ok(ConversationTarget {
        project_id: thread.project_id,
        thread_id: thread.id,
    })
}
