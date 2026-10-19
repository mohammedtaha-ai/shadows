//! The encrypted profile environment. Only the harness resolves values.
mod crypto;
#[cfg(test)]
mod crypto_tests;
mod model;
mod store;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::CoreError;
use crate::app::user_command;
use crate::db::Storage;
use crypto::Keys;
pub use model::{EnvEntry, ImportReport, SecretKey};

pub struct Secrets {
    storage: Arc<Storage>,
    keys: Option<Keys>,
}

impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secrets(..)")
    }
}

/// Also used by the binary when loading `key_env`.
pub fn validate_env_name(name: &str) -> Result<(), CoreError> {
    let bytes = name.as_bytes();
    let valid = (1..=128).contains(&bytes.len())
        && (bytes[0].is_ascii_uppercase() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
        && !matches!(
            name,
            "CLAUDE_CONFIG_DIR" | "ANTHROPIC_API_KEY" | "ANTHROPIC_AUTH_TOKEN"
        );
    if valid {
        Ok(())
    } else {
        Err(CoreError::SecretNameInvalid(name.into()))
    }
}

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
        let keys = self.keys()?;
        validate_env_name(name)?;
        validate_value(name, value)?;
        let c = user_command(
            command_id,
            "secret_env.set",
            serde_json::json!({
                "profile": profile, "name": name, "value_mac": keys.value_mac(value),
            }),
        );
        let sealed = crypto::seal(&keys.aead, &aad(profile, name), value)
            .map_err(|_| CoreError::SecretsUnavailable)?;
        Ok(store::set(&self.storage, c, profile, name, sealed).await?)
    }

    pub async fn remove_env(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        self.keys()?;
        validate_env_name(name)?;
        let c = user_command(
            command_id,
            "secret_env.remove",
            serde_json::json!({
                "profile": profile, "name": name,
            }),
        );
        Ok(store::remove(&self.storage, c, profile, name).await?)
    }

    pub async fn import_needed(&self, profile: &str) -> Result<bool, CoreError> {
        self.keys()?;
        Ok(store::import_needed(&self.storage, profile).await?)
    }

    pub async fn import_env_once(
        &self,
        profile: &str,
        pairs: Vec<(String, String)>,
    ) -> Result<Option<ImportReport>, CoreError> {
        let keys = self.keys()?;
        let mut valid = BTreeMap::new();
        let mut skipped = Vec::new();
        for (name, value) in pairs {
            if validate_env_name(&name).is_err() || validate_value(&name, &value).is_err() {
                skipped.push(name);
            } else {
                valid.insert(name, value);
            }
        }
        Ok(store::import(&self.storage, profile, valid, skipped, keys).await?)
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
                let value = crypto::open(&keys.aead, &aad(profile, &name), &sealed)
                    .map_err(|_| CoreError::SecretUnreadable(name.clone()))?;
                Ok((name, value))
            })
            .collect()
    }
}
