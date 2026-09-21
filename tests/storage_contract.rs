use shadows::events::{Actor, DurableEvent};
use shadows::storage::Storage;

#[tokio::test]
async fn fresh_database_migrates_and_applies_the_connection_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("shadows.sqlite3");
    let storage = Storage::open(&db).await.expect("open should succeed");

    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(journal.to_lowercase(), "wal", "spec §6.23 requires WAL");

    let fk: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(fk, 1, "spec §6.23 requires foreign_keys = ON");

    let mut tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' \
         AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(storage.reader())
    .await
    .unwrap();
    tables.sort();

    assert_eq!(
        tables,
        vec![
            "command_record",
            "durable_event",
            "operation",
            "planning_thread",
            "project",
            "runtime_instance",
            "thread_entry",
        ],
        "spec §7.1: the first migration carries only the milestone's tables"
    );
}

/// Spec §2.4 and cross-cutting rule 5: state and event commit together or not
/// at all. A live publication failure must never roll back committed truth, and
/// a rolled-back transaction must leave no event behind.
#[tokio::test]
async fn state_and_event_commit_atomically_or_not_at_all() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    // A transaction that fails after appending its event leaves nothing behind.
    let outcome = storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)")
                    .bind("p-1")
                    .bind("demo")
                    .bind("Demo")
                    .bind("2026-09-21T00:00:00Z")
                    .execute(&mut *conn)
                    .await?;
                shadows::storage::test_support::append_event_for_test(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::system())
                        .with_project("p-1")
                        .with_payload(serde_json::json!({})),
                    "2026-09-21T00:00:00Z",
                )
                .await?;
                Err::<(), _>(shadows::storage::StorageError::NotFound("forced"))
            })
        })
        .await;
    assert!(outcome.is_err());

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(
        (projects, events),
        (0, 0),
        "rollback must leave neither behind"
    );
}

/// The measured writer policy, encoded as a regression test. Concurrent
/// read-then-write transactions must all succeed. If someone later replaces the
/// serialized write connection with a pool, this test is what fails.
#[tokio::test]
async fn concurrent_read_then_write_transactions_all_succeed() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());

    let mut handles = Vec::new();
    for w in 0..16 {
        let storage = storage.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..25 {
                let id = format!("p-{w}-{i}");
                storage
                    .write_txn(|conn| {
                        let id = id.clone();
                        Box::pin(async move {
                            // Read first, then write: this is the shape that
                            // forces a lock upgrade under deferred BEGIN.
                            let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
                                .fetch_one(&mut *conn)
                                .await?;
                            sqlx::query(
                                "INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)",
                            )
                            .bind(&id)
                            .bind(&id)
                            .bind("x")
                            .bind("2026-09-21T00:00:00Z")
                            .execute(&mut *conn)
                            .await?;
                            Ok(())
                        })
                    })
                    .await
                    .expect("no write transaction may fail under the serialized policy");
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 400);
}
