//! `Projects::ensure_assistant`: AYLUL's project is created once, over the
//! profile home, and marked; an ordinary project is not.

use std::sync::Arc;

use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{AppCore, CoreParts};

#[tokio::test]
async fn ensure_assistant_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let core = AppCore::assemble(CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: Arc::new(
            shadows_core::AppCore::runtime_for_tests(storage.clone())
                .await
                .unwrap()
                .0,
        ),
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let first = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &home)
        .await
        .unwrap();
    let second = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &home)
        .await
        .unwrap();
    assert!(first.assistant);
    assert_eq!(first.id, second.id);
    assert_eq!(
        std::fs::canonicalize(first.directory.as_ref().unwrap()).unwrap(),
        std::fs::canonicalize(&home).unwrap()
    );
    let ordinary = core
        .project_lifecycle()
        .create("c1".into(), "plain", "Plain", tmp.path().to_str().unwrap())
        .await
        .unwrap();
    assert!(!ordinary.assistant);
    let all = core.projects().list().await.unwrap();
    assert_eq!(all.iter().filter(|p| p.slug == "aylul").count(), 1);
}

#[tokio::test]
async fn ensure_assistant_recreates_a_removed_assistant_project() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let core = AppCore::assemble(CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: Arc::new(
            shadows_core::AppCore::runtime_for_tests(storage.clone())
                .await
                .unwrap()
                .0,
        ),
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let first = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &home)
        .await
        .unwrap();
    core.project_lifecycle()
        .remove("rm-1".into(), &first.id)
        .await
        .unwrap();
    // The removed row keeps the slug taken, so this start must not fail on it.
    let again = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &home)
        .await;
    assert!(again.is_ok(), "{again:?}");
}
