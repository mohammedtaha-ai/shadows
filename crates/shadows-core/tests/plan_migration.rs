//! Plan ownership and structured entry migrations on databases lacking their columns.

use std::borrow::Cow;
use std::str::FromStr;

use shadows_core::testing::Storage;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const FROZEN: &str = "a frozen plan version never changes";

#[tokio::test]
async fn migration_0023_keeps_parts_and_starts_without_a_map() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("map.sqlite3");
    let pool = migrated_to(&db, 22).await;
    exec(
        &pool,
        "INSERT INTO project(id,slug,name,directory,created_at)
        VALUES ('P','p','P','C:/p','2026-10-12')",
    )
    .await;
    exec(
        &pool,
        "INSERT INTO design_workspace(project_id,revision,vision_revision,vision_content)
        VALUES ('P',1,0,'{}')",
    )
    .await;
    let old = r#"{"title":"core","responsibility":"logic","design":"","kind":"backend"}"#;
    sqlx::query(
        "INSERT INTO design_part(id,project_id,parent_id,revision,ordinal,content_json)
        VALUES ('part','P',NULL,1,0,?)",
    )
    .bind(old)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let storage = Storage::open(&db).await.unwrap();
    let json: String = sqlx::query_scalar("SELECT content_json FROM design_part")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(json, old);
    let part: shadows_core::PartContent = serde_json::from_str(&json).unwrap();
    assert!(part.serves.is_empty() && part.folders.is_empty() && part.files.is_empty());
    let map: i64 = sqlx::query_scalar("SELECT map_revision FROM design_workspace")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(map, 0);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(storage.reader())
            .await
            .unwrap()
            .is_empty()
    );

    storage.close().await.unwrap();
}

#[tokio::test]
async fn migration_0022_turns_vision_text_into_items() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("vision.sqlite3");
    let pool = migrated_to(&db, 21).await;
    exec(
        &pool,
        "INSERT INTO project (id,slug,name,directory,created_at)
        VALUES ('P','p','P','C:/p','2026-10-12T00:00:00Z')",
    )
    .await;
    let old = serde_json::json!({"purpose":"Sell books", "users":"  ", "goals":"G",
        "boundaries":"\t\n\r\u{a0}", "technical_direction":"Rust"});
    sqlx::query(
        "INSERT INTO design_workspace
        (project_id,revision,vision_revision,vision_content) VALUES ('P',3,3,?)",
    )
    .bind(old.to_string())
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let storage = Storage::open(&db).await.unwrap();
    let (content, counters): (String, String) = sqlx::query_as(
        "SELECT vision_content,vision_counters FROM design_workspace WHERE project_id='P'",
    )
    .fetch_one(storage.reader())
    .await
    .unwrap();
    let vision: shadows_core::VisionContent = serde_json::from_str(&content).unwrap();
    assert_eq!(vision.purpose[0].id, "PU1");
    assert_eq!(vision.purpose[0].text, "Sell books");
    assert_eq!(vision.purpose[0].revision, 1);
    assert_eq!(vision.purpose[0].approved_from, None);
    assert!(vision.users.is_empty() && vision.boundaries.is_empty());
    assert_eq!(vision.goals[0].id, "G1");
    assert_eq!(vision.technical_direction[0].id, "TD1");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&counters).unwrap(),
        serde_json::json!({"purpose":1,"users":0,"goals":1,"boundaries":0,
            "technical_direction":1})
    );
    let row: (i64, String) = sqlx::query_as(
        "SELECT number,content_json FROM design_vision_version WHERE project_id='P'",
    )
    .fetch_one(storage.reader())
    .await
    .unwrap();
    assert_eq!(row, (3, content));

    storage.close().await.unwrap();
}

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
    let mut before = sqlx::migrate!("../core-db/migrations");
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

/// Each thread's chain becomes one Active plan with the same version
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

    storage.close().await.unwrap();
}

/// Tool and subagent bodies become their own kinds; anything else,
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
        // Migration markers are case-sensitive; SQLite's LIKE is not, so these stay text.
        (5, "[Tool: kept as text]".to_string()),
        (6, format!("[SUBAGENT: {card}]")),
        // The title is everything between the first `[tool: ` and the last `]`.
        (7, "[tool: a]b]".to_string()),
        (8, "[tool: ]]".to_string()),
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
    assert_eq!(
        rows[4],
        ("AgentMessage".into(), "[Tool: kept as text]".into(), None)
    );
    assert_eq!(
        rows[5].0, "AgentMessage",
        "upper case is not the card either"
    );
    assert_eq!(rows[6], ("ToolCall".into(), "a]b".into(), None));
    assert_eq!(rows[7], ("ToolCall".into(), "]".into(), None));

    storage.close().await.unwrap();
}

#[tokio::test]
async fn stored_technical_direction_ids_become_td() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("td.sqlite3");
    let pool = migrated_to(&db, 37).await;
    exec(
        &pool,
        "INSERT INTO project(id,slug,name,directory,created_at)
        VALUES ('P','p','P','C:/p','2026-10-31')",
    )
    .await;
    let content = concat!(
        r#"{"purpose":[],"users":[],"goals":[],"boundaries":[],"technical_direction":["#,
        r#"{"id":"T1","revision":1,"text":"T1 Rust","approved_from":null},"#,
        r#"{"id":"T2","revision":1,"text":"Go","approved_from":null}]}"#,
    );
    sqlx::query(
        "INSERT INTO design_workspace(project_id,revision,vision_revision,vision_content)
        VALUES ('P',1,1,?)",
    )
    .bind(content)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO design_vision_version(project_id,number,content_json,created_at)
        VALUES ('P',1,?,'2026-10-31')",
    )
    .bind(content)
    .execute(&pool)
    .await
    .unwrap();
    // The proposals' thread, turn and entry are beside the point: only their item ids,
    // in the row and in the card, move.
    exec(&pool, "PRAGMA foreign_keys = OFF").await;
    for (id, field, item, seq) in [
        ("td", "technical_direction", "T1", 1),
        ("b", "boundaries", "B2", 2),
    ] {
        sqlx::query(
            "INSERT INTO design_vision_proposal(id,project_id,thread_id,operation_id,entry_id,
                action,field,item_id,text,old_text,expected_item_revision,status,
                standards_version,created_at,seq)
            VALUES (?,'P','T','O','E','edit',?,?,'Go','Rust',1,'Pending',1,'2026-10-31',?)",
        )
        .bind(id)
        .bind(field)
        .bind(item)
        .bind(seq)
        .execute(&pool)
        .await
        .unwrap();
        let card = serde_json::json!({
            "id": id, "thread_id": "T", "action": "edit", "field": field, "item_id": item,
            "text": "Go", "old_text": "Rust", "expected_item_revision": 1,
            "status": "Pending", "outdated": false, "created_at": "2026-10-31",
        });
        sqlx::query(
            "INSERT INTO thread_entry(id,thread_id,ordinal,kind,author_kind,author_id,body,
                card_json,created_at)
            VALUES (?,'T',?,'VisionProposal','Agent','Planner','Edit',?,'2026-10-31')",
        )
        .bind(format!("E{seq}"))
        .bind(seq)
        .bind(card.to_string())
        .execute(&pool)
        .await
        .unwrap();
    }
    pool.close().await;
    let storage = Storage::open(&db).await.unwrap();
    let cards: Vec<String> =
        sqlx::query_scalar("SELECT card_json FROM thread_entry ORDER BY ordinal")
            .fetch_all(storage.reader())
            .await
            .unwrap();
    let card_ids: Vec<serde_json::Value> = cards
        .iter()
        .map(|c| serde_json::from_str::<serde_json::Value>(c).unwrap()["item_id"].clone())
        .collect();
    assert_eq!(
        card_ids,
        ["TD1", "B2"],
        "only a technical direction card moves"
    );
    let items: Vec<(String, String)> =
        sqlx::query_as("SELECT id,item_id FROM design_vision_proposal ORDER BY seq")
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(
        items,
        [("td".into(), "TD1".into()), ("b".into(), "B2".into())]
    );
    assert_eq!(shadows_core::VisionField::of_id("T1"), None);
    assert_eq!(
        shadows_core::VisionField::of_id("TD1"),
        Some(shadows_core::VisionField::TechnicalDirection)
    );
    for sql in [
        "SELECT vision_content FROM design_workspace",
        "SELECT content_json FROM design_vision_version",
    ] {
        let json: String = sqlx::query_scalar(sql)
            .fetch_one(storage.reader())
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let items = v["technical_direction"].as_array().unwrap();
        assert_eq!(items[0]["id"], "TD1");
        assert_eq!(items[1]["id"], "TD2");
        assert_eq!(items[0]["text"], "T1 Rust");
    }
    storage.close().await.unwrap();
}
