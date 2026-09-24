### Task A3: One live connection per thread

**Files:**
- Create: `src/planner/sessions.rs`
- Modify: `src/planner/mod.rs` (`pub use sessions::{Sessions, SessionsConfig, OpenSession}`), `src/cli/mod.rs` (build `Sessions` in the app state), `docs/codebase/README.md`
- Test: `tests/sessions.rs`

**Interfaces:**
- Consumes: A2 `Connection`, `SessionStart`, `Opened`, `HarnessEvent`, `ClaudeAdapter`.
- Produces:
  - `SessionsConfig { pub idle_after: Duration, pub cancel_wait: Duration }` with `Default` = 15 min / 10 s.
  - `Sessions::new(adapter: Arc<ClaudeAdapter>, storage: Storage, config: SessionsConfig) -> Arc<Sessions>`
  - `Sessions::open(&self, thread: &ThreadId) -> Result<OpenSession, OpenError>` — reuses a live connection; otherwise reads `turn_context` (directory, `harness_session_id`, later `fork_session_id`), spawns, `start_session(New | Resume(id) | Fork(src))`, then sets the mode option to `acceptEdits` (§12.1 Phase A); a spawn, initialize, session or mode failure is `OpenError::Start(String)` and leaves nothing running.
  - `OpenSession { pub session_id: String, pub how: &'static str, pub options: serde_json::Value, connection: Connection }` with `connection(&self) -> &Connection`.
  - `Sessions::take_events(&self, thread: &ThreadId) -> Option<mpsc::UnboundedReceiver<HarnessEvent>>` and `give_back_events(&self, thread, rx)` — a turn holds its thread's event stream for its duration.
  - `Sessions::terminate(&self, thread: &ThreadId) -> io::Result<()>` — terminates and reaps the adapter tree, drops the connection.
  - `Sessions::touch(&self, thread)` (a turn started or ended) and a reaper task closing connections idle longer than `idle_after`.
  - `Sessions::close_all(&self)` for shutdown.

- [ ] **Step 1: Failing tests** in `tests/sessions.rs` (fixture: storage with a project whose directory is a temp dir and a thread; `Sessions` over a `ClaudeAdapter` whose node is `fake_acp`):

```rust
#[tokio::test]
async fn opening_twice_reuses_one_adapter_and_starts_in_accept_edits() {
    let fx = fixture(SessionsConfig::default()).await;
    let a = fx.sessions.open(&fx.thread).await.unwrap();
    let b = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!(a.session_id, b.session_id);
    assert_eq!(fx.sessions.live_count().await, 1);
    let report = prompt_report(&a).await; // prompt "report", parse the JSON message
    assert_eq!(report["mode"], "acceptEdits");
    assert_eq!(report["how"], "new");
}

#[tokio::test]
async fn a_thread_with_a_recorded_session_resumes_it() {
    let fx = fixture(SessionsConfig::default()).await;
    fx.storage.record_harness_session(&fx.thread, "fake-77").await.unwrap();
    let s = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!((s.session_id.as_str(), s.how), ("fake-77", "resume"));
}

#[tokio::test]
async fn an_idle_connection_is_closed_and_the_next_opening_resumes() {
    let fx = fixture(SessionsConfig { idle_after: Duration::from_millis(300), ..Default::default() }).await;
    fx.storage.record_harness_session(&fx.thread, "fake-5").await.unwrap();
    fx.sessions.open(&fx.thread).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(fx.sessions.live_count().await, 0);
    let again = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!((again.session_id.as_str(), again.how), ("fake-5", "resume"));
}

#[tokio::test]
async fn a_dead_connection_is_replaced_on_the_next_opening() {
    let fx = fixture(SessionsConfig::default()).await;
    let s = fx.sessions.open(&fx.thread).await.unwrap();
    let _ = s.connection().prompt(&s.session_id, "exit").await;
    let again = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!(prompt_text(&again, "hi").await, "hello from fake_acp");
}

#[tokio::test]
async fn a_project_without_its_directory_does_not_start_an_adapter() {
    let fx = fixture(SessionsConfig::default()).await;
    std::fs::remove_dir_all(&fx.project_dir).unwrap();
    assert!(matches!(fx.sessions.open(&fx.thread).await, Err(OpenError::Start(_))));
    assert_eq!(fx.sessions.live_count().await, 0);
}
```

(`live_count` is `#[cfg(feature = "test-support")]`, as `LiveHandles::contains` is.)

- [ ] **Step 2: Run** — expected: compile failure.
- [ ] **Step 3: Implement.** One `tokio::sync::Mutex<HashMap<ThreadId, Live>>`; `Live { handle: ProcessHandle, connection: Connection, session_id, how, options, events: Option<UnboundedReceiver<HarnessEvent>>, last_used: Instant }`. A connection whose process `has_exited()` is removed and reopened. The reaper runs every `idle_after / 4` (at most 60 s), skips a thread whose events are taken (a turn is running), and terminates the rest past `idle_after`. The directory check is Milestone 0's `workspace()` moved here from `spawn.rs` (one owner). File doc: one job — "the live adapter connection each open thread holds".
- [ ] **Step 4:** tests green, clippy, code map, owner line. Commit `feat(planner): one live ACP connection per open thread with idle close (spec §12.2)`.

