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

/// A value sealed for one profile and name. Its fields are private, so the
/// row it is stored in is the one its associated data names.
pub(crate) struct SealedEnv {
    profile: String,
    name: String,
    sealed: Sealed,
    value_mac: String,
}

impl SealedEnv {
    /// The keyed fingerprint of the clear value, for a command record.
    pub(crate) fn value_mac(&self) -> &str {
        &self.value_mac
    }
}

/// One sealed value under its user command, ready for a write transaction.
pub(crate) struct EnvWrite {
    command: CommandContext,
    value: SealedEnv,
}

/// One removal under its user command, ready for a write transaction.
pub(crate) struct EnvRemoval {
    command: CommandContext,
    profile: String,
    name: String,
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
    pub(crate) fn provider_scope(provider: &str) -> String {
        format!("{PROVIDER_SCOPE_PREFIX}{provider}")
    }

    /// Checks the key, the name and the value, then encrypts the value for
    /// `profile`/`name`. Runs before the write transaction, so a refused value
    /// never reaches one and the transaction sees only database failures.
    pub(crate) fn seal(
        &self,
        profile: &str,
        name: &str,
        value: &str,
    ) -> Result<SealedEnv, CoreError> {
        let keys = self.keys()?;
        validate_env_name(name)?;
        validate_value(name, value)?;
        let sealed = crypto::seal(&keys.aead, &aad(profile, name), value)
            .map_err(|_| CoreError::SecretsUnavailable)?;
        Ok(SealedEnv {
            profile: profile.into(),
            name: name.into(),
            sealed,
            value_mac: keys.value_mac(value),
        })
    }

    /// Seals `value` for `profile`/`name` under its `secret_env.set` command, before
    /// any transaction, so a refused value never reaches one.
    pub(crate) fn env_write(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
        value: &str,
    ) -> Result<EnvWrite, CoreError> {
        let value = self.seal(profile, name, value)?;
        let command = CommandContext::user(
            command_id,
            "secret_env.set",
            serde_json::json!({
                "profile": profile, "name": name, "value_mac": value.value_mac(),
            }),
        );
        Ok(EnvWrite { command, value })
    }

    /// The provider key write: its scope and name are this service's.
    pub(crate) fn provider_key_write(
        &self,
        command_id: String,
        provider: &str,
        value: &str,
    ) -> Result<EnvWrite, CoreError> {
        let scope = Self::provider_scope(provider);
        self.env_write(command_id, &scope, PROVIDER_KEY_NAME, value)
    }

    /// Checks the key and the name's shape, and builds the `secret_env.remove` command.
    pub(crate) fn env_removal(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
    ) -> Result<EnvRemoval, CoreError> {
        self.keys()?;
        validate_env_format(name)?;
        let command = CommandContext::user(
            command_id,
            "secret_env.remove",
            serde_json::json!({
                "profile": profile, "name": name,
            }),
        );
        Ok(EnvRemoval {
            command,
            profile: profile.into(),
            name: name.into(),
        })
    }

    /// The provider key removal.
    pub(crate) fn provider_key_removal(
        &self,
        command_id: String,
        provider: &str,
    ) -> Result<EnvRemoval, CoreError> {
        let scope = Self::provider_scope(provider);
        self.env_removal(command_id, &scope, PROVIDER_KEY_NAME)
    }

    /// The recorded entry when `write` replays a command, inside the caller's write.
    /// A replay answers before the caller checks its owner, so removal cannot change it.
    pub(crate) async fn replayed_write_in(
        conn: &mut sqlx::SqliteConnection,
        write: &EnvWrite,
    ) -> Result<Option<EnvEntry>, crate::StorageError> {
        let scope = format!("{}/{}", write.value.profile, write.value.name);
        store::SecretsStore::replayed_in(conn, &write.command, &scope)
            .await?
            .map(|outcome| {
                serde_json::from_str(&outcome)
                    .map_err(|_| crate::StorageError::Unavailable("invalid secret outcome".into()))
            })
            .transpose()
    }

    /// Stores one sealed value and records its command inside the caller's write. The
    /// caller asked `replayed_write_in` and checked the scope's owner first.
    pub(crate) async fn put_in(
        conn: &mut sqlx::SqliteConnection,
        write: EnvWrite,
    ) -> Result<EnvEntry, crate::StorageError> {
        store::SecretsStore::put_in(conn, &write.command, write.value).await
    }

    /// Whether `removal` replays a command, inside the caller's write.
    pub(crate) async fn replayed_removal_in(
        conn: &mut sqlx::SqliteConnection,
        removal: &EnvRemoval,
    ) -> Result<bool, crate::StorageError> {
        let scope = format!("{}/{}", removal.profile, removal.name);
        Ok(
            store::SecretsStore::replayed_in(conn, &removal.command, &scope)
                .await?
                .is_some(),
        )
    }

    /// Removes one value and records its command inside the caller's write. The caller
    /// asked `replayed_removal_in` and checked the scope's owner first.
    pub(crate) async fn remove_in(
        conn: &mut sqlx::SqliteConnection,
        removal: EnvRemoval,
    ) -> Result<(), crate::StorageError> {
        let EnvRemoval {
            command,
            profile,
            name,
        } = removal;
        store::SecretsStore::delete_in(conn, &command, &profile, &name).await
    }

    /// Removes every value of one scope inside the caller's transaction.
    pub(crate) async fn remove_scope_in(
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

    /// Stores one value of any scope. Owner checks belong to the scope's owner, which
    /// writes through `put_in` instead.
    pub async fn set_env(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        let write = self.env_write(command_id, profile, name, value)?;
        Ok(self
            .store
            .write(move |conn| {
                Box::pin(async move {
                    if let Some(entry) = Self::replayed_write_in(conn, &write).await? {
                        return Ok(entry);
                    }
                    Self::put_in(conn, write).await
                })
            })
            .await?)
    }

    /// Removes one value of any scope; a replay removes nothing again.
    pub async fn remove_env(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        let removal = self.env_removal(command_id, profile, name)?;
        Ok(self
            .store
            .write(move |conn| {
                Box::pin(async move {
                    if Self::replayed_removal_in(conn, &removal).await? {
                        return Ok(());
                    }
                    Self::remove_in(conn, removal).await
                })
            })
            .await?)
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
