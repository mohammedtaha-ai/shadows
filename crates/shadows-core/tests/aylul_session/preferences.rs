use super::*;

#[tokio::test]
async fn aylul_choices_have_no_default_model() {
    let w = world().await;
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        choices
            .models
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["fake-large", "fake-small"]
    );
    for m in choices.models {
        assert_eq!(m.label, m.id);
    }
    assert_eq!(choices.current.model, "fake-large");
    assert!(
        w.core
            .harness()
            .change_model(&w.thread, "default")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn remembered_model_is_per_agent() {
    let w = world().await;
    let other = another(&w, "Work").await;
    reply(&w, &w.thread, "fake-small", "low").await;
    reply(&w, &other, "fake-large", "max").await;
    assert_eq!(
        w.storage.remembered_settings("claude-code").await.unwrap(),
        None
    );
    assert_eq!(
        w.storage.remembered_settings("aylul:aylul").await.unwrap(),
        Some(("fake-small".into(), Some("low".into())))
    );
    w.sessions.terminate(&w.thread).await.unwrap();
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(choices.current.model, "fake-small");
    assert_eq!(choices.current.effort.as_deref(), Some("low"));
    let choices = w
        .core
        .harness()
        .change_model(&w.thread, "fake-large")
        .await
        .unwrap();
    assert_ne!(choices.current.effort.as_deref(), Some("max"));
    let choices = w
        .core
        .harness()
        .change_model(&w.thread, "fake-small")
        .await
        .unwrap();
    assert_eq!(choices.current.effort.as_deref(), Some("low"));
}
