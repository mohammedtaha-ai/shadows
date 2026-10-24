//! Durable command identity on the foundation's public API.

use core_db::{CommandContext, DatabaseWrites, Storage, StorageError};
use serde_json::json;

#[tokio::test]
async fn commands_replay_only_with_identical_persistence_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let command = CommandContext::user("cmd".into(), "project.create", json!({"slug": "demo"}));
    let first = command.clone();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(
                    CommandContext::classify(conn, &first, "Global", "")
                        .await?
                        .is_none()
                );
                CommandContext::record_command(
                    conn,
                    &first,
                    "Global",
                    "",
                    "Project",
                    "persisted-id",
                    &Storage::now(),
                )
                .await
            })
        })
        .await
        .unwrap();
    let mut changes = Vec::new();
    let mut kind = command.clone();
    kind.command_kind = "project.other".into();
    changes.push(kind);
    let mut schema = command.clone();
    schema.command_schema_ver += 1;
    changes.push(schema);
    let mut fingerprint = command.clone();
    fingerprint.request_fingerprint = CommandContext::fingerprint("project.create", &json!({}));
    changes.push(fingerprint);
    for changed in changes {
        let result = storage
            .write_txn(move |conn| {
                Box::pin(
                    async move { CommandContext::classify(conn, &changed, "Global", "").await },
                )
            })
            .await;
        assert!(matches!(result, Err(StorageError::CommandConflict)));
    }
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert_eq!(
                    CommandContext::classify(conn, &command, "Global", "")
                        .await?
                        .as_deref(),
                    Some("persisted-id"),
                );
                assert!(
                    CommandContext::classify(conn, &command, "Project", "other")
                        .await?
                        .is_none()
                );
                Ok(())
            })
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn command_rollback_is_invisible_to_replay_and_the_write_observer() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let mut observer = DatabaseWrites::observe(&storage).await.unwrap();
    let before = observer.version().await.unwrap();
    let command = CommandContext::user("cmd".into(), "project.create", json!({}));
    let refused = command.clone();
    let result = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                CommandContext::record_command(
                    conn,
                    &refused,
                    "Global",
                    "",
                    "Project",
                    "rolled-back",
                    &Storage::now(),
                )
                .await?;
                Err::<(), _>(StorageError::NotFound("forced"))
            })
        })
        .await;
    assert!(matches!(result, Err(StorageError::NotFound("forced"))));
    assert_eq!(observer.version().await.unwrap(), before);
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(
                    CommandContext::classify(conn, &command, "Global", "")
                        .await?
                        .is_none()
                );
                CommandContext::record_command(
                    conn,
                    &command,
                    "Global",
                    "",
                    "Project",
                    "committed",
                    &Storage::now(),
                )
                .await
            })
        })
        .await
        .unwrap();
    assert_ne!(observer.version().await.unwrap(), before);
}

#[tokio::test]
async fn cancelled_write_rolls_back_command_and_event_before_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let committed = storage.watch_committed();
    let writer = storage.clone();
    let (ready, started) = tokio::sync::oneshot::channel();
    let command = CommandContext::user("cancelled".into(), "project.create", json!({}));
    let retry = command.clone();
    let task = tokio::spawn(async move {
        writer
            .write_txn(move |conn| {
                Box::pin(async move {
                    CommandContext::record_command(
                        conn,
                        &command,
                        "Global",
                        "",
                        "Project",
                        "cancelled",
                        &Storage::now(),
                    )
                    .await?;
                    core_db::JournalEntry::append_event_for_test(
                        conn,
                        core_db::JournalEntry {
                            event_id: "cancelled-event".into(),
                            kind: "Probe".into(),
                            project_id: None,
                            thread_id: None,
                            operation_id: None,
                            actor: core_db::Actor::system(),
                            causation: None,
                            correlation_id: None,
                            payload_json: "{}".into(),
                        },
                        &Storage::now(),
                    )
                    .await?;
                    ready.send(()).unwrap();
                    std::future::pending::<()>().await;
                    Ok(())
                })
            })
            .await
    });
    started.await.unwrap();
    assert!(
        !committed.has_changed().unwrap(),
        "uncommitted writes must not notify"
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                assert!(
                    CommandContext::classify(conn, &retry, "Global", "")
                        .await?
                        .is_none()
                );
                let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
                    .fetch_one(&mut *conn)
                    .await?;
                assert_eq!(events, 0, "recovery must roll back the cancelled event");
                CommandContext::record_command(
                    conn,
                    &retry,
                    "Global",
                    "",
                    "Project",
                    "recovered",
                    &Storage::now(),
                )
                .await
            })
        })
        .await
        .unwrap();
    assert!(
        !committed.has_changed().unwrap(),
        "rolled-back events must never notify"
    );
}
