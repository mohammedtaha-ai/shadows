use clap::Parser;
use shadows::{
    cli::args::Cli,
    desktop_runtime::{DesktopLaunch, DesktopRuntime, RuntimeReceipt},
};
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};

const CONFIGURATION: &str = "00000000-0000-4000-8000-000000000301";

fn command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_shadows"));
    let executable = env!("CARGO_BIN_EXE_shadows");
    cmd.args([
        "serve",
        "--harness",
        executable,
        "--node",
        executable,
        "--adapter",
        executable,
    ]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

struct Server {
    child: Child,
    _root: tempfile::TempDir,
    receipt: RuntimeReceipt,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
async fn server() -> Server {
    let root = tempfile::tempdir().unwrap();
    let record = root.path().join("runtime.json");
    let child = command()
        .args([
            "--bind",
            "127.0.0.1:0",
            "--desktop-configuration",
            CONFIGURATION,
        ])
        .arg("--desktop-record")
        .arg(&record)
        .arg("--db")
        .arg(root.path().join("core.sqlite3"))
        .spawn()
        .unwrap();
    let mut server = Server {
        child,
        _root: root,
        receipt: RuntimeReceipt {
            identity: shadows_http::RuntimeIdentity {
                runtime_id: String::new(),
                configuration_id: CONFIGURATION.into(),
            },
            origin: String::new(),
            database: std::path::PathBuf::new(),
            process: shadows_process::inspect_process(std::process::id())
                .unwrap()
                .unwrap(),
        },
    };
    for _ in 0..200 {
        if let Ok(bytes) = std::fs::read(&record)
            && let Ok(receipt) = serde_json::from_slice::<RuntimeReceipt>(&bytes)
        {
            server.receipt = receipt;
            return server;
        }
        assert!(
            server.child.try_wait().unwrap().is_none(),
            "Core exited before ready"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("Core must publish a ready receipt");
}
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
}

#[test]
fn desktop_flags_are_paired() {
    let executable = env!("CARGO_BIN_EXE_shadows");
    let root = tempfile::tempdir().unwrap();
    let record = root
        .path()
        .join("runtime.json")
        .to_string_lossy()
        .into_owned();
    let base = [
        "shadows",
        "serve",
        "--harness",
        executable,
        "--node",
        executable,
        "--adapter",
        executable,
    ];
    assert!(
        Cli::try_parse_from(base.iter().copied().chain([
            "--desktop-record",
            &record,
            "--desktop-configuration",
            CONFIGURATION,
        ]))
        .is_ok()
    );
    for rest in [
        vec!["--desktop-record", record.as_str()],
        vec!["--desktop-configuration", CONFIGURATION],
        vec![
            "--desktop-record",
            record.as_str(),
            "--desktop-configuration",
            "not-a-uuid",
        ],
        vec![
            "--desktop-record",
            "relative.json",
            "--desktop-configuration",
            CONFIGURATION,
        ],
    ] {
        assert!(Cli::try_parse_from(base.iter().copied().chain(rest)).is_err());
    }
    assert!(Cli::try_parse_from(base).is_ok());
}

#[tokio::test]
async fn runtime_identity_is_installed_after_core_ready() {
    let server = server().await;
    let response = client()
        .get(format!("{}/api/runtime", server.receipt.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let projects = client()
        .get(format!("{}/api/projects", server.receipt.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(projects.status(), 200);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["configuration_id"],
        CONFIGURATION
    );
}

#[tokio::test]
async fn receipt_matches_bound_endpoint_and_process() {
    let server = server().await;
    assert_eq!(server.receipt.process.pid, server.child.id());
    assert!(server.receipt.process.is_current().unwrap());
    assert_eq!(
        server.receipt.process,
        shadows_process::inspect_process(server.child.id())
            .unwrap()
            .unwrap()
    );
    assert_ne!(server.receipt.origin.rsplit(':').next(), Some("0"));
    let response: shadows_http::RuntimeIdentity = client()
        .get(format!("{}/api/runtime", server.receipt.origin))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response, server.receipt.identity);
}

#[tokio::test]
async fn runtime_identity_does_not_expose_configuration() {
    let server = server().await;
    let body: serde_json::Value = client()
        .get(format!("{}/api/runtime", server.receipt.origin))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mut keys: Vec<_> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["configuration_id", "runtime_id"]);
    assert!(uuid::Uuid::parse_str(body["runtime_id"].as_str().unwrap()).is_ok());
}

#[test]
fn runtime_receipt_owner_is_exclusive() {
    let root = tempfile::tempdir().unwrap();
    let launch = DesktopLaunch {
        record_path: root.path().join("receipt.json"),
        configuration_id: CONFIGURATION.into(),
    };
    let owner = DesktopRuntime::claim(&launch, "127.0.0.1:0".parse().unwrap()).unwrap();
    assert!(DesktopRuntime::claim(&launch, "127.0.0.1:0".parse().unwrap()).is_err());
    drop(owner);
    assert!(DesktopRuntime::claim(&launch, "127.0.0.1:0".parse().unwrap()).is_ok());
}

#[test]
fn desktop_metadata_rejects_non_loopback_bind() {
    let root = tempfile::tempdir().unwrap();
    let launch = DesktopLaunch {
        record_path: root.path().join("receipt.json"),
        configuration_id: CONFIGURATION.into(),
    };
    assert!(DesktopRuntime::claim(&launch, "0.0.0.0:4318".parse().unwrap()).is_err());
}

#[tokio::test]
async fn native_origin_is_added_without_replacing_browser_origins() {
    let server = server().await;
    for origin in [
        "http://tauri.localhost",
        "http://localhost:5173",
        "http://127.0.0.1:5173",
    ] {
        let response = client()
            .get(format!("{}/api/runtime", server.receipt.origin))
            .header("Origin", origin)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            200,
            "configured origin must remain allowed"
        );
        assert_eq!(response.headers()["access-control-allow-origin"], origin);
    }
    for origin in ["null", "http://127.0.0.1:49999", "https://foreign.invalid"] {
        let response = client()
            .get(format!("{}/api/runtime", server.receipt.origin))
            .header("Origin", origin)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403, "foreign origin must remain refused");
    }
}

#[test]
fn replacement_receipt_survives_old_runtime_drop() {
    let root = tempfile::tempdir().unwrap();
    let launch = DesktopLaunch {
        record_path: root.path().join("receipt.json"),
        configuration_id: CONFIGURATION.into(),
    };
    let owner = DesktopRuntime::claim(&launch, "127.0.0.1:0".parse().unwrap()).unwrap();
    owner
        .publish("127.0.0.1:4318".parse().unwrap(), root.path())
        .unwrap();
    let mut receipt: RuntimeReceipt =
        serde_json::from_slice(&std::fs::read(&launch.record_path).unwrap()).unwrap();
    receipt.identity.runtime_id = "00000000-0000-4000-8000-000000000302".into();
    std::fs::write(&launch.record_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    drop(owner);
    assert!(
        launch.record_path.exists(),
        "a newer receipt must not be removed"
    );
}

#[test]
fn own_receipt_is_removed_when_runtime_ends() {
    let root = tempfile::tempdir().unwrap();
    let launch = DesktopLaunch {
        record_path: root.path().join("receipt.json"),
        configuration_id: CONFIGURATION.into(),
    };
    let owner = DesktopRuntime::claim(&launch, "127.0.0.1:0".parse().unwrap()).unwrap();
    owner
        .publish("127.0.0.1:4318".parse().unwrap(), root.path())
        .unwrap();
    drop(owner);
    assert!(!launch.record_path.exists());
}

#[tokio::test]
async fn receipt_identifies_actual_database() {
    let server = server().await;
    assert_eq!(
        server.receipt.database,
        std::fs::canonicalize(server._root.path().join("core.sqlite3")).unwrap()
    );
}
