//! Agent validation and mutations.
use super::{Agent, AgentInput, Profiles, store};
use crate::{CoreError, app::user_command};

impl Profiles {
    pub async fn create_agent(
        &self,
        command_id: String,
        mut input: AgentInput,
    ) -> Result<Agent, CoreError> {
        input.name = input.name.trim().into();
        if input.name.is_empty() || input.name.len() > 256 {
            return Err(CoreError::ProfileInvalid("invalid agent name".into()));
        }
        let home = self
            .home_root
            .clone()
            .ok_or(CoreError::AylulNotConfigured)?;
        let command = user_command(command_id, "agent.create", serde_json::json!(input));
        let slug = store::save_agent(&self.storage, command, None, input, home).await?;
        self.agent(&slug).await
    }

    pub async fn update_agent(
        &self,
        command_id: String,
        slug: &str,
        mut input: AgentInput,
    ) -> Result<Agent, CoreError> {
        input.name = input.name.trim().into();
        if input.name.is_empty() || input.name.len() > 256 {
            return Err(CoreError::ProfileInvalid("invalid agent name".into()));
        }
        let home = self
            .home_root
            .clone()
            .ok_or(CoreError::AylulNotConfigured)?;
        let command = user_command(
            command_id,
            "agent.update",
            serde_json::json!({"slug":slug,"input":input}),
        );
        let slug =
            store::save_agent(&self.storage, command, Some(slug.into()), input, home).await?;
        self.agent(&slug).await
    }

    pub async fn delete_agent(&self, command_id: String, slug: &str) -> Result<(), CoreError> {
        let command = user_command(command_id, "agent.delete", serde_json::json!({"slug":slug}));
        Ok(store::delete_agent(&self.storage, command, slug).await?)
    }
}
