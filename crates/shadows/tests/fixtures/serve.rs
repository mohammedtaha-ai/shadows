//! Shared apparatus: the real `shadows serve` binary on a database, for what
//! only the daemon's own startup does. Include on its own:
//!
//! ```ignore
//! #[path = "fixtures/serve.rs"] mod serve;
//! ```

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

/// A running daemon, stopped when dropped.
pub struct Served {
    child: Child,
    /// `http://127.0.0.1:<port>`, as the daemon printed it.
    pub base: String,
    _lines: Lines<BufReader<ChildStdout>>,
    _tmp: tempfile::TempDir,
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `shadows serve` on `db`, running `node` as its adapter's runtime; answers
/// once it serves, when everything startup does has happened. The harness
/// and the adapter script are empty files: `fake_acp` as `node` ignores them,
/// and `tree_probe` never speaks ACP at all.
pub fn serve(db: &Path, node: &str) -> Served {
    let tmp = tempfile::tempdir().unwrap();
    let (harness, adapter) = (tmp.path().join("claude.exe"), tmp.path().join("a.js"));
    std::fs::write(&harness, "").unwrap();
    std::fs::write(&adapter, "").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_shadows"))
        .arg("serve")
        .args(["--db".as_ref(), db.as_os_str()])
        .args(["--bind", "127.0.0.1:0"])
        .args(["--harness".as_ref(), harness.as_os_str()])
        .args(["--node", node])
        .args(["--adapter".as_ref(), adapter.as_os_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the daemon starts");
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let first = lines.next().expect("a line").unwrap();
    let base = first
        .strip_prefix("shadows serve listening on ")
        .unwrap_or_else(|| panic!("unexpected first line: {first}"))
        .to_string();
    Served {
        child,
        base,
        _lines: lines,
        _tmp: tmp,
    }
}
