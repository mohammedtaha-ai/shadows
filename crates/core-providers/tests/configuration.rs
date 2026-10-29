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
        Err(ProvidersError::InvalidField { .. })
    ));
    assert!(providers.providers().await.unwrap().is_empty());
    let commands: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_record")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(commands, 0);

    storage.close().await.unwrap();
}

#[tokio::test]
async fn field_refusals_name_the_first_rule() {
    let (_directory, storage, providers) = fixture().await;
    let cases = [
        (
            serde_json::json!({"name":"", "base_url":"https://example.test", "models":[{"id":"m"}]}),
            "name",
            "Name must be 1 to 256 bytes.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"http://example.test", "models":[{"id":"m"}]}),
            "base_url",
            "Base URL must use HTTPS or loopback HTTP, with a host, no credentials, and no invalid authority or whitespace.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "label":"x".repeat(65), "models":[{"id":"m"}]}),
            "label",
            "Label must be at most 64 characters.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "label":"a\nb", "models":[{"id":"m"}]}),
            "label",
            "Label must not contain control characters.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "models":[]}),
            "models",
            "Models must be 1 to 20 ids.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "models":[{"id":"bad id"}]}),
            "models[0].id",
            "Model ids must be 1 to 128 printable characters with no spaces.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "models":[{"id":"default"}]}),
            "models[0].id",
            "default is reserved.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "models":[{"id":"m"},{"id":"m"}]}),
            "models[1].id",
            "Each model is listed once.",
        ),
        (
            serde_json::json!({"name":"x", "base_url":"https://example.test", "models":[{"id":"m"},{"id":"n", "context_window":999}]}),
            "models[1].context_window",
            "Context window must be from 1000 to 10000000.",
        ),
        (
            serde_json::json!({"preset":"minimax", "name":"other", "models":[{"id":"m"}]}),
            "name",
            "Name must match the preset.",
        ),
        (
            serde_json::json!({"preset":"minimax", "base_url":"https://other.test", "models":[{"id":"m"}]}),
            "base_url",
            "Base URL must match the preset.",
        ),
    ];
    for (value, expected_field, expected_message) in cases {
        let error = providers
            .validated(serde_json::from_value(value).unwrap())
            .unwrap_err();
        assert!(
            matches!(error, ProvidersError::InvalidField { field, message } if field == expected_field && message == expected_message)
        );
    }
    storage.close().await.unwrap();
}
