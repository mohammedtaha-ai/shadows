//! Agent-scoped AYLUL sessions through the real core and synthetic ACP process.
use serde_json::{Value, json};
use shadows_core::testing::{self, LiveHandles, Runtime, Sessions, SessionsConfig, Storage, acp};
use shadows_core::{
    AgentInput, AppCore, AylulConfig, CoreError, CoreParts, OperationId, ProviderInput, SecretKey,
    SendTurn, ThreadId,
};
use std::sync::Arc;
use std::time::Duration;

struct World {
    _tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    core: Arc<AppCore>,
    sessions: Arc<Sessions>,
    thread: ThreadId,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let secrets = testing::secrets(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let runtime = Arc::new(Runtime::start(storage.clone()).await.unwrap().0);
    let sessions = Sessions::new(
        acp::fake_adapter(),
        storage.clone(),
        SessionsConfig {
            secrets: Some(secrets.clone()),
            aylul: Some(AylulConfig {
                home_root: root.clone(),
                project_slug: "aylul".into(),
            }),
            ..acp::test_config()
        },
    );
    let core = AppCore::assemble(CoreParts {
        secrets,
        storage: storage.clone(),
        runtime,
        sessions: sessions.clone(),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(64).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    core.projects()
        .ensure_assistant("aylul", "AYLUL", &root)
        .await
        .unwrap();
    core.providers()
        .create_provider(
            "provider".into(),
            ProviderInput {
                preset: None,
                label: None,
                name: Some("Test".into()),
                base_url: Some("https://example.com/anthropic".into()),
                models: vec!["fake-large".into(), "fake-small".into()]
                    .into_iter()
                    .map(|id| shadows_core::ProviderModelInput {
                        id,
                        context_window: Some(1_000_000),
                    })
                    .collect(),
            },
        )
        .await
        .unwrap();
    core.providers()
        .set_key("key".into(), "test-2", "synthetic-stored-one")
        .await
        .unwrap();
    core.agents()
        .create_agent("agent".into(), agent("AYLUL"))
        .await
        .unwrap();
    let project = core.agents().assistant_project().await.unwrap();
    let thread = core
        .threads()
        .create(
            "thread".into(),
            &project,
            "Chat",
            Some("aylul"),
            Some("aylul"),
        )
        .await
        .unwrap()
        .id;
    World {
        _tmp: tmp,
        storage,
        core,
        sessions,
        thread,
    }
}

fn agent(name: &str) -> AgentInput {
    AgentInput {
        character_color: None,
        name: name.into(),
        provider: "test-2".into(),
        default_model: "fake-large".into(),
    }
}

fn request(model: &str, effort: &str) -> SendTurn {
    SendTurn {
        command_id: uuid::Uuid::new_v4().to_string(),
        prompt: "report".into(),
        model: model.into(),
        effort: Some(effort.into()),
        mode: "acceptEdits".into(),
        focus: None,
        plan: None,
        client_tab: None,
    }
}

async fn send(w: &World, prompt: &str) -> OperationId {
    let mut input = request("fake-large", "high");
    input.prompt = prompt.into();
    let op = w.core.turns().send(w.thread.clone(), input).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let row = w.storage.get_operation(&op).await.unwrap();
            if row.finished_at.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    op
}

async fn report(w: &World) -> serde_json::Value {
    serde_json::to_value(
        w.core
            .usage()
            .usage("2000-01-01T00:00:00Z", "2100-01-01T00:00:00Z")
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn usage_counts_cache_once() {
    let w = world().await;
    send(&w, "turn-usage").await;
    let r = report(&w).await;
    assert_eq!(r["by_provider"][0]["requests"], 1);
    assert_eq!(r["by_provider"][0]["tokens_in"], 60);
    assert_eq!(r["by_provider"][0]["tokens_out"], 4);
    assert_eq!(r["by_model"][0]["model"], "fake-large");
}

#[tokio::test]
async fn usage_ends_atomically_once() {
    let w = world().await;
    sqlx::query(
        "CREATE TRIGGER reject_usage BEFORE INSERT ON aylul_usage
        BEGIN SELECT RAISE(ABORT,'synthetic usage write failure'); END",
    )
    .execute(w.storage.reader())
    .await
    .unwrap();
    let mut input = request("fake-large", "high");
    input.prompt = "turn-usage".into();
    let op = w
        .core
        .turns()
        .send(w.thread.clone(), input.clone())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if w.sessions.offered(&w.thread).await.is_some() {
                let entries = w.core.threads().entries(&w.thread).await.unwrap();
                if entries
                    .iter()
                    .any(|e| e.kind == shadows_core::ThreadEntryKind::AgentMessage)
                {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        w.storage.get_operation(&op).await.unwrap().status_kind,
        "Running"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aylul_usage")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
    let ended: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM durable_event
        WHERE kind='OperationCompleted'",
    )
    .fetch_one(w.storage.reader())
    .await
    .unwrap();
    assert_eq!(ended, 0);
    sqlx::query("DROP TRIGGER reject_usage")
        .execute(w.storage.reader())
        .await
        .unwrap();
    let seen = shadows_agent::events::TurnObservation::default();
    w.storage
        .mark_operation_completed(&op, json!({}), &seen)
        .await
        .unwrap();
    assert!(
        w.storage
            .mark_operation_completed(&op, json!({}), &seen)
            .await
            .is_err()
    );
    assert_eq!(
        w.core.turns().send(w.thread.clone(), input).await.unwrap(),
        op
    );
    assert_eq!(report(&w).await["by_provider"][0]["requests"], 1);
}

#[tokio::test]
async fn usage_snapshot_survives_provider_changes() {
    let w = world().await;
    w.core.harness().open_session(&w.thread).await.unwrap();
    let p = serde_json::from_value(json!({"name":"Renamed","label":"new",
        "base_url":"https://other.test","models":[{"id":"fake-large"},{"id":"fake-small"}]}))
    .unwrap();
    w.core
        .agents()
        .update_provider("rename".into(), "test-2", p)
        .await
        .unwrap();
    send(&w, "turn-usage").await;
    w.core
        .harness()
        .delete_agent("delete-agent".into(), "aylul")
        .await
        .unwrap();
    w.core
        .agents()
        .delete_provider("delete-provider".into(), "test-2")
        .await
        .unwrap();
    let r = report(&w).await;
    assert_eq!(r["by_provider"][0]["provider_name"], "Test");
    assert_eq!(r["by_provider"][0]["label"], Value::Null);
}

#[tokio::test]
async fn usage_refusal_cancel_and_missing_are_honest() {
    let w = world().await;
    for prompt in ["turn-usage-refused", "turn-usage-cancelled", "hi"] {
        send(&w, prompt).await;
    }
    let r = report(&w).await;
    assert_eq!(r["by_provider"][0]["requests"], 3);
    assert_eq!(r["by_provider"][0]["tokens_in"], 120);
    assert_eq!(r["by_provider"][0]["tokens_out"], 8);
    let missing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM aylul_usage WHERE input_tokens IS NULL")
            .fetch_one(w.storage.reader())
            .await
            .unwrap();
    assert_eq!(missing, 1);
    let runtime = Runtime::start(w.storage.clone()).await.unwrap().0;
    let pending = w
        .storage
        .create_pending_operation(&w.thread, &runtime.instance_id)
        .await
        .unwrap();
    w.storage.mark_operation_cancelled(&pending).await.unwrap();
    assert_eq!(report(&w).await["by_provider"][0]["requests"], 3);
    let mut input = request("fake-large", "high");
    input.prompt = "ignore-cancel".into();
    let forced = w.core.turns().send(w.thread.clone(), input).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while w.storage.get_operation(&forced).await.unwrap().status_kind != "Running" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    w.core.turns().stop(&forced).await.unwrap();
    assert_eq!(
        w.storage.get_operation(&forced).await.unwrap().status_kind,
        "Cancelled"
    );
    let unknown: Option<i64> =
        sqlx::query_scalar("SELECT input_tokens FROM aylul_usage WHERE operation_id=?")
            .bind(forced.as_str())
            .fetch_one(w.storage.reader())
            .await
            .unwrap();
    assert!(unknown.is_none());
    assert_eq!(report(&w).await["by_provider"][0]["requests"], 4);
}

#[tokio::test]
async fn usage_costs_require_trusted_turn_delta() {
    let w = world().await;
    let p = serde_json::from_value(json!({"preset":"anthropic",
        "models":[{"id":"fake-large"},{"id":"fake-small"}]}))
    .unwrap();
    w.core
        .agents()
        .update_provider("anthropic".into(), "test-2", p)
        .await
        .unwrap();
    send(&w, "turn-usage").await;
    send(&w, "turn-usage-second").await;
    let values: Vec<Option<f64>> = sqlx::query_scalar(
        "SELECT cost FROM aylul_usage
        ORDER BY recorded_at,operation_id",
    )
    .fetch_all(w.storage.reader())
    .await
    .unwrap();
    assert!((values[0].unwrap() - 0.2).abs() < 1e-12);
    assert!((values[1].unwrap() - 0.3).abs() < 1e-12);
    w.sessions.terminate(&w.thread).await.unwrap();
    send(&w, "turn-usage").await;
    // Resume has no trusted previous cumulative baseline.
    assert!(report(&w).await["by_provider"][0]["cost"].is_null());
    let w = world().await;
    send(&w, "turn-usage").await;
    assert!(report(&w).await["by_provider"][0]["cost"].is_null());
    assert!(
        sqlx::query("UPDATE aylul_usage SET cost=1,currency=NULL")
            .execute(w.storage.reader())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn usage_periods_group_and_sort() {
    let w = world().await;
    assert_eq!(report(&w).await, json!({"by_provider":[],"by_model":[]}));
    for (id, slug, name, when) in [
        ("1", "z", "Old", "2026-10-21T00:00:00Z"),
        ("2", "a", "A", "2026-10-21T00:00:00Z"),
        ("3", "z", "New", "2026-10-21T01:00:00Z"),
    ] {
        sqlx::query(
            "INSERT INTO aylul_usage(operation_id,thread_id,agent_slug,provider_slug,
            provider_name,model,input_tokens,output_tokens,recorded_at)
            VALUES (?,'t','a',?,?,'shared',10,2,?)",
        )
        .bind(id)
        .bind(slug)
        .bind(name)
        .bind(when)
        .execute(w.storage.reader())
        .await
        .unwrap();
    }
    let r = serde_json::to_value(
        w.core
            .usage()
            .usage("2026-10-21T03:00:00+03:00", "2026-10-21T01:00:00Z")
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(r["by_provider"][0]["provider_slug"], "a");
    assert_eq!(r["by_provider"][1]["provider_name"], "Old");
    let r = report(&w).await;
    assert_eq!(r["by_provider"][0]["provider_slug"], "z");
    assert_eq!(r["by_provider"][0]["provider_name"], "New");
    assert_eq!(r["by_model"].as_array().unwrap().len(), 2);
    for (from, to) in [
        ("bad", "bad"),
        ("2026-10-21T00:00:00Z", "2026-10-20T00:00:00Z"),
    ] {
        assert!(matches!(
            w.core.usage().usage(from, to).await,
            Err(CoreError::ProfileInvalid(_))
        ));
    }
}
