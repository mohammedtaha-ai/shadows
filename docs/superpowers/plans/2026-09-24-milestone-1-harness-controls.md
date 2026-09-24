# Milestone 1 — Harness Controls over ACP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run the Planner's harness over the Agent Client Protocol through the pinned `claude-agent-acp` adapter (Phase A), then let the person pick the CLI per conversation and the model, mode and effort per message from what the harness offers, show context and account limits, and add copy and fork-from-last (Phase B).

**Architecture:** One adapter process per open conversation, started by `process/` under Job Object containment and driven through the official `agent-client-protocol` crate over its piped stdio. A turn is a `session/prompt` on that connection; its updates become deltas and durable entries, its answer names the ending, and Stop is `session/cancel` with tree termination as the fallback. Phase B reads models, efforts and modes from the session's `configOptions`, filters modes through Shadows' policy and the project's allowed set, and records every turn's settings in one idempotent transaction. Backend and web are separate agents against a hand-written `api/openapi.json` in Phase B.

**Tech Stack:** Rust (SQLx 0.9 on SQLite, axum 0.8, utoipa, `agent-client-protocol` 2.2.0 with `unstable_session_fork`, `tokio-util` compat), Node ≥ 22 running `@agentclientprotocol/claude-agent-acp` 0.81.1, React 19 + TypeScript (TanStack Query/Router, shadcn/ui on Base UI, Tailwind v4), Vitest.

**Spec:** `docs/superpowers/specs/2026-09-24-harness-controls-design.md` (§12). Every task's requirements include the spec sections it names.

## Global Constraints

- Harness kinds are the strings `claude-code` and `codex`. Codex is `available: false`.
- Shadows' mode policy: Claude Code allows exactly `acceptEdits` (default of every new conversation, never remembered) and `auto`. Codex allows none until decided.
- No list of Claude models or efforts exists anywhere in Shadows' code or tests of product behaviour; they come from the session's `configOptions` (§12.4). Tests use the fake agent's own made-up models.
- The adapter is `@agentclientprotocol/claude-agent-acp` pinned to `0.81.1` in `harness/claude/package.json`, lock file committed. Shadows never patches it.
- Rust crate `agent-client-protocol = { version = "=2.2.0", default-features = false, features = ["unstable_session_fork"] }`. `tokio::process` stays private to `process/` (CLAUDE.md single-ownership rule).
- Three absolute paths configure the harness, never resolved through `PATH`: `--node`, `--adapter`, `--harness` (the Claude executable, passed to the adapter as `CLAUDE_CODE_EXECUTABLE`).
- Idle adapter close: 15 minutes. Harness-confirmed cancel wait: 10 seconds. Both are fields of one config struct so tests can shorten them.
- New error codes, exactly: `HarnessUnavailable` 422, `HarnessStartFailed` 502, `SettingNotOffered` 422, `ModeNotAllowed` 403, `HarnessLocked` 409, `ThreadBusy` 409, `ForkPointNotSupported` 422.
- Nothing the harness did not report is estimated; it stays NULL and shows as unavailable. The context ring never shows a loading state.
- Files: 300 lines needs a stated reason, 500 splits (CLAUDE.md). `storage/mod.rs`, `protocol/`, `tests/storage_contract.rs` are accretion points: new storage capabilities go in new `storage/sqlite/<entity>.rs` files, new tests in new `tests/<topic>.rs` files.
- Every code change that moves a signature regenerates the code map in the same commit: `UPDATE_CODEMAP=1 cargo test --test codemap`. A new module file gets its one-job owner line in `docs/codebase/README.md`.
- Every subagent runs on opus, stated explicitly in the dispatch.
- Test helpers the tasks name (`test_app`, `start_prompt`, `wait_terminal`, `entries`, `http_start`, `post`, `patch`, `get_json`, …) live in `tests/fixtures/` for Rust and `web/src/test/` or `web/src/app/test-app.tsx` for TypeScript. Reuse the Milestone 0 helper of that name where one exists; the task that first uses a missing one adds it there, with the signature its tests imply.

## Review Focus

1. **Stop pressed while Claude is mid-reply, and the adapter ignores the cancel** — the person expects the turn to end within seconds and say Cancelled, and the next message to work on a fresh adapter. (Task A4 test `stop_terminates_the_adapter_when_the_harness_does_not_confirm`; Task A3 test `a_dead_connection_is_replaced_on_the_next_opening`.)
2. **The adapter process dies in the middle of a turn** (crash, killed from Task Manager) — the person expects that turn to show Failed with a reason, not stay Running forever. (Task A4 test `an_adapter_that_exits_mid_turn_fails_the_turn`.)
3. **A retried Send after a network drop** — one message and one turn, never two. The client resends the same `command_id`; the daemon answers a replay before any validation and without opening a session. (Task B3 test `a_replayed_turn_start_starts_nothing_and_returns_the_first_operation`; Task W2 test `a retried Send reuses the same command id`.)
4. **Switching model after choosing an effort the new model does not offer, or a remembered model Claude dropped** — the menu moves to a value the harness offers and the daemon refuses a stale pair. (Task B2 test `a_remembered_model_the_harness_no_longer_offers_is_dropped`; Task B3 test `an_effort_the_model_does_not_offer_is_refused`; Task W2 test `changing model resets an effort it does not offer`.)
5. **The conversation was idle past 15 minutes, or the daemon restarted, and the person sends again** — the adapter starts again, resumes the same session, and Claude remembers. (Task A3 test `an_idle_connection_is_closed_and_the_next_opening_resumes`; Task A5 test `after_a_restart_the_next_turn_resumes_the_recorded_session`.)

---

## Execution map

```text
Task 0 (Codex, throwaway probe) ─► evidence file
Phase A (one backend agent, branch milestone-1/harness-controls):
    A1 → A2 → A3 → A4 → A5 ─► run with Mohammed (gate for Phase B)
Phase B:
    Task 1 (controller: contract draft, commit)
        ├── backend worktree m1-backend:  B1 → B2 → B3 → B4 → B5 → B6
        └── web worktree m1-web:          W1 → W2 → W3 → W4
    Task I (merge, regenerate, reconcile, whole-branch review, run with Mohammed)
```

---

### Task 0: Measure the adapter (Codex, throwaway)

**Files:**
- Create: `docs/evidence/harness/ACP_PROBE.md`
- The probe itself lives outside the repository (a `%TEMP%` directory) and is not committed; the evidence file names where it ran and quotes the raw JSON-RPC lines it relies on.

The weekly limit is near its end: every prompt is a one-line prompt (`say ok`, `what word did I ask you to remember?`). If a turn is refused for the limit, stop, record how far the probe got, and do not retry.

- [ ] **Step 1: Install the pinned adapter** in a new `%TEMP%\acp-probe` directory (Mohammed runs this if the download is slow):

```bash
npm install @agentclientprotocol/claude-agent-acp@0.81.1 @agentclientprotocol/sdk --omit=optional
```

- [ ] **Step 2: Write `probe.ts`** from the adapter's `examples/simple-client.ts`: spawn `node node_modules/@agentclientprotocol/claude-agent-acp/dist/index.js` with `CLAUDE_CODE_EXECUTABLE=%USERPROFILE%\.local\bin\claude.exe`, log every JSON-RPC line in both directions with a timestamp to `wire.log`, and answer every `session/request_permission` with its `reject_once` option.
- [ ] **Step 3: Run the eight checks of spec §12.3's box**, in this order, each in its own session unless stated:
  1. `initialize` + `session/new` (cwd = the probe dir): record the full `configOptions` — model values and names, each model's `thought_level` values, `mode` values, current values. Then `session/set_config_option` to another model and record the new effort list.
  2. Prompt `say ok`: record every `session/update` kind, whether `agent_message_chunk` carries `messageId`, and the prompt's `stopReason`.
  3. Prompt `count slowly from 1 to 200, one number per line`; after the first chunk send `session/cancel`; record the time until the prompt answers and its `stopReason`.
  4. Prompt `remember the word amber`; kill the adapter process tree; start a new adapter; `session/resume` the same id; prompt `what word did I ask you to remember?`; record the answer.
  5. Look for a way to read the context breakdown that does not block: the adapter's `available_commands_update` (is there a `/context` command whose output arrives as a chunk?), and any `_meta` on `usage_update`. Record what exists and how long it takes after a finished turn.
  6. On the session from check 4: `session/fork` (requires the fork capability in `initialize`'s answer — record it); prompt the fork `what word did I ask you to remember?`; then prompt the source `say ok` and record that it still answers.
  7. From checks 2–6: every `usage_update` (fields, `_meta`, where the model is named) and every `_meta["_claude/rateLimit"]` payload.
  8. Prompt in the default mode after `session/set_config_option` mode=`acceptEdits`: `run the shell command: echo probe`. Record the `session/request_permission` params (options with `kind`s, `toolCall.title`) and what Claude says after the rejection.
- [ ] **Step 4: Write the evidence file** — one section per check: the command, the raw lines (trimmed), the finding; the adapter version, `claude --version`, Node version, date. Commit:

```bash
git add docs/evidence/harness/ACP_PROBE.md
git commit -m "docs(evidence): measure the claude-agent-acp adapter for Milestone 1"
```

**Stop and bring it to Mohammed** if check 2 finds no `messageId`, check 3 finds no `cancelled` answer, or check 4 finds the resumed session does not remember: §12.3 changes before Phase A starts.

---

## Phase A — the connection (one backend agent)

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
  - `pub type ChildIn = tokio::process::ChildStdin; pub type ChildOut = tokio::process::ChildStdout;` in `process/`.
  - `ProcessHandle::take_stdio(&mut self) -> Option<(ChildIn, ChildOut)>` — `Some` once, for a spec with `pipe_stdin && capture_stdout`.

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
    let (mut stdin, stdout) = handle.take_stdio().expect("stdio taken once");
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

### Task A2: The ACP connection and the fake agent

**Files:**
- Create: `src/agent/acp.rs` (the connection), `src/agent/events.rs` (what the connection reports), `src/bin/fake_acp.rs`
- Modify: `src/agent/mod.rs` (`pub mod acp; pub mod events;`), `src/agent/claude.rs` (adds `ClaudeAdapter` beside the old harness; the old one goes in A4, with its last caller), `docs/codebase/README.md` (owners)
- Test: `tests/acp_connection.rs`

The stream-json path is not touched here: `planner/` still runs on it until A4 replaces it, so every commit of this task keeps `cargo test` green.

**Interfaces:**
- Produces:
  - `agent::claude::ClaudeAdapter { node: PathBuf, adapter: PathBuf, agent: PathBuf, pub adapter_version: String, pub agent_version: String }`, `ClaudeAdapter::process_spec(&self, cwd: &Path) -> ProcessSpec` — executable `node`, args `[adapter]`, env `[("CLAUDE_CODE_EXECUTABLE", agent)]`, `capture_stdout: true`, `pipe_stdin: true`.
  - `agent::events::HarnessEvent`:
    ```rust
    pub enum HarnessEvent {
        /// Transient text of an agent message; `message_id` groups chunks into one message.
        Chunk { message_id: Option<String>, text: String },
        /// A tool call started; its title is what the person sees.
        ToolCall { id: String, title: String },
        /// A permission request Shadows refused (spec §12.2).
        PermissionRefused { title: String },
        /// Context and, when the adapter forwarded it, the account's rate-limit report.
        Usage { used: u64, size: u64, model: Option<String>, rate_limit: Option<serde_json::Value> },
        /// The complete set of config options, raw; Phase B parses them.
        Options(serde_json::Value),
    }
    ```
  - `agent::acp::SessionStart { New, Resume(String), Fork(String) }`
  - `agent::acp::Opened { pub session_id: String, pub options: serde_json::Value }`
  - `agent::acp::Connection` (Clone): `open(handle: &mut ProcessHandle, events: mpsc::UnboundedSender<HarnessEvent>) -> Result<Connection, AcpError>` (takes stdio, spawns the connection task, sends `initialize`); `start_session(&self, cwd: &Path, how: SessionStart) -> Result<Opened, AcpError>`; `set_option(&self, session: &str, config_id: &str, value: &str) -> Result<serde_json::Value, AcpError>` (returns the complete option set); `prompt(&self, session: &str, text: &str) -> Result<TurnEnd, AcpError>`; `cancel(&self, session: &str)`.
  - `agent::acp::TurnEnd { Ended, Cancelled, Refused(String) }` — `end_turn` → `Ended`; `cancelled` → `Cancelled`; `max_tokens`, `max_turn_requests`, `refusal` → `Refused(<reason>)`.
  - `agent::acp::AcpError { Closed, Rpc(String) }` — `Closed` whenever the connection is gone (the process exited or was killed).

- [ ] **Step 1: The fake agent.** `src/bin/fake_acp.rs` is an ACP agent built on the crate's agent side (`Agent.builder().on_receive_request(..).connect_to(Stdio::new())`, as the crate's `examples/simple_agent.rs`). It ignores its first argument (so tests configure `--node <fake_acp> --adapter anything`). It advertises the fork capability, returns these config options on every session answer, and applies `session/set_config_option` to them:

```text
model:         fake-small (efforts low, high; default high) | fake-large (efforts low, high, max; default high)
thought_level: the current model's efforts
mode:          default | acceptEdits | auto | plan      (auto only while model = fake-large)
```

Run as `fake_acp --version` it prints `fake-claude-1` and exits, so tests can configure it as the Claude executable too and the existing version probe reads it. Its session id is `fake-<n>`; `session/resume` of an id it did not create in this process still succeeds (a new process resumes what an old one created) and `session/fork` returns `fork-of-<id>`. Prompts, by text:

| prompt | behaviour |
|---|---|
| anything not below | chunks `hello ` and `from fake_acp` under message id `m1`, then `end_turn` |
| `two-messages` | `first` under `m1`, a tool call titled `Read notes.md`, `second` under `m2`, `end_turn` |
| `report` | one message whose text is JSON `{ "cwd", "session", "how": "new"/"resume"/"fork", "model", "effort", "mode", "claude": $CLAUDE_CODE_EXECUTABLE }`, `end_turn` |
| `hang` | one chunk, then waits; answers `cancelled` when `session/cancel` arrives |
| `ignore-cancel` | one chunk, then waits forever, ignoring `session/cancel` |
| `exit` | one chunk, then the process exits with code 3 |
| `ask-permission` | sends `session/request_permission` (title `Run echo probe`, options `allow_once`, `reject_once`), then one message `permission: <chosen option kind>`, `end_turn` |
| `usage` | one message, then `usage_update { used: 1234, size: 200000 }` with `_meta: { "_claude/rateLimit": { "unifiedWindows": { "five_hour": { "utilization": 0.25, "resetsAt": 1790212200 }, "seven_day": { "utilization": 0.5, "resetsAt": 1790542800 } } }, "model": "fake-large-answering" }`, `end_turn` |
| `refuse` | `max_tokens` |

Its module doc lists this table, as `fake_claude`'s did.

- [ ] **Step 2: Failing tests** in `tests/acp_connection.rs` (helper `open_fake() -> (ProcessHandle, Connection, UnboundedReceiver<HarnessEvent>)` spawning `fake_acp` through `process::spawn` with `pipe_stdin: true`):

```rust
#[tokio::test]
async fn a_new_session_answers_its_options_and_a_prompt_streams_then_ends() {
    let (_h, c, mut ev) = open_fake().await;
    let opened = c.start_session(&tmp(), SessionStart::New).await.unwrap();
    assert!(opened.session_id.starts_with("fake-"));
    assert!(opened.options.to_string().contains("fake-large"));
    assert_eq!(c.prompt(&opened.session_id, "hi").await.unwrap(), TurnEnd::Ended);
    let chunks = drain(&mut ev);
    assert!(matches!(&chunks[0], HarnessEvent::Chunk { message_id: Some(m), text } if m == "m1" && text == "hello "));
}

#[tokio::test]
async fn a_cancelled_prompt_answers_cancelled() {
    let (_h, c, _ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    let c2 = c.clone();
    let s2 = s.clone();
    let turn = tokio::spawn(async move { c2.prompt(&s2, "hang").await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    c.cancel(&s);
    assert_eq!(turn.await.unwrap().unwrap(), TurnEnd::Cancelled);
}

#[tokio::test]
async fn a_permission_request_is_refused_and_reported() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    c.prompt(&s, "ask-permission").await.unwrap();
    let got = drain(&mut ev);
    assert!(got.iter().any(|e| matches!(e, HarnessEvent::PermissionRefused { title } if title == "Run echo probe")));
    assert!(got.iter().any(|e| matches!(e, HarnessEvent::Chunk { text, .. } if text == "permission: reject_once")));
}

#[tokio::test]
async fn setting_the_model_answers_the_new_effort_list() {
    let (_h, c, _ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    let all = c.set_option(&s, "model", "fake-small").await.unwrap();
    assert!(!all.to_string().contains("\"max\""), "fake-small offers no max");
}

#[tokio::test]
async fn a_process_that_exits_mid_prompt_is_closed() {
    let (_h, c, _ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    assert!(matches!(c.prompt(&s, "exit").await, Err(AcpError::Closed)));
}

#[tokio::test]
async fn usage_carries_context_model_and_rate_limit() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    c.prompt(&s, "usage").await.unwrap();
    let u = drain(&mut ev).into_iter().find_map(|e| match e {
        HarnessEvent::Usage { used, size, model, rate_limit } => Some((used, size, model, rate_limit)),
        _ => None,
    }).unwrap();
    assert_eq!((u.0, u.1, u.2.as_deref()), (1234, 200000, Some("fake-large-answering")));
    assert!(u.3.unwrap()["unifiedWindows"]["seven_day"].is_object());
}
```

(Where Task 0 found the adapter names the model somewhere other than `_meta.model`, the fake and `Usage.model` follow the evidence file, and this test with them.)

- [ ] **Step 3: Run** `cargo test --test acp_connection` — expected: compile failure.
- [ ] **Step 4: Implement `agent/acp.rs`.** The connection task runs the crate's client builder over the child's stdio:

```rust
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

let (stdin, stdout) = handle.take_stdio().ok_or(AcpError::Closed)?;
let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<ConnectionTo<Agent>>();
let notify = events.clone();
let refuse = events.clone();
tokio::spawn(async move {
    let _ = Client
        .builder()
        .on_receive_notification(
            async move |n: SessionNotification, _cx| { forward(&notify, n.update); Ok(()) },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |r: RequestPermissionRequest, responder, _cx| {
                let _ = refuse.send(HarnessEvent::PermissionRefused { title: title_of(&r) });
                responder.respond(reject(&r))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(
            ByteStreams::new(stdin.compat_write(), stdout.compat()),
            |cx: ConnectionTo<Agent>| async move {
                let _ = ready_tx.send(cx);
                std::future::pending::<()>().await;
                Ok(())
            },
        )
        .await;
});
let cx = ready_rx.await.map_err(|_| AcpError::Closed)?;
cx.send_request(InitializeRequest::new(ProtocolVersion::V1)).block_task().await.map_err(rpc)?;
```

`forward` maps `agent_message_chunk` (text blocks only) → `Chunk`, `tool_call` → `ToolCall`, `usage_update` → `Usage` (model and rate limit read from `_meta` as Task 0 recorded), `config_option_update` → `Options`; every other update is dropped with a `trace!`. `reject` selects the option whose `kind` is `reject_once`, else `reject_always`, else answers `Cancelled`. `start_session` sends `NewSessionRequest::new(cwd)`, `ResumeSessionRequest`, or `ForkSessionRequest` and returns the id and the answer's `configOptions` as JSON. Any send whose future fails because the connection ended is `AcpError::Closed`. Types are the crate's `schema::v1` types; read `examples/yolo_one_shot_client.rs` of the pinned version for their constructors. `agent/events.rs` owns only `HarnessEvent`; `agent/acp.rs` owns only the connection.

- [ ] **Step 5:** `cargo test --test acp_connection`, clippy, code map, owners (`agent/acp.rs`: "the ACP client connection to one adapter process"; `agent/events.rs`: "what a harness connection reports"). Commit `feat(agent): ACP client connection and the fake ACP agent (spec §12.2)`.

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

### Task A4: A turn over the connection

**Files:**
- Create: `src/planner/turn.rs` (watcher and stop — one arbitration, moved from `mod.rs`), `src/planner/entries.rs` (events → durable entries)
- Modify: `src/planner/mod.rs` (module doc, re-exports), `src/planner/spawn.rs` (open, register, prompt), `src/planner/handles.rs` (`LiveTurn` without a process handle), `src/planner/shutdown.rs` (`close_all` after stopping turns), `src/protocol/conversation.rs` (start opens the session; `HarnessStartFailed`), `src/protocol/sse.rs` (transient frame from `HarnessEvent::Chunk`), `src/error.rs`, `src/protocol/failure.rs`
- Delete: `ClaudeHarness` (`classify`, `render_content`, `to_process_spec`), the `AgentHarness` trait, `StreamItem`, `src/bin/fake_claude.rs`, `tests/harness_stream.rs`; `docs/codebase/README.md`'s `agent/` reference file becomes `src/agent/acp.rs`
- Test: `tests/planner_turn.rs` (rewrite on `fake_acp`); update `tests/recovery.rs`, `tests/shutdown.rs`, `tests/stream_frames.rs`, `tests/planner_isolation.rs`, `tests/operation_lifecycle.rs`, `tests/thread_session.rs`, `tests/disconnect.rs`, `tests/debug_log.rs`, `tests/serve_smoke.rs` to configure `--node <fake_acp>` and use its prompts

**Interfaces:**
- Consumes: A3 `Sessions`, `OpenSession`; A2 `TurnEnd`, `AcpError`, `HarnessEvent`.
- Produces:
  - `entries::Collector::new() -> Collector`; `Collector::push(&mut self, e: &HarnessEvent) -> Vec<Durable>`; `Collector::finish(&mut self) -> Vec<Durable>`; `Durable { Message(String), Tool(String), PermissionRefused(String) }` — a message is emitted when its `message_id` changes, a tool call or refusal arrives, or the turn finishes; empty text is never emitted.
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
        HarnessEvent::ToolCall { id: "t1".into(), title: "Read notes.md".into() },
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
    assert_eq!(bodies, ["two-messages", "first", "[tool: Read notes.md]", "second"]);
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

## Phase B — the controls

### Task 1: The contract draft (controller)

**Files:**
- Modify: `api/openapi.json` (by hand)

- [ ] **Step 1: Add schemas** under `components.schemas`, keys sorted, matching the existing style (`required` lists, `description` from the spec):
  - `Choice { id: string, label: string, description: string|null, enabled: boolean, reason: string|null }`
  - `SessionChoices { models: Choice[], efforts: Choice[], modes: Choice[], current: TurnSettings }` — `efforts` are the current model's; `modes` are after §12.4's filter, with modes the project does not allow present and `enabled: false`, `reason: "Not allowed in this project"`.
  - `TurnSettings { model: string, mode: string, effort: string }`
  - `LimitWindow { utilization: number, resets_at: integer }`
  - `AccountLimits { five_hour: LimitWindow|null, seven_day: LimitWindow|null, observed_at: string }`
  - `HarnessInfo { kind: string, label: string, available: boolean, reason: string|null, remembered: { model: string, effort: string }|null, limits: AccountLimits|null }`
  - `InvocationView { harness_kind, harness_version, agent_version, requested_model, requested_mode, requested_effort, observed_model: string|null, context_used: integer|null, context_window: integer|null }`
  - `StartTurn { command_id, prompt, model, mode, effort }` (replaces `{ prompt }`)
  - `UpdateThread { command_id, harness }`, `UpdateProject { command_id, allowed_modes: { [harness]: string[] } }`, `ForkThread { command_id, at_entry_id }`
  - `Project` gains `allowed_modes`; `PlanningThread` gains `harness: string`, `forked_from_thread: string|null`; `ThreadEntry` gains `operation_id: string|null` and its `kind` documents `PermissionRefused`; `Operation` gains `invocation: InvocationView|null`; `CreateThread` gains optional `harness`.
  - `ErrorCode` enum gains the seven codes in Global Constraints.
- [ ] **Step 2: Add paths:** `GET /api/harnesses` → `HarnessInfo[]`; `POST /api/threads/{id}/session` → 200 `SessionChoices` (422 `HARNESS_UNAVAILABLE`, 502 `HARNESS_START_FAILED`); `PATCH /api/threads/{id}` → `PlanningThread` (409 `HARNESS_LOCKED`); `PATCH /api/projects/{id}` → `Project`; `POST /api/threads/{id}/fork` → 201 `PlanningThread` (409 `THREAD_BUSY`, 422 `FORK_POINT_NOT_SUPPORTED`); on `POST /api/threads/{id}/turns` add 403 `MODE_NOT_ALLOWED`, 409 `THREAD_BUSY`, 409 `COMMAND_CONFLICT`, 422 `SETTING_NOT_OFFERED`/`HARNESS_UNAVAILABLE`, 502 `HARNESS_START_FAILED`.
- [ ] **Step 3: Extend the `/api/subscribe` description** with two transient frames: `usage` `{ "thread_id": string, "context_used": integer|null, "context_window": integer|null, "limits": AccountLimits|null }` and `options` `{ "thread_id": string, "choices": SessionChoices }`; and that a `durable` frame of kind `OperationCompleted` carries `payload.invocation: InvocationView`.
- [ ] **Step 4: Validate and commit.** `npx --prefix web openapi-typescript api/openapi.json -o NUL` must succeed. Then:

```bash
git add api/openapi.json
git commit -m "docs(api): draft the Milestone 1 Phase B contract by hand (spec §12.10)"
```

`cargo test --test openapi` now fails on purpose until B6 regenerates it; neither track treats that as a regression.

---

## Backend track (worktree `m1-backend`)

### Task B1: Migration, mode policy, storage shapes

**Files:**
- Create: `migrations/0005_harness_controls.sql`, `src/agent/policy.rs`
- Modify: `src/agent/mod.rs`, `src/thread/mod.rs`, `src/project/mod.rs`, `src/storage/sqlite/thread.rs`, `src/storage/sqlite/project.rs`, `src/storage/sqlite/mod.rs` (new `StorageError` variants), `docs/codebase/README.md`
- Test: `tests/harness_schema.rs`

**Interfaces:**
- Produces:
  - `agent::policy::{CLAUDE_CODE, CODEX}: &str`; `policy::allowed_modes(kind: &str) -> &'static [&'static str]` (`["acceptEdits", "auto"]` / `[]`); `policy::default_mode(kind) -> Option<&'static str>`; `policy::is_known(kind) -> bool`; `policy::default_modes() -> BTreeMap<String, Vec<String>>` (`{"claude-code": ["acceptEdits", "auto"]}`).
  - `PlanningThread { .., harness: String, forked_from_thread: Option<ThreadId> }`
  - `ThreadEntry { .., operation_id: Option<OperationId> }`, `NewThreadEntry { .., operation_id: Option<&'a OperationId> }`
  - `Project { .., allowed_modes: BTreeMap<String, Vec<String>> }`
  - `TurnContext { project_directory, harness_session_id, harness: String, project_id: ProjectId, fork_session_id: Option<String> }`
  - `Storage::create_planning_thread(&self, ctx: &CommandContext, project_id: &ProjectId, title: &str, harness: &str) -> Result<PlanningThread, StorageError>`
  - `Storage::create_project(&self, ctx, slug, name, directory, default_modes: &BTreeMap<String, Vec<String>>) -> Result<Project, StorageError>`
  - `Storage::set_thread_harness(&self, ctx: &CommandContext, thread: &ThreadId, harness: &str) -> Result<PlanningThread, StorageError>` — idempotent command; a replay answers the thread as it now stands
  - `Storage::set_project_modes(&self, ctx: &CommandContext, project: &ProjectId, modes: &BTreeMap<String, Vec<String>>) -> Result<Project, StorageError>`
  - `StorageError::{HarnessLocked, ThreadBusy, ForkPointNotSupported}`

- [ ] **Step 1: Write the failing tests** in `tests/harness_schema.rs` (fixture copied from `tests/thread_session.rs`'s `fixture()`; `create_project` gains `&policy::default_modes()`):

```rust
#[test]
fn claude_policy_is_accept_edits_and_auto_and_codex_has_none() {
    assert_eq!(policy::allowed_modes(policy::CLAUDE_CODE), ["acceptEdits", "auto"]);
    assert_eq!(policy::default_mode(policy::CLAUDE_CODE), Some("acceptEdits"));
    assert!(policy::allowed_modes(policy::CODEX).is_empty());
    assert!(!policy::is_known("gemini"));
}

#[tokio::test]
async fn a_thread_defaults_to_claude_code_and_its_harness_locks_at_its_first_operation() {
    let (fx, thread) = fixture().await;
    let t = fx.storage.set_thread_harness(&ctx("h1", "thread.harness"), &thread, "codex").await.unwrap();
    assert_eq!(t.harness, "codex");
    fx.storage.set_thread_harness(&ctx("h2", "thread.harness"), &thread, "claude-code").await.unwrap();
    fx.storage.create_pending_operation(&thread, &fx.runtime.instance_id).await.unwrap();
    let locked = fx.storage.set_thread_harness(&ctx("h3", "thread.harness"), &thread, "codex").await;
    assert!(matches!(locked, Err(StorageError::HarnessLocked)));
    let replay = fx.storage.set_thread_harness(&ctx("h2", "thread.harness"), &thread, "claude-code").await.unwrap();
    assert_eq!(replay.harness, "claude-code");
}

#[tokio::test]
async fn the_lock_holds_below_the_application() {
    let (fx, thread) = fixture().await;
    fx.storage.create_pending_operation(&thread, &fx.runtime.instance_id).await.unwrap();
    let raw = sqlx::query("UPDATE planning_thread SET harness_kind = 'codex' WHERE id = ?")
        .bind(thread.as_str())
        .execute(fx.storage.reader())
        .await;
    assert!(raw.is_err(), "the trigger must refuse a raw update too");
}

#[tokio::test]
async fn a_project_is_created_with_the_policy_modes_and_they_can_be_replaced() {
    let (fx, _) = fixture().await;
    let p = &fx.project;
    assert_eq!(p.allowed_modes["claude-code"], vec!["acceptEdits", "auto"]);
    let only: BTreeMap<_, _> = [("claude-code".to_string(), vec!["acceptEdits".to_string()])].into();
    let p2 = fx.storage.set_project_modes(&ctx("m1", "project.modes"), &p.id, &only).await.unwrap();
    assert_eq!(p2.allowed_modes["claude-code"], vec!["acceptEdits"]);
}

#[tokio::test]
async fn an_entry_can_name_its_operation_and_old_entries_name_none() {
    let (fx, thread) = fixture().await;
    let op = fx.storage.create_pending_operation(&thread, &fx.runtime.instance_id).await.unwrap();
    let e = fx.storage.append_thread_entry(&thread, NewThreadEntry {
        kind: "AgentMessage", author: Actor::system(), body: "x", refs: &[], operation_id: Some(&op),
    }).await.unwrap();
    assert_eq!(e.operation_id.as_ref(), Some(&op));
    let e2 = fx.storage.append_thread_entry(&thread, NewThreadEntry {
        kind: "UserMessage", author: Actor::user("local"), body: "y", refs: &[], operation_id: None,
    }).await.unwrap();
    assert!(e2.operation_id.is_none());
}
```

- [ ] **Step 2: Run** `cargo test --test harness_schema` — expected: compile errors.
- [ ] **Step 3: Write `migrations/0005_harness_controls.sql`:**

```sql
ALTER TABLE planning_thread ADD COLUMN harness_kind TEXT NOT NULL DEFAULT 'claude-code';
ALTER TABLE planning_thread ADD COLUMN forked_from_thread TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT;
ALTER TABLE planning_thread ADD COLUMN forked_from_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE planning_thread ADD COLUMN fork_session_id TEXT NULL;

CREATE TRIGGER planning_thread_fork_all_or_none
BEFORE INSERT ON planning_thread
WHEN NOT ((NEW.forked_from_thread IS NULL AND NEW.forked_from_entry IS NULL AND NEW.fork_session_id IS NULL)
       OR (NEW.forked_from_thread IS NOT NULL AND NEW.forked_from_entry IS NOT NULL AND NEW.fork_session_id IS NOT NULL))
BEGIN SELECT RAISE(ABORT, 'fork_columns_all_or_none'); END;

CREATE TRIGGER planning_thread_fork_immutable
BEFORE UPDATE OF forked_from_thread, forked_from_entry, fork_session_id ON planning_thread
BEGIN SELECT RAISE(ABORT, 'fork_columns_immutable'); END;

CREATE TRIGGER planning_thread_harness_locked
BEFORE UPDATE OF harness_kind ON planning_thread
WHEN NEW.harness_kind <> OLD.harness_kind
 AND EXISTS (SELECT 1 FROM operation WHERE thread_id = OLD.id)
BEGIN SELECT RAISE(ABORT, 'harness_locked'); END;

ALTER TABLE thread_entry ADD COLUMN operation_id TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT;

CREATE TABLE project_mode (
    project_id   TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    harness_kind TEXT NOT NULL,
    mode_id      TEXT NOT NULL,
    PRIMARY KEY (project_id, harness_kind, mode_id)
);
INSERT INTO project_mode (project_id, harness_kind, mode_id)
    SELECT id, 'claude-code', 'acceptEdits' FROM project
    UNION ALL SELECT id, 'claude-code', 'auto' FROM project;

CREATE TABLE agent_invocation (
    id                TEXT PRIMARY KEY,
    operation_id      TEXT NOT NULL UNIQUE REFERENCES operation(id) ON DELETE RESTRICT,
    role              TEXT NOT NULL,
    harness_kind      TEXT NOT NULL,
    harness_path      TEXT NOT NULL,
    harness_version   TEXT NOT NULL,
    agent_path        TEXT NOT NULL,
    agent_version     TEXT NOT NULL,
    requested_model   TEXT NOT NULL,
    requested_mode    TEXT NOT NULL,
    requested_effort  TEXT NOT NULL,
    profile_json      TEXT NOT NULL DEFAULT '{}',
    native_session_id TEXT NULL,
    observed_model    TEXT NULL,
    context_used      INTEGER NULL,
    context_window    INTEGER NULL,
    created_at        TEXT NOT NULL
);
CREATE INDEX agent_invocation_by_operation ON agent_invocation(operation_id, created_at, id);

CREATE TRIGGER agent_invocation_requested_immutable
BEFORE UPDATE OF requested_model, requested_mode, requested_effort,
                 harness_path, harness_version, agent_path, agent_version ON agent_invocation
BEGIN SELECT RAISE(ABORT, 'invocation_requested_immutable'); END;

CREATE TABLE harness_preference (
    harness_kind TEXT PRIMARY KEY,
    model        TEXT NOT NULL,
    effort       TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE harness_limit (
    harness_kind          TEXT PRIMARY KEY,
    five_hour_utilization REAL NULL,
    five_hour_resets_at   INTEGER NULL,
    seven_day_utilization REAL NULL,
    seven_day_resets_at   INTEGER NULL,
    observed_at           TEXT NOT NULL
);
```

- [ ] **Step 4: Implement** the fields and methods in `thread.rs` and `project.rs`. `allowed_modes` is a **set**: read it `ORDER BY harness_kind, mode_id` (an explicit key; never `rowid`), and the field's doc comment says list order carries no meaning. `set_thread_harness` checks `EXISTS operation` first and returns `HarnessLocked` without relying on the trigger, and also maps a SQLite error containing `harness_locked`. `set_project_modes` is an idempotent command (`classify`/`record_command` exactly as `create_project` uses them) that deletes and reinserts the project's rows for the harnesses named. `agent/policy.rs` doc: "the modes Shadows allows per harness (spec §12.4) — a decision of Shadows, not a list of what the harness offers".
- [ ] **Step 5: Fix every existing caller** of `create_planning_thread`, `create_project`, `NewThreadEntry` (pass `"claude-code"`, `policy::default_modes()`, `operation_id: None`). `cargo test` all green.
- [ ] **Step 6:** code map, owners, commit:

```bash
UPDATE_CODEMAP=1 cargo test --test codemap
git add migrations/0005_harness_controls.sql src tests/harness_schema.rs docs/codebase
git commit -m "feat(storage): migration 0005 and the mode policy for harness controls (spec §12.4-§12.7)"
```

### Task B2: Choices from the session, `POST /session`, `GET /api/harnesses`

**Files:**
- Create: `src/agent/choices.rs`, `src/protocol/harness.rs`, `src/storage/sqlite/harness.rs`
- Modify: `src/planner/sessions.rs` (keep the latest options per thread; apply remembered settings at open; forward `Options` changes), `src/protocol/mod.rs`, `src/protocol/openapi.rs`, `src/protocol/sse.rs` (`options` frame), `docs/codebase/README.md`
- Test: `tests/harness_choices.rs`, unit tests in `choices.rs`

**Interfaces:**
- Consumes: B1 `policy`, `Project.allowed_modes`, `TurnContext.harness`; A3 `Sessions`, `OpenSession.options`.
- Produces:
  - `agent::TurnSettings { pub model: String, pub mode: String, pub effort: String }` (Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)
  - `agent::choices::Offered { pub models: Vec<Choice>, pub efforts: Vec<Choice>, pub modes: Vec<Choice>, pub current: TurnSettings, pub ids: OptionIds }` where `OptionIds { model: String, effort: Option<String>, mode: String }` are the ACP `configId`s found by category.
  - `choices::parse(options: &serde_json::Value) -> Result<Offered, String>` — reads `category` `model`/`thought_level`/`mode`, `options` flat or grouped, keeps the harness's order.
  - `choices::for_client(offered: &Offered, harness: &str, allowed: &[String]) -> SessionChoices` — drops modes outside `policy::allowed_modes(harness)`; marks the rest `enabled: false, reason: Some("Not allowed in this project")` when not in `allowed`.
  - `choices::refusal(offered: &Offered, harness: &str, s: &TurnSettings) -> Option<(&'static str, String)>` — `("model", id)`, `("effort", id)`, or `("mode", id)` for the first value not on offer or outside the policy.
  - `Storage::remembered_settings(&self, kind: &str) -> Result<Option<(String, String)>, StorageError>`, `Storage::latest_limits(&self, kind: &str) -> Result<Option<AccountLimits>, StorageError>`, `pub(in crate::storage) async fn remember_settings(conn, kind, &TurnSettings, ts)`, `Storage::record_limits(&self, kind: &str, limits: &AccountLimits) -> Result<(), StorageError>` (upsert, latest wins).
  - `AccountLimits { five_hour: Option<LimitWindow>, seven_day: Option<LimitWindow>, observed_at: String }`, `LimitWindow { utilization: f64, resets_at: i64 }` in `agent::events` (B4 fills them).
  - `Sessions::offered(&self, thread) -> Option<Offered>` (the latest).
  - Routes `POST /api/threads/{id}/session` → `SessionChoices`, `GET /api/harnesses` → `Vec<HarnessInfo>`.

- [ ] **Step 1: Unit tests** in `choices.rs` with the fake agent's options as a JSON literal copied from `fake_acp`:

```rust
#[test]
fn parse_reads_models_efforts_and_modes_by_category_in_the_harness_order() {
    let o = parse(&fake_options("fake-large")).unwrap();
    assert_eq!(ids(&o.models), ["fake-small", "fake-large"]);
    assert_eq!(ids(&o.efforts), ["low", "high", "max"]);
    assert_eq!(ids(&o.modes), ["default", "acceptEdits", "auto", "plan"]);
    assert_eq!(o.current.model, "fake-large");
}

#[test]
fn only_policy_modes_reach_the_client_and_disallowed_ones_say_why() {
    let o = parse(&fake_options("fake-large")).unwrap();
    let c = for_client(&o, "claude-code", &["acceptEdits".to_string()]);
    assert_eq!(ids(&c.modes), ["acceptEdits", "auto"]);
    let auto = c.modes.iter().find(|m| m.id == "auto").unwrap();
    assert_eq!((auto.enabled, auto.reason.as_deref()), (false, Some("Not allowed in this project")));
}

#[test]
fn a_mode_the_harness_withholds_for_this_model_is_not_offered() {
    let o = parse(&fake_options("fake-small")).unwrap(); // fake withholds auto for fake-small
    let c = for_client(&o, "claude-code", &["acceptEdits".into(), "auto".into()]);
    assert_eq!(ids(&c.modes), ["acceptEdits"]);
}

#[test]
fn refusal_names_what_is_not_offered() {
    let o = parse(&fake_options("fake-small")).unwrap();
    let ok = TurnSettings { model: "fake-small".into(), mode: "acceptEdits".into(), effort: "high".into() };
    assert_eq!(refusal(&o, "claude-code", &ok), None);
    assert_eq!(refusal(&o, "claude-code", &TurnSettings { effort: "max".into(), ..ok.clone() }).unwrap().0, "effort");
    assert_eq!(refusal(&o, "claude-code", &TurnSettings { mode: "plan".into(), ..ok.clone() }).unwrap().0, "mode");
    assert_eq!(refusal(&o, "claude-code", &TurnSettings { model: "gpt".into(), ..ok }).unwrap().0, "model");
}
```

`refusal` checks the effort against the chosen model's efforts: when `s.model` differs from `o.current.model`, the caller (B3) sets the model first and re-parses, so `refusal` is always called on the options for `s.model`.

- [ ] **Step 2: Integration tests** in `tests/harness_choices.rs`:

```rust
#[tokio::test]
async fn opening_a_session_answers_the_harness_choices_after_the_policy() {
    let app = test_app().await;
    let (s, c) = post(&app, &format!("/api/threads/{}/session", app.thread), json!({})).await;
    assert_eq!(s, 200);
    assert_eq!(names(&c["models"]), ["fake-small", "fake-large"]);
    assert_eq!(names(&c["modes"]), ["acceptEdits", "auto"]);
    assert_eq!(c["current"]["mode"], "acceptEdits");
}

#[tokio::test]
async fn harnesses_lists_claude_runnable_and_codex_not() {
    let app = test_app().await;
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!((h[0]["kind"].as_str(), h[0]["available"].as_bool()), (Some("claude-code"), Some(true)));
    assert!(h[0]["remembered"].is_null());
    assert_eq!((h[1]["kind"].as_str(), h[1]["available"].as_bool()), (Some("codex"), Some(false)));
    assert!(h[1]["reason"].is_string());
}

#[tokio::test]
async fn a_new_session_starts_at_the_remembered_model_and_effort() {
    let app = test_app().await;
    app.storage.remember_for_test("claude-code", "fake-small", "low").await; // test-support helper over remember_settings
    let (_, c) = post(&app, &format!("/api/threads/{}/session", app.thread), json!({})).await;
    assert_eq!((c["current"]["model"].as_str(), c["current"]["effort"].as_str()), (Some("fake-small"), Some("low")));
}

#[tokio::test]
async fn a_remembered_model_the_harness_no_longer_offers_is_dropped() {
    let app = test_app().await;
    app.storage.remember_for_test("claude-code", "retired-model", "high").await;
    let (s, c) = post(&app, &format!("/api/threads/{}/session", app.thread), json!({})).await;
    assert_eq!(s, 200);
    assert_eq!(c["current"]["model"], "fake-large"); // the fake's own default
}

#[tokio::test]
async fn a_codex_thread_cannot_open_a_session() {
    let app = test_app().await;
    let t = create_thread(&app, json!({ "command_id": "c9", "title": "x", "harness": "codex" })).await;
    let (s, b) = post(&app, &format!("/api/threads/{}/session", t["id"].as_str().unwrap()), json!({})).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("HARNESS_UNAVAILABLE")));
}
```

(The last test needs B5's `harness` on create; if B5 has not landed, create the thread through storage with `"codex"` directly.)

- [ ] **Step 3: Run** — expected: compile failure / 404.
- [ ] **Step 4: Implement.** `Sessions::open` applies remembered settings after the mode: `set_option(ids.model, m)` then `set_option(ids.effort, e)` only when each is on offer (re-parse after the model); a value not on offer is skipped with an `info!` naming it. Every `set_option` answer and every `HarnessEvent::Options` replaces the thread's stored `Offered` and publishes an `options` SSE frame for that thread. `GET /api/harnesses`: Claude Code is `available` when node, adapter and Claude paths were configured (they are required flags, so always in a running daemon); Codex `available: false, reason: "Coming later"`. `remembered` and `limits` from storage.
- [ ] **Step 5:** `cargo test`, clippy, code map, owners (`agent/choices.rs`: "reading the harness's offered choices"; `protocol/harness.rs`: "the harness and session-choice routes"; `storage/sqlite/harness.rs`: "per-harness remembered settings and limits"). Commit `feat(agent): choices from the ACP session, POST /session and GET /api/harnesses (spec §12.4)`.

### Task B3: Starting a turn as one command

**Files:**
- Create: `src/storage/sqlite/turn.rs`, `tests/turn_command.rs`
- Modify: `src/protocol/conversation.rs` (`StartTurn`, `start`), `src/planner/spawn.rs` (takes a committed operation; sets the session's options before the prompt), `src/agent/mod.rs` (`AgentInvocation` carries `TurnSettings`), `src/error.rs` + `src/protocol/failure.rs` (codes), `src/storage/sqlite/operation.rs`, `src/planner/turn.rs` (agent entries carry `operation_id`)
- Test: `tests/turn_command.rs`; update every caller of `PlannerTurn::start`

**Interfaces:**
- Consumes: B1 (`TurnContext`, `NewThreadEntry.operation_id`, `policy`), B2 (`TurnSettings`, `choices::refusal`, `remember_settings`, `Sessions::offered`).
- Produces:
  - `pub struct NewTurn<'a> { pub thread_id: &'a ThreadId, pub runtime: &'a RuntimeInstanceId, pub prompt: &'a str, pub role: &'a str, pub harness_kind: &'a str, pub harness_path: &'a str, pub harness_version: &'a str, pub agent_path: &'a str, pub agent_version: &'a str, pub settings: &'a TurnSettings }`
  - `pub struct StartedTurn { pub operation_id: OperationId, pub entry_id: ThreadEntryId, pub replayed: bool }`
  - `Storage::start_turn(&self, ctx: &CommandContext, turn: NewTurn<'_>) -> Result<StartedTurn, StorageError>` — `ThreadBusy` when a non-terminal operation exists; one transaction as §12.7; a replay inside the transaction returns the recorded ids with `replayed: true`.
  - `Storage::replayed_turn(&self, ctx: &CommandContext) -> Result<Option<StartedTurn>, StorageError>` — read-only; `CommandConflict` on a fingerprint mismatch.
  - `PlannerTurnRequest { thread_id: ThreadId, operation_id: OperationId, prompt: String, settings: TurnSettings }` — `PlannerTurn::start` no longer creates the operation.
  - `ErrorCode::{HarnessUnavailable, SettingNotOffered, ModeNotAllowed, HarnessLocked, ThreadBusy, ForkPointNotSupported}` with the statuses in Global Constraints.
  - `LiveHandles::close_for_test(&self)` behind `test-support`, beside `contains`, calling the `pub(crate)` `close` that shutdown uses first.

- [ ] **Step 1: Failing tests** in `tests/turn_command.rs` (storage helper `start(fx, cmd_id, prompt, settings)`; HTTP helper `http_start(app, thread, body) -> (StatusCode, Value)`; `open(app)` posts `/session`):

```rust
fn small_edits() -> TurnSettings {
    TurnSettings { model: "fake-small".into(), mode: "acceptEdits".into(), effort: "high".into() }
}

#[tokio::test]
async fn one_command_writes_entry_operation_invocation_and_record_together() {
    let (fx, thread) = fixture().await;
    let started = start(&fx, "t1", "hello", &small_edits()).await.unwrap();
    let entries = fx.storage.list_thread_entries(&thread).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].operation_id.as_ref(), Some(&started.operation_id));
    let ops = fx.storage.list_operations_for_thread(&thread).await.unwrap();
    let inv = ops[0].invocation.as_ref().expect("invocation written with Pending");
    assert_eq!((inv.requested_model.as_str(), inv.requested_mode.as_str(), inv.requested_effort.as_str()),
               ("fake-small", "acceptEdits", "high"));
    assert_eq!((inv.harness_version.as_str(), inv.agent_version.as_str()), ("fake-adapter-1", "fake-claude-1"));
    assert!(inv.observed_model.is_none());
}

#[tokio::test]
async fn a_replayed_turn_start_starts_nothing_and_returns_the_first_operation() {
    let (fx, thread) = fixture().await;
    let first = start(&fx, "t1", "hello", &small_edits()).await.unwrap();
    fx.storage.mark_operation_failed(&first.operation_id, FailureStage::Prepare, "test").await.unwrap();
    let again = start(&fx, "t1", "hello", &small_edits()).await.unwrap();
    assert!(again.replayed);
    assert_eq!(again.operation_id, first.operation_id);
    assert_eq!(fx.storage.list_thread_entries(&thread).await.unwrap().len(), 1);
    assert_eq!(fx.storage.list_operations_for_thread(&thread).await.unwrap().len(), 1);
}

#[tokio::test]
async fn the_same_command_id_with_another_body_is_a_conflict() {
    let (fx, _) = fixture().await;
    start(&fx, "t1", "hello", &small_edits()).await.unwrap();
    let other = start(&fx, "t1", "hello", &TurnSettings { model: "fake-large".into(), ..small_edits() }).await;
    assert!(matches!(other, Err(StorageError::CommandConflict)));
}

#[tokio::test]
async fn a_second_turn_while_one_is_running_is_thread_busy() {
    let (fx, _) = fixture().await;
    start(&fx, "t1", "hello", &small_edits()).await.unwrap();
    assert!(matches!(start(&fx, "t2", "again", &small_edits()).await, Err(StorageError::ThreadBusy)));
}

#[tokio::test]
async fn a_mode_the_project_no_longer_allows_is_refused() {
    let app = test_app().await;
    patch_modes(&app, &["acceptEdits"]).await;
    let (status, body) = http_start(&app, &app.thread, json!({
        "command_id": "t1", "prompt": "hi", "model": "fake-large", "mode": "auto", "effort": "high" })).await;
    assert_eq!((status.as_u16(), body["code"].as_str()), (403, Some("MODE_NOT_ALLOWED")));
    assert!(app.storage.list_thread_entries(&app.thread).await.unwrap().is_empty(), "nothing durable");
}

#[tokio::test]
async fn an_effort_the_model_does_not_offer_is_refused() {
    let app = test_app().await;
    let (status, body) = http_start(&app, &app.thread, json!({
        "command_id": "t1", "prompt": "hi", "model": "fake-small", "mode": "acceptEdits", "effort": "max" })).await;
    assert_eq!((status.as_u16(), body["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    assert!(app.storage.list_thread_entries(&app.thread).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_replay_is_answered_even_after_the_mode_was_disallowed() {
    let app = test_app().await;
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "fake-large", "mode": "auto", "effort": "high" });
    let (s1, b1) = http_start(&app, &app.thread, body.clone()).await;
    assert_eq!(s1.as_u16(), 202);
    wait_terminal(&app, &b1["operation_id"]).await;
    patch_modes(&app, &["acceptEdits"]).await;
    let (s2, b2) = http_start(&app, &app.thread, body).await;
    assert_eq!((s2.as_u16(), &b2["operation_id"]), (202, &b1["operation_id"]));
}

#[tokio::test]
async fn a_replay_is_answered_while_the_daemon_is_stopping() {
    let app = test_app().await;
    let body = json!({ "command_id": "t1", "prompt": "hi", "model": "fake-small", "mode": "acceptEdits", "effort": "high" });
    let (_, first) = http_start(&app, &app.thread, body.clone()).await;
    wait_terminal(&app, &first["operation_id"]).await;
    app.handles.close_for_test().await; // what shutdown does first
    let (s, again) = http_start(&app, &app.thread, body).await;
    assert_eq!((s.as_u16(), &again["operation_id"]), (202, &first["operation_id"]));
    let (s2, b2) = http_start(&app, &app.thread, json!({
        "command_id": "t2", "prompt": "new", "model": "fake-small", "mode": "acceptEdits", "effort": "high" })).await;
    assert_eq!((s2.as_u16(), b2["code"].as_str()), (503, Some("RUNTIME_STOPPING")));
}

#[tokio::test]
async fn the_harness_runs_with_the_chosen_model_mode_and_effort() {
    let app = test_app().await;
    let (_, b) = http_start(&app, &app.thread, json!({
        "command_id": "t1", "prompt": "report", "model": "fake-large", "mode": "auto", "effort": "max" })).await;
    wait_terminal(&app, &b["operation_id"]).await;
    let r: Value = serde_json::from_str(&last_agent_entry(&app).await.body).unwrap();
    assert_eq!((r["model"].as_str(), r["mode"].as_str(), r["effort"].as_str()), (Some("fake-large"), Some("auto"), Some("max")));
}
```

(The fake adapter's version is its `package.json` stand-in: tests write `{"version":"fake-adapter-1"}` beside the configured adapter path, and the fake answers `--version` with `fake-claude-1`, which the test configures as the Claude path.)

- [ ] **Step 2: Run** `cargo test --test turn_command` — expected: compile failure.
- [ ] **Step 3: Implement `Storage::start_turn`** in `storage/sqlite/turn.rs` inside one `write_txn`: `classify(conn, ctx, "thread", thread_id)` → on a record, parse `{operation_id, entry_id}` and return `replayed: true`; else `ThreadBusy` if `operation WHERE thread_id = ? AND status_kind NOT IN ('Completed','Failed','Cancelled','Interrupted')` exists; refuse for a stopped runtime (the check `create_pending_operation` makes); insert the operation and its `OperationCreated` event (extract `pub(in crate::storage) async fn insert_pending(conn, ..)` from `create_pending_operation`, both call it); append the user entry with `operation_id` (extract `append_entry_in(conn, ..)`); insert `agent_invocation`; `harness::remember_settings`; `record_command(conn, ctx, "thread", thread_id, "operation", &json!({operation_id, entry_id}).to_string(), ts)`.
- [ ] **Step 4: Rewrite `protocol/conversation.rs::start`, replay first:**
  1. build the `CommandContext` and the fingerprint over `{thread_id, prompt, model, mode, effort}`;
  2. `storage.replayed_turn(&ctx)` — a match returns its `StartedTurn` at once: no validation, no session, no spawn, even while stopping; a mismatch is `CommandConflict`;
  3. new commands only: `is_closed` → `RuntimeStopping`;
  4. `turn_context`; harness known and available (`HarnessUnavailable`); `sessions.open` (`HarnessStartFailed`); if `model` differs from the session's current model, `set_option(ids.model, model)` so the efforts are the chosen model's — the only session change allowed before the transaction, since it records nothing; `choices::refusal` (`SettingNotOffered`); project `allowed_modes` (`ModeNotAllowed`);
  5. `storage.start_turn` (classifies again inside its transaction, so two concurrent first requests produce one turn);
  6. hand to `PlannerTurn::start` only when `replayed` is false.

  `StartTurn` becomes `{ command_id: String, prompt: String, model: String, mode: String, effort: String }`.
- [ ] **Step 5: `PlannerTurn::start`** begins after the transaction: sets effort and mode with `set_option` when they differ from the session's current values (a refusal is `Failed { stage: Prepare }` naming the setting, nothing sent to the model), registers, commits `Running`, prompts. Entries the turn writes pass `operation_id: Some(&op)`.
- [ ] **Step 6: Errors.** Add the `ErrorCode` variants (SCREAMING_SNAKE like the rest); map `StorageError::ThreadBusy` → 409, `HarnessLocked` → 409, `ForkPointNotSupported` → 422; constructors `Failure::setting_not_offered(what, id)`, `Failure::harness_unavailable()`, `Failure::mode_not_allowed(mode)`.
- [ ] **Step 7:** Update every other caller of `PlannerTurn::start` to create the operation via `storage.start_turn` first (fixture helper in `tests/fixtures/`). `cargo test`, clippy, code map. Commit `feat(turns): start a turn as one idempotent command with its settings (spec §12.7)`.

### Task B4: Context, limits, and the answering model

**Files:**
- Modify: `src/agent/acp.rs` (`Usage` parsing per Task 0), `src/planner/turn.rs` (keep the turn's last usage; record limits; publish), `src/planner/sessions.rs` (latest usage per thread), `src/storage/sqlite/operation.rs` (`mark_operation_completed` gains the observation), `src/storage/sqlite/operation_read.rs` + `src/operation/mod.rs` (`Operation.invocation`), `src/protocol/sse.rs` (`usage` frame)
- Test: `tests/harness_observation.rs`, unit tests in `acp.rs`

**Interfaces:**
- Consumes: A2 `HarnessEvent::Usage`; B2 `AccountLimits`, `record_limits`; B3 invocation row.
- Produces:
  - `agent::events::limits_from(rate_limit: &serde_json::Value, observed_at: &str) -> Option<AccountLimits>` — reads `unifiedWindows.five_hour` / `.seven_day` `{ utilization, resetsAt }`; `None` when neither window is present.
  - `agent::events::TurnObservation { pub observed_model: Option<String>, pub context_used: Option<u64>, pub context_window: Option<u64> }`, `TurnObservation::from_usage(last: Option<&HarnessEvent>) -> Self` (a `size` of 0 is `None`).
  - `Storage::mark_operation_completed(&self, op: &OperationId, outcome: serde_json::Value, observation: &TurnObservation) -> Result<(), StorageError>` — writes the invocation's observed columns in the same transaction and puts `invocation` (`InvocationView`) in the `OperationCompleted` payload.
  - `operation::InvocationView` (serde + ToSchema, fields as Task 1).

- [ ] **Step 1: Unit tests** (`events.rs`):

```rust
#[test]
fn limits_read_both_windows_with_reset_times() {
    let v = json!({ "unifiedWindows": {
        "five_hour": { "utilization": 0.75, "resetsAt": 1790212200 },
        "seven_day": { "utilization": 0.91, "resetsAt": 1790542800 } } });
    let l = limits_from(&v, "2026-09-24T10:00:00Z").unwrap();
    assert_eq!(l.five_hour.unwrap().resets_at, 1790212200);
    assert!((l.seven_day.unwrap().utilization - 0.91).abs() < 1e-9);
}

#[test]
fn a_report_without_windows_is_no_limits_rather_than_zero() {
    assert!(limits_from(&json!({ "status": "allowed" }), "t").is_none());
}

#[test]
fn an_observation_without_a_size_reports_no_window() {
    let u = HarnessEvent::Usage { used: 10, size: 0, model: None, rate_limit: None };
    let o = TurnObservation::from_usage(Some(&u));
    assert_eq!((o.context_used, o.context_window), (Some(10), None));
    assert_eq!(TurnObservation::from_usage(None).context_used, None);
}
```

- [ ] **Step 2: Integration tests** in `tests/harness_observation.rs`:

```rust
#[tokio::test]
async fn a_completed_turn_records_what_the_harness_reported() {
    let app = test_app().await;
    let done = wait_terminal(&app, &start_settled(&app, "usage").await).await;
    let inv = done.invocation.unwrap();
    assert_eq!(inv.observed_model.as_deref(), Some("fake-large-answering"));
    assert_eq!((inv.context_used, inv.context_window), (Some(1234), Some(200000)));
    let limits = app.storage.latest_limits("claude-code").await.unwrap().unwrap();
    assert!((limits.seven_day.unwrap().utilization - 0.5).abs() < 1e-9);
}

#[tokio::test]
async fn a_turn_without_usage_leaves_the_observation_unavailable() {
    let app = test_app().await;
    let inv = wait_terminal(&app, &start_settled(&app, "hi").await).await.invocation.unwrap();
    assert!(inv.context_used.is_none() && inv.context_window.is_none() && inv.observed_model.is_none());
}

#[tokio::test]
async fn a_usage_frame_reaches_a_subscriber_of_the_thread() {
    let app = test_app().await;
    let mut sub = subscribe(&app, &app.thread).await;
    start_settled(&app, "usage").await;
    let frame = next_frame_named(&mut sub, "usage").await;
    assert_eq!(frame["context_used"], 1234);
    assert!(frame["limits"]["seven_day"].is_object());
}
```

(`start_settled(app, prompt)` is B3's HTTP start with the fake's default settings, added to `tests/fixtures/`.)

- [ ] **Step 3: Run** — expected: failures.
- [ ] **Step 4: Implement.** The watcher keeps the last `Usage` of the turn; on a `Usage` with `rate_limit`, `record_limits(harness, &limits_from(..))`; every `Usage` becomes a `usage` frame `{ thread_id, context_used, context_window, limits }` (limits: the stored latest). `Completed` writes `TurnObservation::from_usage(last)`. Failed and cancelled turns record no observation (NULLs = unavailable), stated in a comment citing §12.7.
- [ ] **Step 5: The breakdown, only if Task 0 check 5 found a read that does not block a turn.** If it did: add `context_breakdown TEXT NULL` (JSON) to `agent_invocation` in a migration `0006`, read it after `end_turn` exactly as the evidence file describes, store it with the observation, carry it in `InvocationView.breakdown`, and add it to the contract with the difference reported in B6. If it did not: change nothing and leave spec §12.8's OPEN block; say which in the report.
- [ ] **Step 6:** `cargo test`, clippy, code map. Commit `feat(agent): record context, limits and the answering model from the session (spec §12.8)`.

### Task B5: Thread and project routes

**Files:**
- Create: `src/protocol/thread.rs` (PATCH thread; B6 adds fork here)
- Modify: `src/protocol/project.rs` (`CreateThread.harness`, `PATCH /api/projects/{id}`), `src/planner/sessions.rs` (closing the thread's adapter on a harness change), `src/protocol/mod.rs`, `src/protocol/openapi.rs`, `docs/codebase/README.md`
- Test: `tests/thread_routes.rs`

**Interfaces:**
- Consumes: B1 `set_thread_harness`, `set_project_modes`, `policy`; A3 `Sessions::terminate`.
- Produces: `PATCH /api/threads/{id}` (`UpdateThread { command_id, harness }`), `PATCH /api/projects/{id}` (`UpdateProject { command_id, allowed_modes }`); `create_project` passes `policy::default_modes()`. A replay with the same fingerprint answers the thread as it now stands with 200 and changes nothing; `HarnessLocked` only answers a new command.

- [ ] **Step 1: Failing tests** in `tests/thread_routes.rs`:

```rust
#[tokio::test]
async fn a_thread_can_be_created_on_a_harness_and_changed_until_its_first_turn() {
    let app = test_app().await;
    let t = create_thread(&app, json!({ "command_id": "c9", "title": "x", "harness": "codex" })).await;
    assert_eq!(t["harness"], "codex");
    let path = format!("/api/threads/{}", t["id"].as_str().unwrap());
    let (s, t2) = patch(&app, &path, json!({ "command_id": "h1", "harness": "claude-code" })).await;
    assert_eq!((s, t2["harness"].as_str()), (200, Some("claude-code")));
    start_ok(&app, t["id"].as_str().unwrap()).await;
    let (s3, b3) = patch(&app, &path, json!({ "command_id": "h2", "harness": "codex" })).await;
    assert_eq!((s3, b3["code"].as_str()), (409, Some("HARNESS_LOCKED")));
    let (s4, b4) = patch(&app, &path, json!({ "command_id": "h1", "harness": "claude-code" })).await;
    assert_eq!((s4, b4["harness"].as_str()), (200, Some("claude-code")));
}

#[tokio::test]
async fn changing_the_harness_closes_the_threads_adapter() {
    let app = test_app().await;
    post(&app, &format!("/api/threads/{}/session", app.thread), json!({})).await;
    assert_eq!(app.sessions.live_count().await, 1);
    patch(&app, &format!("/api/threads/{}", app.thread), json!({ "command_id": "h1", "harness": "codex" })).await;
    assert_eq!(app.sessions.live_count().await, 0);
}

#[tokio::test]
async fn an_unknown_harness_or_a_mode_outside_the_policy_is_refused_when_set() {
    let app = test_app().await;
    let (s, b) = patch(&app, &format!("/api/threads/{}", app.thread), json!({ "command_id": "h1", "harness": "gemini" })).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    let (s2, b2) = patch(&app, &format!("/api/projects/{}", app.project), json!({
        "command_id": "m1", "allowed_modes": { "claude-code": ["bypassPermissions"] } })).await;
    assert_eq!((s2, b2["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
}

#[tokio::test]
async fn a_turn_on_a_codex_thread_is_harness_unavailable() {
    let app = test_app().await;
    let t = create_thread(&app, json!({ "command_id": "c9", "title": "x", "harness": "codex" })).await;
    let (s, b) = http_start(&app, t["id"].as_str().unwrap(), json!({
        "command_id": "t1", "prompt": "hi", "model": "fake-small", "mode": "acceptEdits", "effort": "high" })).await;
    assert_eq!((s.as_u16(), b["code"].as_str()), (422, Some("HARNESS_UNAVAILABLE")));
}

#[tokio::test]
async fn settings_and_limits_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app_at(dir.path()).await;
    start_and_finish(&app, "usage", json!({ "model": "fake-large", "mode": "auto", "effort": "max" })).await;
    patch(&app, &format!("/api/projects/{}", app.project), json!({
        "command_id": "m1", "allowed_modes": { "claude-code": ["acceptEdits"] } })).await;
    shut_down_app(app).await;
    let app = test_app_at(dir.path()).await;
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(h[0]["remembered"], json!({ "model": "fake-large", "effort": "max" }));
    assert!(h[0]["limits"]["seven_day"].is_object());
    let p: Vec<Value> = get_json(&app, "/api/projects").await;
    assert_eq!(p[0]["allowed_modes"]["claude-code"], json!(["acceptEdits"]));
    let ops: Vec<Value> = get_json(&app, &format!("/api/threads/{}/operations", app.thread)).await;
    assert_eq!(ops[0]["invocation"]["context_used"], 1234);
}
```

- [ ] **Step 2:** Run — expected: 404/405.
- [ ] **Step 3:** Implement the routes; validate the harness with `policy::is_known` and modes against `policy::allowed_modes(harness)` (`SettingNotOffered`). A successful harness change calls `sessions.terminate(thread)`.
- [ ] **Step 4:** `cargo test`, clippy, code map, owners. Commit `feat(protocol): choose a thread's harness and a project's allowed modes (spec §12.5-§12.6)`.

### Task B6: Fork, then regenerate the contract

**Files:**
- Create: `src/storage/sqlite/fork.rs`, `tests/fork.rs`
- Modify: `src/protocol/thread.rs` (fork route), `src/planner/sessions.rs` (a fork's first opening), `api/openapi.json` (regenerated), `docs/codebase/*`

**Interfaces:**
- Consumes: B1 fork columns, `TurnContext.fork_session_id`; A2 `SessionStart::Fork`.
- Produces: `Storage::fork_thread(&self, ctx: &CommandContext, source: &ThreadId, at_entry: &ThreadEntryId) -> Result<PlanningThread, StorageError>`; `POST /api/threads/{id}/fork` → 201 `PlanningThread`.

- [ ] **Step 1: Failing tests** in `tests/fork.rs`:

```rust
#[tokio::test]
async fn fork_copies_entries_keeps_their_operation_and_leaves_the_source_alone() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let src = entries_json(&app, &app.thread).await;
    let last = src.last().unwrap()["id"].as_str().unwrap().to_string();
    let (s, fork) = post(&app, &format!("/api/threads/{}/fork", app.thread), json!({ "command_id": "f1", "at_entry_id": last })).await;
    assert_eq!(s, 201);
    assert_eq!(fork["forked_from_thread"], app.thread.as_str());
    let copied = entries_json(&app, fork["id"].as_str().unwrap()).await;
    assert_eq!(copied.len(), src.len());
    assert_ne!(copied[0]["id"], src[0]["id"]);
    assert_eq!(copied.last().unwrap()["operation_id"], src.last().unwrap()["operation_id"]);
    assert_eq!(entries_json(&app, &app.thread).await, src, "source unchanged");
    assert!(get_json::<Vec<Value>>(&app, &format!("/api/threads/{}/operations", fork["id"].as_str().unwrap())).await.is_empty());
}

#[tokio::test]
async fn the_forks_first_opening_forks_the_source_session() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let src_session = app.storage.turn_context(&app.thread).await.unwrap().harness_session_id.unwrap();
    let fork = fork_last(&app).await;
    let fork_id = fork["id"].as_str().unwrap();
    start_and_finish_on(&app, fork_id, "report", default_settings()).await;
    let r: Value = serde_json::from_str(&last_agent_entry_on(&app, fork_id).await.body).unwrap();
    assert_eq!(r["how"], "fork");
    assert_eq!(r["session"], format!("fork-of-{src_session}"));
    let recorded = app.storage.turn_context(&ThreadId::from_literal(fork_id)).await.unwrap().harness_session_id.unwrap();
    assert_eq!(recorded, format!("fork-of-{src_session}"));
}

#[tokio::test]
async fn only_the_last_completed_entry_is_a_fork_point() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let first = entries_json(&app, &app.thread).await[0]["id"].as_str().unwrap().to_string();
    let (s, b) = post(&app, &format!("/api/threads/{}/fork", app.thread), json!({ "command_id": "f1", "at_entry_id": first })).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("FORK_POINT_NOT_SUPPORTED")));
}

#[tokio::test]
async fn a_stopped_last_turn_is_not_a_fork_point() {
    let app = test_app().await;
    let op = start_settled(&app, "hang").await;
    wait_for_entry_or_delta(&app).await;
    stop(&app, &op).await;
    let (s, b) = fork_last_raw(&app).await;
    assert_eq!((s, b["code"].as_str()), (422, Some("FORK_POINT_NOT_SUPPORTED")));
}

#[tokio::test]
async fn fork_while_a_turn_runs_is_thread_busy_and_a_replay_returns_the_same_fork() {
    let app = test_app().await;
    start_and_finish(&app, "hello", default_settings()).await;
    let a = fork_last(&app).await;
    let b = fork_last(&app).await; // same command_id "f1"
    assert_eq!(a["id"], b["id"]);
    let _running = start_settled(&app, "hang").await;
    let (s, body) = fork_last_raw_with(&app, "f2").await;
    assert_eq!((s, body["code"].as_str()), (409, Some("THREAD_BUSY")));
}
```

(A "stopped" last turn: `hang` with a harness-confirmed cancel writes `Cancelled`, so the last entry's operation is not `Completed`.)

- [ ] **Step 2:** Run — expected: route missing.
- [ ] **Step 3: `fork_thread`** in one transaction: `classify` for replay (return the recorded thread); `ThreadBusy` if a non-terminal operation exists on the source; `ForkPointNotSupported` unless `at_entry` is the highest-ordinal entry, has `operation_id`, that operation is `Completed`, and the source has `harness_session_id`; insert the thread (same project, same `harness_kind`, title `"<source title> (fork)"`, the three fork columns); copy entries with new ids, ordinals 1..n, same `kind`, `author`, `body`, `refs_json`, `operation_id`; set `next_entry_ordinal`; `record_command`.
- [ ] **Step 4: A fork's first opening.** `Sessions::open` chooses `SessionStart::Fork(fork_session_id)` when the thread has no `harness_session_id` and has `fork_session_id`; the session it returns is recorded when the fork's first turn ends (A4's rule, unchanged).
- [ ] **Step 5: Regenerate the contract.** `UPDATE_OPENAPI=1 cargo test --test openapi`, then `git diff` against Task 1's commit. Every difference goes into the report with its reason. Semantic differences (a field name, type, nullability, status code, or path) are fixed in Rust to match the draft unless the draft is wrong — then stop and report (§12.12 rule 4). Cosmetic differences (descriptions, ordering utoipa imposes) are accepted and listed.
- [ ] **Step 6:** `cargo test` (all, now including `openapi`), clippy, code map, owners. Commit `feat(threads): fork from the last completed message; regenerate the contract (spec §12.9)`.

---

## Web track (worktree `m1-web`)

The web agent works from the draft `api/openapi.json` (Task 1). First step of W1 is `npm run gen:api` in `web/`. It never edits `api/openapi.json`, never opens `src/`, `tests/`, `migrations/`, `harness/`, never runs the daemon.

### Task W1: API functions, frames, UI primitives

**Files:**
- Modify: `web/src/api/schema.d.ts` (generated), `web/src/api/client.ts`, `web/src/api/queries.ts`, `web/src/stream/frames.ts`, `web/src/stream/thread-stream.ts`
- Create: `web/src/components/ui/dropdown-menu.tsx`, `web/src/components/ui/tooltip.tsx` (via `npx shadcn@latest add dropdown-menu tooltip`, Base UI registry as the existing components), `web/src/test/contract-fixtures.ts`
- Test: `web/src/stream/frames.test.ts`, `web/src/api/client.test.ts`

**Interfaces:**
- Produces (in `client.ts`):
  - `type HarnessInfo`, `Choice`, `SessionChoices`, `AccountLimits`, `InvocationView`, `TurnSettings = { model: string; mode: string; effort: string }` — re-exported from `schema.d.ts`.
  - `listHarnesses(): Promise<HarnessInfo[]>`; `openSession(threadId: string): Promise<SessionChoices>`
  - `startTurn(threadId: string, commandId: string, prompt: string, settings: TurnSettings): Promise<string>` — the id is the caller's; this function never makes one
  - `setThreadHarness(threadId: string, commandId: string, harness: string): Promise<PlanningThread>`
  - `setProjectModes(projectId: string, commandId: string, allowedModes: Record<string, string[]>): Promise<Project>`
  - `forkThread(threadId: string, commandId: string, atEntryId: string): Promise<PlanningThread>`
  - `harnessesQuery` (key `['harnesses']`) in `queries.ts`
  - `parseUsage(data: string): UsageFrame` and `parseOptions(data: string): OptionsFrame` in `frames.ts`; `ThreadStream` emits `{ type: 'usage', usage }` and `{ type: 'options', choices }`
  - `contract-fixtures.ts`: `claudeHarness: HarnessInfo`, `codexHarness: HarnessInfo`, `fakeChoices: SessionChoices` (models `fake-small`, `fake-large`; efforts of `fake-large` `low`, `high`, `max`; modes `acceptEdits`, `auto`), `projectWithModes(modes)`, `completedOperation(invocation)` — typed against `schema.d.ts` so a contract change breaks them at compile time.

- [ ] **Step 1: Failing tests:**

```ts
// frames.test.ts
it('parses a usage frame with limits', () => {
  const u = parseUsage('{"thread_id":"t1","context_used":1234,"context_window":200000,"limits":{"five_hour":{"utilization":0.25,"resets_at":1790212200},"seven_day":null,"observed_at":"2026-09-24T02:49:00Z"}}')
  expect(u.contextUsed).toBe(1234)
  expect(u.limits?.fiveHour?.utilization).toBe(0.25)
  expect(u.limits?.sevenDay).toBeNull()
})
it('refuses a usage frame without thread_id', () => {
  expect(() => parseUsage('{"context_used":1,"context_window":2,"limits":null}')).toThrow(FrameError)
})
it('parses an options frame', () => {
  const o = parseOptions(JSON.stringify({ thread_id: 't1', choices: fakeChoices }))
  expect(o.choices.models.map((m) => m.id)).toEqual(['fake-small', 'fake-large'])
})

// client.test.ts
it("startTurn sends the settings and the caller's command id", async () => {
  const bodies: unknown[] = []
  vi.stubGlobal('fetch', async (r: Request) => { bodies.push(await r.json()); return Response.json({ operation_id: 'op1' }, { status: 202 }) })
  await startTurn('t1', 'cmd-1', 'hi', { model: 'fake-small', mode: 'acceptEdits', effort: 'high' })
  expect(bodies[0]).toEqual({ command_id: 'cmd-1', prompt: 'hi', model: 'fake-small', mode: 'acceptEdits', effort: 'high' })
})
it('openSession posts to the thread session route', async () => {
  const urls: string[] = []
  vi.stubGlobal('fetch', async (r: Request) => { urls.push(`${r.method} ${new URL(r.url).pathname}`); return Response.json(fakeChoices) })
  await openSession('t1')
  expect(urls).toEqual(['POST /api/threads/t1/session'])
})
```

- [ ] **Step 2:** `npm test` — expected: failures on missing exports.
- [ ] **Step 3:** Implement; add the shadcn components; `npm run typecheck && npm run lint && npm test` green. Commit `feat(web): API for harnesses, sessions, settings, fork; usage and options frames (spec §12.10)`.

### Task W2: CLI picker, session opening, composer bar

**Files:**
- Create: `web/src/app/conversation/turn-settings.ts` (pure state), `web/src/app/conversation/cli-picker.tsx`, `web/src/app/conversation/composer-bar.tsx`, `web/src/app/conversation/use-session.ts`
- Modify: `web/src/app/conversation/composer.tsx` (sends settings; "Runs in" moves into the bar), `web/src/app/conversation/conversation.tsx` (header picker; opens the session), `web/src/app/test-app.tsx` (helpers `choose`, `menuItem`)
- Test: `web/src/app/conversation/turn-settings.test.ts`, `web/src/app/conversation/composer-bar.test.tsx`

**Interfaces:**
- Consumes: W1 `openSession`, `startTurn`, `setThreadHarness`, `harnessesQuery`, fixtures.
- Produces:
  - `useSession(threadId): { state: 'connecting' } | { state: 'ready'; choices: SessionChoices } | { state: 'failed'; message: string; retry: () => void }` — opens on mount and when the harness changes; an `options` frame replaces `choices`.
  - `initialSettings(c: SessionChoices): TurnSettings` — `c.current`, except a mode that is disabled moves to the first enabled mode.
  - `withModel(c: SessionChoices, s: TurnSettings, model: string): TurnSettings` — the model changes; the effort stays only if the new choices (after the `options` frame the daemon sends) offer it, else the first offered effort; until those choices arrive the effort menu is disabled.
  - `sendable(c: SessionChoices, s: TurnSettings): boolean` — false when the mode is disabled or no mode is enabled.

- [ ] **Step 1: Failing unit tests** (`turn-settings.test.ts`):

```ts
it('starts from the session current values', () => {
  expect(initialSettings(fakeChoices)).toEqual(fakeChoices.current)
})
it('changing model resets an effort it does not offer', () => {
  const small = { ...fakeChoices, current: { ...fakeChoices.current, model: 'fake-small' }, efforts: [choice('low'), choice('high')] }
  expect(withModel(small, { model: 'fake-large', mode: 'acceptEdits', effort: 'max' }, 'fake-small').effort).toBe('low')
})
it('a mode the project disallowed is not sendable', () => {
  const c = { ...fakeChoices, modes: [choice('acceptEdits'), { ...choice('auto'), enabled: false, reason: 'Not allowed in this project' }] }
  expect(sendable(c, { ...c.current, mode: 'auto' })).toBe(false)
  expect(initialSettings({ ...c, current: { ...c.current, mode: 'auto' } }).mode).toBe('acceptEdits')
})
```

- [ ] **Step 2: Failing app tests** (`composer-bar.test.tsx`, `startApp` with answers for `GET /api/harnesses`, `GET /api/projects`, `GET /api/projects/p1/threads`, `GET /api/threads/t1/entries`, `GET /api/threads/t1/operations`, `POST /api/threads/t1/session`, `POST /api/threads/t1/turns`, `PATCH /api/threads/t1`):

```tsx
it('says it is connecting, then builds every menu from the session', async () => {
  let release!: () => void
  const app = await startApp('/projects/p1/threads/t1', answers({
    session: () => new Promise((r) => { release = () => r(Response.json(fakeChoices)) }) }))
  await until(() => app.text().includes('Connecting to Claude Code…'))
  act(() => release())
  await until(() => app.button('fake-large') !== undefined)
  expect(app.text()).not.toContain('Connecting')
  app.unmount()
})
it('a failed opening shows the message and a retry, never an endless spinner', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({
    session: () => Response.json({ code: 'HARNESS_START_FAILED', message: 'node was not found' }, { status: 502 }) }))
  await until(() => app.text().includes('node was not found'))
  expect(app.button('Retry')).toBeDefined()
  app.unmount()
})
it('sends the chosen model, mode and effort', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers())
  await until(() => app.button('fake-large') !== undefined)
  await choose(app, 'Accept edits', 'Auto')
  typeInto(app.container.querySelector('textarea')!, 'hi')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.includes('POST /api/threads/t1/turns'))
  expect(app.bodies.at(-1)).toMatchObject({ model: 'fake-large', mode: 'auto', prompt: 'hi' })
  app.unmount()
})
it('the CLI picker changes the harness before the first turn and shows a lock after', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({ operations: [] }))
  await until(() => app.button('Claude Code') !== undefined)
  act(() => app.button('Claude Code')!.click())
  expect(menuItem('Codex')?.getAttribute('aria-disabled')).toBe('true')
  app.unmount()
  const locked = await startApp('/projects/p1/threads/t1', answers({ operations: [completedOperation(null)] }))
  await until(() => locked.container.querySelector('[aria-label="CLI locked for this conversation"]') !== null)
  locked.unmount()
})
it('a retried Send reuses the same command id', async () => {
  let n = 0
  const app = await startApp('/projects/p1/threads/t1', answers({
    start: () => (++n === 1 ? Promise.reject(new TypeError('network down')) : Response.json({ operation_id: 'op1' }, { status: 202 })) }))
  await until(() => app.button('Send') !== undefined)
  typeInto(app.container.querySelector('textarea')!, 'hi')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 1)
  act(() => app.button('Send')!.click())
  await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 2)
  const [a, b] = app.bodies.slice(-2) as { command_id: string }[]
  expect(b.command_id).toBe(a.command_id)
  app.unmount()
})
it('changing a setting after a failed send makes a new command id', async () => {
  let n = 0
  const app = await startApp('/projects/p1/threads/t1', answers({
    start: () => (++n === 1 ? Promise.reject(new TypeError('network down')) : Response.json({ operation_id: 'op1' }, { status: 202 })) }))
  await until(() => app.button('Send') !== undefined)
  typeInto(app.container.querySelector('textarea')!, 'hi')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 1)
  await choose(app, 'Accept edits', 'Auto')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 2)
  const [a, b] = app.bodies.slice(-2) as { command_id: string }[]
  expect(b.command_id).not.toBe(a.command_id)
  app.unmount()
})
it('shows the refusal when the daemon refuses the mode and keeps the prompt', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({
    start: () => Response.json({ code: 'MODE_NOT_ALLOWED', message: 'auto is not allowed' }, { status: 403 }) }))
  await until(() => app.button('Send') !== undefined)
  typeInto(app.container.querySelector('textarea')!, 'hi')
  act(() => app.button('Send')!.click())
  await until(() => app.text().includes('auto is not allowed'))
  expect(app.container.querySelector('textarea')!.value).toBe('hi')
  app.unmount()
})
```

- [ ] **Step 3:** Implement per spec §12.11 and the agreed mockup (bar under the box, flat: left `+`, mode, folder; right model, effort, ring slot). `+` is present and disabled with `title="Attachments come later"`. **The command id belongs to a pending send:** the composer keeps `{ commandId, prompt, settings }`, makes the id with `newCommandId()` when Send is pressed and no pending send matches the current prompt and settings, reuses it for a retry of that same send, and drops it on success or when the prompt or a setting changes. The CLI picker does the same for `setThreadHarness`, then reopens the session.
- [ ] **Step 4:** typecheck, lint, test green. Commit `feat(web): CLI picker, session opening, composer bar (spec §12.11)`.

### Task W3: Context ring and the answering model

**Files:**
- Create: `web/src/app/conversation/context-ring.tsx`, `web/src/app/conversation/usage.ts`
- Modify: `web/src/app/conversation/composer-bar.tsx` (mount the ring), `web/src/app/conversation/messages.tsx` (observed ≠ requested note), `web/src/app/conversation/use-conversation.ts` (latest usage from invocations and `usage` frames)
- Test: `web/src/app/conversation/usage.test.ts`, `web/src/app/conversation/context-ring.test.tsx`

**Interfaces:**
- Produces: `contextShown(used: number | null, window: number | null): { used: number; window: number; percent: number } | null` (null unless both present and `window > 0`; percent rounded); `formatTokens(n: number): string` (`126.8k`, `1M`); `resetIn(resetsAt: number, now: number): string` (`4h18m`, `3d21h`).

- [ ] **Step 1: Failing tests:**

```ts
it('shows context only when both numbers were reported', () => {
  expect(contextShown(126800, 1_000_000)).toEqual({ used: 126800, window: 1_000_000, percent: 13 })
  expect(contextShown(126800, null)).toBeNull()
  expect(contextShown(10, 0)).toBeNull()
})
it('formats tokens and reset times', () => {
  expect(formatTokens(126800)).toBe('126.8k'); expect(formatTokens(1_000_000)).toBe('1M')
  expect(resetIn(1000 + 4 * 3600 + 18 * 60, 1000)).toBe('4h18m')
  expect(resetIn(1000 + 3 * 86400 + 21 * 3600, 1000)).toBe('3d21h')
})
```

```tsx
it('opens at once with no figures and never shows a spinner', async () => {
  const r = renderRing({ usage: null, limits: null })
  act(() => r.trigger().click())
  expect(r.text()).toContain('No figures yet')
  expect(r.container.querySelector('[role="progressbar"][aria-busy="true"]')).toBeNull()
})
it('shows the summary level: context, five-hour and weekly with resets, and when observed', async () => {
  const r = renderRing({ usage: { contextUsed: 126800, contextWindow: 1_000_000 },
    limits: { fiveHour: { utilization: 0.16, resetsAt: nowSec + 4 * 3600 + 18 * 60 }, sevenDay: { utilization: 0.96, resetsAt: nowSec + 86400 }, observedAt: '2026-09-24T02:49:00Z' } })
  act(() => r.trigger().click())
  for (const t of ['126.8k / 1M (13%)', '5-hour limit', 'Resets in 4h18m', '16%', 'Weekly', '96%', 'updated']) expect(r.text()).toContain(t)
})
it('a reply whose observed model differs from the requested one says both', async () => {
  // operations answer with invocation { requested_model: 'fake-small', observed_model: 'fake-large-answering' }
  const app = await startApp('/projects/p1/threads/t1', answers({ operations: [completedOperation({ ...invocationFixture, requested_model: 'fake-small', observed_model: 'fake-large-answering' })] }))
  await until(() => app.text().includes('Asked for fake-small · answered by fake-large-answering'))
  app.unmount()
})
```

(The breakdown level renders only when `InvocationView` carries a breakdown, which exists only if B4 Step 5 added it; W3 renders the summary and leaves a `breakdown?` prop unused otherwise — Task I wires it if the regenerated contract has it.)

- [ ] **Step 2–4:** implement, green, commit `feat(web): context ring that never waits, limits, answering model (spec §12.8, §12.11)`.

### Task W4: Message actions, refused permissions, allowed modes

**Files:**
- Create: `web/src/app/conversation/message-actions.tsx`, `web/src/app/project-modes.tsx`
- Modify: `web/src/app/conversation/messages.tsx`, `web/src/app/project-page.tsx`
- Test: `web/src/app/conversation/message-actions.test.tsx`, `web/src/app/project-modes.test.tsx`

- [ ] **Step 1: Failing tests:**

```tsx
it('copy is on every message and copies its text', async () => {
  const writes: string[] = []
  vi.stubGlobal('navigator', { clipboard: { writeText: async (t: string) => { writes.push(t) } } })
  const app = await startApp('/projects/p1/threads/t1', answers({ entries: [userEntry('u1', 'hi'), agentEntry('a1', 'hello')] }))
  await until(() => app.buttons('Copy').length === 2)
  act(() => app.buttons('Copy')[1].click())
  await until(() => writes.length === 1)
  expect(writes).toEqual(['hello'])
  app.unmount()
})
it('fork shows only on the last message of an idle thread and opens the fork', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({
    entries: [userEntry('u1', 'hi'), agentEntry('a1', 'hello')], operations: [completedOperation(null)],
    fork: () => Response.json({ ...threadFixture, id: 't9' }, { status: 201 }) }))
  await until(() => app.buttons('Fork').length === 1)
  act(() => app.buttons('Fork')[0].click())
  await until(() => app.calls.includes('POST /api/threads/t1/fork'))
  expect(app.bodies.at(-1)).toMatchObject({ at_entry_id: 'a1' })
  await until(() => app.path() === '/projects/p1/threads/t9')
  app.unmount()
})
it('fork is absent while a turn runs', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({ operations: [runningOperation()] }))
  await until(() => app.buttons('Copy').length > 0)
  expect(app.buttons('Fork')).toHaveLength(0)
  app.unmount()
})
it('a refused fork shows the daemon message and stays', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({
    operations: [completedOperation(null)],
    fork: () => Response.json({ code: 'FORK_POINT_NOT_SUPPORTED', message: 'fork from the last completed message' }, { status: 422 }) }))
  await until(() => app.buttons('Fork').length === 1)
  act(() => app.buttons('Fork')[0].click())
  await until(() => app.text().includes('fork from the last completed message'))
  expect(app.path()).toBe('/projects/p1/threads/t1')
  app.unmount()
})
it('a refused permission renders as a quiet line naming what was asked', async () => {
  const app = await startApp('/projects/p1/threads/t1', answers({ entries: [entryOfKind('r1', 'PermissionRefused', 'Run echo probe')] }))
  await until(() => app.text().includes('Run echo probe'))
  expect(app.text()).toContain('refused in Accept edits')
  expect(app.text()).toContain('Auto would allow it')
  app.unmount()
})
it('the project page edits allowed modes', async () => {
  const app = await startApp('/projects/p1', answers({ project: projectWithModes({ 'claude-code': ['acceptEdits', 'auto'] }) }))
  await until(() => app.checkbox('Auto') !== undefined)
  act(() => app.checkbox('Auto')!.click())
  await until(() => app.calls.includes('PATCH /api/projects/p1'))
  expect(app.bodies.at(-1)).toMatchObject({ allowed_modes: { 'claude-code': ['acceptEdits'] } })
  act(() => app.checkbox('Accept edits')!.click())
  await until(() => app.text().includes('No mode left: turns cannot start'))
  app.unmount()
})
```

(`buttons`, `checkbox`, `path` and the entry/operation factories are added to `test-app.tsx` / `contract-fixtures.ts` if absent.)

- [ ] **Step 2–4:** implement (actions appear on hover and on keyboard focus; copy shows "Copied" for 1.5 s), green, commit `feat(web): copy, fork, refused permissions, allowed modes (spec §12.9, §12.5, §12.11)`.

---

## Task I: Integrate, review, run (controller)

- [ ] **Step 1: Merge.** On `milestone-1/harness-controls`: merge `m1-backend`, then `m1-web`. Conflict only possible in `api/openapi.json` (backend wins) and `docs/codebase/*` (backend owns).
- [ ] **Step 2: Reconcile the client.** In `web/`: `npm run gen:api`, `npm run typecheck`. Every type error is fixed in `web/` only, by a web agent dispatch that gets B6's difference list (and wires the breakdown if B4 added it). Then `npm run lint && npm test && npm run build`.
- [ ] **Step 3: Full gate.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (count recorded), web gate as above (count recorded).
- [ ] **Step 4: One whole-branch review** (opus): reviewer fixes what it finds, runs the gate, commits, reports (CLAUDE.md). The controller verifies the report — commits exist, diff matches, counts real — and does not re-review.
- [ ] **Step 5: Run it with Mohammed.** Release build; `shadows serve --node … --adapter … --harness … --debug`; Vite; walk §12.13's Phase B lines one by one with him; record in `docs/evidence/milestone1/ACCEPTANCE.md` (environment, versions, gate counts, each line pass/fail, what is not established). Update `docs/status.md`.
- [ ] **Step 6:** Push, open the PR, CI green, merge, delete the branch (CLAUDE.md branches rule).
