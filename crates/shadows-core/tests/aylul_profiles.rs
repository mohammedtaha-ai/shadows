//! Provider configuration is transactional; deleted agent identities remain reserved.
use shadows_core::{AgentInput, CoreError, ProviderInput, SecretKey, testing};
use std::sync::Arc;

struct World {
    _tmp: tempfile::TempDir,
    storage: Arc<testing::Storage>,
    secrets: Arc<shadows_core::Secrets>,
    profiles: Arc<shadows_core::Profiles>,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        testing::Storage::open(&tmp.path().join("test.db"))
            .await
            .unwrap(),
    );
    let secrets = testing::secrets(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let profiles = testing::profiles(storage.clone(), secrets.clone(), tmp.path().join("home"));
    World {
        _tmp: tmp,
        storage,
        secrets,
        profiles,
    }
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
    w.profiles
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
