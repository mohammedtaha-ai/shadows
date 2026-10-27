//! A parent-owned directory for tests that deliberately leave process teardown unavailable.
use std::path::PathBuf;

pub fn run(test: &str) -> Option<PathBuf> {
    const DIRECTORY: &str = "SHADOWS_TEST_PROCESS_DIRECTORY";
    if let Some(directory) = std::env::var_os(DIRECTORY) {
        return Some(PathBuf::from(directory));
    }
    let directory = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env(DIRECTORY, directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "process fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    directory
        .close()
        .expect("the exited process releases its database and adapters");
    None
}
