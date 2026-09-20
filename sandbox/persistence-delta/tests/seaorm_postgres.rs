#![cfg(feature = "seaorm-current")]

use shadows_persistence_delta::{AtomicFixture, seaorm_current::SeaOrmStore};

const URL: &str = "postgres://shadows:test@127.0.0.1:5433/shadows";

#[tokio::test]
async fn current_seaorm_proves_postgres_delta_contract() {
    let store = SeaOrmStore::reset_and_migrate(URL)
        .await
        .expect("fresh latest");

    let fixture = AtomicFixture::new("project:seaorm");
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
        .atomic_command(AtomicFixture::new("project:seaorm"))
        .await
        .expect("second command");
    assert!(second.durable_seq > first.durable_seq);
    let events = store
        .events_after("project:seaorm", first.durable_seq)
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

    let failed = AtomicFixture::new("project:seaorm");
    let failed_id = failed.operation.id;
    assert!(store.atomic_command_then_fail(failed).await.is_err());
    assert!(store.operation(failed_id).await.expect("lookup").is_none());

    SeaOrmStore::reset(URL).await.expect("reset for fixture");
    SeaOrmStore::migrate_v1(URL).await.expect("v1 fixture");
    SeaOrmStore::seed_v1_fixture(URL)
        .await
        .expect("seed v1 row");
    SeaOrmStore::migrate_latest(URL)
        .await
        .expect("v1 to latest");
    assert!(
        SeaOrmStore::v1_fixture_survived(URL)
            .await
            .expect("fixture read")
    );
    assert_eq!(
        SeaOrmStore::migration_count(URL)
            .await
            .expect("migration count"),
        2
    );

    SeaOrmStore::reset(URL)
        .await
        .expect("reset for concurrent migration");
    let attempts = (0..4).map(|_| SeaOrmStore::migrate_latest(URL));
    let results = futures::future::join_all(attempts).await;
    let successes = results.iter().filter(|result| result.is_ok()).count();
    let failures = results.len() - successes;
    eprintln!("seaorm concurrent migration startups: {successes} succeeded, {failures} failed");
    if let Some(error) = results.iter().find_map(|result| result.as_ref().err()) {
        eprintln!("seaorm first concurrent migration error: {error}");
    }
    assert!(
        successes >= 1,
        "at least one migrator must establish the schema"
    );
    assert_eq!(
        SeaOrmStore::migration_count(URL)
            .await
            .expect("final migration count"),
        2
    );
}
