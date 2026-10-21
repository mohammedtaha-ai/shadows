//! AYLUL agent configuration.
mod agents;
pub(crate) mod extension_files;
mod extension_model;
mod extension_store;
mod extensions;
pub use extension_model::{AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Item};
mod listing;
mod model;
mod person;
mod usage;
mod usage_store;
pub use usage::{ModelUsage, ProviderUsage, UsageReport};
mod presets;
pub(crate) mod probe;
mod providers;
pub use presets::{PresetRegion, ProviderPreset};
pub use probe::{ListedModel, ProviderTest, ProviderTestOutcome, SavedProviderTest, TestRefusal};
mod store;

use crate::db::Storage;
use crate::{CoreError, ProjectId};
pub use model::{
    Agent, AgentInput, AgentWorkspace, Person, PersonInput, Provider, ProviderInput, ProviderModel,
    ProviderModelInput, Theme,
};
use std::path::PathBuf;
use std::sync::Arc;
pub(crate) use store::{require_live_agent_in, require_provider_in};

pub struct Profiles {
    extension_lock: Arc<tokio::sync::Mutex<()>>,
    storage: Arc<Storage>,

    home_root: Option<PathBuf>,
    sessions: SessionsLink,
}

/// The sessions that close an agent's adapters. `Sessions` is built after
/// `Profiles`, so the link is filled once, by `Sessions`, when it is built.
#[derive(Default)]
struct SessionsLink(std::sync::OnceLock<std::sync::Weak<crate::harness::Sessions>>);

impl SessionsLink {
    fn upgrade(&self) -> Option<Arc<crate::harness::Sessions>> {
        self.0.get().and_then(std::sync::Weak::upgrade)
    }
}

impl Profiles {
    pub(crate) fn new(storage: Arc<Storage>, home_root: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            extension_lock: Arc::default(),
            storage,
            home_root,
            sessions: SessionsLink::default(),
        })
    }

    /// Called once by the `Sessions` built over these profiles; a second
    /// call keeps the first.
    pub(crate) fn attach_sessions(&self, sessions: std::sync::Weak<crate::harness::Sessions>) {
        let _ = self.sessions.0.set(sessions);
    }

    pub async fn providers(&self) -> Result<Vec<Provider>, CoreError> {
        Ok(store::providers(&self.storage).await?)
    }

    pub async fn provider(&self, slug: &str) -> Result<Provider, CoreError> {
        Ok(store::provider(&self.storage, slug).await?)
    }

    pub async fn agents(&self) -> Result<Vec<Agent>, CoreError> {
        Ok(store::agents(&self.storage).await?)
    }

    pub async fn agent(&self, slug: &str) -> Result<Agent, CoreError> {
        Ok(store::agent(&self.storage, slug).await?)
    }

    pub async fn workspace(&self) -> Result<AgentWorkspace, CoreError> {
        Ok(AgentWorkspace {
            project_id: self.assistant_project().await?,
            agents: self.agents().await?,
        })
    }

    pub async fn assistant_project(&self) -> Result<ProjectId, CoreError> {
        self.storage
            .list_projects()
            .await?
            .into_iter()
            .find(|p| p.slug == "aylul" && p.assistant)
            .map(|p| p.id)
            .ok_or(CoreError::AylulNotConfigured)
    }
}
