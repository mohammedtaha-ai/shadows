//! Agent validation, normalized replay and stored metadata through the owner API.
use core_agents::{AgentInput, Agents, AgentsError};
use core_db::Storage;
use core_projects::Projects;
use core_providers::{ProviderInput, Providers};
use core_secrets::{SecretKey, Secrets};
use std::sync::Arc;

struct World {
    storage: Arc<Storage>,
    agents: Arc<Agents>,
    secrets: Arc<Secrets>,
    providers: Arc<Providers>,
    _tmp: tempfile::TempDir,
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
        Projects::new(storage.clone()),
        Some(tmp.path().join("home")),
    );
    World {
        _tmp: tmp,
        storage,
        agents,
        secrets,
        providers,
    }
}

fn agent(name: &str) -> AgentInput {
    AgentInput {
        character_color: None,
        description: None,
        instructions: None,
        outfit: None,
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

    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn unknown_but_well_formed_color_is_kept() {
    let w = seeded().await;
    let a = colored_agent(&w, "future-color-2", "a").await.unwrap();
    assert_eq!(
        serde_json::to_value(a).unwrap()["character_color"],
        "future-color-2"
    );

    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
}

fn profile(fields: serde_json::Value) -> AgentInput {
    let mut input = serde_json::json!({"name":"Work", "provider":"provider",
        "default_model":"model-a"});
    input
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    serde_json::from_value(input).unwrap()
}

async fn sql(w: &World, statement: &'static str, slug: &'static str) {
    w.storage
        .write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(statement).bind(slug).execute(conn).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn existing_agents_migrate_with_an_empty_profile() {
    let w = seeded().await;
    sql(
        &w,
        "INSERT INTO aylul_agent (slug,name,provider,default_model,created_at,updated_at,
         character_color) VALUES (?,'Old','provider','model-a','t','t','purple')",
        "old",
    )
    .await;
    let a = w.agents.agent("old").await.unwrap();
    assert_eq!((a.description.as_str(), a.instructions.as_str()), ("", ""));
    assert_eq!(a.outfit, "default");
    assert_eq!(w.agents.home_instructions("old").await.unwrap(), None);

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn description_is_one_trimmed_line_of_200() {
    let w = seeded().await;
    let long = "é".repeat(200);
    let a = w
        .agents
        .create_agent(
            "a".into(),
            profile(serde_json::json!({"description": format!(" {long} ")})),
        )
        .await
        .unwrap();
    assert_eq!(a.description, long);
    for (i, bad) in ["é".repeat(201), "one\ntwo".into(), "one\rtwo".into()]
        .into_iter()
        .enumerate()
    {
        let input = profile(serde_json::json!({"description": bad}));
        assert!(matches!(
            w.agents.create_agent(format!("b{i}"), input).await,
            Err(AgentsError::Invalid(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn instructions_are_bounded_and_refuse_nul() {
    let w = seeded().await;
    let full = "x".repeat(32 * 1024);
    let a = w
        .agents
        .create_agent(
            "a".into(),
            profile(serde_json::json!({"instructions": full})),
        )
        .await
        .unwrap();
    assert_eq!(a.instructions.len(), 32 * 1024);
    for (i, bad) in ["x".repeat(32 * 1024 + 1), "a\0b".into()]
        .into_iter()
        .enumerate()
    {
        let input = profile(serde_json::json!({"instructions": bad}));
        assert!(matches!(
            w.agents.create_agent(format!("b{i}"), input).await,
            Err(AgentsError::Invalid(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn instructions_equal_to_the_template_are_stored_empty() {
    let w = seeded().await;
    let template = w.agents.instructions_template();
    let a = w
        .agents
        .create_agent(
            "a".into(),
            profile(serde_json::json!({"instructions": template})),
        )
        .await
        .unwrap();
    assert_eq!(a.instructions, "");
    let home = w.agents.home_instructions(&a.slug).await.unwrap();
    assert_eq!(home.as_deref(), Some(template));

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn outfit_shape_is_checked_and_unknown_is_kept() {
    let w = seeded().await;
    let a = w
        .agents
        .create_agent(
            "a".into(),
            profile(serde_json::json!({"outfit": "future-hat-2"})),
        )
        .await
        .unwrap();
    assert_eq!(a.outfit, "future-hat-2");
    for (i, bad) in ["", "Hat", "1hat", "a_b", &"a".repeat(33)]
        .into_iter()
        .enumerate()
    {
        let input = profile(serde_json::json!({"outfit": bad}));
        assert!(matches!(
            w.agents.create_agent(format!("b{i}"), input).await,
            Err(AgentsError::Invalid(_))
        ));
    }

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn update_without_profile_fields_keeps_them() {
    let w = seeded().await;
    let fields = serde_json::json!({"description":"d","instructions":"mine","outfit":"hat"});
    w.agents
        .create_agent("a".into(), profile(fields))
        .await
        .unwrap();
    let a = w
        .agents
        .update_agent("u".into(), "work", agent("Work"))
        .await
        .unwrap();
    assert_eq!(
        (a.description.as_str(), a.instructions.as_str()),
        ("d", "mine")
    );
    assert_eq!(a.outfit, "hat");

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn home_instructions_import_once_and_never_again() {
    let w = seeded().await;
    let home = w._tmp.path().join("home/profiles");
    for (name, file) in [
        ("Mine", Some("custom")),
        ("Gone", None),
        ("Same", Some("T")),
    ] {
        w.agents
            .create_agent(name.into(), agent(name))
            .await
            .unwrap();
        let slug: &'static str = Box::leak(name.to_lowercase().into_boxed_str());
        sql(
            &w,
            "UPDATE aylul_agent SET instructions=NULL WHERE slug=?",
            slug,
        )
        .await;
        let path = home.join(slug).join("CLAUDE.md");
        assert!(!path.exists(), "the opening owns CLAUDE.md creation");
        match file {
            Some("T") => std::fs::write(&path, w.agents.instructions_template()).unwrap(),
            Some(text) => std::fs::write(&path, text).unwrap(),
            None => {}
        }
    }
    assert!(
        w.agents
            .import_home_instructions()
            .await
            .unwrap()
            .is_empty()
    );
    let text = |slug: &'static str| {
        let agents = w.agents.clone();
        async move { agents.agent(slug).await.unwrap().instructions }
    };
    assert_eq!(text("mine").await, "custom");
    assert_eq!(text("gone").await, "");
    assert_eq!(text("same").await, "");
    assert!(w.agents.home_instructions("gone").await.unwrap().is_some());
    std::fs::write(home.join("mine/CLAUDE.md"), "changed").unwrap();
    assert!(
        w.agents
            .import_home_instructions()
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(text("mine").await, "custom");

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn an_unreadable_home_file_stays_unimported() {
    let w = seeded().await;
    w.agents
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    sql(
        &w,
        "UPDATE aylul_agent SET instructions=NULL WHERE slug=?",
        "work",
    )
    .await;
    let path = w._tmp.path().join("home/profiles/work/CLAUDE.md");
    std::fs::write(&path, [0xff, 0xfe, 0x41]).unwrap();
    assert_eq!(w.agents.import_home_instructions().await.unwrap(), ["work"]);
    assert_eq!(w.agents.home_instructions("work").await.unwrap(), None);
    std::fs::write(&path, "a\0b").unwrap();
    assert_eq!(w.agents.import_home_instructions().await.unwrap(), ["work"]);

    w.storage.close().await.unwrap();
}
