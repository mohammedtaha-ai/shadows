//! AYLUL agents behavior.
use super::*;

#[tokio::test]
async fn deleted_agent_slug_is_never_reused() {
    let w = seeded().await;
    let core = super::threads::core(&w).await;
    w.app
        .agents()
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    core.harness()
        .delete_agent("d".into(), "work")
        .await
        .unwrap();
    assert!(w.app.agents().agent("work").await.unwrap().deleted);
    assert!(w.app.agents().agents().await.unwrap().is_empty());
    assert_eq!(
        w.app
            .agents()
            .create_agent("a2".into(), agent("Work"))
            .await
            .unwrap()
            .slug,
        "work-2"
    );
    core.harness()
        .delete_agent("d2".into(), "work-2")
        .await
        .unwrap();
    w.app
        .agents()
        .delete_provider("dp".into(), "provider")
        .await
        .unwrap();
    assert!(w.app.agents().agent("work").await.unwrap().deleted);

    w.app
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn delete_agent_removes_its_variables() {
    let w = seeded().await;
    let core = super::threads::core(&w).await;
    w.app
        .agents()
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    w.secrets
        .set_env("s".into(), "work", "EXTRA", "synthetic")
        .await
        .unwrap();
    core.harness()
        .delete_agent("d".into(), "work")
        .await
        .unwrap();
    assert!(w.secrets.list_env("work").await.unwrap().is_empty());

    w.app
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn reserved_agent_variable_is_refused() {
    let w = seeded().await;
    for name in [
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_MODEL",
        "ANTHROPIC_CUSTOM_MODEL_OPTION",
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
    ] {
        assert!(matches!(
            w.secrets
                .set_env(name.into(), "work", name, "synthetic")
                .await,
            Err(shadows_core::SecretsError::NameReserved(_))
        ));
    }

    w.app
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
