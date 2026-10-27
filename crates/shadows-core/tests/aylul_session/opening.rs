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

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
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
        WHERE profile='provider/test-2'",
    )
    .fetch_one(w.storage.reader())
    .await
    .unwrap();
    assert_eq!(rows, 1);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn deleted_agent_with_live_adapter_is_not_found() {
    let w = world().await;
    another(&w, "Work").await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(w.sessions.live_count().await, 1);
    w.core
        .harness()
        .delete_agent("delete".into(), "aylul")
        .await
        .unwrap();
    assert_eq!(w.sessions.live_count().await, 0);
    assert!(matches!(
        w.core
            .turns()
            .send(w.thread.clone(), request("fake-large", "high"))
            .await
            .map_err(CoreError::from),
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

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn deleting_last_agent_is_not_found_not_unavailable() {
    let w = world().await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.core
        .harness()
        .delete_agent("delete".into(), "aylul")
        .await
        .unwrap();
    assert!(matches!(
        w.core
            .harness()
            .open_session(&w.thread)
            .await
            .map_err(CoreError::from),
        Err(CoreError::AylulProfileNotFound(_))
    ));
    assert!(matches!(
        w.core
            .turns()
            .send(w.thread.clone(), request("fake-large", "high"))
            .await
            .map_err(CoreError::from),
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

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn missing_provider_key_is_named() {
    let w = world().await;
    w.core
        .providers()
        .remove_key("remove".into(), "test-2")
        .await
        .unwrap();
    assert!(
        matches!(w.core.harness().open_session(&w.thread).await.map_err(CoreError::from),
        Err(CoreError::AylulKeyMissing(provider)) if provider == "test-2")
    );
    assert_eq!(w.sessions.live_count().await, 0);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
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
    assert!(
        matches!(w.core.harness().open_session(&w.thread).await.map_err(CoreError::from),
        Err(CoreError::SecretNameReserved(name)) if name == "ANTHROPIC_BASE_URL")
    );
    assert_eq!(w.sessions.live_count().await, 0);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

async fn provider_window(w: &World, size: u32) {
    let input = serde_json::from_value(json!({"preset":"minimax","models":[
        {"id":"fake-large","context_window":1000},
        {"id":"fake-small","context_window":size}]}))
    .unwrap();
    let mut a = agent("AYLUL");
    a.default_model = "fake-small".into();
    w.core
        .agents()
        .update_provider(format!("p-{size}"), "test-2", input)
        .await
        .unwrap();
    w.core
        .agents()
        .update_agent(format!("a-{size}"), "aylul", a)
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_uses_the_default_models_window() {
    let w = world().await;
    provider_window(&w, 321000).await;
    let report = reply(&w, &w.thread, "fake-small", "high").await;
    assert_eq!(report["context_tokens"], "321000");
    assert_eq!(report["compact_window"], "321000");
    let settings: Value =
        serde_json::from_slice(&std::fs::read(w.home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["availableModels"],
        json!(["fake-large", "fake-small"])
    );

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn auto_compact_window_is_capped_or_omitted() {
    let w = world().await;
    for size in [99999, 100000, 1000001] {
        provider_window(&w, size).await;
        let report = reply(&w, &w.thread, "fake-small", "high").await;
        assert_eq!(report["context_tokens"], size.to_string());
        let expected = if size < 100000 {
            Value::Null
        } else {
            json!(size.min(1000000).to_string())
        };
        assert_eq!(report["compact_window"], expected);
        w.sessions.terminate(&w.thread).await.unwrap();
    }

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn switching_model_uses_the_new_models_window() {
    let w = world().await;
    provider_window(&w, 321000).await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    let choices = w
        .core
        .harness()
        .change_model(&w.thread, "fake-large")
        .await
        .unwrap();
    assert_eq!(choices.current.model, "fake-large");
    let report = reply(&w, &w.thread, "fake-large", "high").await;
    assert_eq!(report["context_tokens"], "1000");
    assert!(report["compact_window"].is_null());

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_and_turn_model_selection_use_the_same_window() {
    let w = world().await;
    provider_window(&w, 321000).await;
    let report = reply(&w, &w.thread, "fake-large", "high").await;
    assert_eq!(report["context_tokens"], "1000");
    assert!(report["compact_window"].is_null());
    w.sessions.terminate(&w.thread).await.unwrap();
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(choices.current.model, "fake-large");
    let report = reply(&w, &w.thread, "fake-large", "high").await;
    assert_eq!(report["context_tokens"], "1000");
    assert!(report["compact_window"].is_null());

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_refused_remembered_model_uses_the_fallback_models_window() {
    let w = world().await;
    let input = serde_json::from_value(json!({"preset":"minimax","models":[
        {"id":"fake-large","context_window":321000},
        {"id":"fake-small","context_window":200000},
        {"id":"fake-locked","context_window":1000}]}))
    .unwrap();
    w.core
        .agents()
        .update_provider("locked-window".into(), "test-2", input)
        .await
        .unwrap();
    w.sessions
        .remember_for_test("aylul:aylul", "fake-locked", None)
        .await;
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(choices.current.model, "fake-large");
    let report = reply(&w, &w.thread, "fake-large", "high").await;
    assert_eq!(report["context_tokens"], "321000");
    assert_eq!(report["compact_window"], "321000");

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_leased_adapter_is_not_replaced_by_a_model_pick() {
    let w = world().await;
    provider_window(&w, 321000).await;
    let opened = w.sessions.open(&w.thread).await.unwrap();
    let events = w.sessions.lease_events(&w.thread, &opened).await.unwrap();
    let pid = w.sessions.pid(&w.thread).await;
    let changed = w.core.harness().change_model(&w.thread, "fake-large").await;
    assert!(matches!(
        changed.map_err(CoreError::from),
        Err(CoreError::Refused { .. })
    ));
    assert_eq!(w.sessions.pid(&w.thread).await, pid);
    w.sessions
        .give_back_events(&w.thread, &opened, events)
        .await;
    let report = reply(&w, &w.thread, "fake-small", "high").await;
    assert_eq!(report["context_tokens"], "321000");

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// The connection notices its adapter's exit on its own reader task, so a
/// stopped adapter reads as closed a moment later on a slow machine.
async fn closes(closed: impl Fn() -> bool) -> bool {
    for _ in 0..100 {
        if closed() {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    closed()
}

#[tokio::test]
async fn model_replacement_keeps_turn_output_and_closes_the_current_adapter() {
    let w = world().await;
    provider_window(&w, 321000).await;
    let first = reply(&w, &w.thread, "fake-small", "high").await;
    assert_eq!(first["context_tokens"], "321000");
    let previous = w.sessions.open(&w.thread).await.unwrap();
    let old_pid = w.sessions.pid(&w.thread).await.unwrap();
    w.core
        .harness()
        .change_model(&w.thread, "fake-large")
        .await
        .unwrap();
    let current_pid = w.sessions.pid(&w.thread).await.unwrap();
    assert_ne!(current_pid, old_pid);
    assert!(closes(|| previous.connection().is_closed()).await);
    w.sessions
        .terminate_adapter(&w.thread, &previous)
        .await
        .unwrap();
    assert_eq!(w.sessions.pid(&w.thread).await, Some(current_pid));
    let second = reply(&w, &w.thread, "fake-large", "high").await;
    assert_eq!(second["context_tokens"], "1000");
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].kind, ThreadEntryKind::UserMessage);
    assert_eq!(entries[1].kind, ThreadEntryKind::AgentMessage);
    assert_eq!(entries[2].kind, ThreadEntryKind::UserMessage);
    assert_eq!(entries[3].kind, ThreadEntryKind::AgentMessage);
    let current = w.sessions.open(&w.thread).await.unwrap();
    w.sessions.close_all().await.unwrap();
    assert_eq!(w.sessions.live_count().await, 0);
    assert!(closes(|| current.connection().is_closed()).await);
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    assert_eq!(w.sessions.live_count().await, 0);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_failed_model_replacement_cannot_prompt_with_stale_process_limits() {
    let w = world().await;
    provider_window(&w, 321000).await;
    let opened = w.sessions.open(&w.thread).await.unwrap();
    assert!(w.sessions.force_termination_failure(&w.thread).await);
    assert!(
        w.core
            .harness()
            .change_model(&w.thread, "fake-large")
            .await
            .is_err()
    );
    let sent = w
        .core
        .turns()
        .send(w.thread.clone(), request("fake-large", "high"))
        .await
        .map_err(CoreError::from);
    assert!(
        matches!(sent, Err(CoreError::HarnessStartFailed(_))),
        "{sent:?}"
    );
    assert!(
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(w.sessions.lease_events(&w.thread, &opened).await.is_err());

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_names_the_person() {
    let w = world().await;
    for (i, calls, expected) in [(0, Some("Lex"), "Lex"), (1, None, "Alex")] {
        let input = serde_json::from_value(json!({"name":"Alex", "aylul_calls_me":calls,
            "language":"en", "theme":"dark"}))
        .unwrap();
        w.core
            .person()
            .update_person(format!("person-{i}"), input)
            .await
            .unwrap();
        let report = reply(&w, &w.thread, "fake-large", "high").await;
        let append = report["append"].as_str().unwrap();
        assert!(append.contains(expected));
        assert_eq!(append.lines().count(), 1);
        let changed = serde_json::from_value(json!({"name":"Alex",
            "aylul_calls_me":"Changed", "language":"en", "theme":"dark"}))
        .unwrap();
        w.core
            .person()
            .update_person(format!("live-person-{i}"), changed)
            .await
            .unwrap();
        let retained = reply(&w, &w.thread, "fake-large", "high").await;
        assert_eq!(retained["append"], report["append"]);
        w.sessions.terminate(&w.thread).await.unwrap();
    }

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn opening_without_person_appends_nothing() {
    let w = world().await;
    let empty = serde_json::from_value(json!({"name":"  ", "aylul_calls_me":null,
        "language":null, "theme":"system"}))
    .unwrap();
    w.core
        .person()
        .update_person("empty-person".into(), empty)
        .await
        .unwrap();
    let report = reply(&w, &w.thread, "fake-large", "high").await;
    assert!(report["append"].is_null());

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
