//! Encrypted environment values for trusted provider probes and harness openings.
mod crypto;
#[cfg(test)]
#[path = "../tests/crypto/invariants.rs"]
mod crypto_tests;
mod error;
mod model;
mod store;

use std::sync::Arc;

use core_db::{CommandContext, Storage};
use crypto::{Keys, Sealed};
pub use error::SecretsError;
pub use model::{EnvEntry, SecretKey};

pub struct Secrets {
    store: store::SecretsStore,
    keys: Option<Keys>,
}

/// A value sealed for one profile and name. Its fields are private, so the
/// row it is stored in is the one its associated data names.
pub struct SealedEnv {
    profile: String,
    name: String,
    sealed: Sealed,
    value_mac: String,
}

impl SealedEnv {
    /// The keyed fingerprint of the clear value, for a command record.
    fn value_mac(&self) -> &str {
        &self.value_mac
    }
}

/// One sealed value under its user command, ready for a write transaction.
pub struct EnvWrite {
    command: CommandContext,
    value: SealedEnv,
}

/// One removal under its user command, ready for a write transaction.
pub struct EnvRemoval {
    command: CommandContext,
    profile: String,
    name: String,
}

impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secrets(..)")
    }
}

impl Secrets {
    /// Validate agent variable names before encryption: reserved names are refused.
    pub fn validate_env_name(name: &str) -> Result<(), SecretsError> {
        if Self::reserved_env_name(name) {
            return Err(SecretsError::NameReserved(name.into()));
        }
        validate_env_format(name)
    }
}

/// The name's shape alone. Removal accepts reserved names so a stored value
/// refused during opening can still be removed.
fn validate_env_format(name: &str) -> Result<(), SecretsError> {
    let bytes = name.as_bytes();
    let valid = (1..=128).contains(&bytes.len())
        && (bytes[0].is_ascii_uppercase() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_');
    if valid {
        Ok(())
    } else {
        Err(SecretsError::NameInvalid(name.into()))
    }
}

impl Secrets {
    pub fn reserved_env_name(name: &str) -> bool {
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
}

/// A provider key lives in the scope `provider/{slug}` under one fixed name.
const PROVIDER_SCOPE_PREFIX: &str = "provider/";
const PROVIDER_KEY_NAME: &str = "PROVIDER_API_KEY";

fn validate_value(name: &str, value: &str) -> Result<(), SecretsError> {
    if value.is_empty() || value.len() > 32768 || value.contains('\0') {
        return Err(SecretsError::ValueInvalid(name.into()));
    }
    Ok(())
}

fn aad(profile: &str, name: &str) -> Vec<u8> {
    format!("{profile}\0{name}").into_bytes()
}

impl Secrets {
    pub fn new(storage: Arc<Storage>, key: Option<SecretKey>) -> Arc<Self> {
        Arc::new(Self {
            store: store::SecretsStore::new(storage),
            keys: key.as_ref().map(Keys::derive),
        })
    }

    /// The scope a provider key is stored under.
    pub fn provider_scope(provider: &str) -> String {
        format!("{PROVIDER_SCOPE_PREFIX}{provider}")
    }

    /// Checks the key, the name and the value, then encrypts the value for
    /// `profile`/`name`. Runs before the write transaction, so a refused value
    /// never reaches one and the transaction sees only database failures.
    pub fn seal(&self, profile: &str, name: &str, value: &str) -> Result<SealedEnv, SecretsError> {
        let keys = self.keys()?;
        Self::validate_env_name(name)?;
        validate_value(name, value)?;
        let sealed = keys
            .aead
            .seal(&aad(profile, name), value)
            .map_err(|_| SecretsError::Unavailable)?;
        Ok(SealedEnv {
            profile: profile.into(),
            name: name.into(),
            sealed,
            value_mac: keys.value_mac(value),
        })
    }

    /// Seals `value` for `profile`/`name` under its `secret_env.set` command, before
    /// any transaction, so a refused value never reaches one.
    pub fn env_write(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
        value: &str,
    ) -> Result<EnvWrite, SecretsError> {
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
    pub fn provider_key_write(
        &self,
        command_id: String,
        provider: &str,
        value: &str,
    ) -> Result<EnvWrite, SecretsError> {
        let scope = Self::provider_scope(provider);
        self.env_write(command_id, &scope, PROVIDER_KEY_NAME, value)
    }

    /// Checks the key and the name's shape, and builds the `secret_env.remove` command.
    pub fn env_removal(
        &self,
        command_id: String,
        profile: &str,
        name: &str,
    ) -> Result<EnvRemoval, SecretsError> {
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
    pub fn provider_key_removal(
        &self,
        command_id: String,
        provider: &str,
    ) -> Result<EnvRemoval, SecretsError> {
        let scope = Self::provider_scope(provider);
        self.env_removal(command_id, &scope, PROVIDER_KEY_NAME)
    }

    /// The recorded entry when `write` replays a command, inside the caller's write.
    /// A replay answers before the caller checks its owner, so removal cannot change it.
    pub async fn replayed_write_in(
        conn: &mut sqlx::SqliteConnection,
        write: &EnvWrite,
    ) -> Result<Option<EnvEntry>, core_db::StorageError> {
        let scope = format!("{}/{}", write.value.profile, write.value.name);
        store::SecretsStore::replayed_in(conn, &write.command, &scope)
            .await?
            .map(|outcome| {
                serde_json::from_str(&outcome).map_err(|_| {
                    core_db::StorageError::Unavailable("invalid secret outcome".into())
                })
            })
            .transpose()
    }

    /// Stores one sealed value and records its command inside the caller's write. The
    /// caller asked `replayed_write_in` and checked the scope's owner first.
    pub async fn put_in(
        conn: &mut sqlx::SqliteConnection,
        write: EnvWrite,
    ) -> Result<EnvEntry, core_db::StorageError> {
        store::SecretsStore::put_in(conn, &write.command, write.value).await
    }

    /// Whether `removal` replays a command, inside the caller's write.
    pub async fn replayed_removal_in(
        conn: &mut sqlx::SqliteConnection,
        removal: &EnvRemoval,
    ) -> Result<bool, core_db::StorageError> {
        let scope = format!("{}/{}", removal.profile, removal.name);
        Ok(
            store::SecretsStore::replayed_in(conn, &removal.command, &scope)
                .await?
                .is_some(),
        )
    }

    /// Removes one value and records its command inside the caller's write. The caller
    /// asked `replayed_removal_in` and checked the scope's owner first.
    pub async fn remove_in(
        conn: &mut sqlx::SqliteConnection,
        removal: EnvRemoval,
    ) -> Result<(), core_db::StorageError> {
        let EnvRemoval {
            command,
            profile,
            name,
        } = removal;
        store::SecretsStore::delete_in(conn, &command, &profile, &name).await
    }

    /// Removes every value of one scope inside the caller's transaction.
    pub async fn remove_scope_in(
        &self,
        conn: &mut sqlx::SqliteConnection,
        scope: &str,
    ) -> Result<(), core_db::StorageError> {
        store::SecretsStore::remove_scope_in(conn, scope).await
    }

    /// A provider's key in clear, for the harness only. `None` when unset or empty.
    pub async fn resolve_provider_key(
        &self,
        provider: &str,
    ) -> Result<Option<String>, SecretsError> {
        Ok(self
            .resolve_env(&Self::provider_scope(provider))
            .await?
            .into_iter()
            .find(|(name, _)| name == PROVIDER_KEY_NAME)
            .map(|(_, key)| key)
            .filter(|key| !key.is_empty()))
    }

    fn keys(&self) -> Result<&Keys, SecretsError> {
        self.keys.as_ref().ok_or(SecretsError::Unavailable)
    }

    pub async fn list_env(&self, profile: &str) -> Result<Vec<EnvEntry>, SecretsError> {
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
    ) -> Result<EnvEntry, SecretsError> {
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
    ) -> Result<(), SecretsError> {
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

    pub async fn provider_key_set(&self, provider: &str) -> Result<bool, SecretsError> {
        Ok(self
            .store
            .list(&Self::provider_scope(provider))
            .await?
            .iter()
            .any(|e| e.name == PROVIDER_KEY_NAME))
    }

    pub async fn resolve_env(&self, profile: &str) -> Result<Vec<(String, String)>, SecretsError> {
        let keys = self.keys()?;
        self.store
            .rows(profile)
            .await?
            .into_iter()
            .map(|(name, sealed)| {
                if Self::reserved_env_name(&name) {
                    return Err(SecretsError::NameReserved(name));
                }
                let value = keys
                    .aead
                    .open(&aad(profile, &name), &sealed)
                    .map_err(|_| SecretsError::Unreadable(name.clone()))?;
                Ok((name, value))
            })
            .collect()
    }
}
