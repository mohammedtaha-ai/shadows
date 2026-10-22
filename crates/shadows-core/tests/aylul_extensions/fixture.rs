//! Agent-scoped AYLUL sessions through the real core and synthetic ACP process.
use serde_json::json;
use shadows_core::ExtensionKind;
use shadows_core::testing::{self, LiveHandles, Sessions, SessionsConfig, Storage, acp};
use shadows_core::{
    AgentInput, AppCore, AylulConfig, CoreParts, ProviderInput, SecretKey, ThreadId,
};
use std::sync::Arc;

pub(super) struct World {
    pub(super) _tmp: tempfile::TempDir,
    pub(super) storage: Arc<Storage>,
    pub(super) core: Arc<AppCore>,
    pub(super) sessions: Arc<Sessions>,
    pub(super) thread: ThreadId,
}

pub(super) async fn world() -> World {
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
        storage,
        core,
        sessions,
        thread,
    }
}

pub(super) fn agent(name: &str) -> AgentInput {
    AgentInput {
        character_color: None,
        name: name.into(),
        provider: "test-2".into(),
        default_model: "fake-large".into(),
    }
}

pub(super) fn source(w: &World, kind: ExtensionKind, name: &str) -> std::path::PathBuf {
    let path = w._tmp.path().join(format!("source-{name}"));
    std::fs::create_dir_all(&path).unwrap();
    match kind {
        ExtensionKind::Skill => std::fs::write(
            path.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: Test greeting\n---\nsay hi\n"),
        )
        .unwrap(),
        ExtensionKind::Plugin => {
            std::fs::create_dir_all(path.join(".claude-plugin")).unwrap();
            std::fs::create_dir_all(path.join("commands")).unwrap();
            std::fs::write(
                path.join(".claude-plugin/plugin.json"),
                json!({
                "name":name,"description":"Test greeting","version":"1.0.0"})
                .to_string(),
            )
            .unwrap();
            std::fs::write(
                path.join("commands/hello.md"),
                "---\ndescription: Test greeting\n---\nsay hi\n",
            )
            .unwrap();
        }
    }
    path
}
