//! SQL for encrypted environment scopes.

use std::sync::Arc;

use sqlx::SqliteConnection;

use super::crypto::Sealed;
use super::{EnvEntry, SealedEnv};
use crate::db::{CommandContext, Storage, StorageError, classify, now, record_command};

/// The command scope and record kind of every secret write.
const COMMAND_SCOPE: &str = "aylul_env";
const RECORD_KIND: &str = "SecretEnv";

/// The service's SQL, over the shared storage handle.
#[derive(Clone)]
pub(super) struct SecretsStore {
    storage: Arc<Storage>,
}

impl SecretsStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub(super) async fn list(&self, profile: &str) -> Result<Vec<EnvEntry>, StorageError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT name, updated_at FROM secret_env WHERE profile = ? ORDER BY name",
        )
        .bind(profile)
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, updated_at)| EnvEntry { name, updated_at })
            .collect())
    }

    pub(super) async fn rows(&self, profile: &str) -> Result<Vec<(String, Sealed)>, StorageError> {
        let rows: Vec<(String, i64, Vec<u8>, Vec<u8>)> = sqlx::query_as(
            "SELECT name, format, nonce, ciphertext FROM secret_env WHERE profile = ? ORDER BY name",
        )
        .bind(profile)
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, format, nonce, ciphertext)| {
                (
                    name,
                    Sealed {
                        format,
                        nonce,
                        ciphertext,
                    },
                )
            })
            .collect())
    }

    /// Whether the agent is live, read outside any write.
    pub(super) async fn require_live_agent(&self, agent: &str) -> Result<(), StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        crate::profiles::require_live_agent_in(&mut conn, agent).await
    }

    pub(super) async fn upsert(
        conn: &mut SqliteConnection,
        value: &SealedEnv,
        ts: &str,
    ) -> Result<(), StorageError> {
        let SealedEnv {
            profile,
            name,
            sealed,
            ..
        } = value;
        sqlx::query(
            "INSERT INTO secret_env (profile, name, format, nonce, ciphertext, updated_at)
             VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT (profile, name) DO UPDATE SET
             format = excluded.format, nonce = excluded.nonce, ciphertext = excluded.ciphertext,
             updated_at = excluded.updated_at",
        )
        .bind(profile)
        .bind(name)
        .bind(sealed.format)
        .bind(&sealed.nonce)
        .bind(&sealed.ciphertext)
        .bind(ts)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// The user-facing writes check that the owner of the scope still exists.
    /// `Secrets::put_in` callers check their own owner instead.
    async fn require_scope_owner(
        conn: &mut SqliteConnection,
        profile: &str,
        agent_only: bool,
    ) -> Result<(), StorageError> {
        if agent_only {
            crate::profiles::require_live_agent_in(conn, profile).await?;
        }
        if let Some(provider) = profile.strip_prefix(super::PROVIDER_SCOPE_PREFIX) {
            crate::profiles::require_provider_in(conn, provider).await?;
        }
        Ok(())
    }

    pub(super) async fn set(
        &self,
        ctx: CommandContext,
        value: SealedEnv,
        agent_only: bool,
    ) -> Result<EnvEntry, StorageError> {
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let scope = format!("{}/{}", value.profile, value.name);
                    if let Some(outcome) = classify(conn, &ctx, COMMAND_SCOPE, &scope).await? {
                        return serde_json::from_str(&outcome).map_err(|_| {
                            StorageError::Unavailable("invalid secret outcome".into())
                        });
                    }
                    Self::require_scope_owner(conn, &value.profile, agent_only).await?;
                    let ts = now();
                    Self::upsert(conn, &value, &ts).await?;
                    let entry = EnvEntry {
                        name: value.name,
                        updated_at: ts.clone(),
                    };
                    let outcome = serde_json::to_string(&entry).expect("metadata serializes");
                    record_command(
                        conn,
                        &ctx,
                        COMMAND_SCOPE,
                        &scope,
                        RECORD_KIND,
                        &outcome,
                        &ts,
                    )
                    .await?;
                    Ok(entry)
                })
            })
            .await
    }

    pub(super) async fn remove(
        &self,
        ctx: CommandContext,
        profile: &str,
        name: &str,
        agent_only: bool,
    ) -> Result<(), StorageError> {
        let (profile, name) = (profile.to_owned(), name.to_owned());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let scope = format!("{profile}/{name}");
                    if classify(conn, &ctx, COMMAND_SCOPE, &scope).await?.is_some() {
                        return Ok(());
                    }
                    Self::require_scope_owner(conn, &profile, agent_only).await?;
                    sqlx::query("DELETE FROM secret_env WHERE profile = ? AND name = ?")
                        .bind(&profile)
                        .bind(&name)
                        .execute(&mut *conn)
                        .await?;
                    let ts = now();
                    let outcome = serde_json::json!({"name": name, "updated_at": ts}).to_string();
                    record_command(
                        conn,
                        &ctx,
                        COMMAND_SCOPE,
                        &scope,
                        RECORD_KIND,
                        &outcome,
                        &ts,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub(super) async fn remove_scope_in(
        conn: &mut SqliteConnection,
        scope: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM secret_env WHERE profile = ?")
            .bind(scope)
            .execute(conn)
            .await?;
        Ok(())
    }
}
