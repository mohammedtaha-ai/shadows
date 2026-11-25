use std::{path::PathBuf, sync::Arc, time::Duration};

use shadows_core::testing::{Harness, Storage, acp};
use shadows_core::{AppCore, HarnessError, KeySource, StartConfig};

async fn fixture() -> (tempfile::TempDir, Arc<Harness>) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&dir.path().join("shadows.sqlite3"))
            .await
            .unwrap(),
    );
    let harness = AppCore::harness_for_tests(
        acp::fake_adapter(),
        storage,
        core_harness::SessionsConfig {
            catalog_root: dir.path().into(),
            ..acp::test_config()
        },
    );
    (dir, harness)
}

#[tokio::test]
async fn the_catalog_is_read_once() {
    let (_dir, harness) = fixture().await;
    let first = harness.catalog("claude-code").await.unwrap();
    assert!(!first.models.iter().any(|model| model.id == "fake-locked"));
    let efforts = |id: &str| {
        first
            .models
            .iter()
            .find(|model| model.id == id)
            .unwrap()
            .efforts
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(efforts("fake-large"), ["low", "high", "max"]);
    assert_eq!(efforts("fake-small"), ["low", "high"]);
    assert!(efforts("fake-tiny").is_empty());
    assert_eq!(first, harness.catalog("claude-code").await.unwrap());
    assert_eq!(harness.catalog_probes_for_test("claude-code"), 1);
}

#[tokio::test]
async fn concurrent_catalog_reads_open_one_probe() {
    let (_dir, harness) = fixture().await;
    let (first, second) = tokio::join!(
        harness.catalog("claude-code"),
        harness.catalog("claude-code")
    );
    assert_eq!(first.unwrap(), second.unwrap());
    assert_eq!(harness.catalog_probes_for_test("claude-code"), 1);
}

#[tokio::test]
async fn a_failed_probe_is_tried_again() {
    let (_dir, harness) = fixture().await;
    harness.fail_next_probe_for_test("claude-code");
    assert!(matches!(
        harness.catalog("claude-code").await,
        Err(HarnessError::HarnessStartFailed(_))
    ));
    assert!(harness.catalog("claude-code").await.is_ok());
    assert_eq!(harness.catalog_probes_for_test("claude-code"), 2);
}

#[tokio::test]
async fn an_aged_catalog_is_read_again() {
    let (_dir, harness) = fixture().await;
    let first = harness.catalog("claude-code").await.unwrap();
    harness.age_catalog_for_test("claude-code").await;
    assert_eq!(first, harness.catalog("claude-code").await.unwrap());
    assert_eq!(harness.catalog_probes_for_test("claude-code"), 2);
    assert_eq!(first, harness.catalog("claude-code").await.unwrap());
    assert_eq!(harness.catalog_probes_for_test("claude-code"), 2);
}

#[tokio::test]
async fn a_failed_reread_keeps_the_last_catalog() {
    let (_dir, harness) = fixture().await;
    let first = harness.catalog("claude-code").await.unwrap();
    harness.age_catalog_for_test("claude-code").await;
    harness.fail_next_probe_for_test("claude-code");
    assert_eq!(first, harness.catalog("claude-code").await.unwrap());
    assert_eq!(first, harness.catalog("claude-code").await.unwrap());
    assert_eq!(harness.catalog_probes_for_test("claude-code"), 2);
}

#[tokio::test]
async fn codex_not_configured_is_unavailable() {
    let (_dir, harness) = fixture().await;
    for _ in 0..2 {
        assert!(matches!(
            harness.catalog("codex").await,
            Err(HarnessError::HarnessUnavailable(_))
        ));
    }
    assert_eq!(harness.catalog_probes_for_test("codex"), 0);
}

#[tokio::test]
async fn aylul_has_no_executor_catalog() {
    let (_dir, harness) = fixture().await;
    assert!(matches!(
        harness.catalog("aylul").await,
        Err(HarnessError::InvalidCommand(_))
    ));
}

#[tokio::test]
#[ignore = "live: needs a signed-in Claude Code install and SHADOWS_* executable paths"]
async fn live_the_claude_code_catalog_lists_each_models_efforts() {
    live_catalog("claude-code", None, None).await;
}

#[tokio::test]
#[ignore = "live: needs SHADOWS_CODEX_PATH and a signed-in SHADOWS_CODEX_HOME"]
async fn live_the_codex_catalog_lists_each_models_efforts() {
    live_catalog(
        "codex",
        Some(required_path("SHADOWS_CODEX_PATH")),
        Some(required_path("SHADOWS_CODEX_HOME")),
    )
    .await;
}

fn required_path(var: &str) -> PathBuf {
    std::env::var_os(var)
        .unwrap_or_else(|| panic!("{var} must be set"))
        .into()
}

async fn live_catalog(kind: &str, codex_path: Option<PathBuf>, codex_home: Option<PathBuf>) {
    let root = tempfile::tempdir().unwrap();
    let path = |name| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .unwrap_or_else(|| root.path().join("absent"))
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let core = AppCore::start(
        &StartConfig {
            secret_key: KeySource::Given(None),
            db_path: root.path().join("core.sqlite3"),
            node_path: path("SHADOWS_NODE_PATH"),
            adapter_path: path("SHADOWS_ADAPTER_PATH"),
            harness_path: path("SHADOWS_HARNESS_PATH"),
            codex_path,
            codex_home,
            aylul: None,
        },
        format!("http://{}/mcp", listener.local_addr().unwrap()),
    )
    .await
    .unwrap();
    let router = shadows_mcp::service(core.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let project = core
        .projects()
        .create(
            "catalog-project".into(),
            "catalog",
            "Catalog",
            root.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let thread = core
        .threads()
        .create(
            "catalog-thread".into(),
            &project.id,
            "Planner",
            Some(kind),
            None,
        )
        .await
        .unwrap();
    let catalog = core.harness().catalog(kind).await.unwrap();
    assert!(catalog.models.iter().any(|model| !model.efforts.is_empty()));
    let offered = core.harness().open_session(&thread.id).await.unwrap();
    let catalog_ids: Vec<_> = catalog.models.iter().map(|model| &model.id).collect();
    let offered_ids: Vec<_> = offered.models.iter().map(|model| &model.id).collect();
    eprintln!("{kind} catalog: {catalog_ids:?}");
    assert!(
        catalog_ids.iter().all(|model| offered_ids.contains(model)),
        "{catalog_ids:?} vs. {offered_ids:?}"
    );
    if kind == "codex" {
        assert_eq!(catalog_ids, offered_ids);
    }
    core.shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    server.abort();
}
