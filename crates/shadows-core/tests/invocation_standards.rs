use core_db::StorageError;
use shadows_core::OperationId;
use shadows_core::testing::turn::{default_turn_settings, new_turn, turn_command};

#[tokio::test]
async fn invocation_standards_preserve_pins_nulls_and_missing_results() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let core = shadows_core::testing::app(db.path().join("pins.sqlite3"))
        .await
        .unwrap();
    let project = core
        .project_lifecycle()
        .create(
            "project".into(),
            "project",
            "Project",
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let other = core
        .project_lifecycle()
        .create(
            "other".into(),
            "other",
            "Other",
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let thread = core
        .threads()
        .create("thread".into(), &project.id, "Thread", None, None)
        .await
        .unwrap();
    let first = core
        .standards()
        .save_standards_additions("first".into(), &project.id, Default::default())
        .await
        .unwrap();
    let settings = default_turn_settings();
    let command = turn_command("turn", &thread.id, "hello", &settings);
    let storage = std::sync::Arc::new(
        shadows_core::testing::Storage::open(&db.path().join("pins.sqlite3"))
            .await
            .unwrap(),
    );
    let runtime = shadows_core::testing::Runtime::start(storage.clone())
        .await
        .unwrap();
    let mut turn = new_turn(&thread.id, &runtime, "hello", &settings);
    turn.standards_additions_version = Some(&first.id);
    let started = shadows_core::Turns::start_turn_for_test(&storage, &command, turn)
        .await
        .unwrap();
    core.standards()
        .save_standards_additions("second".into(), &project.id, Default::default())
        .await
        .unwrap();
    let mut conn = storage.reader().acquire().await.unwrap();
    assert_eq!(
        shadows_core::Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
            .await
            .unwrap()
            .additions,
        Some(first)
    );
    shadows_core::testing::Harness::set_invocation_pin_for_test(
        &mut conn,
        started.operation_id.as_str(),
        None,
    )
    .await
    .unwrap();
    assert!(
        shadows_core::Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
            .await
            .unwrap()
            .additions
            .is_none(),
        "a null pin must not select the newer additions"
    );
    let foreign = core
        .standards()
        .save_standards_additions("foreign".into(), &other.id, Default::default())
        .await
        .unwrap();
    shadows_core::testing::Harness::set_invocation_pin_for_test(
        &mut conn,
        started.operation_id.as_str(),
        Some(&foreign.id),
    )
    .await
    .unwrap();
    assert!(matches!(
        shadows_core::Turns::standards_for_invocation_in(&mut conn, &started.operation_id).await,
        Err(StorageError::NotFound("invocation standards additions"))
    ));
    assert!(matches!(
        shadows_core::Turns::standards_for_invocation_in(&mut conn, &OperationId::generate()).await,
        Err(StorageError::NotFound("invocation"))
    ));
    shadows_core::Turns::mark_operation_failed_for_test(
        &storage,
        &started.operation_id,
        shadows_core::testing::FailureStage::Prepare,
        "fixture ended before spawning",
    )
    .await
    .unwrap();
    core.turns()
        .remove_thread("remove-thread".into(), &thread.id)
        .await
        .unwrap();
    core.project_lifecycle()
        .remove("remove-project".into(), &project.id)
        .await
        .unwrap();
    assert!(matches!(
        shadows_core::Turns::standards_for_invocation_in(&mut conn, &started.operation_id).await,
        Err(StorageError::NotFound("invocation"))
    ));
    core.code().shut_down().await;

    drop(conn);
    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}
