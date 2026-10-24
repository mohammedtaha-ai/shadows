//! Shared extension library acceptance through the real service boundary.
use serde_json::{Value, json};
use shadows_core::ExtensionKind;
#[path = "aylul_extensions/fixture.rs"]
mod fixture;
use fixture::*;

#[tokio::test]
async fn extension_enablement_is_per_agent() {
    let w = world().await;
    w.core
        .agents()
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
            .extensions()
            .add_extension(name.into(), kind, &src)
            .await
            .unwrap();
        w.core
            .extensions()
            .set_agent_extension(format!("enable-{name}"), "aylul", kind, &item.id, true)
            .await
            .unwrap();
        assert!(
            w.core
                .extensions()
                .set_agent_extension("bad".into(), "absent", kind, &item.id, true)
                .await
                .is_err()
        );
    }
    let a =
        serde_json::to_value(w.core.extensions().agent_extensions("aylul").await.unwrap()).unwrap();
    let b =
        serde_json::to_value(w.core.extensions().agent_extensions("other").await.unwrap()).unwrap();
    assert_eq!(a["skills"][0]["enabled"], true);
    assert_eq!(a["plugins"][0]["enabled"], true);
    assert_eq!(b["skills"][0]["enabled"], false);
    assert_eq!(b["plugins"][0]["enabled"], false);
    w.core
        .harness()
        .delete_agent("del".into(), "other")
        .await
        .unwrap();
    assert!(w.core.extensions().agent_extensions("other").await.is_err());
}

#[tokio::test]
async fn extension_deletions_remove_enablement_atomically() {
    let w = world().await;
    let src = source(&w, ExtensionKind::Skill, "delete-skill");
    let i = w
        .core
        .extensions()
        .add_extension("add".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    w.core
        .extensions()
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
    assert!(!w.core.agents().agent("aylul").await.unwrap().deleted);
    assert!(
        w.core
            .extensions()
            .delete_extension("del-item".into(), ExtensionKind::Skill, &i.id)
            .await
            .is_err()
    );
    assert_eq!(
        w.core.extensions().extensions().await.unwrap().skills.len(),
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
            .extensions()
            .set_agent_extension("race".into(), "aylul", ExtensionKind::Skill, &i.id, true)
            .await
            .is_err()
    );
    for name in ["Other", "Third"] {
        w.core
            .agents()
            .create_agent(format!("agent-{name}"), agent(name))
            .await
            .unwrap();
        w.core
            .extensions()
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
        .extensions()
        .delete_extension("del-item".into(), ExtensionKind::Skill, &i.id)
        .await
        .unwrap();
    assert!(
        w.core
            .extensions()
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

#[path = "aylul_extensions/home.rs"]
mod home;

#[path = "aylul_extensions/live.rs"]
mod live;
