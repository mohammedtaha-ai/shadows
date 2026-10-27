//! Agent-scoped AYLUL sessions through the real core and synthetic ACP process.
use serde_json::{Value, json};
use shadows_core::testing::{self, Harness, LiveHandles, SessionsConfig, Storage, acp};
use shadows_core::{
    AgentInput, AppCore, AylulConfig, CoreError, CoreParts, ProviderInput, SecretKey, SendTurn,
    ThreadEntryKind, ThreadId,
};
use std::sync::Arc;
use std::time::Duration;

struct World {
    home: std::path::PathBuf,
    storage: Arc<Storage>,
    core: Arc<AppCore>,
    sessions: Arc<Harness>,
    thread: ThreadId,
    _tmp: tempfile::TempDir,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let secrets = testing::secrets(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let runtime = Arc::new(
        shadows_core::AppCore::runtime_for_tests(storage.clone())
            .await
            .unwrap()
            .0,
    );
    let sessions = shadows_core::AppCore::harness_for_tests(
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
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
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
        home: root.join("profiles/aylul"),
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

async fn another(w: &World, name: &str) -> ThreadId {
    let agent = w
        .core
        .agents()
        .create_agent(name.into(), agent(name))
        .await
        .unwrap();
    let project = w.core.agents().assistant_project().await.unwrap();
    w.core
        .threads()
        .create(
            format!("thread-{name}"),
            &project,
            "Chat",
            Some("aylul"),
            Some(&agent.slug),
        )
        .await
        .unwrap()
        .id
}

fn request(model: &str, effort: &str) -> SendTurn {
    SendTurn {
        attachments: Vec::new(),
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

async fn reply(w: &World, thread: &ThreadId, model: &str, effort: &str) -> Value {
    let op = w
        .core
        .turns()
        .send(thread.clone(), request(model, effort))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let op = shadows_core::Turns::get_operation_for_test(&w.storage, &op)
                .await
                .unwrap();
            if op.finished_at.is_some() {
                assert_eq!(op.status_kind, "Completed");
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let entries = w.core.threads().entries(thread).await.unwrap();
    serde_json::from_str(
        &entries
            .iter()
            .rev()
            .find(|e| e.kind == ThreadEntryKind::AgentMessage)
            .unwrap()
            .body,
    )
    .unwrap()
}

#[path = "aylul_session/homes.rs"]
mod homes;
#[path = "aylul_session/opening.rs"]
mod opening;
#[path = "aylul_session/preferences.rs"]
mod preferences;
