//! The thread capability's durable contract: planning threads parented to a
//! project and thread entries whose ordinals are allocated transactionally.
//! Separate from `tests/storage_contract.rs`, whose one job is the connection
//! and transaction contracts.

use serde_json::json;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::events::Actor;
use shadows_core::operation::OperationId;
use shadows_core::project::ProjectDirectory;
use shadows_core::storage::{Storage, StorageError};
use shadows_core::thread::{EntryRef, NewThreadEntry, ThreadEntryKind, ThreadId};
use shadows_core::{TaskId, WorkflowId};

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{create_thread, entries_on, fresh_command, test_app};

/// Any directory that exists: these tests are about threads, not about where
/// a turn runs.
fn dir() -> ProjectDirectory {
    ProjectDirectory::resolve(&std::env::temp_dir()).unwrap()
}

fn ctx(command_id: &str, params: &serde_json::Value) -> CommandContext {
    ctx_kind(command_id, "project.create", params)
}

fn ctx_kind(command_id: &str, kind: &str, params: &serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: command_id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, params),
    }
}

/// Spec section 6.5: ordinals are allocated by UPDATE ... RETURNING in the same
/// transaction, never MAX(ordinal)+1. Under concurrency the result must be a
/// contiguous, gapless, duplicate-free run.
#[tokio::test]
async fn concurrent_entry_appends_allocate_contiguous_unique_ordinals() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let project = storage
        .create_project(
            &ctx("cmd-p", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx_kind("cmd-t", "thread.create", &params),
            &project.id,
            "T",
            "claude-code",
        )
        .await
        .unwrap();

    let mut handles = Vec::new();
    for w in 0..8 {
        let storage = storage.clone();
        let thread_id = thread.id.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..25 {
                storage
                    .append_thread_entry(
                        &thread_id,
                        NewThreadEntry {
                            kind: ThreadEntryKind::UserMessage,
                            author: Actor::user("local"),
                            body: &format!("w{w}-i{i}"),
                            refs: &[],
                            operation_id: None,
                        },
                    )
                    .await
                    .unwrap();
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let ordinals: Vec<i64> =
        sqlx::query_scalar("SELECT ordinal FROM thread_entry WHERE thread_id = ? ORDER BY ordinal")
            .bind(thread.id.as_str())
            .fetch_all(storage.reader())
            .await
            .unwrap();

    assert_eq!(ordinals.len(), 200);
    assert_eq!(
        ordinals,
        (1..=200).collect::<Vec<i64>>(),
        "ordinals must be contiguous and unique"
    );

    let next: i64 =
        sqlx::query_scalar("SELECT next_entry_ordinal FROM planning_thread WHERE id = ?")
            .bind(thread.id.as_str())
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(next, 201);
}

/// Entries are read back in ordinal order, never in insertion order.
/// Cross-cutting rule 4.
#[tokio::test]
async fn entries_are_read_in_ordinal_order() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let project = storage
        .create_project(
            &ctx("cmd-p", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx_kind("cmd-t", "thread.create", &params),
            &project.id,
            "T",
            "claude-code",
        )
        .await
        .unwrap();

    for body in ["first", "second", "third"] {
        storage
            .append_thread_entry(
                &thread.id,
                NewThreadEntry {
                    kind: ThreadEntryKind::UserMessage,
                    author: Actor::user("local"),
                    body,
                    refs: &[],
                    operation_id: None,
                },
            )
            .await
            .unwrap();
    }
    let entries = storage.list_thread_entries(&thread.id).await.unwrap();
    assert_eq!(
        entries.iter().map(|e| e.body.as_str()).collect::<Vec<_>>(),
        vec!["first", "second", "third"]
    );
    assert_eq!(
        entries.iter().map(|e| e.ordinal).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

/// Spec sections 4.2 and 6.5: refs are part of ThreadEntry domain truth, not
/// an SQLite-only column that disappears when an entry is loaded again.
#[tokio::test]
async fn entry_refs_round_trip_through_storage() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let project = storage
        .create_project(
            &ctx("cmd-p", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx_kind("cmd-t", "thread.create", &params),
            &project.id,
            "T",
            "claude-code",
        )
        .await
        .unwrap();
    let refs = vec![
        EntryRef::Decision("decision-1".into()),
        EntryRef::Operation(OperationId::from_literal("operation-1")),
    ];

    let appended = storage
        .append_thread_entry(
            &thread.id,
            NewThreadEntry {
                kind: ThreadEntryKind::UserMessage,
                author: Actor::user("local"),
                body: "hello",
                refs: &refs,
                operation_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(appended.refs, refs);

    let listed = storage.list_thread_entries(&thread.id).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].refs, refs);
}

/// Ordinal allocation and entry insertion are one transaction. If insertion
/// fails after UPDATE ... RETURNING, the next successful entry still receives
/// ordinal 1 rather than leaving a gap.
#[tokio::test]
async fn a_failed_entry_insert_rolls_back_its_allocated_ordinal() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let project = storage
        .create_project(
            &ctx("cmd-p", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx_kind("cmd-t", "thread.create", &params),
            &project.id,
            "T",
            "claude-code",
        )
        .await
        .unwrap();

    sqlx::query(
        "CREATE TRIGGER reject_thread_entry BEFORE INSERT ON thread_entry
         BEGIN SELECT RAISE(ABORT, 'forced entry failure'); END",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    storage
        .append_thread_entry(
            &thread.id,
            NewThreadEntry {
                kind: ThreadEntryKind::UserMessage,
                author: Actor::user("local"),
                body: "lost",
                refs: &[],
                operation_id: None,
            },
        )
        .await
        .expect_err("the trigger must reject the entry insert");
    sqlx::query("DROP TRIGGER reject_thread_entry")
        .execute(storage.reader())
        .await
        .unwrap();

    let entry = storage
        .append_thread_entry(
            &thread.id,
            NewThreadEntry {
                kind: ThreadEntryKind::UserMessage,
                author: Actor::user("local"),
                body: "kept",
                refs: &[],
                operation_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        entry.ordinal, 1,
        "failed insertion must not consume an ordinal"
    );
}

/// CLAUDE.md: ordering is explicit. A project's threads are listed in the
/// order they were created, not by `created_at` text (see the project
/// listing's test for why the text cannot give that order).
#[tokio::test]
async fn threads_are_listed_in_creation_order_whatever_their_timestamp_text() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let project = storage
        .create_project(
            &ctx("c-project", &params),
            "demo",
            "Demo",
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let mut created = Vec::new();
    for (i, stamp) in ["2026-01-01T00:00:00.5Z", "2026-01-01T00:00:00.45Z"]
        .into_iter()
        .enumerate()
    {
        let command = format!("c{i}");
        let params = serde_json::json!({ "title": command });
        let thread = storage
            .create_planning_thread(
                &ctx_kind(&command, "thread.create", &params),
                &project.id,
                &command,
                "claude-code",
            )
            .await
            .unwrap();
        sqlx::query("UPDATE planning_thread SET created_at = ? WHERE id = ?")
            .bind(stamp)
            .bind(thread.id.as_str())
            .execute(storage.reader())
            .await
            .unwrap();
        created.push(thread.id);
    }
    let listed: Vec<_> = storage
        .list_threads_for_project(&project.id)
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(listed, created);
}

/// Spec §4.2 (closed by §13.9): the kind is an enum, and each variant is
/// stored as its name, so text written before the enum still reads.
#[tokio::test]
async fn every_entry_kind_round_trips_as_its_stored_name() {
    let app = test_app().await;
    let thread = create_thread(
        &app,
        json!({ "command_id": fresh_command(), "title": "t", "harness": "claude-code" }),
    )
    .await;
    let thread_id = ThreadId::from_literal(thread["id"].as_str().unwrap());
    let all = [
        ThreadEntryKind::UserMessage,
        ThreadEntryKind::AgentMessage,
        ThreadEntryKind::PermissionRefused,
        ThreadEntryKind::PlanView,
        ThreadEntryKind::PlanApproved,
    ];
    for kind in all {
        app.storage
            .append_thread_entry(
                &thread_id,
                NewThreadEntry {
                    kind,
                    author: Actor::system(),
                    body: "b",
                    refs: &[],
                    operation_id: None,
                },
            )
            .await
            .unwrap();
    }
    let kinds: Vec<_> = entries_on(&app, &thread_id)
        .await
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(kinds, all);
    let stored: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM thread_entry WHERE thread_id = ? ORDER BY ordinal")
            .bind(thread_id.as_str())
            .fetch_all(app.storage.reader())
            .await
            .unwrap();
    assert_eq!(
        stored,
        [
            "UserMessage",
            "AgentMessage",
            "PermissionRefused",
            "PlanView",
            "PlanApproved"
        ]
    );
}

/// §13.9: the person's message keeps the task it was about.
#[tokio::test]
async fn a_task_reference_round_trips() {
    let app = test_app().await;
    let thread = create_thread(
        &app,
        json!({ "command_id": fresh_command(), "title": "t", "harness": "claude-code" }),
    )
    .await;
    let thread_id = ThreadId::from_literal(thread["id"].as_str().unwrap());
    let refs = [
        EntryRef::Workflow(WorkflowId::from_literal("w1")),
        EntryRef::Task(TaskId::from_literal("t1")),
    ];
    app.storage
        .append_thread_entry(
            &thread_id,
            NewThreadEntry {
                kind: ThreadEntryKind::UserMessage,
                author: Actor::user("local"),
                body: "change this",
                refs: &refs,
                operation_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(entries_on(&app, &thread_id).await[0].refs, refs.to_vec());

    let stored: String = sqlx::query_scalar(
        "SELECT refs_json FROM thread_entry WHERE thread_id = ? AND ordinal = 1",
    )
    .bind(thread_id.as_str())
    .fetch_one(app.storage.reader())
    .await
    .unwrap();
    assert_eq!(stored, r#"[{"Workflow":"w1"},{"Task":"t1"}]"#);

    sqlx::query("UPDATE thread_entry SET refs_json = ? WHERE thread_id = ? AND ordinal = 1")
        .bind(r#"[{"Workflow":"w1"}]"#)
        .bind(thread_id.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(entries_on(&app, &thread_id).await[0].refs, refs[..1]);
}

/// A stored kind the enum does not name is a broken row, refused by name
/// rather than read as some default.
#[tokio::test]
async fn an_unknown_stored_kind_is_a_constraint_failure() {
    let app = test_app().await;
    app.storage
        .append_thread_entry(
            &app.thread,
            NewThreadEntry {
                kind: ThreadEntryKind::UserMessage,
                author: Actor::user("local"),
                body: "b",
                refs: &[],
                operation_id: None,
            },
        )
        .await
        .unwrap();
    sqlx::query("UPDATE thread_entry SET kind = 'Bogus' WHERE thread_id = ?")
        .bind(app.thread.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    let error = app
        .storage
        .list_thread_entries(&app.thread)
        .await
        .expect_err("an unknown kind must not read");
    assert!(
        matches!(&error, StorageError::Constraint(m) if m == "unknown thread entry kind: Bogus"),
        "{error:?}"
    );
}
