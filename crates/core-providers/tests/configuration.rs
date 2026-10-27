use std::sync::Arc;

use core_db::{Storage, StorageError};
use core_providers::{ProviderInput, ProviderModelInput, Providers, ProvidersError};
use core_secrets::{SecretKey, Secrets};

async fn fixture() -> (tempfile::TempDir, Arc<Storage>, Arc<Providers>) {
    let directory = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&directory.path().join("provider.sqlite"))
            .await
            .unwrap(),
    );
    let secrets = Secrets::new(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let providers = Providers::new(storage.clone(), secrets);
    (directory, storage, providers)
}

fn input(name: &str) -> ProviderInput {
    ProviderInput {
        name: Some(name.to_owned()),
        base_url: Some("https://example.test/v1".to_owned()),
        preset: None,
        label: None,
        models: vec![ProviderModelInput {
            id: "fixture-model".to_owned(),
            context_window: None,
        }],
    }
}

#[tokio::test]
async fn public_configuration_replays_and_keeps_keys_out_of_metadata() {
    let (_directory, _storage, providers) = fixture().await;
    let first = providers
        .create_provider("create".into(), input("Fixture"))
        .await
        .unwrap();
    assert_eq!(first.models[0].context_window, 200_000);
    assert!(!first.key_set);
    providers
        .set_key("set-key".into(), &first.slug, "synthetic-provider-key")
        .await
        .unwrap();
    let replay = providers
        .create_provider("create".into(), input("Fixture"))
        .await
        .unwrap();
    assert_eq!(replay.slug, first.slug);
    assert!(replay.key_set);
    assert!(
        !serde_json::to_string(&replay)
            .unwrap()
            .contains("synthetic-provider-key")
    );
    assert_eq!(providers.providers().await.unwrap().len(), 1);
    assert!(matches!(
        providers
            .create_provider("create".into(), input("Other"))
            .await,
        Err(ProvidersError::Storage(StorageError::CommandConflict))
    ));
    providers
        .remove_key("remove-key".into(), &first.slug)
        .await
        .unwrap();
    assert!(!providers.provider(&first.slug).await.unwrap().key_set);

    _storage.close().await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_writes_no_provider_or_command() {
    let (_directory, storage, providers) = fixture().await;
    let mut invalid = input("Fixture");
    invalid.base_url = Some("http://example.test".to_owned());
    assert!(matches!(
        providers.create_provider("invalid".into(), invalid).await,
        Err(ProvidersError::InvalidProfile(_))
    ));
    assert!(providers.providers().await.unwrap().is_empty());
    let commands: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_record")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(commands, 0);

    storage.close().await.unwrap();
}
