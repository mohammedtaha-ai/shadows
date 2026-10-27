use std::sync::Arc;

use core_db::{Storage, StorageError};
use core_runtime::{Runtime, RuntimeError, RuntimeInstanceId, StopKind};

#[tokio::test]
async fn startup_failure_keeps_its_storage_cause_without_committing_identity_or_events() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&directory.path().join("refused.sqlite"))
            .await
            .unwrap(),
    );
    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "CREATE TRIGGER refuse_runtime BEFORE INSERT ON runtime_instance \
                     BEGIN SELECT RAISE(ABORT, 'runtime registration refused'); END",
                )
                .execute(conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let error = match Runtime::start(storage.clone()).await {
        Ok(_) => panic!("a refused registration must fail startup"),
        Err(error) => error,
    };
    let RuntimeError::Storage(cause) = error;
    assert!(cause.to_string().contains("runtime registration refused"));
    let rows: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM runtime_instance), (SELECT COUNT(*) FROM durable_event)",
    )
    .fetch_one(storage.reader())
    .await
    .unwrap();
    assert_eq!(rows, (0, 0));
    assert_eq!(*storage.watch_committed().borrow(), 0);

    storage.close().await.unwrap();
}

async fn fixture() -> (tempfile::TempDir, Arc<Storage>, Runtime) {
    let directory = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&directory.path().join("runtime.sqlite"))
            .await
            .unwrap(),
    );
    let runtime = Runtime::start(storage.clone()).await.unwrap();
    (directory, storage, runtime)
}

#[tokio::test]
async fn registration_persists_identity_and_journal_together() {
    let (_directory, storage, runtime) = fixture().await;
    let row: (String, String, Option<String>) =
        sqlx::query_as("SELECT id, version, stopped_at FROM runtime_instance WHERE id = ?")
            .bind(runtime.instance_id().as_str())
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(row.0, runtime.instance_id().as_str());
    assert_eq!(row.1, env!("CARGO_PKG_VERSION"));
    assert_eq!(row.2, None);
    let event: (i64, String, String) =
        sqlx::query_as("SELECT seq, kind, payload_json FROM durable_event ORDER BY seq")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(event.1, "RuntimeStarted");
    let payload: serde_json::Value = serde_json::from_str(&event.2).unwrap();
    assert_eq!(payload["runtime_instance_id"], row.0);
    assert_eq!(*storage.watch_committed().borrow(), event.0);
    assert_eq!(
        RuntimeInstanceId::from_recorded(row.0),
        runtime.instance_id().clone()
    );

    storage.close().await.unwrap();
}

#[tokio::test]
async fn stop_and_journal_roll_back_together() {
    let (_directory, storage, runtime) = fixture().await;
    let id = runtime.instance_id().clone();
    let result = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Runtime::stop_in(conn, id.as_str(), StopKind::Escalated, &Storage::now()).await?;
                Err::<(), _>(StorageError::NotFound("forced rollback"))
            })
        })
        .await;
    assert!(matches!(
        result,
        Err(StorageError::NotFound("forced rollback"))
    ));
    let id = runtime.instance_id().clone();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Runtime::require_active_in(conn, &id).await?;
                assert_eq!(Runtime::stop_kind_in(conn, id.as_str()).await?, None);
                Ok(())
            })
        })
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(*storage.watch_committed().borrow(), 1);

    storage.close().await.unwrap();
}

#[tokio::test]
async fn stopped_runtime_refuses_work_and_duplicate_stop_without_another_event() {
    let (_directory, storage, runtime) = fixture().await;
    let id = runtime.instance_id().clone();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Runtime::stop_in(conn, id.as_str(), StopKind::Graceful, &Storage::now()).await?;
                assert_eq!(
                    Runtime::stop_kind_in(conn, id.as_str()).await?,
                    Some("Graceful".to_owned())
                );
                Ok(())
            })
        })
        .await
        .unwrap();
    let id = runtime.instance_id().clone();
    let active = storage
        .write_txn(move |conn| Box::pin(async move { Runtime::require_active_in(conn, &id).await }))
        .await;
    assert!(matches!(
        active,
        Err(StorageError::TransitionConflict { .. })
    ));
    let id = runtime.instance_id().clone();
    let second = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Runtime::stop_in(conn, id.as_str(), StopKind::Escalated, &Storage::now()).await
            })
        })
        .await;
    assert!(matches!(
        second,
        Err(StorageError::TransitionConflict { .. })
    ));
    let events: Vec<(String, String)> =
        sqlx::query_as("SELECT kind, payload_json FROM durable_event ORDER BY seq")
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].0, "RuntimeStopped");
    let payload: serde_json::Value = serde_json::from_str(&events[1].1).unwrap();
    assert_eq!(payload["stop_kind"], "Graceful");
    assert_eq!(*storage.watch_committed().borrow(), 2);

    storage.close().await.unwrap();
}
