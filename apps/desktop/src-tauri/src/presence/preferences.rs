use super::binding::{BindingView, ConversationTarget};
use crate::core_connection::config::{read_json, write_json};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Preferences {
    configuration_id: Option<String>,
    agent: Option<String>,
    target: Option<ConversationTarget>,
    previews: bool,
}
pub fn load(root: &Path, configuration_id: Option<String>) -> Result<BindingView, &'static str> {
    let saved = read_json::<Preferences>(&root.join("desktop-presence.json"))
        .map_err(|_| "companion_preferences_invalid")?;
    let mut view = BindingView {
        revision: 0,
        generation: 1,
        configuration_id,
        agent: None,
        target: None,
        previews: true,
    };
    if let Some(saved) = saved {
        if saved.configuration_id == view.configuration_id {
            if saved
                .agent
                .as_ref()
                .is_some_and(|agent| agent.is_empty() || agent.len() > 128)
                || saved.target.as_ref().is_some_and(|target| {
                    saved.agent.is_none()
                        || uuid::Uuid::parse_str(&target.project_id).is_err()
                        || uuid::Uuid::parse_str(&target.thread_id).is_err()
                })
            {
                return Err("companion_preferences_invalid");
            }
            view.agent = saved.agent;
            view.target = saved.target;
            view.previews = saved.previews;
        }
    }
    Ok(view)
}
pub fn save(root: &Path, view: &BindingView) -> Result<(), &'static str> {
    write_json(
        &root.join("desktop-presence.json"),
        &Preferences {
            configuration_id: view.configuration_id.clone(),
            agent: view.agent.clone(),
            target: view.target.clone(),
            previews: view.previews,
        },
    )
    .map_err(|_| "companion_preferences_unavailable")
}
