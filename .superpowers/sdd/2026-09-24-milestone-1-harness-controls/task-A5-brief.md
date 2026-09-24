### Task A5: Restart, docs, and the run with Mohammed

**Files:**
- Modify: `tests/recovery.rs` (one new test), `docs/status.md`, `docs/codebase/README.md`, `CLAUDE.md` (the `agent/` ownership row names `Connection` instead of `AgentHarness::start` only if that row names a type that no longer exists)
- Create: `docs/evidence/milestone1/PHASE_A_RUN.md`

- [ ] **Step 1: Failing test** in `tests/recovery.rs`:

```rust
#[tokio::test]
async fn after_a_restart_the_next_turn_resumes_the_recorded_session() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app_at(dir.path()).await;
    wait_terminal(&app, &start_prompt(&app, "hi").await).await;
    let session = app.storage.turn_context(&app.thread).await.unwrap().harness_session_id.unwrap();
    shut_down_app(app).await;
    let app = test_app_at(dir.path()).await; // same database, no live adapter
    wait_terminal(&app, &start_prompt(&app, "report").await).await;
    let r: Value = serde_json::from_str(&entries(&app).await.last().unwrap().body).unwrap();
    assert_eq!((r["how"].as_str(), r["session"].as_str()), (Some("resume"), Some(session.as_str())));
}
```

- [ ] **Step 2:** Run; fix whatever it finds; full gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (record the count).
- [ ] **Step 3: Run it with Mohammed.** `cargo build --release`; `shadows serve --node <node.exe> --adapter E:\Globalprojects\shadows\harness\claude\node_modules\@agentclientprotocol\claude-agent-acp\dist\index.js --harness %USERPROFILE%\.local\bin\claude.exe --debug`; start Vite; open a project; send a message; Stop a long reply; ask for a shell command (a `PermissionRefused` line appears); restart the daemon; send again and check Claude remembers. Record each step pass/fail, the versions, and the test count in `docs/evidence/milestone1/PHASE_A_RUN.md`; update `docs/status.md`.
- [ ] **Step 4:** Commit `docs(evidence): Phase A run over ACP` and stop: Phase B starts only after Mohammed accepts this run.

---

