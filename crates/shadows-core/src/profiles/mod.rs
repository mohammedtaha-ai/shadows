//! AYLUL agent configuration.
mod agents;
mod model;
mod providers;
mod store;

use crate::db::Storage;
use crate::{CoreError, ProjectId};
pub use model::{Agent, AgentInput, AgentWorkspace, Provider, ProviderInput};
use std::path::PathBuf;
use std::sync::Arc;
pub(crate) use store::{require_live_agent_in, require_provider_in};

pub struct Profiles {
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
