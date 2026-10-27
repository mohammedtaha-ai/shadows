//! Cross-owner persistence through the application's shared foundation.

use shadows_core::testing::{CommandContext, ProjectDirectory, Storage, create_project};
use std::sync::Arc;

#[tokio::test]
async fn project_failure_rolls_back_command_and_journal() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let directory = ProjectDirectory::resolve(tmp.path()).unwrap();
    let modes = shadows_agent::policy::default_modes();
    let first = CommandContext::user("first".into(), "project.create", serde_json::json!({}));
    create_project(&storage, &first, "same", "First", &directory, &modes)
        .await
        .unwrap();
    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "CREATE TRIGGER reject_second_command BEFORE INSERT ON command_record
             WHEN NEW.command_id = 'second'
             BEGIN SELECT RAISE(ABORT, 'test command refusal'); END",
                )
                .execute(&mut *conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let committed = storage.watch_committed();
    let failed = CommandContext::user("second".into(), "project.create", serde_json::json!({}));
    assert!(
        create_project(&storage, &failed, "second", "Second", &directory, &modes)
            .await
            .is_err()
    );
    for table in ["project", "command_record", "durable_event"] {
        let count: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
                .fetch_one(storage.reader())
                .await
                .unwrap();
        assert_eq!(count, 1, "failed creation must leave {table} unchanged");
    }
    assert!(
        !committed.has_changed().unwrap(),
        "failed creation must not notify"
    );
    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("DROP TRIGGER reject_second_command")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    create_project(&storage, &failed, "second", "Second", &directory, &modes)
        .await
        .unwrap();
    assert!(
        committed.has_changed().unwrap(),
        "recovery must commit and notify"
    );

    storage.close().await.unwrap();
}
