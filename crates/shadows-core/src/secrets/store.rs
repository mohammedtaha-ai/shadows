//! SQL for encrypted environment scopes.

use sqlx::SqliteConnection;

use super::EnvEntry;
use super::crypto::Sealed;
use crate::db::{CommandContext, Storage, StorageError, classify, now, record_command};

pub(super) async fn list(storage: &Storage, profile: &str) -> Result<Vec<EnvEntry>, StorageError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT name, updated_at FROM secret_env WHERE profile = ? ORDER BY name")
            .bind(profile)
            .fetch_all(storage.reader())
            .await?;
    Ok(rows
        .into_iter()
        .map(|(name, updated_at)| EnvEntry { name, updated_at })
        .collect())
}

pub(super) async fn rows(
    storage: &Storage,
    profile: &str,
) -> Result<Vec<(String, Sealed)>, StorageError> {
    let rows: Vec<(String, i64, Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT name, format, nonce, ciphertext FROM secret_env WHERE profile = ? ORDER BY name",
    )
    .bind(profile)
    .fetch_all(storage.reader())
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
    profile: &str,
    name: &str,
    sealed: Sealed,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO secret_env (profile, name, format, nonce, ciphertext, updated_at)
         VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT (profile, name) DO UPDATE SET
         format = excluded.format, nonce = excluded.nonce, ciphertext = excluded.ciphertext,
         updated_at = excluded.updated_at",
    )
    .bind(profile)
    .bind(name)
    .bind(sealed.format)
    .bind(sealed.nonce)
    .bind(sealed.ciphertext)
    .bind(ts)
    .execute(conn)
    .await?;
    Ok(())
}

pub(super) async fn set(
    storage: &Storage,
    ctx: CommandContext,
    profile: &str,
    name: &str,
    sealed: Sealed,
    agent_only: bool,
) -> Result<EnvEntry, StorageError> {
    let (profile, name) = (profile.to_owned(), name.to_owned());
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let scope = format!("{profile}/{name}");
                if let Some(outcome) = classify(conn, &ctx, "aylul_env", &scope).await? {
                    return serde_json::from_str(&outcome)
                        .map_err(|_| StorageError::Unavailable("invalid secret outcome".into()));
                }
                if agent_only {
                    crate::profiles::require_live_agent_in(conn, &profile).await?;
                }
                if let Some(provider) = profile.strip_prefix("provider/") {
                    crate::profiles::require_provider_in(conn, provider).await?;
                }
                let ts = now();
                upsert(conn, &profile, &name, sealed, &ts).await?;
                let entry = EnvEntry {
                    name,
                    updated_at: ts.clone(),
                };
                let outcome = serde_json::to_string(&entry).expect("metadata serializes");
                record_command(conn, &ctx, "aylul_env", &scope, "SecretEnv", &outcome, &ts).await?;
                Ok(entry)
            })
        })
        .await
}

pub(super) async fn remove(
    storage: &Storage,
    ctx: CommandContext,
    profile: &str,
    name: &str,
    agent_only: bool,
) -> Result<(), StorageError> {
    let (profile, name) = (profile.to_owned(), name.to_owned());
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let scope = format!("{profile}/{name}");
                if classify(conn, &ctx, "aylul_env", &scope).await?.is_some() {
                    return Ok(());
                }
                if agent_only {
                    crate::profiles::require_live_agent_in(conn, &profile).await?;
                }
                if let Some(provider) = profile.strip_prefix("provider/") {
                    crate::profiles::require_provider_in(conn, provider).await?;
                }
                sqlx::query("DELETE FROM secret_env WHERE profile = ? AND name = ?")
                    .bind(&profile)
                    .bind(&name)
                    .execute(&mut *conn)
                    .await?;
                let ts = now();
                let outcome = serde_json::json!({"name": name, "updated_at": ts}).to_string();
                record_command(conn, &ctx, "aylul_env", &scope, "SecretEnv", &outcome, &ts).await?;
                Ok(())
            })
        })
        .await
}

/// Remove one scope inside a caller's configuration deletion transaction.
pub(crate) async fn remove_scope_in(
    conn: &mut SqliteConnection,
    scope: &str,
) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM secret_env WHERE profile = ?")
        .bind(scope)
        .execute(conn)
        .await?;
    Ok(())
}
