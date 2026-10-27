use std::sync::Arc;

use core_db::{Storage, StorageError};
use core_usage::{Usage, UsageError, UsageRecord};

fn record(id: &str, preset: &str, cost: f64) -> UsageRecord {
    UsageRecord {
        operation_id: id.into(),
        thread_id: "thread".into(),
        agent_slug: "agent".into(),
        provider_slug: "provider".into(),
        provider_name: "Historical provider".into(),
        label: None,
        preset: Some(preset.into()),
        model: "model".into(),
        input_tokens: Some(10),
        output_tokens: Some(4),
        cached_read_tokens: Some(20),
        cached_write_tokens: Some(30),
        total_tokens: Some(64),
        cost: Some((cost, "USD".into())),
    }
}

async fn write(storage: &Storage, record: UsageRecord, ts: &'static str) {
    storage
        .write_txn(move |conn| Box::pin(Usage::record_in(conn, record, ts)))
        .await
        .unwrap();
}

#[tokio::test]
async fn reports_keep_exact_endpoints_cache_counts_and_historical_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let usage = Usage::new(storage.clone());
    for (id, ts) in [
        ("before", "2026-01-01T00:00:00.099Z"),
        ("inside", "2026-01-01T00:00:00.100Z"),
        ("after", "2026-01-01T00:00:00.200Z"),
    ] {
        write(&storage, record(id, "anthropic", 0.2), ts).await;
    }
    let report = usage
        .usage("2026-01-01T00:00:00.100Z", "2026-01-01T00:00:00.200Z")
        .await
        .unwrap();
    let provider = &report.by_provider[0];
    assert_eq!(provider.requests, 1);
    assert_eq!(provider.tokens_in, 60);
    assert_eq!(provider.tokens_out, 4);
    assert_eq!(provider.provider_name, "Historical provider");
    assert_eq!(provider.cost, Some(0.2));
    assert_eq!(report.by_model[0].model, "model");

    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_partial_cost_group_is_unknown_and_a_refused_write_rolls_back() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let usage = Usage::new(storage.clone());
    let ts = "2026-01-01T00:00:00Z";
    write(&storage, record("known", "anthropic", 0.2), ts).await;
    write(&storage, record("untrusted", "other", 999.0), ts).await;
    let error = storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Usage::record_in(conn, record("rolled-back", "anthropic", 0.3), ts).await?;
                Usage::record_in(conn, record("known", "anthropic", 0.3), ts).await
            })
        })
        .await
        .unwrap_err();
    assert!(matches!(error, StorageError::Constraint(_)));
    let report = usage
        .usage("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(report.by_provider[0].requests, 2);
    assert_eq!(report.by_provider[0].cost, None);
    assert_eq!(report.by_provider[0].currency, None);

    storage.close().await.unwrap();
}

#[tokio::test]
async fn invalid_or_reversed_periods_keep_the_application_message() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let usage = Usage::new(storage.clone());
    for (from, to) in [
        ("bad", "2026-01-02T00:00:00Z"),
        ("2026-01-02T00:00:00Z", "2026-01-01T00:00:00Z"),
    ] {
        let error = usage.usage(from, to).await.unwrap_err();
        assert!(matches!(error, UsageError::InvalidPeriod));
        assert_eq!(error.to_string(), "invalid profile: invalid usage period");
    }

    storage.close().await.unwrap();
}
