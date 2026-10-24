//! AYLUL agents: permanent identities, their homes, their environment and who uses a
//! provider.
mod error;
mod home;
mod model;
mod store;

pub(crate) use error::AgentsError;
pub use model::{Agent, AgentInput, AgentWorkspace};

use crate::projects::Projects;
use crate::{CoreError, ProjectId};
use core_db::{CommandContext, Storage};
use core_providers::{Provider, ProviderInput, Providers};
use core_secrets::{EnvEntry, Secrets};
use sqlx::SqliteConnection;
use std::path::PathBuf;
use std::sync::Arc;
use store::{AgentsStore, COMMAND_SCOPE};

pub struct Agents {
    store: AgentsStore,
    providers: Arc<Providers>,
    secrets: Arc<Secrets>,
    projects: Projects,
    home_root: Option<PathBuf>,
}

impl Agents {
    pub(crate) fn new(
        storage: Arc<Storage>,
        providers: Arc<Providers>,
        secrets: Arc<Secrets>,
        projects: Projects,
        home_root: Option<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store: AgentsStore::new(storage),
            providers,
            secrets,
            projects,
            home_root,
        })
    }

    pub async fn agents(&self) -> Result<Vec<Agent>, CoreError> {
        Ok(self.store.list().await?)
    }

    pub async fn agent(&self, slug: &str) -> Result<Agent, CoreError> {
        Ok(self.store.get(slug).await?)
    }

    pub async fn workspace(&self) -> Result<AgentWorkspace, CoreError> {
        Ok(AgentWorkspace {
            project_id: self.assistant_project().await?,
            agents: self.agents().await?,
        })
    }

    pub async fn assistant_project(&self) -> Result<ProjectId, CoreError> {
        self.projects
            .by_slug("aylul")
            .await?
            .filter(|p| p.assistant)
            .map(|p| p.id)
            .ok_or(CoreError::AylulNotConfigured)
    }

    pub async fn create_agent(
        &self,
        command_id: String,
        input: AgentInput,
    ) -> Result<Agent, CoreError> {
        let input = Self::validated(input)?;
        let root = self.root()?;
        let slug = self.store.save(command_id, None, input, root).await?;
        self.agent(&slug).await
    }

    pub async fn update_agent(
        &self,
        command_id: String,
        slug: &str,
        input: AgentInput,
    ) -> Result<Agent, CoreError> {
        let input = Self::validated(input)?;
        let root = self.root()?;
        let slug = self
            .store
            .save(command_id, Some(slug.into()), input, root)
            .await?;
        self.agent(&slug).await
    }

    /// Replaces a provider's configuration, refusing to remove a live agent's default
    /// model, in one write.
    pub async fn update_provider(
        &self,
        command_id: String,
        slug: &str,
        input: ProviderInput,
    ) -> Result<Provider, CoreError> {
        let input = self.providers.validated(input)?;
        let command = CommandContext::user(
            command_id,
            "provider.update",
            serde_json::json!({"slug": slug, "input": input}),
        );
        let slug = slug.to_owned();
        let slug = self
            .store
            .write(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        CommandContext::classify(conn, &command, COMMAND_SCOPE.0, COMMAND_SCOPE.1)
                            .await?
                    {
                        return Ok(id);
                    }
                    Providers::models_in(conn, &slug).await?;
                    let ids: Vec<_> = input.models.iter().map(|m| m.id.clone()).collect();
                    AgentsStore::users_in(conn, &slug, Some(&ids)).await?;
                    Providers::update_in(conn, &slug, &input).await?;
                    AgentsStore::record_in(conn, &command, &slug).await?;
                    Ok(slug)
                })
            })
            .await?;
        self.providers.provider(&slug).await.map_err(Into::into)
    }

    /// Removes a provider no live agent uses, with its key, in one write.
    pub async fn delete_provider(&self, command_id: String, slug: &str) -> Result<(), CoreError> {
        let command = CommandContext::user(
            command_id,
            "provider.delete",
            serde_json::json!({"slug":slug}),
        );
        let (providers, slug) = (self.providers.clone(), slug.to_owned());
        self.store
            .write(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &command, COMMAND_SCOPE.0, COMMAND_SCOPE.1)
                        .await?
                        .is_some()
                    {
                        return Ok(());
                    }
                    Providers::models_in(conn, &slug).await?;
                    AgentsStore::users_in(conn, &slug, None).await?;
                    providers.delete_in(conn, &slug).await?;
                    Ok(AgentsStore::record_in(conn, &command, &slug).await?)
                })
            })
            .await
    }

    pub async fn list_env(&self, slug: &str) -> Result<Vec<EnvEntry>, CoreError> {
        self.store.require_live(slug).await?;
        self.secrets.list_env(slug).await.map_err(Into::into)
    }

    /// Stores one of a live agent's variables, in one write.
    pub async fn set_env(
        &self,
        command_id: String,
        slug: &str,
        name: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        let write = self.secrets.env_write(command_id, slug, name, value)?;
        let slug = slug.to_owned();
        self.store
            .write(move |conn| {
                Box::pin(async move {
                    if let Some(entry) = Secrets::replayed_write_in(conn, &write).await? {
                        return Ok(entry);
                    }
                    AgentsStore::require_live_in(conn, &slug).await?;
                    Ok(Secrets::put_in(conn, write).await?)
                })
            })
            .await
    }

    /// Removes one of a live agent's variables, in one write.
    pub async fn remove_env(
        &self,
        command_id: String,
        slug: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        let removal = self.secrets.env_removal(command_id, slug, name)?;
        let slug = slug.to_owned();
        self.store
            .write(move |conn| {
                Box::pin(async move {
                    if Secrets::replayed_removal_in(conn, &removal).await? {
                        return Ok(());
                    }
                    AgentsStore::require_live_in(conn, &slug).await?;
                    Ok(Secrets::remove_in(conn, removal).await?)
                })
            })
            .await
    }

    /// The agent's isolated home, where its sessions run.
    pub(crate) fn home(&self, slug: &str) -> Result<PathBuf, CoreError> {
        Ok(self.root()?.join("profiles").join(slug))
    }

    /// Refuses an unknown or deleted agent inside the caller's write.
    pub(crate) async fn require_live_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), AgentsError> {
        AgentsStore::require_live_in(conn, slug).await
    }

    /// Tombstones a live agent and removes its variables inside the caller's write,
    /// under its own command. The caller removes what other owners keep for the agent.
    pub(crate) async fn delete_in(
        &self,
        conn: &mut SqliteConnection,
        command_id: String,
        slug: &str,
    ) -> Result<(), CoreError> {
        let command =
            CommandContext::user(command_id, "agent.delete", serde_json::json!({"slug":slug}));
        if CommandContext::classify(conn, &command, COMMAND_SCOPE.0, COMMAND_SCOPE.1)
            .await?
            .is_some()
        {
            return Ok(());
        }
        AgentsStore::require_live_in(conn, slug).await?;
        AgentsStore::tombstone_in(conn, slug).await?;
        self.secrets.remove_scope_in(conn, slug).await?;
        Ok(AgentsStore::record_in(conn, &command, slug).await?)
    }

    fn root(&self) -> Result<PathBuf, CoreError> {
        self.home_root.clone().ok_or(CoreError::AylulNotConfigured)
    }

    fn validated(mut input: AgentInput) -> Result<AgentInput, CoreError> {
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
        Ok(input)
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
