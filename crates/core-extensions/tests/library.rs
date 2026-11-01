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
    storage: Arc<Storage>,
    extensions: Arc<Extensions>,
    secrets: Arc<Secrets>,
    _tmp: tempfile::TempDir,
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
        secrets.clone(),
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
                description: None,
                instructions: None,
                outfit: None,
            },
        )
        .await
        .unwrap();
    World {
        extensions: Extensions::new(storage.clone(), secrets.clone(), Some(root)),
        secrets,
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
        ExtensionKind::Mcp => panic!("MCP items have no folder"),
    }
    path
}

#[tokio::test]
async fn bundled_items_import_once_at_start() {
    let w = world().await;
    w.extensions.import_bundled().await.unwrap();
    let library = w.extensions.extensions().await.unwrap();
    assert_eq!(library.skills.len(), 1);
    let skill = &library.skills[0];
    assert_eq!(skill.name, "aylul-hello");
    assert!(skill.bundled);
    let text = std::fs::read_to_string(
        w._tmp
            .path()
            .join("home/library")
            .join(&skill.id)
            .join("SKILL.md"),
    )
    .unwrap();
    assert!(text.contains("name: aylul-hello"));
}

#[tokio::test]
async fn bundled_staging_does_not_overwrite_linked_files() {
    use sha2::{Digest, Sha256};
    let w = world().await;
    let bytes = include_bytes!("../assets/library/skills/aylul-hello/SKILL.md");
    let relative = "SKILL.md";
    let mut hash = Sha256::new();
    hash.update((relative.len() as u64).to_le_bytes());
    hash.update(relative.as_bytes());
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
    let stage = w
        ._tmp
        .path()
        .join("home/.bundled-stage")
        .join(format!("skill-aylul-hello-{:x}", hash.finalize()));
    std::fs::create_dir_all(&stage).unwrap();
    let victim = w._tmp.path().join("outside-home.txt");
    std::fs::write(&victim, "keep this file").unwrap();
    std::fs::hard_link(&victim, stage.join("SKILL.md")).unwrap();
    w.extensions.import_bundled().await.unwrap();
    assert_eq!(std::fs::read_to_string(victim).unwrap(), "keep this file");
}

#[tokio::test]
async fn bundled_staging_does_not_follow_a_linked_parent() {
    let w = world().await;
    let outside = w._tmp.path().join("outside-home");
    std::fs::create_dir(&outside).unwrap();
    let stage = w._tmp.path().join("home").join(".bundled-stage");
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(&stage)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &stage).unwrap();
    let result = w.extensions.import_bundled().await;
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    result.unwrap();
}

#[tokio::test]
async fn a_restart_replays_the_bundled_import() {
    let w = world().await;
    w.extensions.import_bundled().await.unwrap();
    let first = w.extensions.extensions().await.unwrap().skills[0]
        .id
        .clone();
    let restart = Extensions::new(
        w.storage.clone(),
        w.secrets.clone(),
        Some(w._tmp.path().join("home")),
    );
    restart.import_bundled().await.unwrap();
    let skills = restart.extensions().await.unwrap().skills;
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, first);
}

#[tokio::test]
async fn a_deleted_bundled_item_is_declined() {
    let w = world().await;
    w.extensions.import_bundled().await.unwrap();
    let item = w.extensions.extensions().await.unwrap().skills.remove(0);
    w.extensions
        .delete_extension("delete-bundled".into(), ExtensionKind::Skill, &item.id)
        .await
        .unwrap();
    w.extensions.import_bundled().await.unwrap();
    assert!(w.extensions.extensions().await.unwrap().skills.is_empty());
    let declined: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aylul_extension_declined WHERE kind='skill' AND name='aylul-hello'",
    )
    .fetch_one(w.storage.reader())
    .await
    .unwrap();
    assert_eq!(declined, 1);
}

#[tokio::test]
async fn a_bundled_item_starts_switched_off() {
    let w = world().await;
    w.extensions.import_bundled().await.unwrap();
    let items = w.extensions.agent_extensions("aylul").await.unwrap().skills;
    assert_eq!(items.len(), 1);
    assert!(!items[0].enabled);
}

#[tokio::test]
async fn no_aylul_configuration_imports_nothing() {
    let w = world().await;
    let unconfigured = Extensions::new(w.storage.clone(), w.secrets.clone(), None);
    unconfigured.import_bundled().await.unwrap();
    assert!(unconfigured.extensions().await.unwrap().skills.is_empty());
    assert!(!w._tmp.path().join("home/.bundled-stage").exists());
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

    w.storage.close().await.unwrap();
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

    drop(writes);
    w.storage.close().await.unwrap();
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

    w.storage.close().await.unwrap();
}

#[tokio::test]
async fn mcp_item_is_configuration_with_write_only_values() {
    let w = world().await;
    let input = serde_json::from_value(json!({
        "name": "github", "transport": {"type": "http", "url": "https://example.com/mcp",
            "headers": [{"name": "Authorization", "value": "synthetic-secret"}]}
    }))
    .unwrap();
    let item = w.extensions.add_mcp("mcp-add".into(), input).await.unwrap();
    let out = serde_json::to_string(&item).unwrap();
    assert!(out.contains("Authorization"));
    assert!(!out.contains("synthetic-secret"));
    assert_eq!(w.extensions.extensions().await.unwrap().mcp.len(), 1);
}

fn mcp_input(name: &str, values: serde_json::Value) -> core_extensions::McpInput {
    serde_json::from_value(json!({"name":name,"transport":{
        "type":"stdio","command":"server","args":["--test"],"env":values}}))
    .unwrap()
}

#[tokio::test]
async fn mcp_input_is_checked() {
    let w = world().await;
    for name in ["shadows", "Bad", "a.b", "", " server "] {
        assert!(
            w.extensions
                .add_mcp(format!("bad-{name}"), mcp_input(name, json!([])))
                .await
                .is_err()
        );
    }
    for values in [
        json!([{"name":"TOKEN"}]),
        json!([{"name":"a","value":"one"},{"name":"a","value":"two"}]),
        json!([{"name":"bad.name","value":"one"}]),
        json!([{"name":"ANTHROPIC_API_KEY","value":"one"}]),
    ] {
        assert!(
            w.extensions
                .add_mcp(uuid::Uuid::new_v4().to_string(), mcp_input("valid", values))
                .await
                .is_err()
        );
    }
    let invalid = serde_json::from_value(json!({
        "name": "server", "transport": {
            "type": "http", "url": "http://remote.example/mcp", "headers": []
        }
    }))
    .unwrap();
    assert!(
        w.extensions
            .add_mcp("bad-url".into(), invalid)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn mcp_edit_keeps_its_id_switches_and_unsent_values() {
    let w = world().await;
    let item = w
        .extensions
        .add_mcp(
            "create".into(),
            mcp_input(
                "server",
                json!([
        {"name":"KEEP","value":"synthetic-keep"},{"name":"DROP","value":"synthetic-drop"}]),
            ),
        )
        .await
        .unwrap();
    w.extensions
        .set_agent_extension("switch".into(), "aylul", ExtensionKind::Mcp, &item.id, true)
        .await
        .unwrap();
    let updated = w
        .extensions
        .update_mcp(
            "edit".into(),
            &item.id,
            mcp_input(
                "renamed",
                json!([
        {"name":"KEEP"},{"name":"NEW","value":"synthetic-new"}]),
            ),
        )
        .await
        .unwrap();
    assert_eq!(updated.id, item.id);
    let failed = w
        .extensions
        .update_mcp(
            "bad-edit".into(),
            &item.id,
            mcp_input("bad-change", json!([{"name":"MISSING"}])),
        )
        .await;
    assert!(failed.is_err());
    assert_eq!(
        w.extensions.extensions().await.unwrap().mcp[0].name,
        "renamed"
    );
    assert!(w.extensions.agent_extensions("aylul").await.unwrap().mcp[0].enabled);
    let values = w
        .secrets
        .resolve_env(&Secrets::mcp_scope(&item.id))
        .await
        .unwrap();
    assert_eq!(
        values,
        [
            ("KEEP".into(), "synthetic-keep".into()),
            ("NEW".into(), "synthetic-new".into())
        ]
    );
}

#[tokio::test]
async fn mcp_delete_removes_switches_and_scope_in_one_write() {
    let w = world().await;
    let item = w
        .extensions
        .add_mcp(
            "create".into(),
            mcp_input("server", json!([{"name":"TOKEN","value":"synthetic"}])),
        )
        .await
        .unwrap();
    w.extensions
        .set_agent_extension("on".into(), "aylul", ExtensionKind::Mcp, &item.id, true)
        .await
        .unwrap();
    w.storage
        .write_txn_as::<_, _, core_db::StorageError>(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "CREATE TRIGGER reject_secret_delete BEFORE DELETE ON secret_env
            BEGIN SELECT RAISE(ABORT, 'refused'); END",
                )
                .execute(conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    assert!(
        w.extensions
            .delete_extension("delete".into(), ExtensionKind::Mcp, &item.id)
            .await
            .is_err()
    );
    assert!(w.extensions.agent_extensions("aylul").await.unwrap().mcp[0].enabled);
    assert_eq!(
        w.secrets
            .list_env(&Secrets::mcp_scope(&item.id))
            .await
            .unwrap()
            .len(),
        1
    );
    w.storage
        .write_txn_as::<_, _, core_db::StorageError>(|conn| {
            Box::pin(async move {
                sqlx::query("DROP TRIGGER reject_secret_delete")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    w.extensions
        .delete_extension("delete".into(), ExtensionKind::Mcp, &item.id)
        .await
        .unwrap();
    assert!(
        w.extensions
            .agent_extensions("aylul")
            .await
            .unwrap()
            .mcp
            .is_empty()
    );
    assert!(
        w.secrets
            .list_env(&Secrets::mcp_scope(&item.id))
            .await
            .unwrap()
            .is_empty()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM aylul_agent_mcp")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn mcp_commands_replay_and_conflict() {
    let w = world().await;
    let values = json!([{"name":"TOKEN","value":"synthetic-one"}]);
    let first = w
        .extensions
        .add_mcp("create".into(), mcp_input("server", values.clone()))
        .await
        .unwrap();
    let replay = w
        .extensions
        .add_mcp("create".into(), mcp_input("server", values))
        .await
        .unwrap();
    assert_eq!(first.id, replay.id);
    assert!(
        w.extensions
            .add_mcp(
                "create".into(),
                mcp_input("server", json!([{"name":"TOKEN","value":"synthetic-two"}]))
            )
            .await
            .is_err()
    );
    w.extensions
        .delete_extension("delete".into(), ExtensionKind::Mcp, &first.id)
        .await
        .unwrap();
    let replay = w
        .extensions
        .add_mcp(
            "create".into(),
            mcp_input("server", json!([{"name":"TOKEN","value":"synthetic-one"}])),
        )
        .await
        .unwrap();
    assert_eq!(first.id, replay.id);
}

#[tokio::test]
async fn mcp_values_never_leave_secrets() {
    let w = world().await;
    let marker = "synthetic-private-mcp-value";
    let item = w
        .extensions
        .add_mcp(
            "create".into(),
            mcp_input("server", json!([{"name":"TOKEN","value":marker}])),
        )
        .await
        .unwrap();
    assert!(!serde_json::to_string(&item).unwrap().contains(marker));
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT request_fingerprint,outcome_ref FROM command_record")
            .fetch_all(w.storage.reader())
            .await
            .unwrap();
    assert!(
        rows.iter().all(
            |(fingerprint, outcome)| !fingerprint.contains(marker) && !outcome.contains(marker)
        )
    );
    let config: String = sqlx::query_scalar("SELECT config_json FROM aylul_mcp_server WHERE id=?")
        .bind(&item.id)
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert!(!config.contains(marker));
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT ciphertext FROM secret_env WHERE profile=?")
        .bind(Secrets::mcp_scope(&item.id))
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert!(
        !bytes
            .windows(marker.len())
            .any(|window| window == marker.as_bytes())
    );
    for filename in ["test.db", "test.db-wal", "test.db-shm"] {
        if let Ok(bytes) = std::fs::read(w._tmp.path().join(filename)) {
            assert!(
                !bytes
                    .windows(marker.len())
                    .any(|window| window == marker.as_bytes())
            );
        }
    }
}

#[tokio::test]
async fn agent_mcp_servers_resolve_only_enabled_items() {
    let w = world().await;
    let on = w
        .extensions
        .add_mcp(
            "one".into(),
            mcp_input("one", json!([{"name":"TOKEN","value":"synthetic-one"}])),
        )
        .await
        .unwrap();
    w.extensions
        .add_mcp(
            "two".into(),
            mcp_input("two", json!([{"name":"TOKEN","value":"synthetic-two"}])),
        )
        .await
        .unwrap();
    w.extensions
        .set_agent_extension("on".into(), "aylul", ExtensionKind::Mcp, &on.id, true)
        .await
        .unwrap();
    let servers = w.extensions.agent_mcp_servers("aylul").await.unwrap();
    assert_eq!(servers.len(), 1);
    assert!(!format!("{servers:?}").contains("synthetic-one"));
    match &servers[0] {
        core_extensions::McpServerConfig::Stdio { name, env, .. } => {
            assert_eq!(name, "one");
            assert_eq!(env, &[("TOKEN".into(), "synthetic-one".into())]);
        }
        _ => panic!("wrong transport"),
    }
}
