//! Migration 0005's storage shapes and Shadows' mode policy (spec §12.4-§12.7):
//! a thread's harness locks at its first operation, below the application too;
//! a project starts with the policy's modes; an entry names its turn.

use std::collections::BTreeMap;
use std::sync::Arc;

use shadows_agent::policy;
use shadows_core::Actor;
use shadows_core::Project;
use shadows_core::testing::NewThreadEntry;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::Runtime;
use shadows_core::testing::Storage;
use shadows_core::testing::{CommandContext, fingerprint};
use shadows_core::{ThreadEntryKind, ThreadId};

struct Fixture {
    _tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    runtime: Runtime,
    project: Project,
}

fn ctx(id: &str, kind: &str) -> CommandContext {
    CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &serde_json::json!({ "id": id })),
    }
}

async fn fixture() -> (Fixture, ThreadId) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = shadows_core::AppCore::runtime_for_tests(storage.clone())
        .await
        .unwrap();
    let dir = ProjectDirectory::resolve(tmp.path()).unwrap();
    let project = shadows_core::testing::create_project(
        &storage,
        &ctx("c1", "project.create"),
        "demo",
        "Demo",
        &dir,
        &policy::default_modes(),
    )
    .await
    .unwrap();
    let thread = shadows_core::Threads::create_planning_thread_for_test(
        &storage,
        &ctx("c2", "thread.create"),
        &project.id,
        "T",
        policy::CLAUDE_CODE,
        None,
    )
    .await
    .unwrap();
    assert_eq!(thread.harness, "claude-code");
    assert!(thread.forked_from_thread.is_none());
    let fx = Fixture {
        _tmp: tmp,
        storage,
        runtime,
        project,
    };
    (fx, thread.id)
}

#[test]
fn claude_policy_is_accept_edits_and_auto_and_codex_has_none() {
    assert_eq!(
        policy::allowed_modes(policy::CLAUDE_CODE),
        ["acceptEdits", "auto"]
    );
    assert_eq!(
        policy::default_mode(policy::CLAUDE_CODE),
        Some("acceptEdits")
    );
    assert!(policy::allowed_modes(policy::CODEX).is_empty());
    assert!(!policy::is_known("gemini"));
}

#[tokio::test]
async fn a_thread_defaults_to_claude_code_and_its_harness_locks_at_its_first_operation() {
    let (fx, thread) = fixture().await;
    let t = shadows_core::testing::Harness::set_thread_harness_for_test(
        &fx.storage,
        &ctx("h1", "thread.harness"),
        &thread,
        "codex",
    )
    .await
    .unwrap();
    assert_eq!(t.harness, "codex");
    shadows_core::testing::Harness::set_thread_harness_for_test(
        &fx.storage,
        &ctx("h2", "thread.harness"),
        &thread,
        "claude-code",
    )
    .await
    .unwrap();
    shadows_core::Turns::create_pending_operation_for_test(
        &fx.storage,
        &thread,
        &fx.runtime.instance_id,
    )
    .await
    .unwrap();
    let locked = shadows_core::testing::Harness::set_thread_harness_for_test(
        &fx.storage,
        &ctx("h3", "thread.harness"),
        &thread,
        "codex",
    )
    .await;
    assert!(matches!(
        locked,
        Err(shadows_core::CoreError::Refused {
            code: shadows_core::ErrorCode::HarnessLocked,
            ..
        })
    ));
    let replay = shadows_core::testing::Harness::set_thread_harness_for_test(
        &fx.storage,
        &ctx("h2", "thread.harness"),
        &thread,
        "claude-code",
    )
    .await
    .unwrap();
    assert_eq!(replay.harness, "claude-code");
}

#[tokio::test]
async fn the_lock_holds_below_the_application() {
    let (fx, thread) = fixture().await;
    shadows_core::Turns::create_pending_operation_for_test(
        &fx.storage,
        &thread,
        &fx.runtime.instance_id,
    )
    .await
    .unwrap();
    let raw = sqlx::query("UPDATE planning_thread SET harness_kind = 'codex' WHERE id = ?")
        .bind(thread.as_str())
        .execute(fx.storage.reader())
        .await;
    assert!(raw.is_err(), "the trigger must refuse a raw update too");
}

#[tokio::test]
async fn a_project_is_created_with_the_policy_modes_and_they_can_be_replaced() {
    let (fx, _) = fixture().await;
    let p = &fx.project;
    assert_eq!(p.allowed_modes["claude-code"], vec!["acceptEdits", "auto"]);
    let only: BTreeMap<_, _> =
        [("claude-code".to_string(), vec!["acceptEdits".to_string()])].into();
    let p2 = shadows_core::testing::set_project_modes(
        &fx.storage,
        &ctx("m1", "project.modes"),
        &p.id,
        &only,
    )
    .await
    .unwrap();
    assert_eq!(p2.allowed_modes["claude-code"], vec!["acceptEdits"]);
    let listed = shadows_core::testing::projects(fx.storage.clone())
        .list()
        .await
        .unwrap();
    assert_eq!(listed[0].allowed_modes["claude-code"], vec!["acceptEdits"]);
    assert_eq!(
        listed[0].allowed_modes["codex"],
        Vec::<String>::new(),
        "every known harness is listed, with none allowed"
    );
}

#[tokio::test]
async fn an_entry_can_name_its_operation_and_old_entries_name_none() {
    let (fx, thread) = fixture().await;
    let op = shadows_core::Turns::create_pending_operation_for_test(
        &fx.storage,
        &thread,
        &fx.runtime.instance_id,
    )
    .await
    .unwrap();
    let e = shadows_core::Threads::append_thread_entry_for_test(
        &fx.storage,
        &thread,
        NewThreadEntry {
            kind: ThreadEntryKind::AgentMessage,
            author: Actor::system(),
            body: "x",
            refs: &[],
            card: None,
            operation_id: Some(&op),
        },
    )
    .await
    .unwrap();
    assert_eq!(e.operation_id.as_ref(), Some(&op));
    let e2 = shadows_core::Threads::append_thread_entry_for_test(
        &fx.storage,
        &thread,
        NewThreadEntry {
            kind: ThreadEntryKind::UserMessage,
            author: Actor::user("local"),
            body: "y",
            refs: &[],
            card: None,
            operation_id: None,
        },
    )
    .await
    .unwrap();
    assert!(e2.operation_id.is_none());
    let read = shadows_core::Threads::list_thread_entries_for_test(&fx.storage, &thread)
        .await
        .unwrap();
    assert_eq!(read[0].operation_id.as_ref(), Some(&op));
    assert!(read[1].operation_id.is_none());
}
