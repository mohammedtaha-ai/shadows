//! The encrypted profile environment. Only the harness resolves values.
mod crypto;
#[cfg(test)]
mod crypto_tests;
mod model;
mod store;

use std::sync::Arc;

use crate::CoreError;
use crate::db::CommandContext;
use crate::db::Storage;
use crypto::Keys;
pub use model::{EnvEntry, SecretKey};

pub struct Secrets {
    storage: Arc<Storage>,
    keys: Option<Keys>,
}

impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secrets(..)")
    }
}

/// Validate agent variable names before encryption: reserved names are refused.
pub fn validate_env_name(name: &str) -> Result<(), CoreError> {
    if reserved_env_name(name) {
        return Err(CoreError::SecretNameReserved(name.into()));
    }
    validate_env_format(name)
}

/// The name's shape alone. Removal uses it, so a row stored before its name
/// became reserved can still be removed after the opening names it.
fn validate_env_format(name: &str) -> Result<(), CoreError> {
    let bytes = name.as_bytes();
    let valid = (1..=128).contains(&bytes.len())
        && (bytes[0].is_ascii_uppercase() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_');
    if valid {
        Ok(())
    } else {
        Err(CoreError::SecretNameInvalid(name.into()))
    }
}

pub(crate) fn reserved_env_name(name: &str) -> bool {
    matches!(
        name,
        "CLAUDE_CONFIG_DIR"
            | "ANTHROPIC_API_KEY"
            | "ANTHROPIC_AUTH_TOKEN"
            | "ANTHROPIC_BASE_URL"
            | "ANTHROPIC_MODEL"
            | "ANTHROPIC_DEFAULT_OPUS_MODEL"
            | "ANTHROPIC_DEFAULT_SONNET_MODEL"
            | "ANTHROPIC_DEFAULT_HAIKU_MODEL"
            | "ANTHROPIC_SMALL_FAST_MODEL"
            | "ANTHROPIC_CUSTOM_MODEL_OPTION"
            | "CLAUDE_CODE_MAX_CONTEXT_TOKENS"
            | "CLAUDE_CODE_AUTO_COMPACT_WINDOW"
    )
}

pub(crate) use store::remove_scope_in;

fn validate_value(name: &str, value: &str) -> Result<(), CoreError> {
    if value.is_empty() || value.len() > 32768 || value.contains('\0') {
        return Err(CoreError::SecretValueInvalid(name.into()));
    }
    Ok(())
}

fn aad(profile: &str, name: &str) -> Vec<u8> {
    format!("{profile}\0{name}").into_bytes()
}

impl Secrets {
    pub(crate) fn new(storage: Arc<Storage>, key: Option<SecretKey>) -> Arc<Self> {
        Arc::new(Self {
            storage,
            keys: key.as_ref().map(Keys::derive),
        })
    }

    fn keys(&self) -> Result<&Keys, CoreError> {
        self.keys.as_ref().ok_or(CoreError::SecretsUnavailable)
    }

    pub async fn list_env(&self, profile: &str) -> Result<Vec<EnvEntry>, CoreError> {
        self.keys()?;
        Ok(store::list(&self.storage, profile).await?)
    }

    pub async fn set_env(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        self.set_env_mode(command_id, profile, name, value, false)
            .await
    }
    pub async fn remove_env(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        self.remove_env_mode(command_id, profile, name, false).await
    }
    pub async fn set_agent_env(
        &self,
        command_id: String,
        agent: &str,
        name: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        self.set_env_mode(command_id, agent, name, value, true)
            .await
    }
    pub async fn remove_agent_env(
        &self,
        command_id: String,
        agent: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        self.remove_env_mode(command_id, agent, name, true).await
    }
    pub async fn list_agent_env(&self, agent: &str) -> Result<Vec<EnvEntry>, CoreError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(crate::StorageError::from)?;
        crate::profiles::require_live_agent_in(&mut conn, agent).await?;
        self.list_env(agent).await
    }

    async fn set_env_mode(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
        value: &str,
        agent_only: bool,
    ) -> Result<EnvEntry, CoreError> {
        let keys = self.keys()?;
        validate_env_name(name)?;
        validate_value(name, value)?;
        let c = CommandContext::user(
            command_id,
            "secret_env.set",
            serde_json::json!({
                "profile": profile, "name": name, "value_mac": keys.value_mac(value),
            }),
        );
        let sealed = crypto::seal(&keys.aead, &aad(profile, name), value)
            .map_err(|_| CoreError::SecretsUnavailable)?;
        Ok(store::set(&self.storage, c, profile, name, sealed, agent_only).await?)
    }

    async fn remove_env_mode(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
        agent_only: bool,
    ) -> Result<(), CoreError> {
        self.keys()?;
        validate_env_format(name)?;
        let c = CommandContext::user(
            command_id,
            "secret_env.remove",
            serde_json::json!({
                "profile": profile, "name": name,
            }),
        );
        Ok(store::remove(&self.storage, c, profile, name, agent_only).await?)
    }

    pub async fn set_provider_key(
        &self,
        command_id: String,
        provider: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        self.set_env(
            command_id,
            &format!("provider/{provider}"),
            "PROVIDER_API_KEY",
            value,
        )
        .await
    }

    pub async fn remove_provider_key(
        &self,
        command_id: String,
        provider: &str,
    ) -> Result<(), CoreError> {
        self.remove_env(
            command_id,
            &format!("provider/{provider}"),
            "PROVIDER_API_KEY",
        )
        .await
    }

    pub async fn provider_key_set(&self, provider: &str) -> Result<bool, CoreError> {
        Ok(store::list(&self.storage, &format!("provider/{provider}"))
            .await?
            .iter()
            .any(|e| e.name == "PROVIDER_API_KEY"))
    }

    pub(crate) async fn resolve_env(
        &self,
        profile: &str,
    ) -> Result<Vec<(String, String)>, CoreError> {
        let keys = self.keys()?;
        store::rows(&self.storage, profile)
            .await?
            .into_iter()
            .map(|(name, sealed)| {
                if reserved_env_name(&name) {
                    return Err(CoreError::SecretNameReserved(name));
                }
                let value = crypto::open(&keys.aead, &aad(profile, &name), &sealed)
                    .map_err(|_| CoreError::SecretUnreadable(name.clone()))?;
                Ok((name, value))
            })
            .collect()
    }
}
