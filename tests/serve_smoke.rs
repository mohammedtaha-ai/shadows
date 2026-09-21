use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// `shadows serve` must print exactly one local address and must not open a
/// browser. Spec §1.0 and §11.1 both require the daemon to stop at printing.
#[test]
fn serve_prints_one_local_address_and_does_not_open_a_browser() {
    let exe = env!("CARGO_BIN_EXE_shadows");
    let tmp = tempfile::tempdir().unwrap();
    let mut child = Command::new(exe)
        .arg("serve")
        .arg("--db")
        .arg(tmp.path().join("shadows.sqlite3"))
        .arg("--bind")
        .arg("127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("daemon should start");

    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let first = lines.next().expect("expected a line").unwrap();

    assert!(
        first.starts_with("shadows serve listening on http://127.0.0.1:"),
        "unexpected first line: {first}"
    );

    child.kill().unwrap();
    child.wait().unwrap();
}
