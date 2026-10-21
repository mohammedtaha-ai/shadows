//! Agent validation and mutations.
use super::{Agent, AgentInput, Profiles, store};
use crate::CoreError;
use crate::db::CommandContext;

impl Profiles {
    pub async fn create_agent(
        &self,
        command_id: String,
        mut input: AgentInput,
    ) -> Result<Agent, CoreError> {
        if input
            .character_color
            .as_deref()
            .is_some_and(|color| !valid_color(color))
        {
            return Err(CoreError::ProfileInvalid("invalid character color".into()));
        }
        input.name = input.name.trim().into();
        if input.name.is_empty() || input.name.len() > 256 {
            return Err(CoreError::ProfileInvalid("invalid agent name".into()));
        }
        let home = self
            .home_root
            .clone()
            .ok_or(CoreError::AylulNotConfigured)?;
        let slug = store::save_agent(&self.storage, command_id, None, input, home).await?;
        self.agent(&slug).await
    }

    pub async fn update_agent(
        &self,
        command_id: String,
        slug: &str,
        mut input: AgentInput,
    ) -> Result<Agent, CoreError> {
        if input
            .character_color
            .as_deref()
            .is_some_and(|color| !valid_color(color))
        {
            return Err(CoreError::ProfileInvalid("invalid character color".into()));
        }
        input.name = input.name.trim().into();
        if input.name.is_empty() || input.name.len() > 256 {
            return Err(CoreError::ProfileInvalid("invalid agent name".into()));
        }
        let home = self
            .home_root
            .clone()
            .ok_or(CoreError::AylulNotConfigured)?;
        let slug =
            store::save_agent(&self.storage, command_id, Some(slug.into()), input, home).await?;
        self.agent(&slug).await
    }

    /// The stored part of deleting an agent: its identity is kept as a
    /// tombstone and its variables go, in one transaction. Its live adapters
    /// are the harness's, which calls this before closing them.
    pub(crate) async fn delete_stored_agent(
        &self,
        command_id: String,
        slug: &str,
    ) -> Result<(), CoreError> {
        let command =
            CommandContext::user(command_id, "agent.delete", serde_json::json!({"slug":slug}));
        store::delete_agent(&self.storage, command, slug).await?;
        Ok(())
    }
}

fn valid_color(color: &str) -> bool {
    let bytes = color.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}
