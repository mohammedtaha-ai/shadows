use super::*;

#[tokio::test]
async fn opening_uses_agent_and_provider() {
    let w = world().await;
    let report = reply(&w, &w.thread, "fake-large", "high").await;
    assert_eq!(
        std::fs::canonicalize(report["config_dir"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&w.home).unwrap()
    );
    assert_eq!(
        std::fs::canonicalize(report["cwd"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&w.home).unwrap()
    );
    assert_eq!(report["has_token"], true);
    assert_eq!(report["options"]["settingSources"], json!(["user"]));
    assert!(report["append"].is_null());
    assert!(report["mcp"].is_null());
    assert!(report["options"].get("tools").is_none());
    assert_eq!(report["blocks"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn two_agents_share_one_provider_key() {
    let w = world().await;
    let other = another(&w, "Work").await;
    for thread in [&w.thread, &other] {
        w.core.harness().open_session(thread).await.unwrap();
        let report = reply(&w, thread, "fake-large", "high").await;
        assert_eq!(report["has_token"], true);
    }
    let rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM secret_env
        WHERE profile='provider/test'",
    )
    .fetch_one(w.storage.reader())
    .await
    .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn deleted_agent_with_live_adapter_is_not_found() {
    let w = world().await;
    another(&w, "Work").await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(w.sessions.live_count().await, 1);
    w.core
        .profiles()
        .delete_agent("delete".into(), "aylul")
        .await
        .unwrap();
    assert_eq!(w.sessions.live_count().await, 0);
    assert!(matches!(
        w.core
            .turns()
            .send(w.thread.clone(), request("fake-large", "high"))
            .await,
        Err(CoreError::AylulProfileNotFound(_))
    ));
    assert_eq!(
        w.core
            .threads()
            .get(&w.thread)
            .await
            .unwrap()
            .profile
            .as_deref(),
        Some("aylul")
    );
}

#[tokio::test]
async fn deleting_last_agent_is_not_found_not_unavailable() {
    let w = world().await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.core
        .profiles()
        .delete_agent("delete".into(), "aylul")
        .await
        .unwrap();
    assert!(matches!(
        w.core.harness().open_session(&w.thread).await,
        Err(CoreError::AylulProfileNotFound(_))
    ));
    assert!(matches!(
        w.core
            .turns()
            .send(w.thread.clone(), request("fake-large", "high"))
            .await,
        Err(CoreError::AylulProfileNotFound(_))
    ));
    let list = serde_json::to_value(w.core.harness().list().await.unwrap()).unwrap();
    assert_eq!(
        list.as_array()
            .unwrap()
            .iter()
            .find(|h| h["kind"] == "aylul")
            .unwrap()["available"],
        false
    );
}

#[tokio::test]
async fn missing_provider_key_is_named() {
    let w = world().await;
    w.core
        .secrets()
        .remove_provider_key("remove".into(), "test")
        .await
        .unwrap();
    assert!(matches!(w.core.harness().open_session(&w.thread).await,
        Err(CoreError::AylulKeyMissing(provider)) if provider == "test"));
    assert_eq!(w.sessions.live_count().await, 0);
}

#[tokio::test]
async fn reserved_variable_row_fails_opening() {
    let w = world().await;
    w.core
        .secrets()
        .set_env("env".into(), "aylul", "EXTRA", "synthetic")
        .await
        .unwrap();
    sqlx::query("UPDATE secret_env SET name='ANTHROPIC_BASE_URL' WHERE profile='aylul'")
        .execute(w.storage.reader())
        .await
        .unwrap();
    assert!(matches!(w.core.harness().open_session(&w.thread).await,
        Err(CoreError::SecretNameReserved(name)) if name == "ANTHROPIC_BASE_URL"));
    assert_eq!(w.sessions.live_count().await, 0);
}
