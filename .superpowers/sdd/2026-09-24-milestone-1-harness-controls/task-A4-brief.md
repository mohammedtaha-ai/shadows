### Task A4: A turn over the connection

**Files:**
- Create: `src/planner/turn.rs` (watcher and stop — one arbitration, moved from `mod.rs`), `src/planner/entries.rs` (events → durable entries)
- Modify: `src/planner/mod.rs` (module doc, re-exports), `src/planner/spawn.rs` (open, register, prompt), `src/planner/handles.rs` (`LiveTurn` without a process handle), `src/planner/shutdown.rs` (`close_all` after stopping turns), `src/protocol/conversation.rs` (start opens the session; `HarnessStartFailed`), `src/protocol/sse.rs` (transient frame from `HarnessEvent::Chunk`), `src/error.rs`, `src/protocol/failure.rs`
- Delete: `ClaudeHarness` (`classify`, `render_content`, `to_process_spec`), the `AgentHarness` trait, `StreamItem`, `src/bin/fake_claude.rs`, `tests/harness_stream.rs`; `docs/codebase/README.md`'s `agent/` reference file becomes `src/agent/acp.rs`
- Test: `tests/planner_turn.rs` (rewrite on `fake_acp`); update `tests/recovery.rs`, `tests/shutdown.rs`, `tests/stream_frames.rs`, `tests/planner_isolation.rs`, `tests/operation_lifecycle.rs`, `tests/thread_session.rs`, `tests/disconnect.rs`, `tests/debug_log.rs`, `tests/serve_smoke.rs` to configure `--node <fake_acp>` and use its prompts

**Interfaces:**
- Consumes: A3 `Sessions`, `OpenSession`; A2 `TurnEnd`, `AcpError`, `HarnessEvent`.
- Produces:
  - `entries::Collector::new() -> Collector`; `Collector::push(&mut self, e: &HarnessEvent) -> Vec<Durable>`; `Collector::finish(&mut self) -> Vec<Durable>`; `Durable { Message(String), Tool(String), PermissionRefused(String) }` — a message is emitted when its `message_id` changes, a tool call or refusal arrives, or the turn finishes; empty text is never emitted. A tool call is tracked by id with its latest title and emitted once, when an update reports `status` `completed` or `failed`, or at `finish()` if it never did — under the last title it was given, since the first is generic (ACP_PROBE §8).
  - `LiveTurn { turn_end_seen: Arc<AtomicBool>, cancel_requested: Arc<AtomicBool>, span }` (no handle).
  - `StopOutcome { Cancelled, ResolvedByTurn, NotLive, TerminationFailed }` — `Cancelled`: stop terminated the adapter tree and wrote `Cancelled`; `ResolvedByTurn`: the watcher named the ending (the harness confirmed the cancel, or the turn had already ended).
  - `ErrorCode::HarnessStartFailed` → 502, `Failure::harness_start_failed(reason)`.
  - The transient bus becomes `broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>`; `protocol/sse.rs` renders `Chunk` as the existing `delta` frame and ignores the other variants until Phase B.

- [ ] **Step 1: Unit tests for the collector** (`#[cfg(test)]` in `entries.rs`):

```rust
#[test]
fn chunks_of_one_message_become_one_entry_and_a_tool_call_splits_messages() {
    let mut c = Collector::new();
    let mut out = Vec::new();
    for e in [
        chunk(Some("m1"), "hello "), chunk(Some("m1"), "there"),
        tool("t1", Some("Terminal"), Some("pending")),
        tool("t1", Some("Read notes.md"), None),
        tool("t1", None, Some("completed")),
        chunk(Some("m2"), "done"),
    ] { out.extend(c.push(&e)); }
    out.extend(c.finish());
    assert_eq!(out, [
        Durable::Message("hello there".into()),
        Durable::Tool("Read notes.md".into()),
        Durable::Message("done".into()),
    ]);
}

#[test]
fn a_tool_call_that_never_finished_is_emitted_at_the_end_under_its_last_title() {
    let mut c = Collector::new();
    assert!(c.push(&tool("t9", Some("Terminal"), Some("pending"))).is_empty());
    assert!(c.push(&tool("t9", Some("npm install"), None)).is_empty());
    assert_eq!(c.finish(), [Durable::Tool("npm install".into())]);
}

#[test]
fn a_changed_message_id_closes_the_previous_message() {
    let mut c = Collector::new();
    assert!(c.push(&chunk(Some("m1"), "a")).is_empty());
    assert_eq!(c.push(&chunk(Some("m2"), "b")), [Durable::Message("a".into())]);
    assert_eq!(c.finish(), [Durable::Message("b".into())]);
    assert!(c.finish().is_empty(), "finish twice emits nothing twice");
}
```

- [ ] **Step 2: Integration tests** in `tests/planner_turn.rs` (existing helpers `start_prompt`, `wait_terminal`, `entries`, `stop`; the app is configured with `SessionsConfig { cancel_wait: Duration::from_secs(1), ..Default::default() }`):

```rust
#[tokio::test]
async fn a_turn_streams_and_stores_one_entry_per_message() {
    let app = test_app().await;
    let op = start_prompt(&app, "two-messages").await;
    assert_eq!(wait_terminal(&app, &op).await.status, "Completed");
    let bodies: Vec<_> = entries(&app).await.into_iter().map(|e| e.body).collect();
    assert_eq!(bodies, ["two-messages", "first", "[tool: Read notes.md]", "second"], "the tool entry carries its real title, not \"Terminal\"");
}

#[tokio::test]
async fn the_first_turn_records_the_session_and_the_next_resumes_it() {
    let app = test_app().await;
    wait_terminal(&app, &start_prompt(&app, "hi").await).await;
    let session = app.storage.turn_context(&app.thread).await.unwrap().harness_session_id.unwrap();
    app.sessions.terminate(&app.thread).await.unwrap();
    let op = start_prompt(&app, "report").await;
    wait_terminal(&app, &op).await;
    let r: Value = serde_json::from_str(&entries(&app).await.last().unwrap().body).unwrap();
    assert_eq!((r["how"].as_str(), r["session"].as_str()), (Some("resume"), Some(session.as_str())));
}

#[tokio::test]
async fn stop_is_confirmed_by_the_harness_when_it_answers_cancelled() {
    let app = test_app().await;
    let op = start_prompt(&app, "hang").await;
    wait_for_entry_or_delta(&app).await;
    assert_eq!(stop(&app, &op).await, StopOutcome::ResolvedByTurn);
    assert_eq!(wait_terminal(&app, &op).await.status, "Cancelled");
    assert_eq!(app.sessions.live_count().await, 1, "the adapter survives a confirmed cancel");
}

#[tokio::test]
async fn stop_terminates_the_adapter_when_the_harness_does_not_confirm() {
    let app = test_app().await;
    let op = start_prompt(&app, "ignore-cancel").await;
    wait_for_entry_or_delta(&app).await;
    assert_eq!(stop(&app, &op).await, StopOutcome::Cancelled);
    assert_eq!(wait_terminal(&app, &op).await.status, "Cancelled");
    assert_eq!(app.sessions.live_count().await, 0);
    let next = start_prompt(&app, "hi").await; // a fresh adapter
    assert_eq!(wait_terminal(&app, &next).await.status, "Completed");
}

#[tokio::test]
async fn an_adapter_that_exits_mid_turn_fails_the_turn() {
    let app = test_app().await;
    let op = start_prompt(&app, "exit").await;
    let done = wait_terminal(&app, &op).await;
    assert_eq!(done.status, "Failed");
    assert!(done.failure_reason.unwrap().contains("the harness exited during the turn"));
}

#[tokio::test]
async fn a_refused_stop_reason_fails_the_turn_naming_it() {
    let app = test_app().await;
    let done = wait_terminal(&app, &start_prompt(&app, "refuse").await).await;
    assert_eq!(done.status, "Failed");
    assert!(done.failure_reason.unwrap().contains("max_tokens"));
}

#[tokio::test]
async fn a_permission_request_is_refused_and_recorded() {
    let app = test_app().await;
    wait_terminal(&app, &start_prompt(&app, "ask-permission").await).await;
    let refused: Vec<_> = entries(&app).await.into_iter().filter(|e| e.kind == "PermissionRefused").collect();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].body, "Run echo probe");
}

#[tokio::test]
async fn a_turn_whose_adapter_cannot_start_writes_nothing() {
    let app = test_app_with_node("C:/definitely/missing/node.exe").await;
    let (status, body) = http_start_raw(&app, json!({ "prompt": "hi" })).await;
    assert_eq!((status, body["code"].as_str()), (502, Some("HARNESS_START_FAILED")));
    assert!(entries(&app).await.is_empty());
}
```

- [ ] **Step 3: Run** `cargo test --test planner_turn` — expected: compile failure.
- [ ] **Step 4: Implement.**
  - `protocol/conversation.rs::start`: before `PlannerTurn::start`, `sessions.open(thread)`; `OpenError::Start` → `HarnessStartFailed` with nothing durable written. The request body stays `{ prompt }` in Phase A.
  - `spawn.rs`: keep TX #1 (`Pending`), register `LiveTurn` in `LiveHandles`, commit `Running`, take the thread's events, then hand to the watcher. No process is spawned per turn.
  - `turn.rs` watcher: on taking the thread's event receiver, first discard whatever is already queued (a `try_recv` loop, `trace!` each) — it arrived between turns and belongs to none. Then `tokio::select!` over the prompt future and the event receiver. Every event: `Chunk` goes to the bus as a transient delta; `Collector::push` output is appended as entries (`AgentMessage` for messages, `AgentMessage` with body `[tool: <title>]` for tools, `PermissionRefused` authored `Actor::system()` for refusals); `Usage`/`Options` are forwarded on the bus only (Phase B stores them). When the prompt answers: drain the receiver, `finish()`, set `turn_end_seen`, record the session on a first turn, give the events back, `touch`, then `claim` the registration. If claimed: `Ended` → `Completed`; `Cancelled` with `cancel_requested` → `Cancelled`; `Cancelled` without it → `Failed(Run, "the harness cancelled the turn on its own")`; `Refused(r)` → `Failed(Run, r)`; `Err(Closed)` → `Failed(Run, "the harness exited during the turn")` and `sessions.terminate`; `Err(Rpc(m))` → `Failed(Run, m)`. Not claimed → write nothing (stop owns it).
  - `turn.rs` `stop`: TX #1 `request_cancellation`; if not registered → `NotLive`; set `cancel_requested`, `connection.cancel(session)`; wait up to `cancel_wait` for the registration to be claimed by the watcher → `ResolvedByTurn`. On timeout: `claim` it; `None` (the watcher won the race) → `ResolvedByTurn`; `Some` → `sessions.terminate(thread)`; failure → put the registration back, `TerminationFailed`; success → `mark_operation_cancelled`, `Cancelled`. The watcher's prompt then fails `Closed`, finds nothing to claim, and writes nothing. Module doc keeps Milestone 0's arbitration explanation and adds the table from spec §12.3.
  - `shutdown.rs`: after every turn is stopped, `sessions.close_all()`; `Graceful` is still decided only by every operation being terminal.
  - Protocol routes that matched the old `StopOutcome` variants match the new ones: `Cancelled | ResolvedByTurn` answer as `Cancelled | TerminatedAfterTurnEnd | AlreadyExited` did.
- [ ] **Step 5: Migrate the other tests** listed under **Files** to `fake_acp` prompts (`hang` for a long turn, `exit` for a crash, `two-messages` for durable entries). A test that asserted a stream-json line class is deleted with a line in the report naming it and the test that now covers its behaviour. Add a fixture helper in `tests/fixtures/` when three or more files need it.
- [ ] **Step 6:** `cargo test` all green, clippy, code map, owners (`planner/turn.rs`: "decide and persist how a live Planner turn ends"; `planner/entries.rs`: "turning harness events into durable entries"). Commit `feat(planner): run a turn over the ACP connection; delete the stream-json path (spec §12.3)`.

