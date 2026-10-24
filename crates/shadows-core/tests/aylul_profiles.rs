//! Provider configuration is transactional; deleted agent identities remain reserved.
use shadows_core::{AgentInput, CoreError, ProviderInput, SecretKey, testing};
use std::sync::Arc;

struct World {
    _tmp: tempfile::TempDir,
    storage: Arc<testing::Storage>,
    secrets: Arc<shadows_core::Secrets>,
    app: Arc<shadows_core::AppCore>,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        testing::Storage::open(&tmp.path().join("test.db"))
            .await
            .unwrap(),
    );
    let secrets = testing::secrets(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let app = app(&storage, &secrets, &tmp.path().join("home")).await;
    World {
        _tmp: tmp,
        storage,
        secrets,
        app,
    }
}

/// The core over the world's storage and secrets, with an AYLUL home and an
/// adapter no test starts.
async fn app(
    storage: &Arc<testing::Storage>,
    secrets: &Arc<shadows_core::Secrets>,
    root: &std::path::Path,
) -> Arc<shadows_core::AppCore> {
    use shadows_core::testing::{LiveHandles, SessionsConfig};
    let (runtime, _) = shadows_core::AppCore::runtime_for_tests(storage.clone())
        .await
        .unwrap();
    std::fs::create_dir_all(root).unwrap();
    let sessions = shadows_core::AppCore::harness_for_tests(
        Arc::new(shadows_agent::claude::ClaudeAdapter::new(
            std::env::current_exe().unwrap(),
            "unused.js".into(),
            std::env::current_exe().unwrap(),
            "fixture".into(),
            "fixture".into(),
        )),
        storage.clone(),
        SessionsConfig {
            secrets: Some(secrets.clone()),
            aylul: Some(shadows_core::AylulConfig {
                project_slug: "aylul".into(),
                home_root: root.to_path_buf(),
            }),
            ..Default::default()
        },
    );
    shadows_core::AppCore::assemble(shadows_core::CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        storage: storage.clone(),
        secrets: secrets.clone(),
        runtime: Arc::new(runtime),
        sessions,
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: "".into(),
    })
}

fn provider(name: &str) -> ProviderInput {
    ProviderInput {
        preset: None,
        label: None,
        name: Some(name.into()),
        base_url: Some("https://example.com/anthropic".into()),
        models: vec!["model-a".into(), "model-b".into()]
            .into_iter()
            .map(|id| shadows_core::ProviderModelInput {
                id,
                context_window: None,
            })
            .collect(),
    }
}

fn agent(name: &str) -> AgentInput {
    AgentInput {
        character_color: None,
        name: name.into(),
        provider: "provider".into(),
        default_model: "model-a".into(),
    }
}

async fn seeded() -> World {
    let w = world().await;
    w.app
        .providers()
        .create_provider("p".into(), provider("Provider"))
        .await
        .unwrap();
    w
}

#[path = "aylul_profiles/providers.rs"]
mod providers;

#[path = "aylul_profiles/agents.rs"]
mod agents;

#[path = "aylul_profiles/threads.rs"]
mod threads;

#[path = "aylul_profiles/probe.rs"]
mod probe;
