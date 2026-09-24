# Milestone 1 Phase A — Run over ACP (Linux, cloud container)

**Date:** 2026-09-24 (UTC)
**Status:** Every Phase A step passed against the real harness on Linux. This
is **not** the Windows run the plan's Task A5 Step 3 asks for, and it is not
Mohammed's acceptance. See "What this run does not establish".
**Code:** branch `milestone-1/harness-controls-7p9608` at `bf55377`, plus the
restart test added with this file.
**Operator:** Claude, in a Claude Code cloud session. The API steps used
`curl`. The browser steps used headless Chromium driven by Playwright. Mohammed
followed the run through screenshots in the session.

## Environment

| Component | Version |
|---|---|
| OS | Linux 6.18.44 (cloud container) |
| rustc | 1.94.1 (e408947bf 2026-03-25) |
| Claude Code CLI (agent) | 2.1.281, at `/opt/claude-code/bin/claude` |
| ACP adapter | `@agentclientprotocol/claude-agent-acp` 0.81.1, `npm ci` in `harness/claude/` |
| Node.js (adapter and web client) | v22.22.2 |
| Daemon | `target/release/shadows serve --debug --db <tmp>/shadows.sqlite3 --node <node> --adapter <harness/claude/.../dist/index.js> --harness /opt/claude-code/bin/claude` |
| Web client | `web/`, `npx vite` on `http://localhost:5173` |

The container's Claude Code is authenticated through the host's managed
provider. The adapter inherited that from the daemon's environment. Nothing
was configured for this run.

## Gates

| Gate | Exit code |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --all-targets --features test-support -- -D warnings` | 0 |
| `cargo test --features test-support` | 0 (133 passed) |
| `web/`: `npx tsc -b` | 0 |
| `web/`: `npx vitest run` | 0 (48 passed) |

## Steps

Two passes, in two threads of one project. The API pass came first, then the
browser pass.

| Step | API pass | Browser pass |
|---|---|---|
| Send a message | ✅ `Completed`; entries `UserMessage`, `AgentMessage` ("pong") | ✅ The reply streamed in; the header showed Running, then Completed |
| Stop a long reply (a 2000-word essay; Stop after about 6 s) | ✅ `Cancelled`; the partial essay stored as an `AgentMessage` | ✅ The header showed Stopped; the partial text stayed on screen |
| A command that needs permission | ✅ A `PermissionRefused` entry holding the command; the command did not run | ✅ The same. Claude first asked for confirmation; after "yes" the refusal was recorded and Claude said it would not retry |
| Ctrl+C the daemon | ✅ `shutdown.recorded stop_kind=Graceful`; no adapter process left | ✅ The same |
| Restart, then ask what the conversation said earlier | ✅ "BANANA", the secret word from the first turn | ✅ "MANGO". The new adapter's `claude` child ran with `--resume=<the recorded session>` |

### Observed along the way

- **Stop was confirmed by the harness both times.** The log reads `planner.stop`,
  then `planner.turn_end subtype="error" stop_reason="cancelled"`, then
  `planner.stop: the turn named its own ending`. So this run exercised the
  `ResolvedByTurn` path. The adapter survived the Stop, as §12.3 intends. The
  terminate-after-`cancel_wait` path was not exercised against the real harness.
  `fake_acp` covers it in `tests/planner_turn.rs`.
- **Read-only commands never ask.** `echo probe` ran with no permission request,
  because Claude Code allows read-only commands without asking. Refusal is only
  observable with a command that writes; `rm -rf … && git init …` was used.
- **The interface does not yet say "refused".** A `PermissionRefused` entry
  renders as a small grey line holding only the command. Tool entries render as
  the literal text `[tool: <title>]`. Both are presentation, owned by plan Task
  W4 (refused permissions) and the Phase B client work.
- **One false alarm, recorded so it is not repeated.** A process count taken
  with `grep -c claude-agent-acp` in the same shell that had just started the
  daemon counted that shell's own command line. `ps` showed no surviving
  adapter.

## What this run does not establish

- **Windows.** The plan's acceptance run is on Mohammed's Windows machine,
  where Milestone 0's containment (the Job Object) was proven. Phase B starts
  only after that run.
- **Linux parent-death containment.** The daemon was always stopped with
  Ctrl+C. A daemon killed mid-turn on Linux was not tried; spec §1.5's OPEN
  block still stands.
- **A human in the browser.** Playwright drove the page. Rendering, streaming
  and Stop were checked from screenshots, not by hand.
