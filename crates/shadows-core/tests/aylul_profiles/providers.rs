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
            w.app
                .providers()
                .create_provider(format!("p{i}"), provider(name))
                .await
                .unwrap()
                .slug,
            expected
        );
    }
    let p = w
        .app
        .providers()
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
        input.models = models
            .into_iter()
            .map(|id| shadows_core::ProviderModelInput {
                id,
                context_window: None,
            })
            .collect();
        assert!(
            w.app
                .providers()
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
        input.base_url = Some(url.into());
        assert!(
            w.app
                .providers()
                .create_provider(uuid::Uuid::new_v4().to_string(), input)
                .await
                .is_err()
        );
    }
    for size in [999, 10_000_001] {
        let mut input = provider("Bad");
        input.models[0].context_window = Some(size);
        assert!(
            w.app
                .providers()
                .create_provider(uuid::Uuid::new_v4().to_string(), input)
                .await
                .is_err()
        );
    }
    let mut input = provider("Local");
    input.base_url = Some("http://localhost:8000".into());
    assert!(
        w.app
            .providers()
            .create_provider("local".into(), input)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn provider_in_use_cannot_be_deleted() {
    let w = seeded().await;
    w.app
        .agents()
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    assert!(matches!(
        w.app.agents().delete_provider("d".into(), "provider").await,
        Err(CoreError::ProviderInUse(_))
    ));
}

#[tokio::test]
async fn removing_an_agents_default_model_is_refused() {
    let w = seeded().await;
    w.app
        .agents()
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    let mut input = provider("Provider");
    input.models = vec![shadows_core::ProviderModelInput {
        id: "model-b".into(),
        context_window: None,
    }];
    assert!(matches!(
        w.app
            .agents()
            .update_provider("u".into(), "provider", input)
            .await,
        Err(CoreError::ProviderInUse(_))
    ));
}

#[tokio::test]
async fn delete_provider_removes_its_key() {
    let w = seeded().await;
    w.app
        .providers()
        .set_key("s".into(), "provider", "synthetic")
        .await
        .unwrap();
    w.app
        .agents()
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
    w.app
        .providers()
        .set_key("s".into(), "provider", "synthetic-provider-secret")
        .await
        .unwrap();
    let p = w.app.providers().provider("provider").await.unwrap();
    assert!(p.key_set);
    assert!(
        !serde_json::to_string(&p)
            .unwrap()
            .contains("synthetic-provider-secret")
    );
    w.app
        .providers()
        .remove_key("r".into(), "provider")
        .await
        .unwrap();
    assert!(!w.secrets.provider_key_set("provider").await.unwrap());
}

#[tokio::test]
async fn replay_with_other_input_conflicts() {
    let w = seeded().await;
    let replay = w
        .app
        .providers()
        .create_provider("p".into(), provider("Provider"))
        .await
        .unwrap();
    assert_eq!(replay.slug, "provider");
    assert!(matches!(
        w.app
            .providers()
            .create_provider("p".into(), provider("Other"))
            .await,
        Err(CoreError::Storage(
            shadows_core::StorageError::CommandConflict
        ))
    ));
}

#[tokio::test]
async fn preset_supplies_url_and_name() {
    let w = world().await;
    let input = serde_json::from_value(serde_json::json!({
        "preset": "minimax", "models": [{"id":"model-a"}]
    }))
    .unwrap();
    let p = w
        .app
        .providers()
        .create_provider("preset".into(), input)
        .await
        .unwrap();
    assert_eq!(p.name, "MiniMax");
    assert_eq!(p.base_url, "https://api.minimax.io/anthropic");
}

#[tokio::test]
async fn presets_list_in_fixed_order() {
    let w = world().await;
    let ids: Vec<_> = w
        .app
        .providers()
        .provider_presets()
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(
        ids,
        [
            "minimax",
            "minimax-china",
            "anthropic",
            "deepseek",
            "openrouter",
            "moonshot",
            "moonshot-china"
        ]
    );
}

#[tokio::test]
async fn china_presets_use_china_hosts() {
    let w = world().await;
    let presets = w.app.providers().provider_presets();
    assert_eq!(presets[1].base_url, "https://api.minimaxi.com/anthropic");
    assert_eq!(presets[6].base_url, "https://api.moonshot.cn/anthropic");
    assert_eq!(
        serde_json::to_value(&presets[1]).unwrap()["region"],
        "china"
    );
}

async fn refused(value: serde_json::Value) {
    let w = world().await;
    let input = serde_json::from_value(value).unwrap();
    assert!(matches!(
        w.app.providers().create_provider("bad".into(), input).await,
        Err(CoreError::ProfileInvalid(_))
    ));
}

#[tokio::test]
async fn preset_with_other_url_is_refused() {
    for field in ["name", "base_url"] {
        let mut v = serde_json::json!({"preset":"minimax","models":[{"id":"m"}]});
        v[field] = "https://other.test".into();
        refused(v).await;
    }
}

#[tokio::test]
async fn custom_requires_name_and_url() {
    for v in [
        serde_json::json!({"models":[{"id":"m"}]}),
        serde_json::json!({"name":"Custom","models":[{"id":"m"}]}),
        serde_json::json!({"base_url":"https://example.test","models":[{"id":"m"}]}),
    ] {
        refused(v).await;
    }
}

#[tokio::test]
async fn unknown_preset_is_refused() {
    refused(serde_json::json!({"preset":"unknown","models":[{"id":"m"}]})).await;
}

#[tokio::test]
async fn label_is_trimmed_and_empty_is_null() {
    let w = world().await;
    for (i, label, expected) in [(0, "  Work  ", Some("Work")), (1, "  ", None)] {
        let input = serde_json::from_value(serde_json::json!({
            "preset":"minimax","models":[{"id":"m"}],"label":label}))
        .unwrap();
        let p = w
            .app
            .providers()
            .create_provider(format!("p{i}"), input)
            .await
            .unwrap();
        assert_eq!(p.label.as_deref(), expected);
    }
    for label in ["x".repeat(65), "bad\nlabel".into()] {
        refused(serde_json::json!({"preset":"minimax","models":[{"id":"m"}],"label":label})).await;
    }
}

#[tokio::test]
async fn model_window_defaults_and_is_bounded() {
    let w = world().await;
    for (i, size) in [None, Some(1000), Some(10_000_000)].into_iter().enumerate() {
        let input = serde_json::from_value(serde_json::json!({
            "preset":"minimax", "models":[{"id":"m", "context_window":size}]
        }))
        .unwrap();
        let p = w
            .app
            .providers()
            .create_provider(format!("w{i}"), input)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(p).unwrap()["models"][0]["context_window"],
            size.unwrap_or(200_000)
        );
    }
    for size in [999, 10_000_001] {
        refused(serde_json::json!({"preset":"minimax",
            "models":[{"id":"m","context_window":size}]}))
        .await;
    }
}

#[tokio::test]
async fn model_ids_keep_their_rules() {
    for ids in [
        vec![],
        vec!["m"; 21],
        vec!["m", "m"],
        vec!["default"],
        vec!["a b"],
        vec![""],
        vec!["a\nb"],
    ] {
        let models: Vec<_> = ids
            .into_iter()
            .map(|id| serde_json::json!({"id":id}))
            .collect();
        refused(serde_json::json!({"preset":"minimax","models":models})).await;
    }
    refused(serde_json::json!({"preset":"minimax","models":[{"id":"a".repeat(129)}]})).await;
}

#[tokio::test]
async fn test_is_a_reserved_provider_slug() {
    let w = world().await;
    for (command, name, expected) in [
        ("first", "Test", "test-2"),
        ("second", " TEST ", "test-3"),
        ("third", "test---", "test-4"),
        ("occupied", "Test-2", "test-2-2"),
    ] {
        let p = w
            .app
            .providers()
            .create_provider(command.into(), provider(name))
            .await
            .unwrap();
        assert_eq!(p.slug, expected);
    }
    let replay = w
        .app
        .providers()
        .create_provider("first".into(), provider("Test"))
        .await
        .unwrap();
    assert_eq!(replay.slug, "test-2");
    let renamed = w
        .app
        .agents()
        .update_provider("rename".into(), "test-2", provider("Renamed"))
        .await
        .unwrap();
    assert_eq!(renamed.slug, "test-2");
    let mut a = agent("Test");
    a.provider = "test-2".into();
    assert_eq!(
        w.app
            .agents()
            .create_agent("agent".into(), a)
            .await
            .unwrap()
            .slug,
        "test"
    );
}

#[tokio::test]
async fn existing_test_provider_keeps_its_identity() {
    let w = world().await;
    let p = w
        .app
        .providers()
        .create_provider("p".into(), provider("Legacy"))
        .await
        .unwrap();
    sqlx::query("UPDATE aylul_provider SET slug='test' WHERE slug=?")
        .bind(p.slug)
        .execute(w.storage.reader())
        .await
        .unwrap();
    let p = w
        .app
        .agents()
        .update_provider("u".into(), "test", provider("Renamed"))
        .await
        .unwrap();
    assert_eq!(p.slug, "test");
    assert_eq!(
        w.app.providers().provider("test").await.unwrap().name,
        "Renamed"
    );
    w.app
        .agents()
        .delete_provider("d".into(), "test")
        .await
        .unwrap();
}
