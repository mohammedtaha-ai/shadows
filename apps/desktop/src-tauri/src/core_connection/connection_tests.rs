use super::{config::LaunchConfig, launch, Connection};
use shadows_process::ProcessIdentity;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

fn configured(root: &Path) -> LaunchConfig {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let executable = repo
        .join("target/debug/shadows.exe")
        .canonicalize()
        .unwrap();
    let db = root.join("core.sqlite3");
    std::fs::write(&db, "").unwrap();
    let home = root.join("home");
    std::fs::create_dir(&home).unwrap();
    let aylul = root.join("aylul.json");
    std::fs::write(
        &aylul,
        serde_json::to_vec(&serde_json::json!({"home_root":home})).unwrap(),
    )
    .unwrap();
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", socket.local_addr().unwrap());
    drop(socket);
    LaunchConfig {
        configuration_id: String::new(),
        executable: executable.clone(),
        db,
        node: executable.clone(),
        adapter: executable.clone(),
        harness: executable,
        aylul_config: aylul,
        origin,
    }
    .normalized()
    .unwrap()
}
fn stop(identity: &ProcessIdentity) {
    if identity.pid != std::process::id() && identity.is_current().unwrap_or(false) {
        let _ = std::process::Command::new("taskkill.exe")
            .args(["/PID", &identity.pid.to_string(), "/F"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        for _ in 0..100 {
            if !identity.is_current().unwrap_or(false) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
struct Fixture {
    directory: tempfile::TempDir,
    config: LaunchConfig,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("desktop عربي space ")
            .tempdir()
            .unwrap();
        let config = configured(directory.path());
        Self { directory, config }
    }
    fn root(&self) -> &Path {
        self.directory.path()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for suffix in ["json", "pending.json"] {
            let path = launch::record_path(self.root(), &self.config.configuration_id)
                .with_extension(suffix);
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(identity) = value.get("process").and_then(|process| {
                        serde_json::from_value::<ProcessIdentity>(process.clone()).ok()
                    }) {
                        stop(&identity);
                    }
                }
            }
        }
    }
}

#[test]
fn configuration_rejects_missing_relative_and_remote_paths() {
    let fixture = Fixture::new();
    let mut config = fixture.config.clone();
    config.db = PathBuf::from("relative.sqlite3");
    assert_eq!(config.normalized().unwrap_err(), "core_path_invalid");
    let mut config = fixture.config.clone();
    config.db = fixture.root().join("missing.sqlite3");
    assert!(config.normalized().is_err());
    for origin in [
        "http://example.com:4318",
        "http://127.0.0.1:0",
        "http://user@127.0.0.1:4318",
        "http://127.0.0.1:4318/path",
        "http://127.0.0.1:4318?q=x",
    ] {
        let mut config = fixture.config.clone();
        config.origin = origin.into();
        assert_eq!(config.normalized().unwrap_err(), "core_origin_invalid");
    }
}
#[test]
fn two_concurrent_starts_reuse_one_core() {
    let fixture = Fixture::new();
    let root = fixture.root().to_path_buf();
    let config = fixture.config.clone();
    let second =
        std::thread::spawn(move || launch::ensure_running(&config, &root, Duration::from_secs(30)));
    let first =
        launch::ensure_running(&fixture.config, fixture.root(), Duration::from_secs(30)).unwrap();
    let second = second.join().unwrap().unwrap();
    assert_eq!(first.runtime_id, second.runtime_id);
    assert_eq!(first.configuration_id, fixture.config.configuration_id);
    let receipt = launch::read_receipt(&launch::record_path(
        fixture.root(),
        &fixture.config.configuration_id,
    ))
    .unwrap();
    assert!(receipt.process.is_current().unwrap());
    assert_eq!(receipt.database, fixture.config.db);
}
#[test]
fn occupied_foreign_port_never_starts_another_process() {
    let fixture = Fixture::new();
    let socket = fixture.config.socket().unwrap();
    let _listener = std::net::TcpListener::bind(socket).unwrap();
    let result = launch::ensure_running(&fixture.config, fixture.root(), Duration::from_secs(1));
    assert_eq!(result.unwrap_err(), "core_address_in_use");
    assert!(!launch::pending_path(fixture.root(), &fixture.config.configuration_id).exists());
}
#[test]
fn live_receipt_cannot_claim_another_database() {
    let fixture = Fixture::new();
    launch::ensure_running(&fixture.config, fixture.root(), Duration::from_secs(30)).unwrap();
    let mut changed = fixture.config.clone();
    changed.db = fixture.root().join("other.sqlite3");
    std::fs::write(&changed.db, "").unwrap();
    assert_eq!(
        launch::ensure_running(&changed, fixture.root(), Duration::from_secs(1)).unwrap_err(),
        "core_identity_mismatch"
    );
}
#[test]
fn desktop_shutdown_does_not_stop_core() {
    let fixture = Fixture::new();
    let manager = Connection::new(fixture.root().to_path_buf());
    manager.save(fixture.config.clone()).unwrap();
    let view = manager.retry();
    assert_eq!(view.state, "ready");
    let receipt = launch::read_receipt(&launch::record_path(
        fixture.root(),
        &fixture.config.configuration_id,
    ))
    .unwrap();
    drop(manager);
    assert!(receipt.process.is_current().unwrap());
    let resumed =
        launch::ensure_running(&fixture.config, fixture.root(), Duration::from_secs(1)).unwrap();
    assert_eq!(resumed.runtime_id, receipt.runtime_id);
}
#[test]
fn outdated_startup_result_cannot_replace_new_configuration() {
    let fixture = Fixture::new();
    let manager = Connection::new(fixture.root().to_path_buf());
    manager.save(fixture.config.clone()).unwrap();
    let (old_epoch, _) = manager.begin().unwrap();
    let (new_epoch, _) = manager.begin().unwrap();
    let ready = super::ReadyConnection {
        origin: fixture.config.origin.clone(),
        configuration_id: fixture.config.configuration_id.clone(),
        runtime_id: "00000000-0000-4000-8000-000000000401".into(),
    };
    manager.finish(new_epoch, Ok(ready));
    let revision = manager.snapshot().revision;
    manager.finish(old_epoch, Err("core_unavailable"));
    assert_eq!(manager.snapshot().state, "ready");
    assert_eq!(manager.snapshot().revision, revision);
}
#[test]
fn failed_startup_requires_explicit_retry() {
    let fixture = Fixture::new();
    let manager = Connection::new(fixture.root().to_path_buf());
    manager.save(fixture.config.clone()).unwrap();
    let (epoch, _) = manager.begin().unwrap();
    manager.finish(epoch, Err("core_start_timeout"));
    assert_eq!(manager.snapshot().state, "failed");
    assert!(manager.ready().is_none());
    assert_eq!(
        manager.snapshot().error.as_deref(),
        Some("core_start_timeout")
    );
}

#[test]
fn corrupt_saved_configuration_is_a_fixed_failure() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("desktop-core.json"), "{broken").unwrap();
    let manager = Connection::new(directory.path().to_path_buf());
    assert_eq!(manager.snapshot().state, "failed");
    assert_eq!(
        manager.snapshot().error.as_deref(),
        Some("core_config_invalid")
    );
    assert!(manager.configured().is_none());
}
#[test]
fn reused_pid_receipt_cannot_attach_to_foreign_port() {
    let fixture = Fixture::new();
    let _listener = std::net::TcpListener::bind(fixture.config.socket().unwrap()).unwrap();
    let mut process = shadows_process::inspect_process(std::process::id())
        .unwrap()
        .unwrap();
    process.started_at += 1;
    let receipt = serde_json::json!({
        "runtime_id":uuid::Uuid::new_v4(),"configuration_id":fixture.config.configuration_id,
        "origin":fixture.config.origin,"database":fixture.config.db,"process":process
    });
    let record = launch::record_path(fixture.root(), &fixture.config.configuration_id);
    super::config::write_json(&record, &receipt).unwrap();
    assert_eq!(
        launch::ensure_running(&fixture.config, fixture.root(), Duration::from_secs(1))
            .unwrap_err(),
        "core_address_in_use"
    );
}
#[test]
fn timeout_and_retry_retain_the_pending_process_identity() {
    let fixture = Fixture::new();
    let process = shadows_process::inspect_process(std::process::id())
        .unwrap()
        .unwrap();
    let pending = launch::pending_path(fixture.root(), &fixture.config.configuration_id);
    super::config::write_json(
        &pending,
        &serde_json::json!({
            "configuration_id":fixture.config.configuration_id,"process":process
        }),
    )
    .unwrap();
    let before = std::fs::read(&pending).unwrap();
    for _ in 0..2 {
        assert_eq!(
            launch::ensure_running(&fixture.config, fixture.root(), Duration::from_millis(50))
                .unwrap_err(),
            "core_start_timeout"
        );
        assert_eq!(std::fs::read(&pending).unwrap(), before);
    }
}

#[test]
fn configuration_replacement_rejects_previous_ready_result() {
    let fixture = Fixture::new();
    let manager = Connection::new(fixture.root().to_path_buf());
    manager.save(fixture.config.clone()).unwrap();
    let (epoch, _) = manager.begin().unwrap();
    let mut changed = fixture.config.clone();
    changed.origin = "http://127.0.0.1:43398".into();
    manager.save(changed).unwrap();
    manager.finish(
        epoch,
        Ok(super::ReadyConnection {
            configuration_id: fixture.config.configuration_id.clone(),
            runtime_id: uuid::Uuid::new_v4().to_string(),
            origin: fixture.config.origin.clone(),
        }),
    );
    assert_eq!(manager.snapshot().state, "starting");
    assert_ne!(
        manager.snapshot().configuration_id.as_deref(),
        Some(fixture.config.configuration_id.as_str())
    );
    assert!(manager.ready().is_none());
}
