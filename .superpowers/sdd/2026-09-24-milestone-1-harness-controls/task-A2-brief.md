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
        /// A `tool_call` or `tool_call_update`. The first title is generic
        /// ("Terminal"); later updates carry the real one and the final
        /// `status` (`completed` / `failed`). Absent fields did not change.
        ToolCall { id: String, title: Option<String>, status: Option<String> },
        /// A permission request Shadows refused (spec §12.2).
        PermissionRefused { title: String },
        /// Context and, when the adapter forwarded it, the account's rate-limit report.
        Usage { used: u64, size: u64, model: Option<String>, rate_limit: Option<serde_json::Value> },
        /// The complete set of config options, raw; Phase B parses them.
        Options(serde_json::Value),
    }
    ```
  - `agent::acp::SessionStart { New, Resume(String), Fork(String) }` — `Fork(src)` sends `session/fork` on `src`, then `session/resume` on the id it answers, because a fork's id is not live until resumed (ACP_PROBE §6); `Opened` then carries the fork's id and the resume's options.
  - `agent::acp::Opened { pub session_id: String, pub options: serde_json::Value }`
  - `agent::acp::Connection` (Clone): `open(handle: &mut ProcessHandle, events: mpsc::UnboundedSender<HarnessEvent>) -> Result<Connection, AcpError>` (takes stdio, spawns the connection task, sends `initialize`); `start_session(&self, cwd: &Path, how: SessionStart) -> Result<Opened, AcpError>`; `set_option(&self, session: &str, config_id: &str, value: &str) -> Result<serde_json::Value, AcpError>` (returns the complete option set); `prompt(&self, session: &str, text: &str) -> Result<TurnEnd, AcpError>`; `cancel(&self, session: &str)`.
  - `agent::acp::TurnEnd { Ended, Cancelled, Refused(String) }` — `end_turn` → `Ended`; `cancelled` → `Cancelled`; `max_tokens`, `max_turn_requests`, `refusal` → `Refused(<reason>)`.
  - `agent::acp::AcpError { Closed, Rpc(String) }` — `Closed` whenever the connection is gone (the process exited or was killed).

- [ ] **Step 1: The fake agent.** `src/bin/fake_acp.rs` is an ACP agent built on the crate's agent side (`Agent.builder().on_receive_request(..).connect_to(Stdio::new())`, as the crate's `examples/simple_agent.rs`). It ignores its first argument (so tests configure `--node <fake_acp> --adapter anything`). It advertises the fork capability, returns these config options on every session answer, and applies `session/set_config_option` to them:

```text
model:         fake-large (efforts low, high, max; default high; auto mode)   ← the default
               fake-small (efforts low, high; default high; no auto mode)
               fake-tiny  (no thought_level option at all; no auto mode)
               fake-locked (listed; selecting it is refused:
                            "Usage credits are required for this model · model not changed")
thought_level: the current model's efforts; the option is absent for fake-tiny
mode:          default | acceptEdits | plan | auto | bypassPermissions   (all listed for every model)
```

It mirrors what ACP_PROBE measured on the real adapter: every new or resumed session starts at `fake-large` in mode `auto` (the person's defaults, whatever it ran with before); choosing a model without auto mode while the mode is `auto` moves the mode to `acceptEdits`; setting mode `auto` while such a model is current is refused with "auto mode is not available for this model".

Run as `fake_acp --version` it prints `fake-claude-1` and exits, so tests can configure it as the Claude executable too and the existing version probe reads it. Its session id is `fake-<n>`; `session/resume` of an id it did not create in this process still succeeds (a new process resumes what an old one created). `session/fork` answers `fork-of-<id>` **without making it live**: a prompt to it before `session/resume` on that id is refused "Session not found", as the real adapter does. Prompts, by text:

| prompt | behaviour |
|---|---|
| anything not below | chunks `hello ` and `from fake_acp` under message id `m1`, then `end_turn` |
| `two-messages` | `first` under `m1`; a `tool_call` titled `Terminal` (`pending`); a `tool_call_update` retitling it `Read notes.md`; a `tool_call_update` with `status: completed`; `second` under `m2`; `end_turn` |
| `report` | one message whose text is JSON `{ "cwd", "session", "how": "new"/"resume"/"fork", "model", "effort", "mode", "claude": $CLAUDE_CODE_EXECUTABLE }` (`how` is `fork` for a `fork-of-` id), `end_turn` |
| `/context` | one message: a markdown table `| Category | Tokens | Percentage |` with rows `Messages 3.8k 0.4%`, `System tools 19.1k 1.9%`, `Free space 923.9k 92.4%`; the first `/context` of a process sleeps 1 s first, like the real first call |
| `hang` | one chunk, then waits; answers `cancelled` when `session/cancel` arrives |
| `ignore-cancel` | one chunk, then waits forever, ignoring `session/cancel` |
| `exit` | one chunk, then the process exits with code 3 |
| `ask-permission` | sends `session/request_permission` (title `Run echo probe`, options `allow_once`, `reject_once`), then one message `permission: <chosen option kind>`, `end_turn` |
| `usage` | `usage_update { used: 1234, size: 200000, _meta: { "_claude/model": "fake-large-answering" } }`, one message, then `usage_update { used: 1234, size: 1000000 }` with `_meta: { "_claude/rateLimit": { "unifiedWindows": { "five_hour": { "utilization": 0.25, "resetsAt": 1790212200 }, "seven_day": { "utilization": 0.5, "resetsAt": 1790542800 } } }, "_claude/model": "fake-large-answering" }`, `end_turn` |
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
    let usages: Vec<_> = drain(&mut ev).into_iter().filter_map(|e| match e {
        HarnessEvent::Usage { used, size, model, rate_limit } => Some((used, size, model, rate_limit)),
        _ => None,
    }).collect();
    assert_eq!(usages.len(), 2);
    let last = usages.last().unwrap();
    assert_eq!((last.0, last.1, last.2.as_deref()), (1234, 1_000_000, Some("fake-large-answering")));
    assert!(last.3.as_ref().unwrap()["unifiedWindows"]["seven_day"].is_object());
}

#[tokio::test]
async fn a_tool_call_reports_its_real_title_and_final_status() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    c.prompt(&s, "two-messages").await.unwrap();
    let tools: Vec<_> = drain(&mut ev).into_iter().filter_map(|e| match e {
        HarnessEvent::ToolCall { title, status, .. } => Some((title, status)),
        _ => None,
    }).collect();
    assert_eq!(tools, [
        (Some("Terminal".to_string()), Some("pending".to_string())),
        (Some("Read notes.md".to_string()), None),
        (None, Some("completed".to_string())),
    ]);
}

#[tokio::test]
async fn a_fork_is_resumed_before_it_is_answered() {
    let (_h, c, _ev) = open_fake().await;
    let src = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    let fork = c.start_session(&tmp(), SessionStart::Fork(src.clone())).await.unwrap();
    assert_eq!(fork.session_id, format!("fork-of-{src}"));
    assert_eq!(c.prompt(&fork.session_id, "hi").await.unwrap(), TurnEnd::Ended, "live after the resume");
}

#[tokio::test]
async fn a_model_the_account_cannot_use_is_refused_with_the_harness_message() {
    let (_h, c, _ev) = open_fake().await;
    let s = c.start_session(&tmp(), SessionStart::New).await.unwrap().session_id;
    let err = c.set_option(&s, "model", "fake-locked").await.unwrap_err();
    assert!(matches!(err, AcpError::Rpc(m) if m.contains("Usage credits are required")));
}
```

- [ ] **Step 3: Run** `cargo test --test acp_connection` — expected: compile failure.
- [ ] **Step 4: Implement `agent/acp.rs`.** The connection task runs the crate's client builder over the child's stdio:

```rust
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

let (stdin, stdout, stderr) = handle.take_stdio().ok_or(AcpError::Closed)?;
forward_stderr(stderr); // each line → tracing::debug!(target: "harness.stderr", ...)
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

`forward` maps `agent_message_chunk` (text blocks only) → `Chunk`, `tool_call` and `tool_call_update` → `ToolCall` (the fields each carries), `usage_update` → `Usage` (model from `_meta["_claude/model"]`, rate limit from `_meta["_claude/rateLimit"]`, ACP_PROBE §7), `config_option_update` → `Options`; every other update (`available_commands_update`, `session_info_update`, `current_mode_update`, thoughts) is dropped with a `trace!`. `title_of` is the request's `toolCall.title`. `reject` selects the option whose `kind` is `reject_once`, else `reject_always`, else answers `Cancelled`. `start_session` sends `NewSessionRequest::new(cwd)`, `ResumeSessionRequest`, or `ForkSessionRequest` followed by `ResumeSessionRequest` on the answered id, and returns the id and the last answer's `configOptions` as JSON. A JSON-RPC error answer is `AcpError::Rpc` carrying `data.details` when present, else `message`. Any send whose future fails because the connection ended is `AcpError::Closed`. Types are the crate's `schema::v1` types; read `examples/yolo_one_shot_client.rs` of the pinned version for their constructors. `agent/events.rs` owns only `HarnessEvent`; `agent/acp.rs` owns only the connection.

- [ ] **Step 5:** `cargo test --test acp_connection`, clippy, code map, owners (`agent/acp.rs`: "the ACP client connection to one adapter process"; `agent/events.rs`: "what a harness connection reports"). Commit `feat(agent): ACP client connection and the fake ACP agent (spec §12.2)`.

