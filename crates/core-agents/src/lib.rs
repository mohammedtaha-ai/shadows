//! AYLUL agents: permanent identities, their homes, their environment and who uses a
//! provider.
mod error;
mod home;
mod model;
mod store;

pub use error::AgentsError;
pub use model::{Agent, AgentInput, AgentWorkspace};

use core_db::{CommandContext, Storage};
use core_projects::ProjectId;
use core_projects::Projects;
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
    pub fn new(
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

    pub async fn agents(&self) -> Result<Vec<Agent>, AgentsError> {
        self.store.list().await
    }

    pub async fn agent(&self, slug: &str) -> Result<Agent, AgentsError> {
        self.store.get(slug).await
    }

    pub async fn workspace(&self) -> Result<AgentWorkspace, AgentsError> {
        Ok(AgentWorkspace {
            project_id: self.assistant_project().await?,
            agents: self.agents().await?,
        })
    }

    pub async fn assistant_project(&self) -> Result<ProjectId, AgentsError> {
        self.projects
            .by_slug("aylul")
            .await?
            .filter(|p| p.assistant)
            .map(|p| p.id)
            .ok_or(AgentsError::NotConfigured)
    }

    pub async fn create_agent(
        &self,
        command_id: String,
        input: AgentInput,
    ) -> Result<Agent, AgentsError> {
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
    ) -> Result<Agent, AgentsError> {
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
    ) -> Result<Provider, AgentsError> {
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
    pub async fn delete_provider(&self, command_id: String, slug: &str) -> Result<(), AgentsError> {
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

    pub async fn list_env(&self, slug: &str) -> Result<Vec<EnvEntry>, AgentsError> {
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
    ) -> Result<EnvEntry, AgentsError> {
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
    ) -> Result<(), AgentsError> {
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
    pub fn home(&self, slug: &str) -> Result<PathBuf, AgentsError> {
        Ok(self.root()?.join("profiles").join(slug))
    }

    /// Refuses an unknown or deleted agent inside the caller's write.
    pub async fn require_live_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), AgentsError> {
        AgentsStore::require_live_in(conn, slug).await
    }

    /// Tombstones a live agent and removes its variables inside the caller's write,
    /// under its own command. The caller removes what other owners keep for the agent.
    pub async fn delete_in(
        &self,
        conn: &mut SqliteConnection,
        command_id: String,
        slug: &str,
    ) -> Result<(), AgentsError> {
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

    fn root(&self) -> Result<PathBuf, AgentsError> {
        self.home_root.clone().ok_or(AgentsError::NotConfigured)
    }

    fn validated(mut input: AgentInput) -> Result<AgentInput, AgentsError> {
        if input
            .character_color
            .as_deref()
            .is_some_and(|color| !valid_id(color))
        {
            return Err(AgentsError::Invalid("invalid character color".into()));
        }
        if input
            .outfit
            .as_deref()
            .is_some_and(|outfit| !valid_id(outfit))
        {
            return Err(AgentsError::Invalid("invalid outfit".into()));
        }
        if let Some(description) = input.description.as_mut() {
            *description = description.trim().into();
            if description.chars().count() > 200 || description.contains(['\n', '\r']) {
                return Err(AgentsError::Invalid("invalid description".into()));
            }
        }
        if let Some(text) = input.instructions.as_mut() {
            if text.len() > 32 * 1024 || text.contains('\0') {
                return Err(AgentsError::Invalid("invalid instructions".into()));
            }
            if text == home::TEMPLATE {
                text.clear();
            }
        }
        input.name = input.name.trim().into();
        if input.name.is_empty() || input.name.len() > 256 {
            return Err(AgentsError::Invalid("invalid agent name".into()));
        }
        Ok(input)
    }
}

/// The id rule colours and outfits share.
fn valid_id(color: &str) -> bool {
    let bytes = color.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}
