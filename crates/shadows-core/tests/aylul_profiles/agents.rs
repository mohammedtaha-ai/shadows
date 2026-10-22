//! AYLUL agents behavior.
use super::*;

#[tokio::test]
async fn agent_default_must_be_a_provider_model() {
    let w = seeded().await;
    let mut input = agent("Work");
    input.default_model = "missing".into();
    assert!(matches!(
        w.app.agents().create_agent("a".into(), input).await,
        Err(CoreError::ProfileInvalid(_))
    ));
}

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
            Err(CoreError::SecretNameReserved(_))
        ));
    }
}

#[tokio::test]
async fn color_defaults_to_purple() {
    let w = seeded().await;
    let a = w
        .app
        .agents()
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(a).unwrap()["character_color"],
        "purple"
    );
}

async fn colored_agent(
    w: &World,
    color: &str,
    command: &str,
) -> Result<shadows_core::Agent, CoreError> {
    let input = serde_json::from_value(serde_json::json!({"name":"Work", "provider":"provider",
        "default_model":"model-a", "character_color":color}))
    .unwrap();
    w.app.agents().create_agent(command.into(), input).await
}

#[tokio::test]
async fn color_id_shape_is_checked() {
    let w = seeded().await;
    for (i, color) in [
        "",
        "Blue",
        "1blue",
        "-blue",
        "blue red",
        "blue_2",
        "b\n",
        "a".repeat(33).as_str(),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(matches!(
            colored_agent(&w, color, &format!("c{i}")).await,
            Err(CoreError::ProfileInvalid(_))
        ));
    }
}

#[tokio::test]
async fn unknown_but_well_formed_color_is_kept() {
    let w = seeded().await;
    let a = colored_agent(&w, "future-color-2", "a").await.unwrap();
    assert_eq!(
        serde_json::to_value(a).unwrap()["character_color"],
        "future-color-2"
    );
}

#[tokio::test]
async fn update_without_color_keeps_it() {
    let w = seeded().await;
    colored_agent(&w, "future-color", "a").await.unwrap();
    let a = w
        .app
        .agents()
        .update_agent("u".into(), "work", agent("Work"))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(a).unwrap()["character_color"],
        "future-color"
    );
    let explicit = serde_json::from_value(serde_json::json!({"name":"Work", "provider":"provider",
        "default_model":"model-a", "character_color":"future-color"}))
    .unwrap();
    w.app
        .agents()
        .update_agent("u".into(), "work", explicit)
        .await
        .unwrap();
    let different =
        serde_json::from_value(serde_json::json!({"name":"Work", "provider":"provider",
        "default_model":"model-a", "character_color":"other-color"}))
        .unwrap();
    assert!(matches!(
        w.app
            .agents()
            .update_agent("u".into(), "work", different)
            .await,
        Err(CoreError::Storage(
            shadows_core::StorageError::CommandConflict
        ))
    ));
}
