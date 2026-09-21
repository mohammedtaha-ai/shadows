//! The thread capability's durable contract: planning threads parented to a
//! project and thread entries whose ordinals are allocated transactionally.
//! Separate from `tests/storage_contract.rs`, whose one job is the connection
//! and transaction contracts.

use shadows::command::{CommandContext, fingerprint};
use shadows::storage::Storage;

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
        .create_project(&ctx("cmd-p", &params), "demo", "Demo")
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx_kind("cmd-t", "thread.create", &params),
            &project.id,
            "T",
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
                        "UserMessage",
                        "User",
                        "local",
                        &format!("w{w}-i{i}"),
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
            .bind(&thread.id)
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
            .bind(&thread.id)
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
        .create_project(&ctx("cmd-p", &params), "demo", "Demo")
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(
            &ctx_kind("cmd-t", "thread.create", &params),
            &project.id,
            "T",
        )
        .await
        .unwrap();

    for body in ["first", "second", "third"] {
        storage
            .append_thread_entry(&thread.id, "UserMessage", "User", "local", body)
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
