//! Shared extension library acceptance through the real service boundary.
use serde_json::{Value, json};
use shadows_core::{CoreError, ExtensionKind, testing};
#[path = "aylul_extensions/fixture.rs"]
mod fixture;
use fixture::*;

#[tokio::test]
async fn extension_import_copies_and_redacts_source() {
    let w = world().await;
    for (kind, name) in [
        (ExtensionKind::Skill, "demo-skill"),
        (ExtensionKind::Plugin, "demo-plugin"),
    ] {
        let path = source(&w, kind, name);
        let item = w
            .core
            .profiles()
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
    let library = serde_json::to_value(w.core.profiles().extensions().await.unwrap()).unwrap();
    assert!(library["skills"][0].get("enabled").is_none());
    let view =
        serde_json::to_value(w.core.profiles().agent_extensions("aylul").await.unwrap()).unwrap();
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
        .core
        .profiles()
        .add_extension("add".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    let mut writes = testing::database_writes(&w.storage).await.unwrap();
    w.core
        .profiles()
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
    w.core
        .profiles()
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
        w.core
            .profiles()
            .set_agent_extension(
                "toggle".into(),
                "aylul",
                ExtensionKind::Skill,
                &item.id,
                false
            )
            .await,
        Err(CoreError::Storage(
            shadows_core::StorageError::CommandConflict
        ))
    ));
    w.core
        .profiles()
        .delete_extension("delete".into(), ExtensionKind::Skill, &item.id)
        .await
        .unwrap();
    w.core
        .profiles()
        .delete_extension("delete".into(), ExtensionKind::Skill, &item.id)
        .await
        .unwrap();
    std::fs::remove_dir_all(&src).unwrap();
    assert_eq!(
        w.core
            .profiles()
            .add_extension("add".into(), ExtensionKind::Skill, &src)
            .await
            .unwrap()
            .id,
        item.id
    );
    assert!(
        w.core
            .profiles()
            .add_extension("add".into(), ExtensionKind::Plugin, &src)
            .await
            .is_err()
    );
    assert!(
        w.core
            .profiles()
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
async fn extension_enablement_is_per_agent() {
    let w = world().await;
    w.core
        .profiles()
        .create_agent("b".into(), agent("Other"))
        .await
        .unwrap();
    for (kind, name) in [
        (ExtensionKind::Skill, "demo-skill"),
        (ExtensionKind::Plugin, "demo-plugin"),
    ] {
        let src = source(&w, kind, name);
        let item = w
            .core
            .profiles()
            .add_extension(name.into(), kind, &src)
            .await
            .unwrap();
        w.core
            .profiles()
            .set_agent_extension(format!("enable-{name}"), "aylul", kind, &item.id, true)
            .await
            .unwrap();
        assert!(
            w.core
                .profiles()
                .set_agent_extension("bad".into(), "absent", kind, &item.id, true)
                .await
                .is_err()
        );
    }
    let a =
        serde_json::to_value(w.core.profiles().agent_extensions("aylul").await.unwrap()).unwrap();
    let b =
        serde_json::to_value(w.core.profiles().agent_extensions("other").await.unwrap()).unwrap();
    assert_eq!(a["skills"][0]["enabled"], true);
    assert_eq!(a["plugins"][0]["enabled"], true);
    assert_eq!(b["skills"][0]["enabled"], false);
    assert_eq!(b["plugins"][0]["enabled"], false);
    w.core
        .harness()
        .delete_agent("del".into(), "other")
        .await
        .unwrap();
    assert!(w.core.profiles().agent_extensions("other").await.is_err());
}

#[tokio::test]
async fn extension_deletions_remove_enablement_atomically() {
    let w = world().await;
    let src = source(&w, ExtensionKind::Skill, "delete-skill");
    let i = w
        .core
        .profiles()
        .add_extension("add".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    w.core
        .profiles()
        .set_agent_extension("enable".into(), "aylul", ExtensionKind::Skill, &i.id, true)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_delete BEFORE DELETE ON aylul_agent_extension
        BEGIN SELECT RAISE(ABORT,'synthetic deletion failure'); END",
    )
    .execute(w.storage.reader())
    .await
    .unwrap();
    assert!(
        w.core
            .harness()
            .delete_agent("del-agent".into(), "aylul")
            .await
            .is_err()
    );
    assert!(!w.core.profiles().agent("aylul").await.unwrap().deleted);
    assert!(
        w.core
            .profiles()
            .delete_extension("del-item".into(), ExtensionKind::Skill, &i.id)
            .await
            .is_err()
    );
    assert_eq!(
        w.core.profiles().extensions().await.unwrap().skills.len(),
        1
    );
    sqlx::query("DROP TRIGGER reject_delete")
        .execute(w.storage.reader())
        .await
        .unwrap();
    w.core
        .harness()
        .delete_agent("del-agent".into(), "aylul")
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aylul_agent_extension")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert!(
        w.core
            .profiles()
            .set_agent_extension("race".into(), "aylul", ExtensionKind::Skill, &i.id, true)
            .await
            .is_err()
    );
    for name in ["Other", "Third"] {
        w.core
            .profiles()
            .create_agent(format!("agent-{name}"), agent(name))
            .await
            .unwrap();
        w.core
            .profiles()
            .set_agent_extension(
                format!("enable-{name}"),
                &name.to_ascii_lowercase(),
                ExtensionKind::Skill,
                &i.id,
                true,
            )
            .await
            .unwrap();
    }
    w.core
        .profiles()
        .delete_extension("del-item".into(), ExtensionKind::Skill, &i.id)
        .await
        .unwrap();
    assert!(
        w.core
            .profiles()
            .extensions()
            .await
            .unwrap()
            .skills
            .is_empty()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aylul_agent_extension")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn extension_input_is_contained() {
    let w = world().await;
    let invalid = w._tmp.path().join("invalid");
    std::fs::create_dir(&invalid).unwrap();
    assert!(
        w.core
            .profiles()
            .add_extension("invalid".into(), ExtensionKind::Skill, &invalid)
            .await
            .is_err()
    );
    let src = source(&w, ExtensionKind::Skill, "bounded-skill");
    let i = w
        .core
        .profiles()
        .add_extension("ok".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    assert!(matches!(
        w.core
            .profiles()
            .add_extension("duplicate".into(), ExtensionKind::Skill, &src)
            .await,
        Err(CoreError::ExtensionNameConflict)
    ));
    assert!(matches!(
        w.core
            .profiles()
            .set_agent_extension(
                "wrong-kind".into(),
                "aylul",
                ExtensionKind::Plugin,
                &i.id,
                true
            )
            .await,
        Err(CoreError::ExtensionNotFound)
    ));
    let oversized = source(&w, ExtensionKind::Skill, "oversized-skill");
    let file = std::fs::File::create(oversized.join("large")).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    assert!(
        w.core
            .profiles()
            .add_extension("large".into(), ExtensionKind::Skill, &oversized)
            .await
            .is_err()
    );
    assert!(
        w.core
            .profiles()
            .add_extension("ancestor".into(), ExtensionKind::Skill, w._tmp.path())
            .await
            .is_err()
    );
    assert_eq!(
        w.core.profiles().extensions().await.unwrap().skills.len(),
        1
    );
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
            w.core
                .profiles()
                .add_extension("junction".into(), ExtensionKind::Skill, &junction)
                .await
                .is_err()
        );
        std::fs::remove_dir(&junction).unwrap();
        assert!(src.join("SKILL.md").exists());
    }
}

#[path = "aylul_extensions/home.rs"]
mod home;

#[path = "aylul_extensions/live.rs"]
mod live;
