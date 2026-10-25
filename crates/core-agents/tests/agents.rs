//! Agent validation, normalized replay and stored metadata through the owner API.
use core_agents::{AgentInput, Agents, AgentsError};
use core_db::Storage;
use core_projects::Projects;
use core_providers::{ProviderInput, Providers};
use core_secrets::{SecretKey, Secrets};
use std::sync::Arc;

struct World {
    _tmp: tempfile::TempDir,
    agents: Arc<Agents>,
    secrets: Arc<Secrets>,
    providers: Arc<Providers>,
}

async fn seeded() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let secrets = Secrets::new(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let providers = Providers::new(storage.clone(), secrets.clone());
    let input: ProviderInput = serde_json::from_value(serde_json::json!({
        "name":"Provider", "base_url":"https://example.com/anthropic",
        "models":[{"id":"model-a"},{"id":"model-b"}]
    }))
    .unwrap();
    providers.create_provider("p".into(), input).await.unwrap();
    let agents = Agents::new(
        storage.clone(),
        providers.clone(),
        secrets.clone(),
        Projects::new(storage),
        Some(tmp.path().join("home")),
    );
    World {
        _tmp: tmp,
        agents,
        secrets,
        providers,
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

#[tokio::test]
async fn agent_default_must_be_a_provider_model() {
    let w = seeded().await;
    let mut input = agent("Work");
    input.default_model = "missing".into();
    assert!(matches!(
        w.agents.create_agent("a".into(), input).await,
        Err(AgentsError::Invalid(_))
    ));
}

#[tokio::test]
async fn color_defaults_to_purple() {
    let w = seeded().await;
    let a = w
        .agents
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
) -> Result<core_agents::Agent, AgentsError> {
    let input = serde_json::from_value(serde_json::json!({"name":"Work", "provider":"provider",
        "default_model":"model-a", "character_color":color}))
    .unwrap();
    w.agents.create_agent(command.into(), input).await
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
            Err(AgentsError::Invalid(_))
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
        .agents
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
    w.agents
        .update_agent("u".into(), "work", explicit)
        .await
        .unwrap();
    let different =
        serde_json::from_value(serde_json::json!({"name":"Work", "provider":"provider",
        "default_model":"model-a", "character_color":"other-color"}))
        .unwrap();
    assert!(matches!(
        w.agents.update_agent("u".into(), "work", different).await,
        Err(AgentsError::Storage(core_db::StorageError::CommandConflict))
    ));
}

fn provider(name: &str) -> ProviderInput {
    serde_json::from_value(serde_json::json!({
        "name": name, "base_url":"https://example.com/anthropic",
        "models":[{"id":"model-a"},{"id":"model-b"}]
    }))
    .unwrap()
}
#[tokio::test]
async fn provider_in_use_cannot_be_deleted() {
    let w = seeded().await;
    w.agents
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    assert!(matches!(
        w.agents.delete_provider("d".into(), "provider").await,
        Err(AgentsError::ProviderInUse(_))
    ));
}

#[tokio::test]
async fn removing_an_agents_default_model_is_refused() {
    let w = seeded().await;
    w.agents
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    let mut input = provider("Provider");
    input.models = vec![core_providers::ProviderModelInput {
        id: "model-b".into(),
        context_window: None,
    }];
    assert!(matches!(
        w.agents
            .update_provider("u".into(), "provider", input)
            .await,
        Err(AgentsError::ProviderInUse(_))
    ));
}

#[tokio::test]
async fn delete_provider_removes_its_key() {
    let w = seeded().await;
    w.providers
        .set_key("s".into(), "provider", "synthetic")
        .await
        .unwrap();
    w.agents
        .delete_provider("d".into(), "provider")
        .await
        .unwrap();
    assert!(!w.secrets.provider_key_set("provider").await.unwrap());
}
