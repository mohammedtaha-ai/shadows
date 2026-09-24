### Task A1: Pin the adapter, configure it, pipe a child's stdin

**Files:**
- Create: `harness/claude/package.json`, `harness/claude/package-lock.json`, `harness/README.md`
- Modify: `src/config.rs` (three paths), `src/cli/mod.rs` (flags, versions), `src/process/mod.rs` (`pipe_stdin`, `take_stdio`), `.gitignore` (`harness/*/node_modules/`), `Cargo.toml` (`agent-client-protocol`, `tokio-util` with `compat`)
- Test: `tests/harness_config.rs` (extend), `tests/containment.rs` (extend)

**Interfaces:**
- Produces:
  - `Config { .., node_path: PathBuf, adapter_path: PathBuf, harness_path: PathBuf }` — all three built through `config::harness_path` (absolute, exists).
  - `config::adapter_version(adapter_entry: &Path) -> String` — reads `version` from the `package.json` in the entry's parent's parent (`…/claude-agent-acp/dist/index.js` → `…/claude-agent-acp/package.json`); `"unknown"` when unreadable.
  - `ProcessSpec { .., pipe_stdin: bool }` (every existing construction passes `false`).
  - `pub type ChildIn = tokio::process::ChildStdin; pub type ChildOut = tokio::process::ChildStdout; pub type ChildErr = tokio::process::ChildStderr;` in `process/`.
  - `ProcessHandle::take_stdio(&mut self) -> Option<(ChildIn, ChildOut, ChildErr)>` — `Some` once, for a spec with `pipe_stdin && capture_stdout`; such a spec also pipes stderr (the adapter writes diagnostics there, ACP_PROBE "Also seen").

- [ ] **Step 1: Pin the adapter.**

```json
{
  "name": "shadows-harness-claude",
  "private": true,
  "description": "The ACP adapter Shadows runs for Claude Code (spec §12.2). Installed, never patched.",
  "engines": { "node": ">=22" },
  "dependencies": { "@agentclientprotocol/claude-agent-acp": "0.81.1" }
}
```

Run `npm install --omit=optional` in `harness/claude/` (Mohammed runs it if slow) so the lock file exists. `harness/README.md` states in five lines: what `harness/` is, that it belongs to the daemon, how to install (`npm ci --omit=optional`), that updating is a version bump plus a run, and that nothing here is patched.

- [ ] **Step 2: Failing tests.** In `tests/containment.rs`:

```rust
#[tokio::test]
async fn a_piped_child_echoes_stdin_and_its_tree_is_contained() {
    // tree_probe's new `echo` mode copies stdin lines to stdout.
    let mut handle = shadows::process::spawn(ProcessSpec {
        executable: tree_probe_path(),
        args: vec!["echo".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
        pipe_stdin: true,
    }).unwrap();
    let (mut stdin, stdout, _stderr) = handle.take_stdio().expect("stdio taken once");
    assert!(handle.take_stdio().is_none());
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    stdin.write_all(b"ping\n").await.unwrap();
    let mut lines = BufReader::new(stdout).lines();
    assert_eq!(lines.next_line().await.unwrap().as_deref(), Some("ping"));
    handle.terminate_tree().unwrap();
    handle.wait().await.unwrap();
}
```

In `tests/harness_config.rs`:

```rust
#[test]
fn adapter_version_is_read_from_its_package() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"version":"0.81.1"}"#).unwrap();
    std::fs::write(dir.path().join("dist/index.js"), "").unwrap();
    assert_eq!(shadows::config::adapter_version(&dir.path().join("dist/index.js")), "0.81.1");
    assert_eq!(shadows::config::adapter_version(&dir.path().join("nope.js")), "unknown");
}
```

- [ ] **Step 3: Run** `cargo test --test containment --test harness_config` — expected: compile errors (`pipe_stdin`, `take_stdio`, `adapter_version`).
- [ ] **Step 4: Implement.** In `process::spawn`, `pipe_stdin` sets `Stdio::piped()` for stdin instead of `Stdio::null()` (the evidence rule "null the child's stdin unless `--input-format stream-json`" becomes "unless the protocol runs over it"; update that comment). `take_stdio` takes both halves. `tree_probe echo` reads stdin lines and prints them until EOF. `serve` gains `--node <path>` and `--adapter <path>`, both through `config::harness_path`; startup logs `harness.versions` with `adapter_version` and the existing Claude `--version` probe.
- [ ] **Step 5:** `cargo test`, `cargo clippy --all-targets -- -D warnings`, code map. Commit `feat(process): pipe a child's stdin; configure node and the pinned ACP adapter (spec §12.2)`.

