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
        // Required, with no default: spec §1.4 resolves the harness from
        // explicit configuration and refuses a bare name, which would be a PATH
        // lookup. Nothing is spawned in this test; the path only has to be
        // absolute. `tests/harness_config.rs` owns that rule.
        .arg("--harness")
        .arg(tmp.path().join("claude.exe"))
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
    let addr = first
        .trim_start_matches("shadows serve listening on http://")
        .to_string();

    // What it serves is an API, not a page (spec §1), and with no
    // `--allow-origin` it admits Vite's dev server under both of its names.
    let projects = request(&addr, "GET /api/projects HTTP/1.1\r\n");
    let preflights: Vec<String> = shadows::config::DEFAULT_ALLOWED_ORIGINS
        .iter()
        .map(|origin| {
            request(
                &addr,
                &format!(
                    "OPTIONS /api/projects HTTP/1.1\r\nOrigin: {origin}\r\n\
                     Access-Control-Request-Method: POST\r\n"
                ),
            )
        })
        .collect();

    child.kill().unwrap();
    child.wait().unwrap();

    assert!(projects.starts_with("HTTP/1.1 200"), "{projects}");
    for (origin, response) in shadows::config::DEFAULT_ALLOWED_ORIGINS
        .iter()
        .zip(&preflights)
    {
        assert!(
            response
                .to_ascii_lowercase()
                .contains(&format!("access-control-allow-origin: {origin}")),
            "{origin} was not allowed by default:\n{response}"
        );
    }
}

/// One request on its own connection; returns the response's status line and
/// headers.
fn request(addr: &str, head: &str) -> String {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    write!(stream, "{head}Host: {addr}\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    let mut buf = [0u8; 4096];
    while !response.contains("\r\n\r\n") {
        let n = stream.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        response.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    response
}
