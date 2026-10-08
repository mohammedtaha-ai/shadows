//! Migrations on a database written before them: 0012 (spec §16.9) and 0020
//! (§23.8). Their own file: `storage_contract` is a named accretion point.

use std::borrow::Cow;
use std::str::FromStr;

use shadows_core::testing::Storage;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const FROZEN: &str = "a frozen plan version never changes";

/// The database as migration `last` left it.
async fn migrated_to(db: &std::path::Path, last: i64) -> sqlx::SqlitePool {
    let opts = SqliteConnectOptions::from_str(&format!(
        "sqlite://{}",
        db.to_string_lossy().replace('\\', "/")
    ))
    .unwrap()
    .create_if_missing(true)
    .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let mut before = sqlx::migrate!("./migrations");
    before.migrations = Cow::Owned(
        before
            .migrations
            .iter()
            .filter(|m| m.version <= last)
            .cloned()
            .collect(),
    );
    before.run(&pool).await.unwrap();
    pool
}

async fn exec(pool: &sqlx::SqlitePool, sql: &'static str) {
    sqlx::query(sql).execute(pool).await.unwrap();
}

/// §16.9: each thread's chain becomes one Active plan with the same version
/// ids, written by that thread, with no reason; an external from-scratch
/// thread's plan too. Built on the database shape before 0012.
#[tokio::test]
async fn migration_0012_moves_every_version_into_a_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("shadows.sqlite3");
    let pool = migrated_to(&db, 11).await;
    exec(
        &pool,
        r#"INSERT INTO project (id, slug, name, directory, created_at)
           VALUES ('P', 'p', 'P', 'C:/p', '2026-09-01T00:00:00Z')"#,
    )
    .await;
    // B is an external from-scratch thread: its title is its plan's.
    exec(
        &pool,
        "INSERT INTO planning_thread (id, project_id, title, status, created_at, title_source)
         VALUES ('A', 'P', 'Thread A', 'Open', '2026-09-01T00:00:00Z', 'client'),
                ('B', 'P', 'Search', 'Open', '2026-09-01T00:00:00Z', 'plan'),
                ('C', 'P', 'Thread C', 'Open', '2026-09-01T00:00:00Z', 'client')",
    )
    .await;
    exec(
        &pool,
        r#"INSERT INTO workflow (id, thread_id, state, previous_version_id, version,
             revision, title, goal, created_at, updated_at, frozen_at)
         VALUES ('A1', 'A', 'Frozen', NULL, 1, 2, 'Login', 'log in',
                 '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z'),
                ('A2', 'A', 'Draft', 'A1', 2, 0, 'Login', 'log in',
                 '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z', NULL),
                ('B1', 'B', 'Draft', NULL, 1, 0, 'Search', 'find',
                 '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z', NULL)"#,
    )
    .await;
    exec(
        &pool,
        r#"INSERT INTO task (id, workflow_id, number, contract_json, scope_json,
             created_at, updated_at)
         VALUES ('T1', 'A1', 1, '{}', '{}', '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z'),
                ('T2', 'A1', 2, '{}', '{}', '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z')"#,
    )
    .await;
    exec(
        &pool,
        "INSERT INTO task_parent (workflow_id, task_id, parent_id, kind, label, waiting_items)
         VALUES ('A1', 'T2', 'T1', 'needs', 'after login', NULL)",
    )
    .await;
    pool.close().await;

    let storage = Storage::open(&db).await.expect("0012 applies");
    let read = storage.reader();

    let plans: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, project_id, state FROM plan ORDER BY id")
            .fetch_all(read)
            .await
            .unwrap();
    assert_eq!(
        plans,
        vec![
            ("A".into(), "P".into(), "Active".into()),
            ("B".into(), "P".into(), "Active".into()),
        ],
        "one Active plan per thread with versions; thread C has none"
    );

    type Row = (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let versions: Vec<Row> = sqlx::query_as(
        "SELECT id, plan_id, written_by_thread, change_reason, previous_version_id
           FROM workflow ORDER BY id",
    )
    .fetch_all(read)
    .await
    .unwrap();
    assert_eq!(
        versions,
        vec![
            ("A1".into(), "A".into(), Some("A".into()), None, None),
            (
                "A2".into(),
                "A".into(),
                Some("A".into()),
                None,
                Some("A1".into())
            ),
            ("B1".into(), "B".into(), Some("B".into()), None, None),
        ]
    );

    let kept: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM task WHERE workflow_id = 'A1'),
                (SELECT COUNT(*) FROM task_parent WHERE workflow_id = 'A1')",
    )
    .fetch_one(read)
    .await
    .unwrap();
    assert_eq!(kept, (2, 1), "the tasks and the link survive the rebuild");
    let dangling: Vec<(String,)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(read)
        .await
        .unwrap();
    assert!(
        dangling.is_empty(),
        "no row points at nothing: {dangling:?}"
    );

    let update = sqlx::query("UPDATE workflow SET title = 'Other' WHERE id = 'A1'")
        .execute(read)
        .await
        .expect_err("a frozen version is refused");
    assert!(update.to_string().contains(FROZEN), "{update}");
    let insert = sqlx::query(
        r#"INSERT INTO task (id, workflow_id, number, contract_json, scope_json,
             created_at, updated_at)
         VALUES ('T3', 'A1', 3, '{}', '{}', '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z')"#,
    )
    .execute(read)
    .await
    .expect_err("a frozen version's tasks are refused");
    assert!(insert.to_string().contains(FROZEN), "{insert}");
}

/// §23.8: tool and subagent bodies become their own kinds; anything else,
/// including a subagent body whose JSON is broken, is left as it was.
#[tokio::test]
async fn migration_0020_moves_tool_and_subagent_bodies_into_their_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("shadows.sqlite3");
    let pool = migrated_to(&db, 19).await;
    exec(
        &pool,
        r#"INSERT INTO project (id, slug, name, directory, created_at)
           VALUES ('P', 'p', 'P', 'C:/p', '2026-09-01T00:00:00Z')"#,
    )
    .await;
    exec(
        &pool,
        "INSERT INTO planning_thread (id, project_id, title, status, created_at, title_source)
         VALUES ('T', 'P', 'Thread', 'Open', '2026-09-01T00:00:00Z', 'client')",
    )
    .await;
    let card = r#"{"id":"a","title":"List files","status":"completed","steps":[]}"#;
    for (ordinal, body) in [
        (1, "[tool: Read notes.md]".to_string()),
        (2, format!("[subagent: {card}]")),
        (3, "[subagent: {broken]".to_string()),
        (4, "plain text".to_string()),
    ] {
        sqlx::query(
            "INSERT INTO thread_entry (id, thread_id, ordinal, kind, author_kind, author_id,
                body, created_at)
             VALUES (?, 'T', ?, 'AgentMessage', 'Agent', 'Planner', ?, '2026-09-01T00:00:00Z')",
        )
        .bind(format!("E{ordinal}"))
        .bind(ordinal)
        .bind(body)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool.close().await;

    let storage = Storage::open(&db).await.unwrap();
    let rows: Vec<(String, String, Option<String>)> =
        sqlx::query_as("SELECT kind, body, card_json FROM thread_entry ORDER BY ordinal")
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(rows[0], ("ToolCall".into(), "Read notes.md".into(), None));
    assert_eq!(
        (rows[1].0.as_str(), rows[1].1.as_str(), rows[1].2.as_deref()),
        ("Subagent", "List files", Some(card))
    );
    assert_eq!(rows[2].0, "AgentMessage", "broken JSON stays as it was");
    assert_eq!(rows[2].1, "[subagent: {broken]");
    assert_eq!(rows[3], ("AgentMessage".into(), "plain text".into(), None));
}
