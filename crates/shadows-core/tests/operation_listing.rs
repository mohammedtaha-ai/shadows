use shadows_core::testing::FailureStage;
use shadows_core::testing::turn::{default_turn_settings, new_turn, turn_command};

#[tokio::test]
async fn listing_keeps_order_and_missing_invocations_without_other_threads() {
    let db = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let core = shadows_core::testing::app(db.path().join("listing.sqlite3"))
        .await
        .unwrap();
    let project = core
        .project_lifecycle()
        .create(
            "project".into(),
            "project",
            "Project",
            root.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let turns = core.turns();
    let storage = std::sync::Arc::new(
        shadows_core::testing::Storage::open(&db.path().join("listing.sqlite3"))
            .await
            .unwrap(),
    );
    let runtime = shadows_core::testing::Runtime::start(storage.clone())
        .await
        .unwrap();
    let settings = default_turn_settings();
    let mut expected = Vec::new();
    let mut selected = None;
    for name in ["selected", "other"] {
        let thread = core
            .threads()
            .create(name.into(), &project.id, name, None, None)
            .await
            .unwrap();
        assert!(turns.operations(&thread.id).await.unwrap().is_empty());
        if name == "selected" {
            selected = Some(thread.id.clone());
        }
        let count = if name == "selected" { 66 } else { 3 };
        for index in 0..count {
            let command = turn_command(&format!("{name}-{index}"), &thread.id, "hello", &settings);
            let started = shadows_core::Turns::start_turn_for_test(
                &storage,
                &command,
                new_turn(&thread.id, &runtime, "hello", &settings),
            )
            .await
            .unwrap();
            let op = started.operation_id.clone();
            storage
                .write_txn(move |conn| {
                    Box::pin(async move {
                        if index == 1 {
                            shadows_core::testing::Harness::delete_invocation_for_test(
                                conn,
                                started.operation_id.as_str(),
                            )
                            .await?;
                        }
                        Ok(())
                    })
                })
                .await
                .unwrap();
            // The fixture commits turns without spawning an adapter.
            shadows_core::Turns::mark_operation_failed_for_test(
                &storage,
                &op,
                FailureStage::Prepare,
                "fixture",
            )
            .await
            .unwrap();
            if name == "selected" {
                expected.insert(0, (op, index != 1));
            }
        }
    }
    let actual: Vec<_> = turns
        .operations(&selected.unwrap())
        .await
        .unwrap()
        .into_iter()
        .map(|op| (op.id, op.invocation.is_some()))
        .collect();
    assert_eq!(actual, expected);
    core.code().shut_down().await;
}
