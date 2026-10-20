use shadows_process::{ProcessIdentity, ProcessSpec, inspect_process, spawn_persistent};
use std::{
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

struct Probe {
    directory: tempfile::TempDir,
    identity: ProcessIdentity,
}
impl Probe {
    fn stop(&self) -> PathBuf {
        self.directory.path().join("stop")
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        let _ = std::fs::write(self.stop(), "");
        let until = Instant::now() + Duration::from_secs(5);
        while self.identity.is_current().unwrap_or(false) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn launch() -> Probe {
    let directory = tempfile::Builder::new()
        .prefix("persistent عربي space ")
        .tempdir()
        .unwrap();
    let executable = directory.path().join(if cfg!(windows) {
        "probe copy.exe"
    } else {
        "probe copy"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_tree_probe"), &executable).unwrap();
    let receipt = directory.path().join("identity.json");
    let status = Command::new(env!("CARGO_BIN_EXE_tree_probe"))
        .arg("--persistent-launch")
        .arg(&receipt)
        .arg(&executable)
        .arg(directory.path().join("report.json"))
        .arg(directory.path().join("stop"))
        .args([r"C:\Work Space\عربي", "& literal"])
        .env("ANTHROPIC_AUTH_TOKEN", "fixture-only-must-not-inherit")
        .status()
        .unwrap();
    assert!(status.success(), "launcher must finish successfully");
    let identity = serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
    let probe = Probe {
        directory,
        identity,
    };
    let until = Instant::now() + Duration::from_secs(5);
    while !probe.directory.path().join("report.json").exists() {
        assert!(Instant::now() < until, "persistent child must report");
        std::thread::sleep(Duration::from_millis(20));
    }
    probe
}
fn report(probe: &Probe) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(probe.directory.path().join("report.json")).unwrap())
        .unwrap()
}
#[test]
fn persistent_child_survives_launcher_exit() {
    let probe = launch();
    assert!(probe.identity.is_current().unwrap());
    assert_eq!(
        inspect_process(probe.identity.pid).unwrap(),
        Some(probe.identity.clone())
    );
    assert_ne!(probe.identity.started_at, 0);
    assert_eq!(
        std::fs::canonicalize(&probe.identity.executable).unwrap(),
        std::fs::canonicalize(probe.directory.path().join(if cfg!(windows) {
            "probe copy.exe"
        } else {
            "probe copy"
        }))
        .unwrap()
    );
}
#[test]
fn persistent_launch_keeps_argument_boundaries() {
    let probe = launch();
    assert_eq!(
        report(&probe)["args"],
        serde_json::json!([r"C:\Work Space\عربي", "& literal"])
    );
}
#[test]
fn persistent_launch_has_explicit_environment() {
    let probe = launch();
    let value = report(&probe);
    assert_eq!(value["configured"], "configured");
    assert_eq!(value["providerTokenPresent"], false);
}
#[test]
fn persistent_launch_rejects_pipes() {
    for (capture_stdout, pipe_stdin) in [(true, false), (false, true)] {
        let result = spawn_persistent(ProcessSpec {
            executable: env!("CARGO_BIN_EXE_tree_probe").into(),
            args: vec![],
            cwd: std::env::temp_dir(),
            env: vec![],
            capture_stdout,
            pipe_stdin,
        });
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }
}
#[test]
fn process_identity_rejects_reused_pid() {
    let mut identity = inspect_process(std::process::id()).unwrap().unwrap();
    assert!(identity.is_current().unwrap());
    identity.started_at = 0;
    assert!(
        !identity.is_current().unwrap(),
        "a reused pid must not match a stale instance"
    );
    assert!(inspect_process(u32::MAX).unwrap().is_none());
}
