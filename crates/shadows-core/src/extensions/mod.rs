//! The shared extension library and each agent's switches over it.
mod error;
mod files;
mod model;
mod store;

pub(crate) use error::ExtensionsError;
pub use model::{AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Item};

use crate::CoreError;
use crate::db::{CommandContext, Storage};
use sha2::{Digest, Sha256};
use sqlx::SqliteConnection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use store::ExtensionsStore;

pub struct Extensions {
    store: ExtensionsStore,
    /// Serializes library mutations against the harness's snapshot projection.
    lock: Arc<tokio::sync::Mutex<()>>,
    home_root: Option<PathBuf>,
}

impl Extensions {
    pub(crate) fn new(storage: Arc<Storage>, home_root: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            store: ExtensionsStore::new(storage),
            lock: Arc::default(),
            home_root,
        })
    }

    pub async fn extensions(&self) -> Result<ExtensionLibrary, CoreError> {
        Ok(self.store.library().await?)
    }

    pub async fn agent_extensions(&self, slug: &str) -> Result<AgentExtensions, CoreError> {
        Ok(self.store.agent_library(slug).await?)
    }

    pub async fn add_extension(
        &self,
        command_id: String,
        kind: ExtensionKind,
        path: &Path,
    ) -> Result<Item, CoreError> {
        let _guard = self.extension_guard().await;
        if !path.is_absolute() {
            return Err(CoreError::ProfileInvalid(
                "extension source must be absolute".into(),
            ));
        }
        let mut source = PathBuf::new();
        for part in path.components() {
            match part {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    source.pop();
                }
                other => source.push(other.as_os_str()),
            }
        }
        let locator = format!("{:x}", Sha256::digest(source.to_string_lossy().as_bytes()));
        let c = CommandContext::user(
            command_id,
            "extension.add",
            serde_json::json!({"kind":kind,"locator":locator}),
        );
        let root = self
            .home_root
            .as_ref()
            .ok_or(CoreError::AylulNotConfigured)?
            .join("library");
        Ok(self.store.add(c, kind, source, root).await?)
    }

    pub async fn delete_extension(
        &self,
        command_id: String,
        kind: ExtensionKind,
        id: &str,
    ) -> Result<(), CoreError> {
        let _guard = self.extension_guard().await;
        let c = CommandContext::user(
            command_id,
            "extension.delete",
            serde_json::json!({"kind":kind,"id":id}),
        );
        self.store.remove(c, kind, id).await?;
        if let Some(root) = &self.home_root {
            Self::retire(&root.join("library"), id);
        }
        Ok(())
    }

    pub async fn set_agent_extension(
        &self,
        command_id: String,
        slug: &str,
        kind: ExtensionKind,
        id: &str,
        enabled: bool,
    ) -> Result<AgentItem, CoreError> {
        let _guard = self.extension_guard().await;
        let c = CommandContext::user(
            command_id,
            "extension.toggle",
            serde_json::json!({
            "slug":slug,"kind":kind,"id":id,"enabled":enabled}),
        );
        Ok(self.store.toggle(c, slug, kind, id, enabled).await?)
    }

    /// Held while the harness projects the library into a home.
    pub(crate) async fn extension_guard(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.lock.clone().lock_owned().await
    }

    /// Removes every switch of an agent inside the caller's write.
    pub(crate) async fn remove_agent_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), ExtensionsError> {
        ExtensionsStore::remove_agent_in(conn, slug).await
    }
}
