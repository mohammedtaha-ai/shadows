use super::*;

fn settings(w: &World, value: Value) {
    std::fs::write(
        w.home.join("settings.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn settings_env_base_url_fails_opening() {
    let w = world().await;
    settings(
        &w,
        json!({"env":{"ANTHROPIC_BASE_URL":"https://untrusted.example"}}),
    );
    assert!(matches!(w.core.harness().open_session(&w.thread).await,
        Err(CoreError::SecretNameReserved(name)) if name=="ANTHROPIC_BASE_URL"));
    assert_eq!(w.sessions.live_count().await, 0);
}

#[tokio::test]
async fn model_overrides_fails_opening() {
    let w = world().await;
    settings(&w, json!({"modelOverrides":{}}));
    let error = w.core.harness().open_session(&w.thread).await.unwrap_err();
    assert!(matches!(error, CoreError::ProfileInvalid(_)));
    assert!(error.to_string().contains("modelOverrides"));
    assert_eq!(w.sessions.live_count().await, 0);
}

#[tokio::test]
async fn settings_json_keeps_other_keys() {
    let w = world().await;
    settings(
        &w,
        json!({"env":{"EXTRA":"keep"},"permissions":{"allow":["Read"]}}),
    );
    w.core.harness().open_session(&w.thread).await.unwrap();
    let path = w.home.join("settings.json");
    let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["env"], json!({"EXTRA":"keep"}));
    assert_eq!(value["permissions"], json!({"allow":["Read"]}));
    assert_eq!(
        value["availableModels"],
        json!(["fake-large", "fake-small"])
    );
    assert_eq!(value["model"], "fake-large");
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("synthetic-stored-one")
    );
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
}

#[tokio::test]
async fn concurrent_openings_keep_settings_valid() {
    let w = world().await;
    settings(&w, json!({"permissions":{"allow":["Read"]}}));
    let project = w.core.profiles().assistant_project().await.unwrap();
    let other = w
        .core
        .threads()
        .create(
            "second".into(),
            &project,
            "Second",
            Some("aylul"),
            Some("aylul"),
        )
        .await
        .unwrap()
        .id;
    let (a, b) = tokio::join!(
        w.core.harness().open_session(&w.thread),
        w.core.harness().open_session(&other)
    );
    a.unwrap();
    b.unwrap();
    let value: Value =
        serde_json::from_slice(&std::fs::read(w.home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(value["permissions"], json!({"allow":["Read"]}));
    assert_eq!(
        value["availableModels"],
        json!(["fake-large", "fake-small"])
    );
}

#[tokio::test]
async fn non_object_settings_json_fails_named() {
    let w = world().await;
    settings(&w, json!(["keep"]));
    let before = std::fs::read(w.home.join("settings.json")).unwrap();
    let error = w.core.harness().open_session(&w.thread).await.unwrap_err();
    assert!(error.to_string().contains("settings.json"));
    assert_eq!(std::fs::read(w.home.join("settings.json")).unwrap(), before);
}
