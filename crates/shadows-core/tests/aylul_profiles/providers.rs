//! AYLUL providers behavior.
use super::*;

#[tokio::test]
async fn provider_slug_rules() {
    let w = world().await;
    for (i, name, expected) in [
        (0, "مزود", "provider"),
        (1, "مزود", "provider-2"),
        (2, "123", "provider-123"),
        (3, "Work", "work"),
    ] {
        assert_eq!(
            w.profiles
                .create_provider(format!("p{i}"), provider(name))
                .await
                .unwrap()
                .slug,
            expected
        );
    }
    let p = w
        .profiles
        .create_provider("long".into(), provider(&"a".repeat(40)))
        .await
        .unwrap();
    assert_eq!(p.slug, "a".repeat(29));
}

#[tokio::test]
async fn invalid_provider_input_is_refused() {
    let w = world().await;
    for models in [
        vec![],
        vec!["m".into(); 21],
        vec!["a".into(), "a".into()],
        vec!["a b".into()],
        vec!["default".into()],
    ] {
        let mut input = provider("Bad");
        input.models = models;
        assert!(
            w.profiles
                .create_provider(uuid::Uuid::new_v4().to_string(), input)
                .await
                .is_err()
        );
    }
    for url in [
        "http://example.com",
        "https://",
        "file:///tmp/model",
        "https://u:p@host",
    ] {
        let mut input = provider("Bad");
        input.base_url = url.into();
        assert!(
            w.profiles
                .create_provider(uuid::Uuid::new_v4().to_string(), input)
                .await
                .is_err()
        );
    }
    for size in [999, 10_000_001] {
        let mut input = provider("Bad");
        input.context_window = Some(size);
        assert!(
            w.profiles
                .create_provider(uuid::Uuid::new_v4().to_string(), input)
                .await
                .is_err()
        );
    }
    let mut input = provider("Local");
    input.base_url = "http://localhost:8000".into();
    assert!(
        w.profiles
            .create_provider("local".into(), input)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn provider_in_use_cannot_be_deleted() {
    let w = seeded().await;
    w.profiles
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    assert!(matches!(
        w.profiles.delete_provider("d".into(), "provider").await,
        Err(CoreError::ProviderInUse(_))
    ));
}

#[tokio::test]
async fn removing_an_agents_default_model_is_refused() {
    let w = seeded().await;
    w.profiles
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    let mut input = provider("Provider");
    input.models = vec!["model-b".into()];
    assert!(matches!(
        w.profiles
            .update_provider("u".into(), "provider", input)
            .await,
        Err(CoreError::ProviderInUse(_))
    ));
}

#[tokio::test]
async fn delete_provider_removes_its_key() {
    let w = seeded().await;
    w.secrets
        .set_provider_key("s".into(), "provider", "synthetic")
        .await
        .unwrap();
    w.profiles
        .delete_provider("d".into(), "provider")
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secret_env WHERE profile = ?")
        .bind("provider/provider")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn provider_key_is_write_only() {
    let w = seeded().await;
    w.secrets
        .set_provider_key("s".into(), "provider", "synthetic-provider-secret")
        .await
        .unwrap();
    let p = w.profiles.provider("provider").await.unwrap();
    assert!(p.key_set);
    assert!(
        !serde_json::to_string(&p)
            .unwrap()
            .contains("synthetic-provider-secret")
    );
    w.secrets
        .remove_provider_key("r".into(), "provider")
        .await
        .unwrap();
    assert!(!w.secrets.provider_key_set("provider").await.unwrap());
}

#[tokio::test]
async fn replay_with_other_input_conflicts() {
    let w = seeded().await;
    let replay = w
        .profiles
        .create_provider("p".into(), provider("Provider"))
        .await
        .unwrap();
    assert_eq!(replay.slug, "provider");
    assert!(matches!(
        w.profiles
            .create_provider("p".into(), provider("Other"))
            .await,
        Err(CoreError::Storage(
            shadows_core::StorageError::CommandConflict
        ))
    ));
}
