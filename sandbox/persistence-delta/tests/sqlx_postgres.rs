#![cfg(feature = "sqlx-current")]

use shadows_persistence_delta::{AtomicFixture, sqlx_current::SqlxStore};

const URL: &str = "postgres://shadows:test@127.0.0.1:5433/shadows";

#[tokio::test]
async fn current_sqlx_proves_postgres_delta_contract_without_seaquery() {
    let store = SqlxStore::reset_and_migrate(URL)
        .await
        .expect("fresh latest");

    let fixture = AtomicFixture::new("project:sqlx");
    let first = store
        .atomic_command(fixture.clone())
        .await
        .expect("atomic command");
    let retry = store
        .atomic_command(fixture.clone())
        .await
        .expect("idempotent retry");
    assert!(first.inserted);
    assert!(!retry.inserted);
    assert_eq!(first.operation_id, retry.operation_id);
    assert_eq!(first.durable_seq, retry.durable_seq);

    let second = store
        .atomic_command(AtomicFixture::new("project:sqlx"))
        .await
        .expect("second command");
    assert!(second.durable_seq > first.durable_seq);
    let events = store
        .events_after("project:sqlx", first.durable_seq)
        .await
        .expect("events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].durable_seq, second.durable_seq);

    let operation = store
        .operation(first.operation_id)
        .await
        .expect("roundtrip")
        .expect("present");
    assert!(operation.thread_id.is_some());
    assert!(operation.workflow_id.is_some());
    assert!(operation.task_id.is_none());
    assert!(operation.outcome.is_some());
    assert_eq!(events[0].payload["source"], "delta");

    let failed = AtomicFixture::new("project:sqlx");
    let failed_id = failed.operation.id;
    assert!(store.atomic_command_then_fail(failed).await.is_err());
    assert!(store.operation(failed_id).await.expect("lookup").is_none());

    SqlxStore::reset(URL).await.expect("reset for fixture");
    SqlxStore::migrate_v1(URL).await.expect("v1 fixture");
    SqlxStore::seed_v1_fixture(URL).await.expect("seed v1 row");
    SqlxStore::migrate_latest(URL).await.expect("v1 to latest");
    assert!(
        SqlxStore::v1_fixture_survived(URL)
            .await
            .expect("fixture read")
    );
    assert_eq!(
        SqlxStore::migration_count(URL)
            .await
            .expect("migration count"),
        2
    );

    SqlxStore::reset(URL)
        .await
        .expect("reset for concurrent migration");
    let attempts = (0..4).map(|_| SqlxStore::migrate_latest(URL));
    let results = futures::future::join_all(attempts).await;
    let successes = results.iter().filter(|result| result.is_ok()).count();
    let failures = results.len() - successes;
    eprintln!("sqlx concurrent migration startups: {successes} succeeded, {failures} failed");
    assert_eq!(
        failures, 0,
        "official SQLx migrator should lock concurrent runs"
    );
}
