//! The thread capability's durable contract: planning threads parented to a
//! project and thread entries whose ordinals are allocated transactionally.
//! Separate from `tests/storage_contract/`, whose one job is the connection
//! and transaction contracts.

use serde_json::json;
use shadows_core::Actor;
use shadows_core::StorageError;
use shadows_core::testing::NewThreadEntry;
use shadows_core::{EntryRef, ThreadEntryKind, ThreadId};
use shadows_core::{TaskId, WorkflowId};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{create_thread, entries_on, fresh_command, test_app};

/// Entry kinds are stored by name: the kind is an enum, and each variant is
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
        ThreadEntryKind::TodoList,
        ThreadEntryKind::PermissionRefused,
        ThreadEntryKind::PermissionAsked,
        ThreadEntryKind::PermissionAnswered,
        ThreadEntryKind::PlanView,
        ThreadEntryKind::PlanApproved,
    ];
    for kind in all {
        shadows_core::Threads::append_thread_entry_for_test(
            &app.storage,
            &thread_id,
            NewThreadEntry {
                kind,
                author: Actor::system(),
                body: "b",
                refs: &[],
                card: None,
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
            "TodoList",
            "PermissionRefused",
            "PermissionAsked",
            "PermissionAnswered",
            "PlanView",
            "PlanApproved"
        ]
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// The person's message keeps the task it was about.
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
        EntryRef::Workflow(WorkflowId::from_literal("w1").as_str().into()),
        EntryRef::Task(TaskId::from_literal("t1").as_str().into()),
    ];
    shadows_core::Threads::append_thread_entry_for_test(
        &app.storage,
        &thread_id,
        NewThreadEntry {
            kind: ThreadEntryKind::UserMessage,
            author: Actor::user("local"),
            body: "change this",
            refs: &refs,
            card: None,
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

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A stored kind the enum does not name is a broken row, refused by name
/// rather than read as some default.
#[tokio::test]
async fn an_unknown_stored_kind_is_a_constraint_failure() {
    let app = test_app().await;
    shadows_core::Threads::append_thread_entry_for_test(
        &app.storage,
        &app.thread,
        NewThreadEntry {
            kind: ThreadEntryKind::UserMessage,
            author: Actor::user("local"),
            body: "b",
            refs: &[],
            card: None,
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
    let error = shadows_core::Threads::list_thread_entries_for_test(&app.storage, &app.thread)
        .await
        .expect_err("an unknown kind must not read");
    assert!(
        matches!(&error, StorageError::Constraint(m) if m == "unknown thread entry kind: Bogus"),
        "{error:?}"
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
