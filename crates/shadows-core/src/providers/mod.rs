//! AYLUL provider configuration: the models a provider offers, its key and its tests.
mod error;
mod listing;
mod model;
mod presets;
mod probe;
mod slug;
mod store;
mod validation;

pub(crate) use error::ProvidersError;
pub use model::{Provider, ProviderInput, ProviderModel, ProviderModelInput};
pub use presets::{PresetRegion, ProviderPreset};
pub use probe::{ListedModel, ProviderTest, ProviderTestOutcome, SavedProviderTest, TestRefusal};
pub(crate) use slug::SlugBase;

use crate::CoreError;
use crate::secrets::{EnvEntry, Secrets};
use core_db::{CommandContext, Storage};
use sqlx::SqliteConnection;
use std::sync::Arc;
use store::ProvidersStore;

pub struct Providers {
    store: ProvidersStore,
    secrets: Arc<Secrets>,
}

impl Providers {
    pub(crate) fn new(storage: Arc<Storage>, secrets: Arc<Secrets>) -> Arc<Self> {
        Arc::new(Self {
            store: ProvidersStore::new(storage),
            secrets,
        })
    }

    pub async fn providers(&self) -> Result<Vec<Provider>, CoreError> {
        let mut providers = self.store.list().await?;
        for provider in &mut providers {
            provider.key_set = self.secrets.provider_key_set(&provider.slug).await?;
        }
        Ok(providers)
    }

    pub async fn provider(&self, slug: &str) -> Result<Provider, CoreError> {
        let mut provider = self.store.get(slug).await?;
        provider.key_set = self.secrets.provider_key_set(slug).await?;
        Ok(provider)
    }

    pub async fn create_provider(
        &self,
        command_id: String,
        input: ProviderInput,
    ) -> Result<Provider, CoreError> {
        let input = self.validated(input)?;
        let command = CommandContext::user(command_id, "provider.create", serde_json::json!(input));
        let slug = self.store.create(command, input).await?;
        self.provider(&slug).await
    }

    /// Stores the provider's key after checking the provider exists, in one write.
    pub async fn set_key(
        &self,
        command_id: String,
        slug: &str,
        value: &str,
    ) -> Result<EnvEntry, CoreError> {
        let write = self.secrets.provider_key_write(command_id, slug, value)?;
        let slug = slug.to_owned();
        self.store
            .write(move |conn| {
                Box::pin(async move {
                    if let Some(entry) = Secrets::replayed_write_in(conn, &write).await? {
                        return Ok(entry);
                    }
                    ProvidersStore::models_in(conn, &slug).await?;
                    Ok(Secrets::put_in(conn, write).await?)
                })
            })
            .await
    }

    /// Removes the provider's key after checking the provider exists, in one write.
    pub async fn remove_key(&self, command_id: String, slug: &str) -> Result<(), CoreError> {
        let removal = self.secrets.provider_key_removal(command_id, slug)?;
        let slug = slug.to_owned();
        self.store
            .write(move |conn| {
                Box::pin(async move {
                    if Secrets::replayed_removal_in(conn, &removal).await? {
                        return Ok(());
                    }
                    ProvidersStore::models_in(conn, &slug).await?;
                    Ok(Secrets::remove_in(conn, removal).await?)
                })
            })
            .await
    }

    /// The provider's model ids, refusing an unknown provider, inside the caller's write.
    pub(crate) async fn models_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<Vec<String>, ProvidersError> {
        ProvidersStore::models_in(conn, slug).await
    }

    /// Replaces a known provider's configuration inside the caller's write. The caller
    /// validated `input` through `validated` and checked who uses the provider.
    pub(crate) async fn update_in(
        conn: &mut SqliteConnection,
        slug: &str,
        input: &ProviderInput,
    ) -> Result<(), ProvidersError> {
        ProvidersStore::models_in(conn, slug).await?;
        ProvidersStore::upsert_in(conn, slug, input).await
    }

    /// Removes a known provider and its key inside the caller's write.
    pub(crate) async fn delete_in(
        &self,
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<(), CoreError> {
        ProvidersStore::models_in(conn, slug).await?;
        self.secrets
            .remove_scope_in(conn, &Secrets::provider_scope(slug))
            .await?;
        Ok(ProvidersStore::delete_in(conn, slug).await?)
    }
}
