//! Code boundary compatibility checks.

#[tokio::test]
async fn link_reads_keep_the_callers_snapshot_and_replay_removed_projects() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let core = crate::testing::app(db.path().join("code.sqlite3"))
        .await
        .unwrap();
    let home = core
        .projects()
        .create("home".into(), "home", "Home", dir.path().to_str().unwrap())
        .await
        .unwrap();
    let linked = core
        .projects()
        .create(
            "linked".into(),
            "linked",
            "Linked",
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let code = core.code();
    let first = code
        .link("link".into(), &home.id, &linked.id)
        .await
        .unwrap();
    let storage = &code.inner.store.storage;
    let mut snapshot = storage.reader().begin().await.unwrap();
    assert!(
        super::Code::project_reachable_in(&mut snapshot, &home.id, &linked.id)
            .await
            .unwrap()
    );
    core.projects()
        .remove("remove".into(), &linked.id)
        .await
        .unwrap();
    assert_eq!(
        super::Code::selected_project_in(&mut snapshot, &home.id, Some("linked"))
            .await
            .unwrap(),
        linked.id,
        "project metadata and links must use the same snapshot"
    );
    snapshot.commit().await.unwrap();
    let mut snapshot = storage.reader().begin().await.unwrap();
    assert!(
        !super::Code::project_reachable_in(&mut snapshot, &home.id, &linked.id)
            .await
            .unwrap()
    );
    assert!(matches!(
        super::Code::selected_project_in(&mut snapshot, &home.id, Some("linked")).await,
        Err(crate::db::StorageError::GrantScope)
    ));
    snapshot.commit().await.unwrap();
    let replay = code
        .link("link".into(), &home.id, &linked.id)
        .await
        .unwrap();
    assert_eq!(replay.project, first.project);
    assert_eq!(replay.linked, first.linked);
    assert_eq!(replay.created_at, first.created_at);
    assert!(code.links(&home.id).await.unwrap().is_empty());
    core.code().shut_down().await;
}

#[tokio::test]
async fn link_writes_rollback_with_the_callers_transaction() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let core = crate::testing::app(db.path().join("code.sqlite3"))
        .await
        .unwrap();
    let mut ids = Vec::new();
    for slug in ["home", "linked"] {
        ids.push(
            core.projects()
                .create(slug.into(), slug, slug, dir.path().to_str().unwrap())
                .await
                .unwrap()
                .id,
        );
    }
    let code = core.code();
    let storage = &code.inner.store.storage;
    let [home, linked] = [ids[0].clone(), ids[1].clone()];
    let (source, target) = (home.clone(), linked.clone());
    let result: Result<(), crate::db::StorageError> = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(super::Code::put_link_in(conn, &source, &target, "test").await?);
                assert!(!super::Code::put_link_in(conn, &source, &target, "test").await?);
                Err(crate::db::StorageError::NotFound("force rollback"))
            })
        })
        .await;
    assert!(result.is_err());
    assert!(code.links(&home).await.unwrap().is_empty());
    code.link("link".into(), &home, &linked).await.unwrap();
    let result: Result<(), crate::db::StorageError> = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(super::Code::remove_link_in(conn, &home, &linked).await?);
                assert!(!super::Code::remove_link_in(conn, &home, &linked).await?);
                Err(crate::db::StorageError::NotFound("force rollback"))
            })
        })
        .await;
    assert!(result.is_err());
    assert_eq!(code.links(&ids[0]).await.unwrap().len(), 1);
    core.code().shut_down().await;
}

#[tokio::test]
async fn startup_order_uses_creation_sequence_then_project_id_for_ties() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let core = crate::testing::app(db.path().join("code.sqlite3"))
        .await
        .unwrap();
    let mut ids = Vec::new();
    for slug in ["zulu", "alpha", "middle"] {
        let project = core
            .projects()
            .create(slug.into(), slug, slug, dir.path().to_str().unwrap())
            .await
            .unwrap();
        ids.push(project.id);
    }
    let storage = &core.code().inner.store.storage;
    let order = core.code().code_order().await.unwrap();
    assert_eq!(
        order.into_iter().map(|(id, _)| id).collect::<Vec<_>>(),
        ids.iter().rev().cloned().collect::<Vec<_>>(),
        "creation sequence must decide startup order"
    );
    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("DELETE FROM durable_event WHERE kind = 'ProjectCreated'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let order = core.code().code_order().await.unwrap();
    assert_eq!(
        order.into_iter().map(|(id, _)| id).collect::<Vec<_>>(),
        ids,
        "equal activity sequences must sort by project id"
    );
    core.code().shut_down().await;
}
