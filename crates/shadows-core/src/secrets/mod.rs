//! The encrypted profile environment. Only the harness resolves values.
mod crypto;
#[cfg(test)]
mod crypto_tests;
mod model;
mod store;

use std::sync::Arc;

use crate::CoreError;
use crate::db::{CommandContext, Storage};
use crypto::{Keys, Sealed};
pub use model::{EnvEntry, SecretKey};

pub struct Secrets {
    store: store::SecretsStore,
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

/// A provider key lives in the scope `provider/{slug}` under one fixed name.
const PROVIDER_SCOPE_PREFIX: &str = "provider/";
const PROVIDER_KEY_NAME: &str = "PROVIDER_API_KEY";

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
            store: store::SecretsStore::new(storage),
            keys: key.as_ref().map(Keys::derive),
        })
    }

    /// The scope a provider key is stored under.
    pub fn provider_scope(provider: &str) -> String {
        format!("{PROVIDER_SCOPE_PREFIX}{provider}")
    }

    /// Validates a value and encrypts it for `profile`/`name`.
    fn seal_value(&self, profile: &str, name: &str, value: &str) -> Result<Sealed, CoreError> {
        validate_env_name(name)?;
        validate_value(name, value)?;
        crypto::seal(&self.keys()?.aead, &aad(profile, name), value)
            .map_err(|_| CoreError::SecretsUnavailable)
    }

    /// Stores one value inside the caller's transaction. The caller owns the
    /// existence check of the scope and the command record.
    pub async fn put_in(
        &self,
        conn: &mut sqlx::SqliteConnection,
        profile: &str,
        name: &str,
        value: &str,
        ts: &str,
    ) -> Result<EnvEntry, CoreError> {
        let sealed = self.seal_value(profile, name, value)?;
        store::SecretsStore::upsert(conn, profile, name, sealed, ts).await?;
        Ok(EnvEntry {
            name: name.into(),
            updated_at: ts.into(),
        })
    }

    /// Removes every value of one scope inside the caller's transaction.
    pub async fn remove_scope_in(
        &self,
        conn: &mut sqlx::SqliteConnection,
        scope: &str,
    ) -> Result<(), crate::StorageError> {
        store::SecretsStore::remove_scope_in(conn, scope).await
    }

    /// A provider's key in clear, for the harness only. `None` when unset or empty.
    pub(crate) async fn resolve_provider_key(
        &self,
        provider: &str,
    ) -> Result<Option<String>, CoreError> {
        Ok(self
            .resolve_env(&Self::provider_scope(provider))
            .await?
            .into_iter()
            .find(|(name, _)| name == PROVIDER_KEY_NAME)
            .map(|(_, key)| key)
            .filter(|key| !key.is_empty()))
    }

    fn keys(&self) -> Result<&Keys, CoreError> {
        self.keys.as_ref().ok_or(CoreError::SecretsUnavailable)
    }

    pub async fn list_env(&self, profile: &str) -> Result<Vec<EnvEntry>, CoreError> {
        self.keys()?;
        Ok(self.store.list(profile).await?)
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
        self.store.require_live_agent(agent).await?;
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
        let sealed = self.seal_value(profile, name, value)?;
        let c = CommandContext::user(
            command_id,
            "secret_env.set",
            serde_json::json!({
                "profile": profile, "name": name, "value_mac": keys.value_mac(value),
            }),
        );
        Ok(self.store.set(c, profile, name, sealed, agent_only).await?)
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
        Ok(self.store.remove(c, profile, name, agent_only).await?)
    }

    pub async fn set_provider_key(
        &self,
        command_id: String,
        provider: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        self.set_env(
            command_id,
            &Self::provider_scope(provider),
            PROVIDER_KEY_NAME,
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
            &Self::provider_scope(provider),
            PROVIDER_KEY_NAME,
        )
        .await
    }

    pub async fn provider_key_set(&self, provider: &str) -> Result<bool, CoreError> {
        Ok(self
            .store
            .list(&Self::provider_scope(provider))
            .await?
            .iter()
            .any(|e| e.name == PROVIDER_KEY_NAME))
    }

    pub(crate) async fn resolve_env(
        &self,
        profile: &str,
    ) -> Result<Vec<(String, String)>, CoreError> {
        let keys = self.keys()?;
        self.store
            .rows(profile)
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
