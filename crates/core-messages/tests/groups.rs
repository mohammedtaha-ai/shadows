use core_agents::{AgentInput, Agents};
use core_code::Code;
use core_db::Storage;
use core_messages::Messages;
use core_projects::Projects;
use core_providers::{ProviderInput, Providers};
use core_secrets::Secrets;
use core_threads::Threads;
use std::sync::Arc;

struct Fixture {
    _tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    threads: Arc<Threads>,
    messages: Arc<Messages>,
}

async fn fixture() -> Fixture {
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
    let messages = Messages::new(storage.clone(), agents);
    Fixture {
        _tmp: tmp,
        storage,
        threads,
        messages,
    }
}

#[tokio::test]
async fn a_group_gets_one_hidden_thread_per_member() {
    let f = fixture().await;
    let slugs = ["alpha".into(), "beta".into()];
    let group = f
        .messages
        .create_group("group".into(), "Group", &slugs)
        .await
        .unwrap();
    let replay = f
        .messages
        .create_group("group".into(), "Group", &slugs)
        .await
        .unwrap();
    assert_eq!(group.id, replay.id);
    assert_eq!(group.members, replay.members);
    assert_eq!(group.members.len(), 2);
    for member in group.members {
        assert!(
            f.threads
                .list_for_profile(&member.slug)
                .await
                .unwrap()
                .is_empty()
        );
        let thread = f.threads.get(&member.thread_id).await.unwrap();
        assert_eq!(thread.profile.as_deref(), Some(member.slug.as_str()));
    }
}

#[tokio::test]
async fn a_group_refuses_seven_members() {
    let f = fixture().await;
    let slugs = (0..7).map(|i| format!("agent-{i}")).collect::<Vec<_>>();
    assert!(
        f.messages
            .create_group("group".into(), "Group", &slugs)
            .await
            .is_err()
    );
    assert!(f.messages.groups().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_failed_group_creation_leaves_no_thread() {
    let f = fixture().await;
    let slugs = ["alpha".into(), "beta".into(), "missing".into()];
    assert!(
        f.messages
            .create_group("group".into(), "Group", &slugs)
            .await
            .is_err()
    );
    assert!(f.messages.groups().await.unwrap().is_empty());
    let mut conn = f.storage.reader().acquire().await.unwrap();
    assert!(
        Threads::list_for_profile_in(&mut conn, "alpha")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        Threads::list_for_profile_in(&mut conn, "beta")
            .await
            .unwrap()
            .is_empty()
    );
}
