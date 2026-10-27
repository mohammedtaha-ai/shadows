//! The project capability's durable contract: identity creation and the
//! external-command idempotency machinery it introduces, reused by every
//! later mutating command. Separate from `tests/storage_contract/`, whose
//! one job is the connection and transaction contracts.

use shadows_core::testing::CommandContext;
use shadows_core::testing::ProjectDirectory;
use shadows_core::testing::Storage;

/// Any directory that exists: these tests are about identity and
/// idempotency, not about where a turn runs.
fn dir() -> ProjectDirectory {
    ProjectDirectory::resolve(&std::env::temp_dir()).unwrap()
}

fn ctx(command_id: &str, params: &serde_json::Value) -> CommandContext {
    ctx_kind(command_id, "project.create", params)
}

fn ctx_kind(command_id: &str, kind: &str, params: &serde_json::Value) -> CommandContext {
    CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: command_id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint(kind, params),
    }
}

/// Projects come back most recently used first, read from the journal: a new
/// project is ahead of an older one until the older one's thread starts an
/// operation. A thread with no operation moves nothing, and a removed thread's
/// operation still counts, as the order over the thread rows did.
#[tokio::test]
async fn projects_are_ordered_by_the_newest_operation_in_the_journal() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let mut created = Vec::new();
    for slug in ["old", "new"] {
        let params = serde_json::json!({ "slug": slug });
        let project = shadows_core::testing::create_project(
            &storage,
            &ctx(slug, &params),
            slug,
            slug,
            &dir(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
        created.push(project.id);
    }
    let projects = shadows_core::testing::projects(storage.clone());
    let order = async || -> Vec<_> {
        projects
            .by_recent_activity()
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect()
    };
    assert_eq!(order().await, vec![created[1].clone(), created[0].clone()]);

    let idle_params = serde_json::json!({ "thread": "idle" });
    shadows_core::Threads::create_planning_thread_for_test(
        &storage,
        &ctx_kind("idle", "thread.create", &idle_params),
        &created[0],
        "Idle",
        "claude-code",
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        order().await,
        vec![created[1].clone(), created[0].clone()],
        "a thread with no operation must not move its project"
    );

    let runtime =
        shadows_core::testing::Runtime::register_runtime_instance_for_test(&storage, "test")
            .await
            .unwrap();
    let thread_params = serde_json::json!({ "thread": "t" });
    let thread = shadows_core::Threads::create_planning_thread_for_test(
        &storage,
        &ctx_kind("t", "thread.create", &thread_params),
        &created[0],
        "T",
        "claude-code",
        None,
    )
    .await
    .unwrap();
    shadows_core::Turns::create_pending_operation_for_test(&storage, &thread.id, &runtime)
        .await
        .unwrap();
    assert_eq!(order().await, created);

    let remove_params = serde_json::json!({ "thread": thread.id });
    shadows_core::Turns::remove_thread_for_test(
        &storage,
        &ctx_kind("rm", "thread.remove", &remove_params),
        &thread.id,
    )
    .await
    .unwrap();
    assert_eq!(
        order().await,
        created,
        "a removed thread's operation must still count"
    );

    storage.close().await.unwrap();
}
