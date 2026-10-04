use std::str::FromStr;
use std::time::Duration;

use shadows_core::Actor;
use shadows_core::ProjectId;
use shadows_core::testing::DurableEvent;
use shadows_core::testing::Storage;
use sqlx::Connection;

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
            "agent_invocation",
            "code_file",
            "code_setting",
            "code_tag",
            "command_record",
            "design_command_result",
            "design_outcome",
            "design_outcome_part",
            "design_outcome_plan",
            "design_part",
            "design_part_plan",
            "design_workspace",
            "draft_intent",
            "durable_event",
            "harness_limit",
            "harness_model_effort",
            "harness_preference",
            "mcp_grant",
            "operation",
            "plan",
            "planner_instructions_version",
            "planning_thread",
            "project",
            "project_link",
            "project_mode",
            "runtime_instance",
            "task",
            "task_parent",
            "thread_entry",
            "workflow",
        ],
        "spec §7.1: the migrations carry only the milestones' tables \
         (0005 adds §12's four, 0007 §13.15's six, 0008 §15.4's four, 0011 §12.4's per-model effort, 0012 §16.9's plan, 0013 §18's vision)"
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
                sqlx::query("INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,'/d',?)")
                    .bind("p-1")
                    .bind("demo")
                    .bind("Demo")
                    .bind("2026-09-21T00:00:00Z")
                    .execute(&mut *conn)
                    .await?;
                shadows_core::testing::append_event_for_test(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::system())
                        .with_project(&ProjectId::from_literal("p-1"))
                        .with_payload(serde_json::json!({})),
                    "2026-09-21T00:00:00Z",
                )
                .await?;
                Err::<(), _>(shadows_core::StorageError::NotFound("forced"))
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
                                "INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,'/d',?)",
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

/// Finding 2, fix round 1: `write_txn` has no `Drop`-based cleanup (`Drop`
/// cannot run an async `ROLLBACK`), so a panic inside the closure must not
/// leave the single write connection stuck inside an open transaction
/// forever. `tokio::spawn` catches the panic so the test process keeps
/// running — this is the same failure mode as any other panic in the
/// closure, since `write_txn` itself does no catching of its own.
#[tokio::test]
async fn write_txn_recovers_after_a_panicking_transaction() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());

    let storage2 = storage.clone();
    let handle = tokio::spawn(async move {
        let _: Result<(), shadows_core::StorageError> = storage2
            .write_txn(|conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,'/d',?)",
                    )
                    .bind("p-panic")
                    .bind("panic")
                    .bind("x")
                    .bind("2026-09-21T00:00:00Z")
                    .execute(&mut *conn)
                    .await?;
                    panic!("simulated failure mid-transaction")
                })
            })
            .await;
    });
    assert!(handle.await.is_err(), "the spawned task must have panicked");

    // The write connection must recover: an ordinary write_txn afterwards
    // still succeeds and commits, proving the mutex was released and the
    // connection is not stuck inside the panicking call's open transaction.
    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,'/d',?)")
                    .bind("p-after")
                    .bind("after")
                    .bind("x")
                    .bind("2026-09-21T00:00:00Z")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .expect("write_txn must recover after a prior panic left a transaction open");

    let panic_row: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project WHERE id = 'p-panic'")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(
        panic_row, 0,
        "the panicking transaction's insert must have been rolled back"
    );

    let after_row: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project WHERE id = 'p-after'")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(after_row, 1, "the recovery write_txn must have committed");
}

/// Finding 4, fix round 2: `BEGIN IMMEDIATE` must be told apart from a
/// deferred `BEGIN`, not just from failing outright. Per
/// `docs/evidence/persistence/WAL_VALIDATION.md` lines 37-39, a write-only
/// transaction cannot produce the lock-upgrade failure (`SQLITE_BUSY_SNAPSHOT`,
/// code 517) that only a deferred `BEGIN` exhibits — a lone `INSERT` under a
/// deferred `BEGIN` hits ordinary `SQLITE_BUSY` instead, which `busy_timeout`
/// resolves exactly as it does for `BEGIN IMMEDIATE`. So the closure here
/// reads first (pinning a snapshot) and only then writes, and it sleeps
/// between the two so that, under a deferred `BEGIN`, the external writer's
/// commit lands *between* the read and the write — the sleep is what forces
/// the straddle deterministically instead of hoping the two race in the
/// right order.
///
/// Under `BEGIN IMMEDIATE` (this code's real behaviour) the entire call,
/// including this closure, cannot start until the external writer's lock is
/// released — the write lock is already ours by the time the closure's
/// `SELECT` runs, so there is nothing to straddle and the transaction must
/// succeed. Verified empirically: temporarily changing `BEGIN IMMEDIATE` to
/// `BEGIN` in `Storage::write_txn` makes this specific test fail with
/// `SQLITE_BUSY_SNAPSHOT` (517), while every other test in this file still
/// passes — see the fix-round-2 report for the transcript.
#[tokio::test]
async fn write_txn_waits_out_an_external_writer_holding_begin_immediate() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("s.sqlite3");
    let storage = std::sync::Arc::new(Storage::open(&db_path).await.unwrap());

    // A second, independent connection to the same file — standing in for a
    // second daemon or CLI client, not anything write_txn owns.
    let url = format!("sqlite://{}", db_path.to_string_lossy().replace('\\', "/"));
    let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&url)
        .unwrap()
        .busy_timeout(Duration::from_millis(5000));
    let mut external = sqlx::SqliteConnection::connect_with(&opts).await.unwrap();

    use sqlx::Executor;
    external.execute("BEGIN IMMEDIATE").await.unwrap();
    external
        .execute(
            "INSERT INTO project (id, slug, name, directory, created_at) \
             VALUES ('p-ext','ext','x','/d','2026-09-21T00:00:00Z')",
        )
        .await
        .unwrap();

    // Release the external writer's lock shortly after write_txn starts
    // waiting for it.
    let release = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        external.execute("COMMIT").await.unwrap();
    });

    storage
        .write_txn(|conn| {
            Box::pin(async move {
                // Read the table the external writer touched — establishing
                // a snapshot — before writing to it.
                let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
                    .fetch_one(&mut *conn)
                    .await?;
                // Force the external writer's commit (at 200ms) to land
                // between this read and the write below. Under BEGIN
                // IMMEDIATE this sleep happens after the lock is already
                // ours, so it changes nothing; under a deferred BEGIN it is
                // what guarantees the snapshot goes stale before the write
                // is attempted.
                tokio::time::sleep(Duration::from_millis(300)).await;
                sqlx::query("INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,'/d',?)")
                    .bind("p-after-external")
                    .bind("after")
                    .bind("x")
                    .bind("2026-09-21T00:00:00Z")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .expect("write_txn must wait out busy_timeout and then succeed against an external writer");

    release.await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(
        count, 2,
        "both the external writer's insert and ours must be present"
    );
}

/// Finding 3, fix round 1: `causation` and `correlation_id` must round-trip
/// through `append_event` — a future edit to the INSERT column list that
/// silently drops one of the three provenance columns must fail this test.
#[tokio::test]
async fn event_provenance_round_trips_through_append_event() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,'/d',?)")
                    .bind("p-1")
                    .bind("demo")
                    .bind("Demo")
                    .bind("2026-09-21T00:00:00Z")
                    .execute(&mut *conn)
                    .await?;
                shadows_core::testing::append_event_for_test(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::system())
                        .with_project(&ProjectId::from_literal("p-1"))
                        .with_payload(serde_json::json!({}))
                        .with_causation("Command", "cmd-1")
                        .with_correlation("corr-1"),
                    "2026-09-21T00:00:00Z",
                )
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();

    let causation_kind: Option<String> =
        sqlx::query_scalar("SELECT causation_kind FROM durable_event WHERE project_id = 'p-1'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    let causation_ref: Option<String> =
        sqlx::query_scalar("SELECT causation_ref FROM durable_event WHERE project_id = 'p-1'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    let correlation_id: Option<String> =
        sqlx::query_scalar("SELECT correlation_id FROM durable_event WHERE project_id = 'p-1'")
            .fetch_one(storage.reader())
            .await
            .unwrap();

    assert_eq!(causation_kind.as_deref(), Some("Command"));
    assert_eq!(causation_ref.as_deref(), Some("cmd-1"));
    assert_eq!(correlation_id.as_deref(), Some("corr-1"));
}

/// Spec §2.4: live publication happens after commit. The committed-sequence
/// signal must not move for a transaction that rolled back — its event never
/// existed — and must move, to the committed `seq`, once one commits.
#[tokio::test]
async fn the_committed_signal_moves_on_commit_and_never_on_rollback() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let mut committed = storage.watch_committed();

    for commit in [false, true] {
        let outcome = storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let seq = shadows_core::testing::append_event_for_test(
                        conn,
                        &DurableEvent::new("Probe", Actor::system())
                            .with_payload(serde_json::json!({})),
                        "2026-09-21T00:00:00Z",
                    )
                    .await?;
                    if commit {
                        Ok(seq)
                    } else {
                        Err(shadows_core::StorageError::NotFound("forced"))
                    }
                })
            })
            .await;
        match outcome {
            Err(_) => assert!(
                !committed.has_changed().unwrap(),
                "a rolled-back append must not be signalled"
            ),
            Ok(seq) => {
                assert!(committed.has_changed().unwrap(), "a commit is signalled");
                assert_eq!(*committed.borrow_and_update(), seq);
            }
        }
    }
}
