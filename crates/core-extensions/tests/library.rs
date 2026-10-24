//! Immutable library import and replay through the owner's public API.
use core_agents::{AgentInput, Agents};
use core_db::{self as testing, Storage};
use core_extensions::{ExtensionKind, Extensions, ExtensionsError};
use core_projects::Projects;
use core_providers::{ProviderInput, Providers};
use core_secrets::{SecretKey, Secrets};
use serde_json::json;
use std::sync::Arc;

struct World {
    _tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    extensions: Arc<Extensions>,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("home");
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let secrets = Secrets::new(storage.clone(), Some(SecretKey::from_bytes([7; 32])));
    let providers = Providers::new(storage.clone(), secrets.clone());
    let input: ProviderInput = serde_json::from_value(json!({
        "name":"Provider", "base_url":"https://example.com/anthropic",
        "models":[{"id":"model"}]
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
        Projects::new(storage.clone()),
        Some(root.clone()),
    );
    agents
        .create_agent(
            "agent".into(),
            AgentInput {
                name: "AYLUL".into(),
                provider: "provider".into(),
                default_model: "model".into(),
                character_color: None,
            },
        )
        .await
        .unwrap();
    World {
        extensions: Extensions::new(storage.clone(), Some(root)),
        storage,
        _tmp: tmp,
    }
}

fn source(w: &World, kind: ExtensionKind, name: &str) -> std::path::PathBuf {
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

#[tokio::test]
async fn extension_import_copies_and_redacts_source() {
    let w = world().await;
    for (kind, name) in [
        (ExtensionKind::Skill, "demo-skill"),
        (ExtensionKind::Plugin, "demo-plugin"),
    ] {
        let path = source(&w, kind, name);
        let item = w
            .extensions
            .add_extension(name.into(), kind, &path)
            .await
            .unwrap();
        assert!(uuid::Uuid::parse_str(&item.id).is_ok());
        assert_eq!(item.name, name);
        assert!(
            !serde_json::to_string(&item)
                .unwrap()
                .contains(path.to_str().unwrap())
        );
        std::fs::remove_dir_all(&path).unwrap();
        assert!(w._tmp.path().join("home/library").join(&item.id).exists());
    }
    let library = serde_json::to_value(w.extensions.extensions().await.unwrap()).unwrap();
    assert!(library["skills"][0].get("enabled").is_none());
    let view = serde_json::to_value(w.extensions.agent_extensions("aylul").await.unwrap()).unwrap();
    assert_eq!(view["skills"][0]["enabled"], false);
    let records: Vec<String> = sqlx::query_scalar("SELECT outcome_ref FROM command_record")
        .fetch_all(w.storage.reader())
        .await
        .unwrap();
    assert!(
        records
            .iter()
            .all(|s| !s.contains(w._tmp.path().to_str().unwrap()))
    );
}

#[tokio::test]
async fn extension_commands_replay_and_conflict() {
    let w = world().await;
    let src = source(&w, ExtensionKind::Skill, "replay-skill");
    let item = w
        .extensions
        .add_extension("add".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    let mut writes = testing::DatabaseWrites::observe(&w.storage).await.unwrap();
    w.extensions
        .set_agent_extension(
            "toggle".into(),
            "aylul",
            ExtensionKind::Skill,
            &item.id,
            true,
        )
        .await
        .unwrap();
    let before = writes.version().await.unwrap();
    w.extensions
        .set_agent_extension(
            "toggle".into(),
            "aylul",
            ExtensionKind::Skill,
            &item.id,
            true,
        )
        .await
        .unwrap();
    assert_eq!(before, writes.version().await.unwrap());
    assert!(matches!(
        w.extensions
            .set_agent_extension(
                "toggle".into(),
                "aylul",
                ExtensionKind::Skill,
                &item.id,
                false
            )
            .await,
        Err(ExtensionsError::Storage(
            core_db::StorageError::CommandConflict
        ))
    ));
    w.extensions
        .delete_extension("delete".into(), ExtensionKind::Skill, &item.id)
        .await
        .unwrap();
    w.extensions
        .delete_extension("delete".into(), ExtensionKind::Skill, &item.id)
        .await
        .unwrap();
    std::fs::remove_dir_all(&src).unwrap();
    assert_eq!(
        w.extensions
            .add_extension("add".into(), ExtensionKind::Skill, &src)
            .await
            .unwrap()
            .id,
        item.id
    );
    assert!(
        w.extensions
            .add_extension("add".into(), ExtensionKind::Plugin, &src)
            .await
            .is_err()
    );
    assert!(
        w.extensions
            .add_extension(
                "add".into(),
                ExtensionKind::Skill,
                &src.with_file_name("different")
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn extension_input_is_contained() {
    let w = world().await;
    let invalid = w._tmp.path().join("invalid");
    std::fs::create_dir(&invalid).unwrap();
    assert!(
        w.extensions
            .add_extension("invalid".into(), ExtensionKind::Skill, &invalid)
            .await
            .is_err()
    );
    let src = source(&w, ExtensionKind::Skill, "bounded-skill");
    let i = w
        .extensions
        .add_extension("ok".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    assert!(matches!(
        w.extensions
            .add_extension("duplicate".into(), ExtensionKind::Skill, &src)
            .await,
        Err(ExtensionsError::NameConflict)
    ));
    assert!(matches!(
        w.extensions
            .set_agent_extension(
                "wrong-kind".into(),
                "aylul",
                ExtensionKind::Plugin,
                &i.id,
                true
            )
            .await,
        Err(ExtensionsError::NotFound)
    ));
    let oversized = source(&w, ExtensionKind::Skill, "oversized-skill");
    let file = std::fs::File::create(oversized.join("large")).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    assert!(
        w.extensions
            .add_extension("large".into(), ExtensionKind::Skill, &oversized)
            .await
            .is_err()
    );
    assert!(
        w.extensions
            .add_extension("ancestor".into(), ExtensionKind::Skill, w._tmp.path())
            .await
            .is_err()
    );
    assert_eq!(w.extensions.extensions().await.unwrap().skills.len(), 1);
    #[cfg(windows)]
    {
        let junction = w._tmp.path().join("linked-source");
        let mut process = shadows_process::spawn(shadows_process::ProcessSpec {
            executable: std::env::var_os("ComSpec")
                .expect("Windows command processor")
                .into(),
            args: vec![
                "/c".into(),
                "mklink".into(),
                "/J".into(),
                junction.to_string_lossy().into_owned(),
                src.to_string_lossy().into_owned(),
            ],
            cwd: w._tmp.path().into(),
            env: vec![],
            capture_stdout: false,
            pipe_stdin: false,
        })
        .unwrap();
        assert!(process.wait().await.unwrap().success());
        assert!(
            w.extensions
                .add_extension("junction".into(), ExtensionKind::Skill, &junction)
                .await
                .is_err()
        );
        std::fs::remove_dir(&junction).unwrap();
        assert!(src.join("SKILL.md").exists());
    }
}
