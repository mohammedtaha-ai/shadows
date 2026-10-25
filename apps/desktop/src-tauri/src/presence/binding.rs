use super::core_client::Blocking;
use crate::core_connection::ReadyConnection;
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    pub default_model: Option<String>,
}
#[derive(Deserialize)]
pub struct Thread {
    pub id: String,
    pub project_id: String,
    pub harness: String,
    pub profile: Option<String>,
    pub removed_at: Option<String>,
}
pub fn workspace(ready: &ReadyConnection) -> Result<Workspace, &'static str> {
    runtime(ready)?;
    Blocking::new(ready).workspace()
}
pub fn runtime(ready: &ReadyConnection) -> Result<(), &'static str> {
    let identity = Blocking::new(ready).runtime()?;
    if identity.runtime_id != ready.runtime_id
        || identity.configuration_id != ready.configuration_id
    {
        return Err("companion_core_changed");
    }
    Ok(())
}
/// Binding and observation accept the same agents: listed and not deleted.
pub fn eligible_agent(slug: &str, deleted: bool, agent: &str) -> bool {
    slug == agent && !deleted
}
impl Thread {
    /// Binding and observation accept the same threads: this project's AYLUL
    /// thread, run by the bound agent, not removed.
    pub fn eligible(&self, id: &str, project_id: &str, agent: &str) -> bool {
        self.id == id
            && self.project_id == project_id
            && self.harness == "aylul"
            && self.profile.as_deref() == Some(agent)
            && self.removed_at.is_none()
    }
}
pub fn live_agent(workspace: &Workspace, agent: &str) -> Result<(), &'static str> {
    if workspace
        .agents
        .iter()
        .any(|item| eligible_agent(&item.slug, item.deleted, agent))
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
    let thread = Blocking::new(ready).thread(id)?;
    if !thread.eligible(id, &workspace.project_id, agent) {
        return Err("companion_thread_invalid");
    }
    Ok(ConversationTarget {
        project_id: thread.project_id,
        thread_id: thread.id,
    })
}
