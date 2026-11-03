use super::*;

async fn tool_item(w: &World) -> String {
    let input = serde_json::from_value(json!({"name":"docs","transport":{
        "type":"http","url":"https://example.com/mcp","headers":[]
    }}))
    .unwrap();
    let id = w
        .core
        .extensions()
        .add_mcp("mcp".into(), input)
        .await
        .unwrap()
        .id;
    w.core
        .extensions()
        .set_agent_extension(
            "enable".into(),
            "aylul",
            core_extensions::ExtensionKind::Mcp,
            &id,
            true,
        )
        .await
        .unwrap();
    let rules = serde_json::from_value(json!([
        {"tool":"read","rule":"allow"},{"tool":"exec","rule":"deny"}
    ]))
    .unwrap();
    w.core
        .extensions()
        .set_tool_rules("rules".into(), "aylul", &id, rules)
        .await
        .unwrap();
    id
}

fn read_settings(w: &World) -> Value {
    serde_json::from_slice(&std::fs::read(w.home.join("settings.json")).unwrap()).unwrap()
}

async fn close_world(w: World) {
    w.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_writes_allow_and_deny_rules() {
    let w = world().await;
    tool_item(&w).await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    let settings = read_settings(&w);
    assert_eq!(settings["permissions"]["allow"], json!(["mcp__docs__read"]));
    assert!(
        settings["permissions"]["deny"]
            .as_array()
            .unwrap()
            .contains(&json!("mcp__docs__exec"))
    );
    close_world(w).await;
}

#[tokio::test]
async fn dotted_tool_rules_use_claudes_normalized_name() {
    let w = world().await;
    let id = tool_item(&w).await;
    let rules = serde_json::from_value(json!([{"tool":"file.read","rule":"deny"}])).unwrap();
    w.core
        .extensions()
        .set_tool_rules("dotted".into(), "aylul", &id, rules)
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert!(
        read_settings(&w)["permissions"]["deny"]
            .as_array()
            .unwrap()
            .contains(&json!("mcp__docs__file_read"))
    );
    close_world(w).await;
}

#[tokio::test]
async fn colliding_tool_names_keep_only_the_block() {
    let w = world().await;
    let id = tool_item(&w).await;
    let rules = serde_json::from_value(json!([
        {"tool":"file.read","rule":"allow"},{"tool":"file_read","rule":"deny"}
    ]))
    .unwrap();
    w.core
        .extensions()
        .set_tool_rules("collide".into(), "aylul", &id, rules)
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    let settings = read_settings(&w).to_string();
    assert_eq!(settings.matches("mcp__docs__file_read").count(), 1);
    assert!(
        read_settings(&w)["permissions"]["deny"]
            .as_array()
            .unwrap()
            .contains(&json!("mcp__docs__file_read"))
    );
    close_world(w).await;
}

#[tokio::test]
async fn an_agent_without_rules_opens_with_odd_permissions() {
    let w = world().await;
    let id = tool_item(&w).await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    w.core
        .extensions()
        .set_agent_extension(
            "off".into(),
            "aylul",
            core_extensions::ExtensionKind::Mcp,
            &id,
            false,
        )
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    settings(&w, json!({"permissions":{"allow":"Bash(ls)"}}));
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(read_settings(&w)["permissions"]["allow"], json!("Bash(ls)"));
    close_world(w).await;
}

#[tokio::test]
async fn rules_of_an_item_switched_off_are_removed() {
    let w = world().await;
    let id = tool_item(&w).await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert!(read_settings(&w).to_string().contains("mcp__docs__read"));
    w.sessions.terminate(&w.thread).await.unwrap();
    w.core
        .extensions()
        .set_agent_extension(
            "off".into(),
            "aylul",
            core_extensions::ExtensionKind::Mcp,
            &id,
            false,
        )
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    let settings = read_settings(&w).to_string();
    assert!(!settings.contains("mcp__docs__"));
    close_world(w).await;
}

#[tokio::test]
async fn foreign_rules_are_kept() {
    let w = world().await;
    let id = tool_item(&w).await;
    settings(
        &w,
        json!({"permissions":{"allow":["Bash(ls)","mcp__docs__read"]}}),
    );
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert!(read_settings(&w).to_string().contains("mcp__docs__exec"));
    w.sessions.terminate(&w.thread).await.unwrap();
    w.core
        .extensions()
        .set_agent_extension(
            "off".into(),
            "aylul",
            core_extensions::ExtensionKind::Mcp,
            &id,
            false,
        )
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        read_settings(&w)["permissions"]["allow"],
        json!(["Bash(ls)", "mcp__docs__read"])
    );
    close_world(w).await;
}

#[tokio::test]
async fn foreign_rule_order_is_kept() {
    let w = world().await;
    tool_item(&w).await;
    settings(&w, json!({"permissions":{"allow":["Bash(ls)","Alpha"]}}));
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        read_settings(&w)["permissions"]["allow"],
        json!(["Bash(ls)", "Alpha", "mcp__docs__read"])
    );
    close_world(w).await;
}

#[tokio::test]
async fn a_renamed_item_leaves_no_old_rule() {
    let w = world().await;
    let id = tool_item(&w).await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    let input = serde_json::from_value(json!({"name":"docs2","transport":{
        "type":"http","url":"https://example.com/mcp","headers":[]
    }}))
    .unwrap();
    w.core
        .extensions()
        .update_mcp("rename".into(), &id, input)
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    let settings = read_settings(&w).to_string();
    assert!(!settings.contains("mcp__docs__"));
    assert!(settings.contains("mcp__docs2__read"));
    assert!(settings.contains("mcp__docs2__exec"));
    close_world(w).await;
}

#[tokio::test]
async fn interrupted_rule_projection_is_cleaned_at_next_opening() {
    let w = world().await;
    settings(
        &w,
        json!({"permissions":{"allow":["Bash(ls)","mcp__old__read"]}}),
    );
    std::fs::write(
        w.home.join(".aylul-tool-rules.json"),
        serde_json::to_vec(&json!({
            "allow":["mcp__old__read","mcp__new__read"],"deny":[]
        }))
        .unwrap(),
    )
    .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        read_settings(&w)["permissions"]["allow"],
        json!(["Bash(ls)"])
    );
    close_world(w).await;
}

#[tokio::test]
async fn an_unwritable_rule_manifest_leaves_settings_unchanged() {
    let w = world().await;
    tool_item(&w).await;
    std::fs::create_dir(w.home.join(".aylul-tool-rules.json")).unwrap();
    let before = read_settings(&w)["permissions"].clone();
    assert!(w.core.harness().open_session(&w.thread).await.is_err());
    assert_eq!(read_settings(&w)["permissions"], before);
    close_world(w).await;
}

#[tokio::test]
async fn opening_writes_the_stored_instructions() {
    let w = world().await;
    let mut input = agent("AYLUL");
    input.instructions = Some("Use the stored agent instructions.".into());
    w.core
        .agents()
        .update_agent("instructions".into(), "aylul", input)
        .await
        .unwrap();
    std::fs::write(w.home.join("CLAUDE.md"), "stale home file").unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(w.home.join("CLAUDE.md")).unwrap(),
        "Your name is AYLUL.\n\nUse the stored agent instructions."
    );
    w.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_writes_the_template_for_empty_instructions() {
    let w = world().await;
    std::fs::write(w.home.join("CLAUDE.md"), "stale home file").unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(w.home.join("CLAUDE.md")).unwrap(),
        format!(
            "Your name is AYLUL.\n\n{}",
            w.core.agents().instructions_template()
        )
    );
    w.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_leaves_an_unimported_home_file() {
    let w = world().await;
    sqlx::query("UPDATE aylul_agent SET instructions=NULL WHERE slug='aylul'")
        .execute(w.storage.reader())
        .await
        .unwrap();
    std::fs::write(w.home.join("CLAUDE.md"), "legacy instructions").unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(w.home.join("CLAUDE.md")).unwrap(),
        "legacy instructions"
    );
    w.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

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
    assert!(
        matches!(w.core.harness().open_session(&w.thread).await.map_err(CoreError::from),
        Err(CoreError::SecretNameReserved(name)) if name=="ANTHROPIC_BASE_URL")
    );
    assert_eq!(w.sessions.live_count().await, 0);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn model_overrides_fails_opening() {
    let w = world().await;
    settings(&w, json!({"modelOverrides":{}}));
    let error = w.core.harness().open_session(&w.thread).await.unwrap_err();
    let error = CoreError::from(error);
    assert!(matches!(error, CoreError::ProfileInvalid(_)));
    assert!(error.to_string().contains("modelOverrides"));
    assert_eq!(w.sessions.live_count().await, 0);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
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

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn concurrent_openings_keep_settings_valid() {
    let w = world().await;
    settings(&w, json!({"permissions":{"allow":["Read"]}}));
    let project = w.core.agents().assistant_project().await.unwrap();
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

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn non_object_settings_json_fails_named() {
    let w = world().await;
    settings(&w, json!(["keep"]));
    let before = std::fs::read(w.home.join("settings.json")).unwrap();
    let error = w.core.harness().open_session(&w.thread).await.unwrap_err();
    assert!(error.to_string().contains("settings.json"));
    assert_eq!(std::fs::read(w.home.join("settings.json")).unwrap(), before);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
