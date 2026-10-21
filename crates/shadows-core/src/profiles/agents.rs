//! Agent validation and mutations.
use super::{Agent, AgentInput, Profiles, store};
use crate::{CoreError, app::user_command};

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

    pub async fn delete_agent(&self, command_id: String, slug: &str) -> Result<(), CoreError> {
        let command = user_command(command_id, "agent.delete", serde_json::json!({"slug":slug}));
        store::delete_agent(&self.storage, command, slug).await?;
        if let Some(sessions) = self.sessions.upgrade() {
            for thread in self.storage.list_threads_for_profile(slug).await? {
                if let Err(error) = sessions.terminate(&thread.id).await {
                    tracing::warn!(thread_id=%thread.id, %error, "agent.adapter_close_failed");
                }
            }
        }
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
