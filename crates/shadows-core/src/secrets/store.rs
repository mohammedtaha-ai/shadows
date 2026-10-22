//! SQL for encrypted environment scopes.

use std::sync::Arc;

use futures_core::future::BoxFuture;
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
            "SELECT name, format, nonce, ciphertext FROM secret_env WHERE profile = ? ORDER \
                BY name",
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

    async fn upsert(
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

    /// One write on the shared connection.
    pub(super) async fn write<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, StorageError>>,
    {
        self.storage.write_txn(f).await
    }

    /// The recorded outcome when `ctx` replays a command on `scope` (`profile/name`).
    pub(super) async fn replayed_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        scope: &str,
    ) -> Result<Option<String>, StorageError> {
        classify(conn, ctx, COMMAND_SCOPE, scope).await
    }

    pub(super) async fn put_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        value: SealedEnv,
    ) -> Result<EnvEntry, StorageError> {
        let scope = format!("{}/{}", value.profile, value.name);
        let ts = now();
        Self::upsert(conn, &value, &ts).await?;
        let entry = EnvEntry {
            name: value.name,
            updated_at: ts.clone(),
        };
        let outcome = serde_json::to_string(&entry).expect("metadata serializes");
        record_command(conn, ctx, COMMAND_SCOPE, &scope, RECORD_KIND, &outcome, &ts).await?;
        Ok(entry)
    }

    pub(super) async fn delete_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        profile: &str,
        name: &str,
    ) -> Result<(), StorageError> {
        let scope = format!("{profile}/{name}");
        sqlx::query("DELETE FROM secret_env WHERE profile = ? AND name = ?")
            .bind(profile)
            .bind(name)
            .execute(&mut *conn)
            .await?;
        let ts = now();
        let outcome = serde_json::json!({"name": name, "updated_at": ts}).to_string();
        record_command(conn, ctx, COMMAND_SCOPE, &scope, RECORD_KIND, &outcome, &ts).await
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
