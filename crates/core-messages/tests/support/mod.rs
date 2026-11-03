// Each test binary reads a different part of the fixture.
#![allow(dead_code)]

use core_agents::{AgentInput, Agents};
use core_code::Code;
use core_db::Storage;
use core_messages::Messages;
use core_projects::Projects;
use core_providers::{ProviderInput, Providers};
use core_secrets::Secrets;
use core_threads::Threads;
use std::sync::Arc;

pub struct Fixture {
    _tmp: tempfile::TempDir,
    pub storage: Arc<Storage>,
    pub agents: Arc<Agents>,
    pub threads: Arc<Threads>,
    pub messages: Arc<Messages>,
}

pub async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let projects = Projects::new(storage.clone());
    projects
        .ensure_assistant("aylul", "AYLUL", tmp.path())
        .await
        .unwrap();
    let secrets = Secrets::new(storage.clone(), None);
    let providers = Providers::new(storage.clone(), secrets.clone());
    let input: ProviderInput = serde_json::from_value(serde_json::json!({
        "name": "Provider", "base_url": "https://example.com",
        "models": [{"id": "model", "context_window": 200000}]
    }))
    .unwrap();
    providers
        .create_provider("provider".into(), input)
        .await
        .unwrap();
    let agents = Agents::new(
        storage.clone(),
        providers,
        secrets,
        projects.clone(),
        Some(tmp.path().join("agents")),
    );
    for name in ["Alpha", "Beta"] {
        agents
            .create_agent(
                name.into(),
                AgentInput {
                    name: name.into(),
                    provider: "provider".into(),
                    default_model: "model".into(),
                    character_color: None,
                    description: None,
                    instructions: None,
                    outfit: None,
                },
            )
            .await
            .unwrap();
    }
    let threads = Arc::new(Threads::new(
        storage.clone(),
        Code::new(storage.clone(), projects.clone()),
        projects,
    ));
    let messages = Messages::new(storage.clone(), agents.clone());
    Fixture {
        _tmp: tmp,
        storage,
        agents,
        threads,
        messages,
    }
}
