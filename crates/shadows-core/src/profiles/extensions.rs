//! Shared library service rules and command normalization.
use super::{
    AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Item, Profiles, extension_store,
};
use crate::CoreError;
use crate::db::CommandContext;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

impl Profiles {
    pub async fn extensions(&self) -> Result<ExtensionLibrary, CoreError> {
        Ok(extension_store::library(&self.storage).await?)
    }
    pub async fn agent_extensions(&self, slug: &str) -> Result<AgentExtensions, CoreError> {
        Ok(extension_store::agent_library(&self.storage, slug).await?)
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
        Ok(extension_store::add(&self.storage, c, kind, source, root).await?)
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
        extension_store::remove(&self.storage, c, kind, id).await?;
        if let Some(root) = &self.home_root {
            super::extension_files::retire(&root.join("library"), id);
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
        Ok(extension_store::toggle(&self.storage, c, slug, kind, id, enabled).await?)
    }
    pub(crate) async fn extension_guard(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.extension_lock.clone().lock_owned().await
    }
}
