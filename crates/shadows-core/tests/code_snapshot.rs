use std::sync::Arc;

use shadows_core::testing::{self, LiveHandles, Storage, acp};
use shadows_core::{AppCore, Code, CoreError, CoreParts, ErrorCode};

struct World {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    dir: tempfile::TempDir,
    _db: tempfile::TempDir,
}

async fn world() -> World {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("code.sqlite3"))
            .await
            .unwrap(),
    );
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets: testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: Arc::new(AppCore::runtime_for_tests(storage.clone()).await.unwrap().0),
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    World {
        core,
        storage,
        dir,
        _db: db,
    }
}

#[tokio::test]
async fn link_writes_rollback_with_the_callers_transaction() {
    let w = world().await;
    let dir = &w.dir;
    let core = &w.core;
    let mut ids = Vec::new();
    for slug in ["home", "linked"] {
        ids.push(
            core.project_lifecycle()
                .create(slug.into(), slug, slug, dir.path().to_str().unwrap())
                .await
                .unwrap()
                .id,
        );
    }
    let code = core.code();
    let storage = &w.storage;
    let [home, linked] = [ids[0].clone(), ids[1].clone()];
    let (source, target) = (home.clone(), linked.clone());
    let result: Result<(), core_db::StorageError> = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(Code::put_link_in(conn, &source, &target, "test").await?);
                assert!(!Code::put_link_in(conn, &source, &target, "test").await?);
                Err(core_db::StorageError::NotFound("force rollback"))
            })
        })
        .await;
    assert!(result.is_err());
    assert!(code.links(&home).await.unwrap().is_empty());
    core.plans()
        .link_projects("link".into(), &home, &linked)
        .await
        .unwrap();
    let result: Result<(), core_db::StorageError> = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(Code::remove_link_in(conn, &home, &linked).await?);
                assert!(!Code::remove_link_in(conn, &home, &linked).await?);
                Err(core_db::StorageError::NotFound("force rollback"))
            })
        })
        .await;
    assert!(result.is_err());
    assert_eq!(code.links(&ids[0]).await.unwrap().len(), 1);
    core.code().shut_down().await;

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn link_reads_keep_the_callers_snapshot_and_replay_removed_projects() {
    let w = world().await;
    let dir = &w.dir;
    let core = &w.core;
    let home = core
        .project_lifecycle()
        .create("home".into(), "home", "Home", dir.path().to_str().unwrap())
        .await
        .unwrap();
    let linked = core
        .project_lifecycle()
        .create(
            "linked".into(),
            "linked",
            "Linked",
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let code = core.code();
    let first = core
        .plans()
        .link_projects("link".into(), &home.id, &linked.id)
        .await
        .unwrap();
    let storage = &w.storage;
    let mut snapshot = storage.reader().begin().await.unwrap();
    assert!(
        Code::project_reachable_in(&mut snapshot, &home.id, &linked.id)
            .await
            .unwrap()
    );
    core.project_lifecycle()
        .remove("remove".into(), &linked.id)
        .await
        .unwrap();
    assert_eq!(
        Code::selected_project_in(&mut snapshot, &home.id, Some("linked"))
            .await
            .unwrap(),
        linked.id,
        "project metadata and links must use the same snapshot"
    );
    snapshot.commit().await.unwrap();
    let mut snapshot = storage.reader().begin().await.unwrap();
    assert!(
        !Code::project_reachable_in(&mut snapshot, &home.id, &linked.id)
            .await
            .unwrap()
    );
    assert!(matches!(
        Code::selected_project_in(&mut snapshot, &home.id, Some("linked"))
            .await
            .map_err(CoreError::from),
        Err(CoreError::Refused {
            code: ErrorCode::GrantScope,
            ..
        })
    ));
    snapshot.commit().await.unwrap();
    let replay = core
        .plans()
        .link_projects("link".into(), &home.id, &linked.id)
        .await
        .unwrap();
    assert_eq!(replay.project, first.project);
    assert_eq!(replay.linked, first.linked);
    assert_eq!(replay.created_at, first.created_at);
    assert!(code.links(&home.id).await.unwrap().is_empty());
    core.code().shut_down().await;

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
