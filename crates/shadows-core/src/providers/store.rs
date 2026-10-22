//! Transactional SQL for provider configuration.
use super::{Provider, ProviderInput, ProviderModel, ProvidersError, SlugBase};
use crate::CoreError;
use crate::db::{CommandContext, Storage, StorageError, classify, now, record_command};
use futures_core::future::BoxFuture;
use sqlx::{Row, SqliteConnection};
use std::sync::Arc;

/// The scope replays of configuration commands are classified under.
const COMMAND_SCOPE: (&str, &str) = ("Profiles", "aylul");

/// Providers come back with `key_set` false: `Providers` asks the secrets owner.
fn provider_row(row: sqlx::sqlite::SqliteRow) -> Result<Provider, ProvidersError> {
    Ok(Provider {
        slug: row.get("slug"),
        name: row.get("name"),
        base_url: row.get("base_url"),
        preset: row.get("preset"),
        label: row.get("label"),
        models: serde_json::from_str(row.get("models"))?,
        key_set: false,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// The service's SQL, over the shared storage handle.
pub(super) struct ProvidersStore {
    storage: Arc<Storage>,
}

impl ProvidersStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// One write on the shared connection; the closure refuses with any owner's error.
    pub(super) async fn write<F, T>(&self, f: F) -> Result<T, CoreError>
    where
        F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, CoreError>>,
    {
        self.storage.write_txn_as(f).await
    }

    pub(super) async fn list(&self) -> Result<Vec<Provider>, ProvidersError> {
        sqlx::query("SELECT p.* FROM aylul_provider p ORDER BY p.created_at, p.slug")
            .fetch_all(self.storage.reader())
            .await?
            .into_iter()
            .map(provider_row)
            .collect()
    }

    pub(super) async fn get(&self, slug: &str) -> Result<Provider, ProvidersError> {
        let row = sqlx::query("SELECT p.* FROM aylul_provider p WHERE p.slug = ?")
            .bind(slug)
            .fetch_optional(self.storage.reader())
            .await?
            .ok_or_else(|| ProvidersError::NotFound(slug.into()))?;
        provider_row(row)
    }

    pub(super) async fn create(
        &self,
        command: CommandContext,
        input: ProviderInput,
    ) -> Result<String, ProvidersError> {
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        classify(conn, &command, COMMAND_SCOPE.0, COMMAND_SCOPE.1).await?
                    {
                        return Ok(id);
                    }
                    let base = SlugBase::new(input.name.as_deref().unwrap_or(""), "provider");
                    let id = Self::free_slug_in(conn, &base).await?;
                    Self::upsert_in(conn, &id, &input).await?;
                    Self::record_in(conn, &command, &id).await?;
                    Ok(id)
                })
            })
            .await
    }

    /// The first candidate no provider holds; `test` is reserved for creation.
    async fn free_slug_in(
        conn: &mut SqliteConnection,
        base: &SlugBase,
    ) -> Result<String, ProvidersError> {
        let mut n = 1;
        loop {
            let candidate = base.candidate(n);
            let taken: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM aylul_provider WHERE slug=?)")
                    .bind(&candidate)
                    .fetch_one(&mut *conn)
                    .await?;
            if !taken && candidate != "test" {
                return Ok(candidate);
            }
            n += 1;
        }
    }

    pub(super) async fn upsert_in(
        conn: &mut SqliteConnection,
        id: &str,
        input: &ProviderInput,
    ) -> Result<(), ProvidersError> {
        let ts = now();
        sqlx::query(
            "INSERT INTO aylul_provider
            (slug,name,base_url,models,context_window,created_at,updated_at,preset,label)
            VALUES (?,?,?,?,?,?,?,?,?)
            ON CONFLICT(slug) DO UPDATE SET name=excluded.name, base_url=excluded.base_url,
            models=excluded.models, context_window=excluded.context_window,
            preset=excluded.preset, label=excluded.label,
            updated_at=excluded.updated_at",
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.base_url)
        .bind(serde_json::to_string(&input.models)?)
        .bind(200_000)
        .bind(&ts)
        .bind(&ts)
        .bind(&input.preset)
        .bind(&input.label)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(super) async fn models_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<Vec<String>, ProvidersError> {
        let models: Option<String> =
            sqlx::query_scalar("SELECT models FROM aylul_provider WHERE slug=?")
                .bind(slug)
                .fetch_optional(conn)
                .await?;
        let models: Vec<ProviderModel> =
            serde_json::from_str(&models.ok_or_else(|| ProvidersError::NotFound(slug.into()))?)?;
        Ok(models.into_iter().map(|m| m.id).collect())
    }

    pub(super) async fn delete_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), ProvidersError> {
        sqlx::query("DELETE FROM aylul_provider WHERE slug=?")
            .bind(slug)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn record_in(
        conn: &mut SqliteConnection,
        command: &CommandContext,
        id: &str,
    ) -> Result<(), StorageError> {
        let (scope, key) = COMMAND_SCOPE;
        record_command(conn, command, scope, key, "Profile", id, &now()).await
    }
}
