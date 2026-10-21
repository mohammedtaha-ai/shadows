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
    sessions: std::sync::Weak<crate::harness::Sessions>,
}

impl Profiles {
    pub(crate) fn new(
        storage: Arc<Storage>,
        home_root: Option<PathBuf>,
        sessions: std::sync::Weak<crate::harness::Sessions>,
    ) -> Arc<Self> {
        Arc::new(Self {
            extension_lock: Arc::default(),
            storage,
            home_root,
            sessions,
        })
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
