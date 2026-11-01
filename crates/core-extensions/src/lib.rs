//! The shared extension library and each agent's switches over it.
mod bundled;
mod error;
mod files;
mod mcp;
mod model;
mod store;

pub use error::ExtensionsError;
pub use mcp::McpServerConfig;
pub use model::{
    AgentExtensions, AgentItem, AgentMcpItem, ExtensionKind, ExtensionLibrary, Item, McpInput,
    McpItem, McpTransport, McpTransportInput, McpValue, McpValueInput,
};

use core_db::{CommandContext, Storage};
use sha2::{Digest, Sha256};
use sqlx::SqliteConnection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use store::ExtensionsStore;

pub struct Extensions {
    store: ExtensionsStore,
    secrets: Arc<core_secrets::Secrets>,
    /// Serializes library mutations against the harness's snapshot projection.
    lock: Arc<tokio::sync::Mutex<()>>,
    home_root: Option<PathBuf>,
}

impl Extensions {
    pub fn new(
        storage: Arc<Storage>,
        secrets: Arc<core_secrets::Secrets>,
        home_root: Option<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store: ExtensionsStore::new(storage),
            secrets,
            lock: Arc::default(),
            home_root,
        })
    }

    pub async fn extensions(&self) -> Result<ExtensionLibrary, ExtensionsError> {
        self.store.library().await
    }

    pub async fn agent_extensions(&self, slug: &str) -> Result<AgentExtensions, ExtensionsError> {
        self.store.agent_library(slug).await
    }

    pub async fn add_extension(
        &self,
        command_id: String,
        kind: ExtensionKind,
        path: &Path,
    ) -> Result<Item, ExtensionsError> {
        let _guard = self.extension_guard().await;
        if kind == ExtensionKind::Mcp {
            return Err(ExtensionsError::Invalid(
                "MCP items are configuration".into(),
            ));
        }
        if !path.is_absolute() {
            return Err(ExtensionsError::Invalid(
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
            .ok_or(ExtensionsError::NotConfigured)?
            .join("library");
        self.store.add(c, kind, source, root, false).await
    }

    pub async fn delete_extension(
        &self,
        command_id: String,
        kind: ExtensionKind,
        id: &str,
    ) -> Result<(), ExtensionsError> {
        let _guard = self.extension_guard().await;
        let c = CommandContext::user(
            command_id,
            "extension.delete",
            serde_json::json!({"kind":kind,"id":id}),
        );
        self.store.remove(c, kind, id, self.secrets.clone()).await?;
        if kind != ExtensionKind::Mcp
            && let Some(root) = &self.home_root
        {
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
    ) -> Result<AgentItem, ExtensionsError> {
        let _guard = self.extension_guard().await;
        let c = CommandContext::user(
            command_id,
            "extension.toggle",
            serde_json::json!({
            "slug":slug,"kind":kind,"id":id,"enabled":enabled}),
        );
        self.store.toggle(c, slug, kind, id, enabled).await
    }

    /// Held while the harness projects the library into a home.
    pub async fn extension_guard(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.lock.clone().lock_owned().await
    }

    /// Removes every switch of an agent inside the caller's write.
    pub async fn remove_agent_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), ExtensionsError> {
        ExtensionsStore::remove_agent_in(conn, slug).await
    }
}
