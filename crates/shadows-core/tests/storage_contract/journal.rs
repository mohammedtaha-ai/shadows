//! The journal's append: provenance, and the committed signal.

use shadows_core::Actor;
use shadows_core::ProjectId;
use shadows_core::testing::DurableEvent;
use shadows_core::testing::Storage;

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
                sqlx::query(
                    "INSERT INTO project (id, slug, name, directory, created_at) \
                     VALUES (?,?,?,'/d',?)",
                )
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
