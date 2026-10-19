# AYLUL on the laptop (step 2(a)) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** AYLUL desktop shows the person's Claude Code sessions, lets them answer
permission requests, and turns an auto-mode refusal into a narrow permission rule
they approve, all on one Windows laptop.

**Architecture:** A Rust `aylul-hook.exe`, registered in Claude Code's hooks,
carries each hook event over a token-guarded named pipe to a Node sidecar. The
sidecar keeps sessions, holds requests and refusals until the person answers,
writes approved rules into the project's `.claude/settings.local.json`, and stores
everything in `node:sqlite`. The Tauri app starts the sidecar, relays two screens'
calls to it over stdio, and stops it on exit.

**Tech Stack:** Rust 2021 (hook: `serde_json` only), Node 24.15+ running
TypeScript directly (type stripping), `node:net`, `node:sqlite`, `node:test`;
Tauri 2.11 and React 19 (existing desktop app).

**Spec:** `docs/aylul/specs/2026-10-13-aylul-hook-laptop-design.md`
(read it first). Evidence: `docs/evidence/nylra/2026-10-13-aylul-hook-probe.md`.

## Global Constraints

- Windows only. Node `>=24.15.0` (`node:sqlite` and `.ts` type stripping need it).
- The sidecar has **no runtime dependencies**. Dev only: `typescript@7.0.2`
  (same as the desktop app) and `@types/node` 24.x.
- `aylul-hook` depends on `serde_json` only.
- Sidecar TypeScript uses erasable syntax only (no `enum`, no `namespace`, no
  parameter properties); imports inside the sidecar end in `.ts`.
- Local cargo: `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target`,
  `CARGO_INCREMENTAL=0`. The `aylul-hook.exe` path in run steps assumes that
  target dir.
- No Docker or containers. Push with `git -c http.version=HTTP/1.1 push`. No PR,
  no merge unless the person says so.
- AYLUL never answers `allow` and never writes a rule without the person.
- A step never carries file contents, patches or tool output (spec §3).
- A proposed or approved rule is exact (spec §5): one command with no `*`, one
  file as `/path` from the project root, one `domain:host`, or one MCP tool.
  Claude Code reads `*` in a Bash rule as any text and a path without a leading
  `/` as any folder, so both are refused.
- The project is the session's root, `CLAUDE_PROJECT_DIR`, carried by
  `aylul-hook` as `project_dir`. The event's `cwd` follows `cd` into subfolders,
  whose `.claude/settings.local.json` Claude Code does not read.
- Every source module gets a colocated `contract.yaml` in the
  [template](../../../../../shadows-backend/docs/contracts/TEMPLATE.yaml)'s shape;
  `docs/aylul/codebase/README.md` gets one row per new module.
- Hold times chain inside each other: sidecar 580 s / hook 590 s / Claude Code
  600 s for `PermissionRequest`; sidecar 120 s / hook 125 s / Claude Code 130 s
  for `PermissionDenied`; other events: hook 3 s, sidecar answers at once.

## Review Focus

1. Two Claude Code sessions ask at the same time: each request gets its own id
   and answer (Task 5, `two requests are answered independently`).
2. The person approves a rule after the two-minute hold ended: the rule is still
   written, nothing is sent to the finished hook (Task 5, `a late approval still
   writes the rule`).
3. Claude ran `cd src` before the refusal: the rule still lands in the root's
   `.claude/settings.local.json`, anchored there (Task 3 `the project is the root`,
   Task 8 `a refusal is previewed and approved into the project settings`).
4. `~/.claude/settings.json` holds the person's own hooks next to AYLUL's:
   install keeps theirs and replaces only AYLUL's entries (Task 7).
5. The sidecar restarted while a hook waited, so the pipe in `hook.json` is gone:
   the hook prints nothing and exits 0 (Task 1 `connect_error_gives_no_output`,
   Task 2 round trip `no sidecar`).
6. The person edits the proposed rule into a wider one (`Bash(git *)`,
   `Edit(src/**)`, `WebFetch(domain:*)`): it is refused before anything is
   written (Task 4 `only exact rules are accepted`).

---

### Task 1: `aylul-hook` crate

**Files:**
- Create: `apps/desktop/hook/Cargo.toml`
- Create: `apps/desktop/hook/src/lib.rs`
- Create: `apps/desktop/hook/src/main.rs`
- Create: `apps/desktop/hook/contract.yaml`

**Interfaces:**
- Produces: `aylul-hook.exe`. Reads stdin (one Claude Code hook event, JSON),
  reads `%LOCALAPPDATA%\AYLUL\hook.json` (`{"pipe": "...", "token": "..."}`),
  writes `{"v":1,"token":...,"project_dir":<CLAUDE_PROJECT_DIR or omitted>,"event":<input>}\n`
  to the pipe, reads one line `{"output": <object>}` or `{}`, prints `output`
  (if any) to stdout, exits 0 always.
- `project_dir` is the session's project root, from the `CLAUDE_PROJECT_DIR`
  variable Claude Code sets for every hook. The event's own `cwd` follows `cd`
  into subfolders, so rules and sessions use `project_dir`, never `cwd`.
- A pipe busy with another hook (`ERROR_PIPE_BUSY`, 231) is retried every 20 ms,
  50 times, inside the event's deadline.

- [ ] **Step 1: Create the crate manifest**

`apps/desktop/hook/Cargo.toml`:

```toml
[package]
name = "aylul-hook"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
serde_json = "1"

[profile.release]
opt-level = "s"
strip = true

[workspace]
```

- [ ] **Step 2: Write the failing tests and the library skeleton**

`apps/desktop/hook/src/lib.rs`:

```rust
//! One agent hook call: read the event, carry it to AYLUL's sidecar, return
//! the sidecar's answer. Every failure returns `None`, so Claude Code behaves
//! as if AYLUL were absent.
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

pub struct Endpoint {
    pub pipe: String,
    pub token: String,
}

/// Reads `hook.json`. `None` when it is missing or malformed.
pub fn read_endpoint(path: &Path) -> Option<Endpoint> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    Some(Endpoint {
        pipe: value.get("pipe")?.as_str()?.to_owned(),
        token: value.get("token")?.as_str()?.to_owned(),
    })
}

/// How long one event may wait for the sidecar; shorter than Claude Code's
/// own hook timeout for that event.
pub fn deadline(event: &Value) -> Duration {
    match event.get("hook_event_name").and_then(Value::as_str) {
        Some("PermissionRequest") => Duration::from_secs(590),
        Some("PermissionDenied") => Duration::from_secs(125),
        _ => Duration::from_secs(3),
    }
}

/// Windows `ERROR_PIPE_BUSY`: every instance of the pipe is serving another hook.
const ERROR_PIPE_BUSY: i32 = 231;

/// Opens the pipe, retrying while it is busy. Any other error returns at once.
pub fn connect_retrying<S>(mut open: impl FnMut() -> io::Result<S>, tries: u32, pause: Duration) -> io::Result<S> {
    let mut left = tries;
    loop {
        match open() {
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) && left > 1 => {
                left -= 1;
                thread::sleep(pause);
            }
            result => return result,
        }
    }
}

/// Sends one request line and returns the reply's `output`, if any.
pub fn exchange<S: Read + Write>(mut stream: S, token: &str, project_dir: Option<&str>, event: Value) -> Option<String> {
    let mut request = json!({ "v": 1, "token": token, "event": event });
    if let Some(dir) = project_dir {
        request["project_dir"] = Value::from(dir);
    }
    let mut line = request.to_string();
    line.push('\n');
    stream.write_all(line.as_bytes()).ok()?;
    stream.flush().ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    let reply: Value = serde_json::from_str(reply.trim()).ok()?;
    match reply.get("output") {
        Some(output) if output.is_object() => Some(output.to_string()),
        _ => None,
    }
}

/// The whole call, cut at `wait`.
pub fn run_within<S, C>(
    input: &str,
    endpoint: Option<Endpoint>,
    project_dir: Option<String>,
    connect: C,
    wait: Duration,
) -> Option<String>
where
    S: Read + Write + Send + 'static,
    C: FnOnce(&str) -> io::Result<S> + Send + 'static,
{
    let event: Value = serde_json::from_str(input).ok()?;
    let endpoint = endpoint?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = connect(&endpoint.pipe)
            .ok()
            .and_then(|stream| exchange(stream, &endpoint.token, project_dir.as_deref(), event));
        let _ = tx.send(result);
    });
    rx.recv_timeout(wait).ok().flatten()
}

/// The whole call with the event's own deadline.
pub fn run<S, C>(input: &str, endpoint: Option<Endpoint>, project_dir: Option<String>, connect: C) -> Option<String>
where
    S: Read + Write + Send + 'static,
    C: FnOnce(&str) -> io::Result<S> + Send + 'static,
{
    let wait = serde_json::from_str::<Value>(input)
        .map(|event| deadline(&event))
        .unwrap_or(Duration::from_secs(3));
    run_within(input, endpoint, project_dir, connect, wait)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Fake {
        reply: io::Cursor<Vec<u8>>,
        sent: Arc<Mutex<Vec<u8>>>,
    }
    impl Read for Fake {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.reply.read(buf)
        }
    }
    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.sent.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    const EVENT: &str = r#"{"session_id":"s1","hook_event_name":"Stop"}"#;

    fn endpoint() -> Option<Endpoint> {
        Some(Endpoint { pipe: r"\\.\pipe\x".into(), token: "t0k".into() })
    }

    fn fake(reply: &str) -> (Fake, Arc<Mutex<Vec<u8>>>) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        (Fake { reply: io::Cursor::new(reply.as_bytes().to_vec()), sent: sent.clone() }, sent)
    }

    #[test]
    fn no_endpoint_gives_no_output() {
        let (stream, _) = fake("{\"output\":{\"a\":1}}\n");
        assert_eq!(run(EVENT, None, None, move |_| Ok(stream)), None);
    }

    #[test]
    fn malformed_input_gives_no_output() {
        let (stream, _) = fake("{\"output\":{\"a\":1}}\n");
        assert_eq!(run("not json", endpoint(), None, move |_| Ok(stream)), None);
    }

    #[test]
    fn connect_error_gives_no_output() {
        let failing = |_: &str| -> io::Result<Fake> { Err(io::Error::from(io::ErrorKind::NotFound)) };
        assert_eq!(run(EVENT, endpoint(), None, failing), None);
    }

    #[test]
    fn reply_output_is_returned() {
        let reply = r#"{"output":{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}}"#;
        let (stream, _) = fake(&format!("{reply}\n"));
        let out = run(EVENT, endpoint(), None, move |_| Ok(stream)).expect("output");
        let expected: Value = serde_json::from_str(reply).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), expected["output"]);
    }

    #[test]
    fn empty_reply_gives_no_output() {
        let (stream, _) = fake("{}\n");
        assert_eq!(run(EVENT, endpoint(), None, move |_| Ok(stream)), None);
    }

    #[test]
    fn request_carries_version_token_and_event() {
        let (stream, sent) = fake("{}\n");
        run(EVENT, endpoint(), Some(r"C:\work\proj".into()), move |_| Ok(stream));
        let line = String::from_utf8(sent.lock().unwrap().clone()).unwrap();
        assert!(line.ends_with('\n'));
        let request: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(request["v"], 1);
        assert_eq!(request["token"], "t0k");
        assert_eq!(request["project_dir"], r"C:\work\proj");
        assert_eq!(request["event"]["hook_event_name"], "Stop");
    }

    #[test]
    fn no_project_dir_is_left_out() {
        let (stream, sent) = fake("{}\n");
        run(EVENT, endpoint(), None, move |_| Ok(stream));
        let line = String::from_utf8(sent.lock().unwrap().clone()).unwrap();
        let request: Value = serde_json::from_str(line.trim()).unwrap();
        assert!(request.get("project_dir").is_none());
    }

    #[test]
    fn a_busy_pipe_is_retried_and_other_errors_are_not() {
        let mut busy = 2;
        let opened = connect_retrying(
            || {
                if busy > 0 {
                    busy -= 1;
                    Err(io::Error::from_raw_os_error(ERROR_PIPE_BUSY))
                } else {
                    Ok(())
                }
            },
            50,
            Duration::from_millis(1),
        );
        assert!(opened.is_ok());
        let mut calls = 0;
        let missing = connect_retrying(
            || -> io::Result<()> {
                calls += 1;
                Err(io::Error::from(io::ErrorKind::NotFound))
            },
            50,
            Duration::from_millis(1),
        );
        assert!(missing.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn stalled_sidecar_is_cut_at_the_deadline() {
        let stalled = |_: &str| -> io::Result<Fake> {
            thread::sleep(Duration::from_secs(5));
            Err(io::Error::from(io::ErrorKind::TimedOut))
        };
        let started = std::time::Instant::now();
        assert_eq!(run_within(EVENT, endpoint(), None, stalled, Duration::from_millis(100)), None);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn deadlines_follow_the_event() {
        let of = |name: &str| deadline(&json!({ "hook_event_name": name }));
        assert_eq!(of("PermissionRequest"), Duration::from_secs(590));
        assert_eq!(of("PermissionDenied"), Duration::from_secs(125));
        assert_eq!(of("PostToolUse"), Duration::from_secs(3));
    }

    #[test]
    fn endpoint_file_is_read_or_ignored() {
        let dir = std::env::temp_dir().join(format!("aylul-hook-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.json");
        std::fs::write(&good, r#"{"pipe":"\\\\.\\pipe\\p","token":"t"}"#).unwrap();
        let bad = dir.join("bad.json");
        std::fs::write(&bad, "{").unwrap();
        let read = read_endpoint(&good).expect("endpoint");
        assert_eq!(read.pipe, r"\\.\pipe\p");
        assert_eq!(read.token, "t");
        assert!(read_endpoint(&bad).is_none());
        assert!(read_endpoint(&dir.join("missing.json")).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
```

`apps/desktop/hook/src/main.rs`:

```rust
use std::io::Read;

fn main() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let endpoint = std::env::var_os("LOCALAPPDATA")
        .map(|dir| std::path::PathBuf::from(dir).join("AYLUL").join("hook.json"))
        .and_then(|path| aylul_hook::read_endpoint(&path));
    let project_dir = std::env::var("CLAUDE_PROJECT_DIR").ok().filter(|dir| !dir.is_empty());
    let open = |pipe: &str| {
        let pipe = pipe.to_owned();
        aylul_hook::connect_retrying(
            || std::fs::OpenOptions::new().read(true).write(true).open(&pipe),
            50,
            std::time::Duration::from_millis(20),
        )
    };
    if let Some(output) = aylul_hook::run(&input, endpoint, project_dir, open) {
        print!("{output}");
    }
}
```

- [ ] **Step 3: Run the tests**

Run (from `apps/desktop`):
`cargo test --manifest-path hook/Cargo.toml`
Expected: 11 passed. If a test fails, fix `lib.rs`, not the test.

- [ ] **Step 4: Format, lint, build release**

Run:
`cargo fmt --manifest-path hook/Cargo.toml` then
`cargo clippy --manifest-path hook/Cargo.toml --all-targets -- -D warnings` then
`cargo build --release --manifest-path hook/Cargo.toml`
Expected: no warnings; `C:/Users/Mohammed/AppData/Local/shadows-target/release/aylul-hook.exe` exists.

- [ ] **Step 5: Contract**

`apps/desktop/hook/contract.yaml`:

```yaml
name: aylul-hook
version: 1
status: current
source: apps/desktop/hook
responsibility: Carry one Claude Code hook event to AYLUL's sidecar and print its answer.
public_api:
  run: Parse the event, take the endpoint, exchange one line each way within the event's deadline; None on any failure.
  run_within: run with an explicit deadline (tests).
  exchange: Write {"v":1,"token","project_dir"?,"event"} and a newline; read one line; return its object `output`.
  connect_retrying: Retry opening while the pipe is busy (231); any other error returns at once.
  deadline: 590 s PermissionRequest, 125 s PermissionDenied, 3 s otherwise.
  read_endpoint: Read {"pipe","token"} from hook.json; None when missing or malformed.
obligations:
  - rule: No endpoint, malformed input, a connect error or an empty reply print nothing.
    tested_by: no_endpoint_gives_no_output, malformed_input_gives_no_output, connect_error_gives_no_output, empty_reply_gives_no_output
  - rule: A stalled sidecar is cut at the deadline.
    tested_by: stalled_sidecar_is_cut_at_the_deadline
  - rule: The request carries version, token, CLAUDE_PROJECT_DIR when set, and the event unchanged.
    tested_by: request_carries_version_token_and_event, no_project_dir_is_left_out
  - rule: A busy pipe is retried; other connect errors are not.
    tested_by: a_busy_pipe_is_retried_and_other_errors_are_not
  - rule: The real executable round-trips with the sidecar's pipe server.
    tested_by: 'integration: apps/desktop/sidecar/src/hooks/roundtrip.test.ts'
gaps:
  - at: main
    gap: The process always exits 0; a failure is visible only as no output.
```

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/hook
git commit -m "feat(hook): aylul-hook carries one hook event to the sidecar"
```

---

### Task 2: Sidecar package, hook endpoint and pipe server

**Files:**
- Create: `apps/desktop/sidecar/package.json`, `apps/desktop/sidecar/tsconfig.json`
- Create: `apps/desktop/sidecar/src/protocol.ts`
- Create: `apps/desktop/sidecar/src/hooks/endpoint.ts`
- Create: `apps/desktop/sidecar/src/hooks/server.ts`
- Test: `apps/desktop/sidecar/src/hooks/server.test.ts`, `apps/desktop/sidecar/src/hooks/roundtrip.test.ts`
- Create: `apps/desktop/sidecar/src/hooks/contract.yaml`

**Interfaces:**
- Consumes: `aylul-hook.exe` from Task 1 (path in `AYLUL_HOOK_EXE`).
- Produces:
  - `type HookEvent` (protocol.ts, fields below), `type HookReply = { output?: Record<string, unknown> }`
  - `createEndpoint(dir: string): Endpoint`, `removeEndpoint(e: Endpoint): void`,
    `type Endpoint = { pipe: string; token: string; file: string }`
  - `type Route = (event: HookEvent, closed: AbortSignal) => Promise<HookReply>`
  - `startHookServer(pipe: string, token: string, route: Route): Promise<net.Server>`

- [ ] **Step 1: Package and TypeScript config**

`apps/desktop/sidecar/package.json`:

```json
{
  "name": "aylul-sidecar",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "engines": { "node": ">=24.15.0" },
  "scripts": {
    "start": "node src/main.ts",
    "test": "node --test --test-concurrency=1 \"src/**/*.test.ts\"",
    "typecheck": "tsc --noEmit -p ."
  },
  "devDependencies": { "typescript": "7.0.2" }
}
```

Then run in `apps/desktop/sidecar`: `npm install --save-dev --save-exact @types/node@24`
Expected: `package-lock.json` created; `@types/node` pinned to an exact 24.x version.

`apps/desktop/sidecar/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2024",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "noEmit": true,
    "allowImportingTsExtensions": true,
    "erasableSyntaxOnly": true,
    "verbatimModuleSyntax": true,
    "types": ["node"],
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

- [ ] **Step 2: Protocol types**

`apps/desktop/sidecar/src/protocol.ts`:

```ts
// What aylul-hook carries: one Claude Code hook event in, one reply out.
export type HookEvent = {
  session_id: string;
  hook_event_name: string;
  cwd?: string;
  transcript_path?: string;
  prompt_id?: string;
  permission_mode?: string;
  tool_name?: string;
  tool_input?: Record<string, unknown>;
  tool_response?: unknown;
  tool_use_id?: string;
  reason?: string;
  notification_type?: string;
  message?: string;
  last_assistant_message?: string;
  source?: string;
  /**
   * The session's project root (CLAUDE_PROJECT_DIR), added by the server from
   * aylul-hook's request; Claude Code does not send it. `cwd` follows `cd`, so
   * rules and sessions use this, never `cwd`.
   */
  project_dir?: string;
};

export type HookReply = { output?: Record<string, unknown> };
```

- [ ] **Step 3: Write the failing server tests**

`apps/desktop/sidecar/src/hooks/server.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { connect } from 'node:net';
import { randomBytes } from 'node:crypto';
import { startHookServer, type Route } from './server.ts';

const pipeName = () => String.raw`\\.\pipe\aylul-test-` + randomBytes(8).toString('hex');
const token = 'a'.repeat(64);
const event = { session_id: 's1', hook_event_name: 'Stop' };
const line = (t: string) => JSON.stringify({ v: 1, token: t, event }) + '\n';

function call(pipe: string, text: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = connect(pipe, () => socket.write(text));
    let out = '';
    socket.setEncoding('utf8');
    socket.on('data', d => { out += d; });
    socket.on('close', () => resolve(out));
    socket.on('error', reject);
  });
}

test('a request with the token gets the route reply as one line', async () => {
  const pipe = pipeName();
  const server = await startHookServer(pipe, token, async () => ({ output: { ok: true } }));
  try {
    assert.deepEqual(JSON.parse(await call(pipe, line(token))), { output: { ok: true } });
  } finally { server.close(); }
});

test('a request without the right token is dropped unanswered', async () => {
  const pipe = pipeName();
  let called = false;
  const server = await startHookServer(pipe, token, async () => { called = true; return {}; });
  try {
    assert.equal(await call(pipe, line('b'.repeat(64))), '');
    assert.equal(called, false);
  } finally { server.close(); }
});

test('a malformed line is dropped unanswered', async () => {
  const pipe = pipeName();
  const server = await startHookServer(pipe, token, async () => ({}));
  try {
    assert.equal(await call(pipe, '{not json\n'), '');
  } finally { server.close(); }
});

test('the project root from aylul-hook reaches the route; one sent inside the event does not', async () => {
  const pipe = pipeName();
  const seen: Array<string | undefined> = [];
  const server = await startHookServer(pipe, token, async e => { seen.push(e.project_dir); return {}; });
  try {
    await call(pipe, JSON.stringify({ v: 1, token, project_dir: 'C:\\proj', event }) + '\n');
    await call(pipe, JSON.stringify({ v: 1, token, event: { ...event, project_dir: 'C:\\other' } }) + '\n');
    assert.deepEqual(seen, ['C:\\proj', undefined]);
  } finally { server.close(); }
});

test('a failing route answers with no decision', async () => {
  const pipe = pipeName();
  const server = await startHookServer(pipe, token, async () => { throw new Error('boom'); });
  try {
    assert.deepEqual(JSON.parse(await call(pipe, line(token))), {});
  } finally { server.close(); }
});

test('the route learns when the hook hangs up before the answer', async () => {
  const pipe = pipeName();
  let aborted: Promise<void> | undefined;
  const route: Route = (_event, closed) => {
    aborted = new Promise(resolve => closed.addEventListener('abort', () => resolve()));
    return new Promise(() => {});
  };
  const server = await startHookServer(pipe, token, route);
  try {
    await new Promise<void>(resolve => {
      const socket = connect(pipe, () => {
        socket.write(line(token));
        setTimeout(() => { socket.destroy(); resolve(); }, 100);
      });
    });
    assert.ok(aborted);
    await aborted;
  } finally { server.close(); }
});
```

Run: `npm test` (in `apps/desktop/sidecar`). Expected: FAIL, `./server.ts` not found.

- [ ] **Step 4: Endpoint and server**

`apps/desktop/sidecar/src/hooks/endpoint.ts`:

```ts
import { randomBytes } from 'node:crypto';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export type Endpoint = { pipe: string; token: string; file: string };

/**
 * A fresh pipe name and token for this sidecar run, written to `dir/hook.json`.
 * `dir` is under %LOCALAPPDATA%, which only the person's Windows account can
 * read; the token, not the pipe's access list, keeps other accounts out.
 */
export function createEndpoint(dir: string): Endpoint {
  mkdirSync(dir, { recursive: true });
  const pipe = String.raw`\\.\pipe\aylul-` + randomBytes(16).toString('hex');
  const token = randomBytes(32).toString('hex');
  const file = join(dir, 'hook.json');
  writeFileSync(file, JSON.stringify({ pipe, token }));
  return { pipe, token, file };
}

export function removeEndpoint(endpoint: Endpoint): void {
  rmSync(endpoint.file, { force: true });
}
```

`apps/desktop/sidecar/src/hooks/server.ts`:

```ts
import { createServer, type Server, type Socket } from 'node:net';
import { timingSafeEqual } from 'node:crypto';
import type { HookEvent, HookReply } from '../protocol.ts';

/** Answers one hook event. `closed` aborts if the hook hangs up before the answer. */
export type Route = (event: HookEvent, closed: AbortSignal) => Promise<HookReply>;

const MAX_LINE = 1024 * 1024;

export function startHookServer(pipe: string, token: string, route: Route): Promise<Server> {
  const expected = Buffer.from(token);
  const server = createServer(socket => serve(socket, expected, route));
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(pipe, () => {
      server.off('error', reject);
      resolve(server);
    });
  });
}

function serve(socket: Socket, expected: Buffer, route: Route): void {
  const closed = new AbortController();
  let buffer = '';
  let handled = false;
  socket.setEncoding('utf8');
  socket.on('error', () => {});
  socket.on('close', () => closed.abort());
  socket.on('data', chunk => {
    if (handled) return;
    buffer += chunk;
    if (buffer.length > MAX_LINE) { socket.destroy(); return; }
    const end = buffer.indexOf('\n');
    if (end < 0) return;
    handled = true;
    const event = parse(buffer.slice(0, end), expected);
    if (!event) { socket.destroy(); return; }
    route(event, closed.signal)
      .catch((): HookReply => ({}))
      .then(reply => { if (!socket.destroyed) socket.end(JSON.stringify(reply) + '\n'); });
  });
}

function parse(line: string, expected: Buffer): HookEvent | undefined {
  try {
    const value = JSON.parse(line);
    const got = Buffer.from(typeof value?.token === 'string' ? value.token : '');
    if (value?.v !== 1 || got.length !== expected.length || !timingSafeEqual(got, expected)) return undefined;
    const event = value.event;
    if (typeof event?.hook_event_name !== 'string' || typeof event?.session_id !== 'string') return undefined;
    const projectDir = typeof value.project_dir === 'string' && value.project_dir ? value.project_dir : undefined;
    return { ...event, project_dir: projectDir } as HookEvent;
  } catch {
    return undefined;
  }
}
```

- [ ] **Step 5: Run the server tests**

Run: `npm test`. Expected: 6 passed.

- [ ] **Step 6: Round trip with the real hook**

`apps/desktop/sidecar/src/hooks/roundtrip.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createEndpoint, removeEndpoint } from './endpoint.ts';
import { startHookServer } from './server.ts';

const exe = process.env.AYLUL_HOOK_EXE;
const event = JSON.stringify({ session_id: 's1', hook_event_name: 'Stop' });

function runHook(localAppData: string): Promise<{ code: number | null; stdout: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(exe!, [], { env: { ...process.env, LOCALAPPDATA: localAppData, CLAUDE_PROJECT_DIR: 'C:\\proj-x' } });
    let stdout = '';
    child.stdout.setEncoding('utf8').on('data', d => { stdout += d; });
    child.on('error', reject);
    child.on('close', code => resolve({ code, stdout }));
    child.stdin.end(event);
  });
}

test('aylul-hook.exe carries an event and prints the answer', { skip: !exe && 'set AYLUL_HOOK_EXE' }, async () => {
  const home = mkdtempSync(join(tmpdir(), 'aylul-'));
  const endpoint = createEndpoint(join(home, 'AYLUL'));
  const server = await startHookServer(endpoint.pipe, endpoint.token,
    async e => ({ output: { got: e.hook_event_name, project: e.project_dir } }));
  try {
    const result = await runHook(home);
    assert.equal(result.code, 0);
    assert.deepEqual(JSON.parse(result.stdout), { got: 'Stop', project: 'C:\\proj-x' });
  } finally {
    server.close();
    removeEndpoint(endpoint);
    rmSync(home, { recursive: true, force: true });
  }
});

test('aylul-hook.exe with no sidecar prints nothing and exits 0', { skip: !exe && 'set AYLUL_HOOK_EXE' }, async () => {
  const home = mkdtempSync(join(tmpdir(), 'aylul-'));
  try {
    const result = await runHook(home);
    assert.equal(result.code, 0);
    assert.equal(result.stdout, '');
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});
```

Run (PowerShell, in `apps/desktop/sidecar`):
`$env:AYLUL_HOOK_EXE='C:/Users/Mohammed/AppData/Local/shadows-target/release/aylul-hook.exe'; npm test`
Expected: 8 passed, 0 skipped. CI sets the variable too (Task 11), so the round
trip is never skipped there.

- [ ] **Step 7: Typecheck**

Run: `npm run typecheck`. Expected: no errors.

- [ ] **Step 8: Contract**

`apps/desktop/sidecar/src/hooks/contract.yaml` (Task 8 adds `route`):

```yaml
name: sidecar-hook-endpoint
version: 1
status: current
source: apps/desktop/sidecar/src/hooks
responsibility: Accept aylul-hook connections on a per-run pipe and hand each event to a route.
public_api:
  createEndpoint: New random pipe name and 32-byte token, written to hook.json under the given directory.
  removeEndpoint: Delete hook.json.
  startHookServer: Listen on the pipe; first line must carry v 1 and the token; set event.project_dir from the line's project_dir; reply one line with the route's answer.
obligations:
  - rule: A line without the right token, or malformed, is dropped unanswered.
    tested_by: a request without the right token is dropped unanswered, a malformed line is dropped unanswered
  - rule: Only the request's top-level project_dir (from CLAUDE_PROJECT_DIR) sets event.project_dir.
    tested_by: the project root from aylul-hook reaches the route; one sent inside the event does not
  - rule: A failing route answers {} (no decision).
    tested_by: a failing route answers with no decision
  - rule: The route's signal aborts when the hook hangs up first.
    tested_by: the route learns when the hook hangs up before the answer
  - rule: The real aylul-hook.exe round-trips, and prints nothing without a sidecar.
    tested_by: aylul-hook.exe carries an event and prints the answer, aylul-hook.exe with no sidecar prints nothing and exits 0
gaps:
  - at: createEndpoint
    gap: The pipe keeps Node's default access list; only the token in a user-readable file keeps other accounts out.
```

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/sidecar
git commit -m "feat(sidecar): token-guarded hook pipe, proven with aylul-hook.exe"
```

---

### Task 3: Sessions and trimmed steps

**Files:**
- Create: `apps/desktop/sidecar/src/sessions/steps.ts`, `apps/desktop/sidecar/src/sessions/session.ts`
- Test: `apps/desktop/sidecar/src/sessions/sessions.test.ts`
- Create: `apps/desktop/sidecar/src/sessions/contract.yaml`

**Interfaces:**
- Consumes: `HookEvent` (Task 2).
- Produces:
  - `type Step = { at: string; tool: string; target: string; summary: string }`
  - `trimStep(event: HookEvent, at: string): Step`, `oneLine(text: string, max?: number): string`
  - `type SessionState = 'running' | 'waiting' | 'ended'`
  - `type Session = { id: string; agent: 'claude_code'; project: string; state: SessionState; startedAt: string; updatedAt: string; lastStep: Step | null; lastMessage: string | null }`
  - `applyEvent(current: Session | undefined, event: HookEvent, at: string): { session: Session; step?: Step }`

- [ ] **Step 1: Write the failing tests**

`apps/desktop/sidecar/src/sessions/sessions.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { trimStep, oneLine } from './steps.ts';
import { applyEvent, MAX_MESSAGE } from './session.ts';

const at = '2026-10-13T19:00:00.000Z';
const write = {
  session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', tool_name: 'Write',
  tool_input: { file_path: 'C:\\p\\a.txt', content: 'SECRET-CONTENT' },
  tool_response: { content: 'SECRET-CONTENT', originalFile: 'OLD-SECRET', structuredPatch: [{ lines: ['+SECRET'] }] },
};

test('a step keeps tool and path and drops every content field', () => {
  const step = trimStep(write, at);
  assert.deepEqual(step, { at, tool: 'Write', target: 'C:\\p\\a.txt', summary: 'Write C:\\p\\a.txt' });
  assert.equal(JSON.stringify(step).includes('SECRET'), false);
});

test('a Bash step is the command on one line with its description', () => {
  const step = trimStep({ session_id: 's1', hook_event_name: 'PostToolUse', tool_name: 'Bash',
    tool_input: { command: 'npm\n  test', description: 'Run tests' }, tool_response: { stdout: 'SECRET' } }, at);
  assert.deepEqual(step, { at, tool: 'Bash', target: 'npm test', summary: 'Run tests' });
});

test('oneLine collapses whitespace and cuts long text', () => {
  assert.equal(oneLine(' a\n\tb '), 'a b');
  assert.equal(oneLine('x'.repeat(200), 10), 'x'.repeat(9) + '…');
});

test('a session moves through its states and keeps the last step and message', () => {
  let { session } = applyEvent(undefined, { session_id: 's1', hook_event_name: 'SessionStart', cwd: 'C:\\p' }, at);
  assert.equal(session.state, 'running');
  assert.equal(session.project, 'C:\\p');
  const afterStep = applyEvent(session, write, at);
  assert.equal(afterStep.step?.tool, 'Write');
  assert.equal(afterStep.session.lastStep?.target, 'C:\\p\\a.txt');
  session = applyEvent(afterStep.session, { session_id: 's1', hook_event_name: 'Notification', notification_type: 'permission_prompt' }, at).session;
  assert.equal(session.state, 'waiting');
  session = applyEvent(session, { session_id: 's1', hook_event_name: 'Stop', last_assistant_message: 'Done.' }, at).session;
  assert.equal(session.lastMessage, 'Done.');
  assert.equal(session.state, 'waiting');
  session = applyEvent(session, { session_id: 's1', hook_event_name: 'SessionEnd' }, at).session;
  assert.equal(session.state, 'ended');
});

test('an auth_success notice does not mark the session waiting', () => {
  const { session } = applyEvent(undefined, { session_id: 's1', hook_event_name: 'Notification', notification_type: 'auth_success' }, at);
  assert.equal(session.state, 'running');
});

test('the last message is kept whole up to its limit', () => {
  const long = 'a'.repeat(MAX_MESSAGE + 50);
  const { session } = applyEvent(undefined, { session_id: 's1', hook_event_name: 'Stop', last_assistant_message: long }, at);
  assert.equal(session.lastMessage?.length, MAX_MESSAGE);
});

test('the project is the root from aylul-hook, not the folder Claude moved into', () => {
  const first = applyEvent(undefined, { session_id: 's1', hook_event_name: 'SessionStart', cwd: 'C:\\p\\src', project_dir: 'C:\\p' }, at).session;
  assert.equal(first.project, 'C:\\p');
  const later = applyEvent(first, { session_id: 's1', hook_event_name: 'Stop', cwd: 'C:\\p\\src\\deep' }, at).session;
  assert.equal(later.project, 'C:\\p');
});

test('applyEvent does not change the session it was given', () => {
  const first = applyEvent(undefined, { session_id: 's1', hook_event_name: 'SessionStart', cwd: 'C:\\p' }, at).session;
  applyEvent(first, { session_id: 's1', hook_event_name: 'SessionEnd' }, at);
  assert.equal(first.state, 'running');
});
```

Run: `npm test`. Expected: FAIL, `./steps.ts` not found.

- [ ] **Step 2: Implement**

`apps/desktop/sidecar/src/sessions/steps.ts`:

```ts
import type { HookEvent } from '../protocol.ts';

export type Step = { at: string; tool: string; target: string; summary: string };

const TARGET_KEYS = ['file_path', 'notebook_path', 'path', 'url', 'pattern'];

/** Tool name, its target and one line. Never file contents, patches or tool output. */
export function trimStep(event: HookEvent, at: string): Step {
  const tool = event.tool_name ?? 'unknown';
  const input = event.tool_input ?? {};
  if (tool === 'Bash') {
    const command = oneLine(String(input.command ?? ''));
    const description = typeof input.description === 'string' ? oneLine(input.description) : '';
    return { at, tool, target: command, summary: description || command };
  }
  const key = TARGET_KEYS.find(k => typeof input[k] === 'string');
  const target = key ? oneLine(String(input[key])) : '';
  return { at, tool, target, summary: target ? `${tool} ${target}` : tool };
}

export function oneLine(text: string, max = 160): string {
  const line = text.replace(/\s+/g, ' ').trim();
  return line.length > max ? line.slice(0, max - 1) + '…' : line;
}
```

`apps/desktop/sidecar/src/sessions/session.ts`:

```ts
import type { HookEvent } from '../protocol.ts';
import { trimStep, type Step } from './steps.ts';

export type SessionState = 'running' | 'waiting' | 'ended';
export type Session = {
  id: string;
  agent: 'claude_code';
  project: string;
  state: SessionState;
  startedAt: string;
  updatedAt: string;
  lastStep: Step | null;
  lastMessage: string | null;
};

export const MAX_MESSAGE = 2000;
const WAITING_NOTICES = new Set(['permission_prompt', 'idle_prompt', 'elicitation_dialog', 'agent_needs_input']);

/** The session after one hook event, and the step it added, if any. */
export function applyEvent(current: Session | undefined, event: HookEvent, at: string): { session: Session; step?: Step } {
  const session: Session = current
    ? { ...current, updatedAt: at }
    : { id: event.session_id, agent: 'claude_code', project: '', state: 'running',
        startedAt: at, updatedAt: at, lastStep: null, lastMessage: null };
  // The project root wins; cwd (which follows `cd`) only fills in when the root never arrived.
  if (event.project_dir) session.project = event.project_dir;
  else if (!session.project && event.cwd) session.project = event.cwd;
  switch (event.hook_event_name) {
    case 'SessionStart':
      session.state = 'running';
      break;
    case 'PostToolUse': {
      const step = trimStep(event, at);
      session.lastStep = step;
      session.state = 'running';
      return { session, step };
    }
    case 'PermissionRequest':
      session.state = 'waiting';
      break;
    case 'Notification':
      if (WAITING_NOTICES.has(event.notification_type ?? '')) session.state = 'waiting';
      break;
    case 'Stop':
      session.state = 'waiting';
      if (typeof event.last_assistant_message === 'string') session.lastMessage = clip(event.last_assistant_message);
      break;
    case 'SessionEnd':
      session.state = 'ended';
      break;
  }
  return { session };
}

function clip(text: string): string {
  return text.length > MAX_MESSAGE ? text.slice(0, MAX_MESSAGE - 1) + '…' : text;
}
```

- [ ] **Step 3: Run tests and typecheck**

Run: `npm test` then `npm run typecheck`. Expected: all pass (16 tests so far with
`AYLUL_HOOK_EXE` set).

- [ ] **Step 4: Contract**

`apps/desktop/sidecar/src/sessions/contract.yaml`:

```yaml
name: sidecar-sessions
version: 1
status: current
source: apps/desktop/sidecar/src/sessions
responsibility: Turn hook events into session state and trimmed steps.
public_api:
  trimStep: Tool, target (path, URL, pattern or one-line Bash command) and a one-line summary.
  oneLine: Collapse whitespace; cut at max with an ellipsis.
  applyEvent: New session object after one event; never mutates the given one.
obligations:
  - rule: No content, patch or tool output field reaches a step.
    tested_by: a step keeps tool and path and drops every content field, a Bash step is the command on one line with its description
  - rule: SessionStart/PostToolUse running; PermissionRequest, waiting notices and Stop waiting; SessionEnd ended.
    tested_by: a session moves through its states and keeps the last step and message, an auth_success notice does not mark the session waiting
  - rule: The last message is kept up to MAX_MESSAGE characters.
    tested_by: the last message is kept whole up to its limit
  - rule: The session's project is project_dir; cwd only fills in when it never arrived.
    tested_by: the project is the root from aylul-hook, not the folder Claude moved into
gaps:
  - at: trimStep
    gap: A Bash command is kept as typed, so a secret written inline in a command is stored on the laptop.
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/sidecar/src/sessions
git commit -m "feat(sidecar): sessions and steps trimmed of file contents"
```

---

### Task 4: Narrow rules and the settings file writer

**Files:**
- Create: `apps/desktop/sidecar/src/agents/rules.ts`, `apps/desktop/sidecar/src/agents/settings-file.ts`
- Test: `apps/desktop/sidecar/src/agents/rules.test.ts`, `apps/desktop/sidecar/src/agents/settings-file.test.ts`
- Create: `apps/desktop/sidecar/src/agents/contract.yaml` (Task 7 extends it)

**Interfaces:**
- Produces:
  - `proposeRule(tool: string, input: Record<string, unknown>, project: string): string | null`
    (`project` is the session's `project_dir`, never `cwd`)
  - `isNarrowRule(rule: string): boolean`
  - `type Plan = { path: string; before: string; after: string }`
  - `planEdit(path: string, change: (settings: Record<string, unknown>) => void): Plan`
  - `planAllowRule(path: string, rule: string): Plan`
  - `fingerprint(text: string): string` (sha-256 hex of the file text a preview saw)
  - `expectBefore(plan: Plan, seen: string): Plan` (`settings_changed` unless `fingerprint(plan.before) === seen`)
  - `writePlan(plan: Plan, backupDir: string, now: Date): string` (backup path, `''` when the file did not exist)
  - `diffLines(plan: Plan): string[]` (`'+ '` / `'- '` prefixed changed lines)
  - Errors thrown as `Error(code)`: `settings_invalid`, `settings_changed`

- [ ] **Step 1: Write the failing rule tests**

`apps/desktop/sidecar/src/agents/rules.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { proposeRule, isNarrowRule } from './rules.ts';

const project = 'C:\\work\\proj';

test('Bash and PowerShell propose the exact command', () => {
  assert.equal(proposeRule('Bash', { command: 'npm run deploy:staging' }, project), 'Bash(npm run deploy:staging)');
  assert.equal(proposeRule('PowerShell', { command: 'Get-ChildItem src' }, project), 'PowerShell(Get-ChildItem src)');
});

test('a command with a newline, a closing parenthesis or a * proposes nothing', () => {
  assert.equal(proposeRule('Bash', { command: 'a\nb' }, project), null);
  assert.equal(proposeRule('Bash', { command: 'echo $(date)' }, project), null);
  assert.equal(proposeRule('Bash', { command: '  ' }, project), null);
  // Claude Code reads * in a Bash rule as "any text", so the rule would cover more than this command.
  assert.equal(proposeRule('Bash', { command: 'rm -rf build/*' }, project), null);
});

test('file tools propose the path anchored at the project root', () => {
  assert.equal(proposeRule('Write', { file_path: 'C:\\work\\proj\\src\\app.ts' }, project), 'Edit(/src/app.ts)');
  // Without the leading /, Edit(a.txt) would match an a.txt in every folder.
  assert.equal(proposeRule('Edit', { file_path: 'c:\\WORK\\proj\\a.txt' }, project), 'Edit(/a.txt)');
  assert.equal(proposeRule('Read', { file_path: 'C:\\work\\proj\\x.md' }, project), 'Read(/x.md)');
});

test('a file outside the project, on another drive, or with a pattern character proposes nothing', () => {
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\other\\a.txt' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'D:\\proj\\a.txt' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\proj' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\proj\\[draft].md' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\proj\\a.txt' }, ''), null);
});

test('WebFetch proposes the host and MCP the single tool', () => {
  assert.equal(proposeRule('WebFetch', { url: 'https://example.com/a?b=1' }, project), 'WebFetch(domain:example.com)');
  assert.equal(proposeRule('WebFetch', { url: 'not a url' }, project), null);
  assert.equal(proposeRule('mcp__github__create_issue', {}, project), 'mcp__github__create_issue');
  assert.equal(proposeRule('Task', {}, project), null);
});

test('only exact rules are accepted', () => {
  for (const broad of [
    'Bash', 'Bash(*)', 'Bash(:*)', 'Bash( )', 'Bash(npm test:*)', 'Bash(git *)', 'Bash(rm -rf build/*)', 'PowerShell(*)',
    'Edit', 'Edit(**)', 'Edit(/**)', 'Edit(**/*)', 'Edit(src/**)', 'Edit(/src/**)', 'Edit(a.txt)', 'Edit(/)', 'Edit(/src/)',
    'Edit(//c/x.txt)', 'Edit(~/x.txt)', 'Edit(/../x.txt)', 'Read(/a?.md)', 'Write(/a.txt)',
    'WebFetch(domain:*)', 'WebFetch(domain:*.example.com)', 'WebFetch(example.com)',
    '*', '', 'mcp__github', 'mcp__github__*', 'Task(x)',
  ])
    assert.equal(isNarrowRule(broad), false, broad);
  for (const narrow of ['Bash(npm test)', 'PowerShell(npm test)', 'Edit(/src/app.ts)', 'Read(/docs/x.md)',
    'WebFetch(domain:example.com)', 'mcp__github__create_issue'])
    assert.equal(isNarrowRule(narrow), true, narrow);
});

test('every proposed rule is accepted as narrow', () => {
  const proposed = [
    proposeRule('Bash', { command: 'npm run deploy:staging' }, project),
    proposeRule('PowerShell', { command: 'Get-ChildItem src' }, project),
    proposeRule('Write', { file_path: 'C:\\work\\proj\\src\\app.ts' }, project),
    proposeRule('Read', { file_path: 'C:\\work\\proj\\x.md' }, project),
    proposeRule('WebFetch', { url: 'https://example.com/a' }, project),
    proposeRule('mcp__github__create_issue', {}, project),
  ];
  for (const rule of proposed) assert.equal(rule !== null && isNarrowRule(rule), true, String(rule));
});
```

Run: `npm test`. Expected: FAIL, `./rules.ts` not found.

- [ ] **Step 2: Implement rules**

`apps/desktop/sidecar/src/agents/rules.ts`:

```ts
import { win32 } from 'node:path';

const SHELL_TOOLS = new Set(['Bash', 'PowerShell']);
const FILE_TOOLS = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit', 'Read']);
/** Characters Claude Code's path rules read as a pattern, not as themselves. */
const PATTERN = /[*?[\]]/;

/**
 * The narrowest Claude Code permission rule covering this call, or null when
 * none can be proposed safely. `project` is the session's project root
 * (CLAUDE_PROJECT_DIR); a `/path` rule in settings.local.json is anchored there.
 */
export function proposeRule(tool: string, input: Record<string, unknown>, project: string): string | null {
  if (SHELL_TOOLS.has(tool)) {
    const command = typeof input.command === 'string' ? input.command.trim() : '';
    if (!command || /[\r\n*]/.test(command) || command.includes(')')) return null;
    return `${tool}(${command})`;
  }
  if (FILE_TOOLS.has(tool)) {
    const file = input.file_path ?? input.notebook_path;
    if (typeof file !== 'string' || !project) return null;
    const rel = win32.relative(project, file);
    if (!rel || rel.startsWith('..') || win32.isAbsolute(rel)) return null;
    const path = rel.split('\\').join('/');
    if (PATTERN.test(path)) return null;
    return `${tool === 'Read' ? 'Read' : 'Edit'}(/${path})`;
  }
  if (tool === 'WebFetch') {
    try { return `WebFetch(domain:${new URL(String(input.url)).hostname})`; } catch { return null; }
  }
  if (/^mcp__[^*\s()]+__[^*\s()]+$/.test(tool)) return tool;
  return null;
}

/**
 * True only for the shapes proposeRule makes: an exact command, one file
 * anchored at the project root, one host, or one MCP tool. Everything else,
 * including any wildcard, is refused; this is the only check before a rule
 * the person typed is written.
 */
export function isNarrowRule(rule: string): boolean {
  const text = rule.trim();
  if (/^mcp__[^*\s()]+__[^*\s()]+$/.test(text)) return true;
  const match = /^(\w+)\((.*)\)$/s.exec(text);
  if (!match) return false;
  const [, tool, arg] = match;
  if (!arg || arg.trim() !== arg || arg.includes(')') || /[\r\n*]/.test(arg)) return false;
  if (SHELL_TOOLS.has(tool)) return true;
  if (tool === 'Edit' || tool === 'Read')
    return /^\/[^/~]/.test(arg) && !arg.endsWith('/') && !PATTERN.test(arg) && !arg.split('/').includes('..');
  if (tool === 'WebFetch') return /^domain:[a-z0-9-]+(\.[a-z0-9-]+)+$/i.test(arg);
  return false;
}
```

Run: `npm test`. Expected: rule tests pass.

- [ ] **Step 3: Write the failing settings-file tests**

`apps/desktop/sidecar/src/agents/settings-file.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, existsSync, readdirSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { planAllowRule, writePlan, diffLines, fingerprint, expectBefore } from './settings-file.ts';

const now = new Date('2026-10-13T19:00:00.000Z');
function project() {
  const dir = mkdtempSync(join(tmpdir(), 'aylul-proj-'));
  return { dir, settings: join(dir, '.claude', 'settings.local.json'), backups: join(dir, 'backups') };
}

test('a rule is added to a missing file, with no backup', () => {
  const p = project();
  const backup = writePlan(planAllowRule(p.settings, 'Bash(npm test)'), p.backups, now);
  assert.equal(backup, '');
  assert.deepEqual(JSON.parse(readFileSync(p.settings, 'utf8')), { permissions: { allow: ['Bash(npm test)'] } });
});

test('existing settings are kept, a backup is written, and the diff shows the rule', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, JSON.stringify({ hooks: { Stop: [] }, permissions: { allow: ['Read(a.md)'], deny: ['Bash(rm:*)'] } }));
  const plan = planAllowRule(p.settings, 'Bash(npm test)');
  assert.ok(diffLines(plan).some(l => l.startsWith('+ ') && l.includes('Bash(npm test)')));
  const backup = writePlan(plan, p.backups, now);
  assert.ok(existsSync(backup));
  const after = JSON.parse(readFileSync(p.settings, 'utf8'));
  assert.deepEqual(after.permissions, { allow: ['Read(a.md)', 'Bash(npm test)'], deny: ['Bash(rm:*)'] });
  assert.deepEqual(after.hooks, { Stop: [] });
});

test('adding a rule that is already there changes nothing in the diff', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, JSON.stringify({ permissions: { allow: ['Bash(npm test)'] } }, null, 2) + '\n');
  assert.deepEqual(diffLines(planAllowRule(p.settings, 'Bash(npm test)')), []);
});

test('invalid JSON or a wrong shape is refused and the file is untouched', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  for (const text of ['{ "permissions": ', '[]', '{"permissions": []}', '{"permissions": {"allow": "x"}}']) {
    writeFileSync(p.settings, text);
    assert.throws(() => planAllowRule(p.settings, 'Bash(npm test)'), { message: 'settings_invalid' });
    assert.equal(readFileSync(p.settings, 'utf8'), text);
  }
});

test('a file changed after the preview is not overwritten', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, '{}');
  const plan = planAllowRule(p.settings, 'Bash(npm test)');
  writeFileSync(p.settings, '{"model": "x"}');
  assert.throws(() => writePlan(plan, p.backups, now), { message: 'settings_changed' });
  assert.equal(readFileSync(p.settings, 'utf8'), '{"model": "x"}');
});

test('no temporary file is left after a write', () => {
  const p = project();
  writePlan(planAllowRule(p.settings, 'Bash(npm test)'), p.backups, now);
  assert.deepEqual(readdirSync(join(p.dir, '.claude')), ['settings.local.json']);
});

test('a file holding only whitespace is treated as empty, in the plan and the diff', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, '  \n');
  const plan = planAllowRule(p.settings, 'Bash(npm test)');
  assert.ok(diffLines(plan).every(l => l.startsWith('+ ')));
});

test('approving against a preview of a file that changed since is refused', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, '{}');
  const seen = fingerprint(planAllowRule(p.settings, 'Bash(npm test)').before);
  assert.doesNotThrow(() => expectBefore(planAllowRule(p.settings, 'Bash(npm test)'), seen));
  writeFileSync(p.settings, '{"model": "x"}');
  assert.throws(() => expectBefore(planAllowRule(p.settings, 'Bash(npm test)'), seen), { message: 'settings_changed' });
});
```

Run: `npm test`. Expected: FAIL, `./settings-file.ts` not found.

- [ ] **Step 4: Implement the writer**

`apps/desktop/sidecar/src/agents/settings-file.ts`:

```ts
import { copyFileSync, existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { basename, dirname, join } from 'node:path';

export type Plan = { path: string; before: string; after: string };
type Settings = Record<string, unknown>;

const isObject = (v: unknown): v is Settings => typeof v === 'object' && v !== null && !Array.isArray(v);

/** Reads the settings file and returns the change without writing it. */
export function planEdit(path: string, change: (settings: Settings) => void): Plan {
  const before = existsSync(path) ? readFileSync(path, 'utf8') : '';
  let settings: unknown = {};
  if (before.trim()) {
    try { settings = JSON.parse(before); } catch { throw new Error('settings_invalid'); }
  }
  if (!isObject(settings)) throw new Error('settings_invalid');
  const copy = structuredClone(settings);
  change(copy);
  return { path, before, after: JSON.stringify(copy, null, 2) + '\n' };
}

/** Adds `rule` to permissions.allow, keeping everything else. */
export function planAllowRule(path: string, rule: string): Plan {
  return planEdit(path, settings => {
    const permissions = settings.permissions ?? {};
    if (!isObject(permissions)) throw new Error('settings_invalid');
    const allow = permissions.allow ?? [];
    if (!Array.isArray(allow)) throw new Error('settings_invalid');
    if (!allow.includes(rule)) allow.push(rule);
    permissions.allow = allow;
    settings.permissions = permissions;
  });
}

/** The sha-256 of the file text a preview showed; the approval sends it back. */
export function fingerprint(text: string): string {
  return createHash('sha256').update(text).digest('hex');
}

/** The plan, if the file still holds what the person previewed; settings_changed otherwise. */
export function expectBefore(plan: Plan, seen: string): Plan {
  if (fingerprint(plan.before) !== seen) throw new Error('settings_changed');
  return plan;
}

/** Backs up, then replaces the file in one rename. Refuses if it changed since the plan. */
export function writePlan(plan: Plan, backupDir: string, now: Date): string {
  const current = existsSync(plan.path) ? readFileSync(plan.path, 'utf8') : '';
  if (current !== plan.before) throw new Error('settings_changed');
  let backupPath = '';
  if (existsSync(plan.path)) {
    mkdirSync(backupDir, { recursive: true });
    const tag = createHash('sha256').update(plan.path.toLowerCase()).digest('hex').slice(0, 8);
    backupPath = join(backupDir, `${now.toISOString().replace(/[:.]/g, '-')}-${tag}-${basename(plan.path)}`);
    copyFileSync(plan.path, backupPath);
  }
  mkdirSync(dirname(plan.path), { recursive: true });
  const temp = `${plan.path}.aylul-${process.pid}.tmp`;
  writeFileSync(temp, plan.after);
  renameRetrying(temp, plan.path);
  return backupPath;
}

/**
 * Windows refuses a rename with EPERM, EACCES or EBUSY while another program
 * (Claude Code, an antivirus) has the file open; that passes within moments.
 */
function renameRetrying(from: string, to: string): void {
  for (let attempt = 1; ; attempt++) {
    try { renameSync(from, to); return; } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (attempt >= 10 || !['EPERM', 'EACCES', 'EBUSY'].includes(code ?? '')) {
        rmSync(from, { force: true });
        throw error;
      }
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
    }
  }
}

/** Changed lines only, from a longest-common-subsequence line diff. */
export function diffLines(plan: Plan): string[] {
  const a = plan.before.trim() ? JSON.stringify(JSON.parse(plan.before), null, 2).split('\n') : [];
  const b = plan.after.trimEnd().split('\n');
  const lcs: number[][] = Array.from({ length: a.length + 1 }, () => new Array<number>(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--)
    for (let j = b.length - 1; j >= 0; j--)
      lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
  const out: string[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) { i++; j++; }
    else if (lcs[i + 1][j] >= lcs[i][j + 1]) out.push('- ' + a[i++]);
    else out.push('+ ' + b[j++]);
  }
  while (i < a.length) out.push('- ' + a[i++]);
  while (j < b.length) out.push('+ ' + b[j++]);
  return out;
}
```

`diffLines` re-formats `before` the same way as `after`, so a file written with
other indentation shows only real changes.

- [ ] **Step 5: Run tests and typecheck**

Run: `npm test` then `npm run typecheck`. Expected: all pass.

- [ ] **Step 6: Contract**

`apps/desktop/sidecar/src/agents/contract.yaml`:

```yaml
name: sidecar-agent-settings
version: 1
status: current
source: apps/desktop/sidecar/src/agents
responsibility: Propose narrow permission rules and change Claude Code settings files safely.
public_api:
  proposeRule: Exact Bash or PowerShell command, Edit/Read path anchored at the project root (/path), WebFetch host, or one MCP tool; null otherwise.
  isNarrowRule: Accept only the shapes proposeRule makes; any *, ?, [ ], unanchored or directory path, ~ or // path is refused.
  planEdit: Parse the file (missing or whitespace means {}), apply a change to a copy; settings_invalid for non-object JSON.
  planAllowRule: Append the rule to permissions.allow once.
  fingerprint: sha-256 of the file text a preview showed.
  expectBefore: settings_changed unless the file still matches the previewed fingerprint.
  writePlan: settings_changed if the file differs from the plan; timestamped backup; write temp then rename, retrying a locked file.
  diffLines: Changed lines with + and - prefixes.
obligations:
  - rule: Multi-line, parenthesised, wildcard, pattern-character, outside-project and other-drive calls get no proposal.
    tested_by: a command with a newline, a closing parenthesis or a * proposes nothing, a file outside the project, on another drive, or with a pattern character proposes nothing
  - rule: Only exact rules are accepted, and every proposal is one.
    tested_by: only exact rules are accepted, every proposed rule is accepted as narrow
  - rule: Invalid settings are never written; existing keys survive; a backup precedes the write.
    tested_by: invalid JSON or a wrong shape is refused and the file is untouched, existing settings are kept, a backup is written, and the diff shows the rule
  - rule: A file changed after the preview is not overwritten.
    tested_by: a file changed after the preview is not overwritten, approving against a preview of a file that changed since is refused
gaps:
  - at: proposeRule
    gap: Whether Claude Code's Bash rule matches a compound command (a && b) as typed is not checked here.
  - at: writePlan
    gap: The retry on a locked file (EPERM, EACCES, EBUSY) is not exercised by a test.
```

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/sidecar/src/agents
git commit -m "feat(sidecar): narrow rule proposals and a backed-up settings writer"
```

---

### Task 5: Approvals

**Files:**
- Create: `apps/desktop/sidecar/src/approvals/approvals.ts`
- Test: `apps/desktop/sidecar/src/approvals/approvals.test.ts`
- Create: `apps/desktop/sidecar/src/approvals/contract.yaml`

**Interfaces:**
- Consumes: `HookEvent`, `HookReply` (Task 2); `trimStep` (Task 3); `proposeRule`, `isNarrowRule` (Task 4).
- Produces:
  - `type Pending = { id: string; kind: 'request' | 'refusal'; sessionId: string; project: string; tool: string; target: string; reason: string | null; proposedRule: string | null; at: string; held: boolean }`
  - `type Outcome = 'allowed' | 'denied' | 'answered_in_terminal' | 'timed_out' | 'listed' | 'rule_written' | 'dismissed'`
  - `type ApprovalsDeps = { now: () => Date; newId: () => string; writeRule: (project: string, rule: string, seen: string) => { ruleId: string; backupPath: string }; record: (item: Pending, outcome: Outcome) => void; requestHoldMs: number; refusalHoldMs: number }`
    (`seen` is the settings fingerprint the person previewed; `writeRule` refuses with `settings_changed` if the file moved on)
  - `class Approvals { constructor(d: ApprovalsDeps); pending(): Pending[]; request(event, closed): Promise<HookReply>; refusal(event, closed): Promise<HookReply>; answer(id: string, behavior: 'allow' | 'deny'): void; approveRule(id: string, rule: string, seen: string): { backupPath: string }; dismiss(id: string): void }`
  - `project` is the event's `project_dir` (set by the hook server), never `cwd`
  - Errors: `not_pending`, `rule_too_broad`, `project_unknown`

- [ ] **Step 1: Write the failing tests**

`apps/desktop/sidecar/src/approvals/approvals.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Approvals, type Outcome, type Pending } from './approvals.ts';

function setup(hold = { request: 200, refusal: 200 }) {
  let n = 0;
  const records: Array<[string, Outcome]> = [];
  const written: Array<[string, string]> = [];
  const approvals = new Approvals({
    now: () => new Date('2026-10-13T19:00:00.000Z'),
    newId: () => `id${++n}`,
    writeRule: (project, rule, _seen) => { written.push([project, rule]); return { ruleId: 'r1', backupPath: 'b1' }; },
    record: (item: Pending, outcome) => { records.push([item.id, outcome]); },
    requestHoldMs: hold.request,
    refusalHoldMs: hold.refusal,
  });
  return { approvals, records, written };
}
const request = { session_id: 's1', hook_event_name: 'PermissionRequest', cwd: 'C:\\p\\src', project_dir: 'C:\\p', tool_name: 'Bash', tool_input: { command: 'npm test' } };
const refusal = { session_id: 's1', hook_event_name: 'PermissionDenied', cwd: 'C:\\p\\src', project_dir: 'C:\\p', tool_name: 'Bash', tool_input: { command: 'npm run deploy' }, reason: '[Prod Deploy]' };
const open = () => new AbortController();

test('allow from AYLUL answers the hook with allow', async () => {
  const { approvals, records } = setup();
  const reply = approvals.request(request, open().signal);
  const [item] = approvals.pending();
  assert.equal(item.target, 'npm test');
  approvals.answer(item.id, 'allow');
  assert.deepEqual(await reply, { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest', decision: { behavior: 'allow' } } } });
  assert.deepEqual(records, [['id1', 'allowed']]);
  assert.deepEqual(approvals.pending(), []);
});

test('deny from AYLUL answers with deny and a message', async () => {
  const { approvals } = setup();
  const reply = approvals.request(request, open().signal);
  approvals.answer(approvals.pending()[0].id, 'deny');
  const out = (await reply).output as { hookSpecificOutput: { decision: { behavior: string; message: string } } };
  assert.equal(out.hookSpecificOutput.decision.behavior, 'deny');
  assert.ok(out.hookSpecificOutput.decision.message.length > 0);
});

test('an answer in the terminal ends the request without a decision', async () => {
  const { approvals, records } = setup();
  const hook = open();
  const reply = approvals.request(request, hook.signal);
  hook.abort();
  assert.deepEqual(await reply, {});
  assert.deepEqual(records, [['id1', 'answered_in_terminal']]);
  assert.deepEqual(approvals.pending(), []);
});

test('an unanswered request times out with no decision', async () => {
  const { approvals, records } = setup({ request: 30, refusal: 200 });
  assert.deepEqual(await approvals.request(request, open().signal), {});
  assert.deepEqual(records, [['id1', 'timed_out']]);
  assert.deepEqual(approvals.pending(), []);
});

test('two requests are answered independently', async () => {
  const { approvals } = setup();
  const first = approvals.request(request, open().signal);
  const second = approvals.request({ ...request, session_id: 's2' }, open().signal);
  const [a, b] = approvals.pending();
  approvals.answer(b.id, 'deny');
  approvals.answer(a.id, 'allow');
  assert.equal(((await first).output as any).hookSpecificOutput.decision.behavior, 'allow');
  assert.equal(((await second).output as any).hookSpecificOutput.decision.behavior, 'deny');
});

test('answering twice or an unknown id is refused', async () => {
  const { approvals } = setup();
  void approvals.request(request, open().signal);
  const id = approvals.pending()[0].id;
  approvals.answer(id, 'allow');
  assert.throws(() => approvals.answer(id, 'allow'), { message: 'not_pending' });
  assert.throws(() => approvals.answer('nope', 'deny'), { message: 'not_pending' });
});

test('a refusal carries its reason and a proposed rule', async () => {
  const { approvals } = setup();
  void approvals.refusal(refusal, open().signal);
  const [item] = approvals.pending();
  assert.equal(item.kind, 'refusal');
  assert.equal(item.reason, '[Prod Deploy]');
  assert.equal(item.proposedRule, 'Bash(npm run deploy)');
});

test('approving the rule within the hold writes it, then asks Claude to retry', async () => {
  const { approvals, written, records } = setup();
  const reply = approvals.refusal(refusal, open().signal);
  const result = approvals.approveRule(approvals.pending()[0].id, 'Bash(npm run deploy)', 'seen');
  assert.deepEqual(result, { backupPath: 'b1' });
  assert.deepEqual(written, [['C:\\p', 'Bash(npm run deploy)']]);
  assert.deepEqual(await reply, { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } });
  assert.deepEqual(records, [['id1', 'rule_written']]);
});

test('a late approval still writes the rule', async () => {
  const { approvals, written, records } = setup({ request: 200, refusal: 30 });
  assert.deepEqual(await approvals.refusal(refusal, open().signal), {});
  const [item] = approvals.pending();
  assert.equal(item.held, false);
  approvals.approveRule(item.id, 'Bash(npm run deploy)', 'seen');
  assert.deepEqual(written, [['C:\\p', 'Bash(npm run deploy)']]);
  assert.deepEqual(records, [['id1', 'listed'], ['id1', 'rule_written']]);
  assert.deepEqual(approvals.pending(), []);
});

test('a broad rule is refused and nothing is written', () => {
  const { approvals, written } = setup();
  void approvals.refusal(refusal, open().signal);
  const id = approvals.pending()[0].id;
  assert.throws(() => approvals.approveRule(id, 'Bash(*)', 'seen'), { message: 'rule_too_broad' });
  assert.deepEqual(written, []);
  assert.equal(approvals.pending().length, 1);
});

test('a refusal with no project root proposes nothing and cannot be approved', () => {
  const { approvals, written } = setup();
  const { project_dir: _root, ...withoutRoot } = refusal;
  void approvals.refusal(withoutRoot, open().signal);
  const [item] = approvals.pending();
  assert.equal(item.project, '');
  assert.equal(item.proposedRule, null);
  assert.throws(() => approvals.approveRule(item.id, 'Bash(npm run deploy)', 'seen'), { message: 'project_unknown' });
  assert.deepEqual(written, []);
});

test('a dismissed refusal leaves the list and answers a held hook with no decision', async () => {
  const { approvals, records } = setup();
  const reply = approvals.refusal(refusal, open().signal);
  approvals.dismiss(approvals.pending()[0].id);
  assert.deepEqual(await reply, {});
  assert.deepEqual(records, [['id1', 'dismissed']]);
  assert.deepEqual(approvals.pending(), []);
});
```

Run: `npm test`. Expected: FAIL, `./approvals.ts` not found.

- [ ] **Step 2: Implement**

`apps/desktop/sidecar/src/approvals/approvals.ts`:

```ts
import type { HookEvent, HookReply } from '../protocol.ts';
import { trimStep } from '../sessions/steps.ts';
import { isNarrowRule, proposeRule } from '../agents/rules.ts';

export type Pending = {
  id: string;
  kind: 'request' | 'refusal';
  sessionId: string;
  project: string;
  tool: string;
  target: string;
  reason: string | null;
  proposedRule: string | null;
  at: string;
  held: boolean;
};
export type Outcome = 'allowed' | 'denied' | 'answered_in_terminal' | 'timed_out' | 'listed' | 'rule_written' | 'dismissed';
export type ApprovalsDeps = {
  now: () => Date;
  newId: () => string;
  /** Writes the rule if the file still matches the fingerprint `seen` the person previewed. */
  writeRule: (project: string, rule: string, seen: string) => { ruleId: string; backupPath: string };
  record: (item: Pending, outcome: Outcome) => void;
  requestHoldMs: number;
  refusalHoldMs: number;
};

type Item = Pending & { reply?: (reply: HookReply) => void };

const ALLOW: HookReply = { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest', decision: { behavior: 'allow' } } } };
const DENY: HookReply = { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest',
  decision: { behavior: 'deny', message: 'The person denied this in AYLUL.' } } } };
const RETRY: HookReply = { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } };

/** Requests and refusals waiting for the person. Only the person's answer allows anything. */
export class Approvals {
  #items = new Map<string, Item>();
  #d: ApprovalsDeps;

  constructor(deps: ApprovalsDeps) { this.#d = deps; }

  pending(): Pending[] {
    return [...this.#items.values()]
      .map(({ reply: _reply, ...item }) => item)
      .sort((a, b) => a.at.localeCompare(b.at) || a.id.localeCompare(b.id));
  }

  request(event: HookEvent, closed: AbortSignal): Promise<HookReply> {
    const item = this.#item('request', event, null);
    return this.#hold(item, closed, this.#d.requestHoldMs,
      () => { this.#items.delete(item.id); this.#d.record(item, 'timed_out'); },
      () => { this.#items.delete(item.id); this.#d.record(item, 'answered_in_terminal'); });
  }

  refusal(event: HookEvent, closed: AbortSignal): Promise<HookReply> {
    // No root, no rule: it could not be written to the settings file Claude Code reads.
    const rule = event.project_dir ? proposeRule(event.tool_name ?? '', event.tool_input ?? {}, event.project_dir) : null;
    const item = this.#item('refusal', event, rule);
    const list = () => this.#d.record(item, 'listed');
    return this.#hold(item, closed, this.#d.refusalHoldMs, list, list);
  }

  answer(id: string, behavior: 'allow' | 'deny'): void {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'request' || !item.reply) throw new Error('not_pending');
    const reply = item.reply;
    this.#items.delete(id);
    this.#d.record(item, behavior === 'allow' ? 'allowed' : 'denied');
    reply(behavior === 'allow' ? ALLOW : DENY);
  }

  approveRule(id: string, rule: string, seen: string): { backupPath: string } {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'refusal') throw new Error('not_pending');
    if (!isNarrowRule(rule)) throw new Error('rule_too_broad');
    // Without the root a rule could land in a subfolder's settings, which Claude Code never reads.
    if (!item.project) throw new Error('project_unknown');
    const written = this.#d.writeRule(item.project, rule.trim(), seen);
    const reply = item.reply;
    this.#items.delete(id);
    this.#d.record(item, 'rule_written');
    reply?.(RETRY);
    return { backupPath: written.backupPath };
  }

  dismiss(id: string): void {
    const item = this.#items.get(id);
    if (!item || item.kind !== 'refusal') throw new Error('not_pending');
    const reply = item.reply;
    this.#items.delete(id);
    this.#d.record(item, 'dismissed');
    reply?.({});
  }

  #item(kind: Pending['kind'], event: HookEvent, proposedRule: string | null): Item {
    const step = trimStep(event, '');
    return { id: this.#d.newId(), kind, sessionId: event.session_id, project: event.project_dir ?? '',
      tool: step.tool, target: step.target, reason: event.reason ?? null, proposedRule,
      at: this.#d.now().toISOString(), held: true };
  }

  /** Waits for the person, the hook hanging up, or the hold time; whichever is first. */
  #hold(item: Item, closed: AbortSignal, ms: number, onTimeout: () => void, onClosed: () => void): Promise<HookReply> {
    this.#items.set(item.id, item);
    return new Promise(resolve => {
      let done = false;
      const finish = (reply: HookReply) => {
        if (done) return;
        done = true;
        clearTimeout(timer);
        closed.removeEventListener('abort', hangUp);
        item.reply = undefined;
        item.held = false;
        resolve(reply);
      };
      const timer = setTimeout(() => { onTimeout(); finish({}); }, ms);
      const hangUp = () => { onClosed(); finish({}); };
      item.reply = finish;
      if (closed.aborted) hangUp();
      else closed.addEventListener('abort', hangUp, { once: true });
    });
  }
}
```

- [ ] **Step 3: Run tests and typecheck**

Run: `npm test` then `npm run typecheck`. Expected: all pass.

- [ ] **Step 4: Contract**

`apps/desktop/sidecar/src/approvals/contract.yaml`:

```yaml
name: sidecar-approvals
version: 1
status: current
source: apps/desktop/sidecar/src/approvals
responsibility: Hold permission requests and auto-mode refusals until the person answers.
public_api:
  request: Hold until answer, hang-up (answered in terminal) or requestHoldMs (timed out); reply allow, deny or {}.
  refusal: Propose a rule; hold up to refusalHoldMs; stays listed after the hold.
  answer: allow or deny one held request; not_pending otherwise.
  approveRule: Narrow rule and a known project root only; write it against the previewed fingerprint, record rule_written, then retry a still-held hook.
  dismiss: Remove a refusal; a held hook gets {}.
  pending: Current items, oldest first.
obligations:
  - rule: Nothing is allowed without answer or approveRule.
    tested_by: allow from AYLUL answers the hook with allow, an unanswered request times out with no decision
  - rule: The terminal answering first ends the request without a decision.
    tested_by: an answer in the terminal ends the request without a decision
  - rule: Concurrent requests are independent; an id is answered once.
    tested_by: two requests are answered independently, answering twice or an unknown id is refused
  - rule: The rule is written before the retry, and also after the hold ended.
    tested_by: approving the rule within the hold writes it, then asks Claude to retry, a late approval still writes the rule
  - rule: Broad rules are refused.
    tested_by: a broad rule is refused and nothing is written
  - rule: A refusal without project_dir gets no proposal and is never written.
    tested_by: a refusal with no project root proposes nothing and cannot be approved
gaps:
  - at: Approvals
    gap: Items live in memory; a sidecar restart drops listed refusals (their records stay in the store).
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/sidecar/src/approvals
git commit -m "feat(sidecar): requests and refusals held until the person answers"
```

---

### Task 6: Store

**Files:**
- Create: `apps/desktop/sidecar/src/store/store.ts`
- Test: `apps/desktop/sidecar/src/store/store.test.ts`
- Create: `apps/desktop/sidecar/src/store/contract.yaml`

**Interfaces:**
- Consumes: `Session` (Task 3), `Step` (Task 3), `Pending`, `Outcome` (Task 5).
- Produces: `class Store` with
  `constructor(file: string)` (`':memory:'` for tests), `saveSession(s: Session): void`,
  `getSession(id: string): Session | undefined`, `listSessions(limit?: number): Session[]`,
  `addStep(sessionId: string, step: Step): void`, `steps(sessionId: string): Step[]`,
  `recordDecision(item: Pending, outcome: Outcome, at: string): void`,
  `decisions(): Decision[]`, `markTerminalAllowed(sessionId: string, tool: string, target: string): boolean`,
  `addRule(project: string, rule: string, backupPath: string, at: string): string`,
  `rulesFor(project: string): RuleRow[]`,
  `addGrantUse(ruleId: string, sessionId: string, toolUseId: string, at: string): void`,
  `grantUses(): Array<{ ruleId: string; sessionId: string; toolUseId: string }>`, `close(): void`;
  `KEPT_STEPS = 50`.

- [ ] **Step 1: Write the failing tests**

`apps/desktop/sidecar/src/store/store.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Store, KEPT_STEPS } from './store.ts';
import type { Session } from '../sessions/session.ts';
import type { Pending } from '../approvals/approvals.ts';

const session = (id: string, updatedAt: string): Session => ({ id, agent: 'claude_code', project: 'C:\\p', state: 'running',
  startedAt: updatedAt, updatedAt, lastStep: null, lastMessage: null });
const item: Pending = { id: 'i1', kind: 'request', sessionId: 's1', project: 'C:\\p', tool: 'Bash', target: 'npm test',
  reason: null, proposedRule: null, at: '2026-10-13T19:00:00.000Z', held: false };

test('sessions are saved, replaced and listed newest first', () => {
  const store = new Store(':memory:');
  store.saveSession(session('a', '2026-10-13T19:00:00.000Z'));
  store.saveSession(session('b', '2026-10-13T19:01:00.000Z'));
  store.saveSession({ ...session('a', '2026-10-13T19:02:00.000Z'), state: 'ended' });
  assert.deepEqual(store.listSessions().map(s => [s.id, s.state]), [['a', 'ended'], ['b', 'running']]);
  assert.equal(store.getSession('missing'), undefined);
});

test('only the latest steps of a session are kept, in order', () => {
  const store = new Store(':memory:');
  for (let i = 0; i < KEPT_STEPS + 5; i++) store.addStep('s1', { at: String(i), tool: 'Bash', target: `c${i}`, summary: `c${i}` });
  store.addStep('s2', { at: 'x', tool: 'Read', target: 'r', summary: 'r' });
  const steps = store.steps('s1');
  assert.equal(steps.length, KEPT_STEPS);
  assert.equal(steps[0].target, 'c5');
  assert.equal(steps.at(-1)?.target, `c${KEPT_STEPS + 4}`);
  assert.equal(store.steps('s2').length, 1);
});

test('a terminal answer is marked allowed by the matching step', () => {
  const store = new Store(':memory:');
  store.recordDecision(item, 'answered_in_terminal', item.at);
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'other'), false);
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'npm test'), true);
  assert.equal(store.decisions()[0].outcome, 'answered_in_terminal_allowed');
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'npm test'), false);
});

test('rules and their uses are recorded per project', () => {
  const store = new Store(':memory:');
  const id = store.addRule('C:\\p', 'Bash(npm test)', 'C:\\b\\x', '2026-10-13T19:00:00.000Z');
  store.addRule('C:\\q', 'Bash(ls)', '', '2026-10-13T19:00:00.000Z');
  assert.deepEqual(store.rulesFor('C:\\p').map(r => r.rule), ['Bash(npm test)']);
  store.addGrantUse(id, 's1', 'toolu_1', '2026-10-13T19:01:00.000Z');
  assert.deepEqual(store.grantUses(), [{ ruleId: id, sessionId: 's1', toolUseId: 'toolu_1' }]);
});

test('the database survives reopening', () => {
  const file = join(mkdtempSync(join(tmpdir(), 'aylul-db-')), 'aylul.db');
  const first = new Store(file);
  first.saveSession(session('a', '2026-10-13T19:00:00.000Z'));
  first.close();
  const second = new Store(file);
  assert.equal(second.getSession('a')?.id, 'a');
  second.close();
});
```

Run: `npm test`. Expected: FAIL, `./store.ts` not found.

- [ ] **Step 2: Implement**

`apps/desktop/sidecar/src/store/store.ts`:

```ts
import { DatabaseSync } from 'node:sqlite';
import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { randomUUID } from 'node:crypto';
import type { Session } from '../sessions/session.ts';
import type { Step } from '../sessions/steps.ts';
import type { Outcome, Pending } from '../approvals/approvals.ts';

export const KEPT_STEPS = 50;
export type RuleRow = { id: string; project: string; rule: string; backupPath: string; at: string };
export type Decision = { itemId: string; sessionId: string; kind: string; tool: string; target: string; outcome: string; at: string };

const SCHEMA = `
CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, body TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS steps (seq INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS decisions (seq INTEGER PRIMARY KEY AUTOINCREMENT, item_id TEXT NOT NULL, session_id TEXT NOT NULL,
  kind TEXT NOT NULL, tool TEXT NOT NULL, target TEXT NOT NULL, outcome TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS rules (id TEXT PRIMARY KEY, project TEXT NOT NULL, rule TEXT NOT NULL, backup_path TEXT NOT NULL,
  approved_by TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS grant_uses (seq INTEGER PRIMARY KEY AUTOINCREMENT, rule_id TEXT NOT NULL, session_id TEXT NOT NULL,
  tool_use_id TEXT NOT NULL, at TEXT NOT NULL);
`;

/** AYLUL's laptop records. Every list is ordered by an explicit key. */
export class Store {
  #db: DatabaseSync;

  constructor(file: string) {
    if (file !== ':memory:') mkdirSync(dirname(file), { recursive: true });
    this.#db = new DatabaseSync(file);
    this.#db.exec(SCHEMA);
  }

  saveSession(session: Session): void {
    this.#db.prepare(`INSERT INTO sessions (id, body, updated_at) VALUES (?, ?, ?)
      ON CONFLICT(id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at`)
      .run(session.id, JSON.stringify(session), session.updatedAt);
  }

  getSession(id: string): Session | undefined {
    const row = this.#db.prepare('SELECT body FROM sessions WHERE id = ?').get(id) as { body: string } | undefined;
    return row ? JSON.parse(row.body) as Session : undefined;
  }

  listSessions(limit = 50): Session[] {
    const rows = this.#db.prepare('SELECT body FROM sessions ORDER BY updated_at DESC, id LIMIT ?').all(limit) as Array<{ body: string }>;
    return rows.map(r => JSON.parse(r.body) as Session);
  }

  addStep(sessionId: string, step: Step): void {
    this.#db.prepare('INSERT INTO steps (session_id, body) VALUES (?, ?)').run(sessionId, JSON.stringify(step));
    this.#db.prepare(`DELETE FROM steps WHERE session_id = ? AND seq NOT IN
      (SELECT seq FROM steps WHERE session_id = ? ORDER BY seq DESC LIMIT ?)`).run(sessionId, sessionId, KEPT_STEPS);
  }

  steps(sessionId: string): Step[] {
    const rows = this.#db.prepare('SELECT body FROM steps WHERE session_id = ? ORDER BY seq').all(sessionId) as Array<{ body: string }>;
    return rows.map(r => JSON.parse(r.body) as Step);
  }

  recordDecision(item: Pending, outcome: Outcome, at: string): void {
    this.#db.prepare(`INSERT INTO decisions (item_id, session_id, kind, tool, target, outcome, at)
      VALUES (?, ?, ?, ?, ?, ?, ?)`).run(item.id, item.sessionId, item.kind, item.tool, item.target, outcome, at);
  }

  // node:sqlite returns rows with a null prototype; `plain` copies them into ordinary objects.
  decisions(): Decision[] {
    return plain(this.#db.prepare(`SELECT item_id AS itemId, session_id AS sessionId, kind, tool, target, outcome, at
      FROM decisions ORDER BY seq`).all()) as Decision[];
  }

  /** A step after a terminal answer shows the person allowed it there. */
  markTerminalAllowed(sessionId: string, tool: string, target: string): boolean {
    const result = this.#db.prepare(`UPDATE decisions SET outcome = 'answered_in_terminal_allowed' WHERE seq =
      (SELECT seq FROM decisions WHERE outcome = 'answered_in_terminal' AND session_id = ? AND tool = ? AND target = ?
       ORDER BY seq DESC LIMIT 1)`).run(sessionId, tool, target);
    return Number(result.changes) > 0;
  }

  addRule(project: string, rule: string, backupPath: string, at: string): string {
    const id = randomUUID();
    this.#db.prepare(`INSERT INTO rules (id, project, rule, backup_path, approved_by, at) VALUES (?, ?, ?, ?, 'person', ?)`)
      .run(id, project, rule, backupPath, at);
    return id;
  }

  rulesFor(project: string): RuleRow[] {
    return plain(this.#db.prepare(`SELECT id, project, rule, backup_path AS backupPath, at FROM rules
      WHERE project = ? ORDER BY at, id`).all(project)) as RuleRow[];
  }

  addGrantUse(ruleId: string, sessionId: string, toolUseId: string, at: string): void {
    this.#db.prepare('INSERT INTO grant_uses (rule_id, session_id, tool_use_id, at) VALUES (?, ?, ?, ?)')
      .run(ruleId, sessionId, toolUseId, at);
  }

  grantUses(): Array<{ ruleId: string; sessionId: string; toolUseId: string }> {
    return plain(this.#db.prepare(`SELECT rule_id AS ruleId, session_id AS sessionId, tool_use_id AS toolUseId
      FROM grant_uses ORDER BY seq`).all()) as Array<{ ruleId: string; sessionId: string; toolUseId: string }>;
  }

  close(): void { this.#db.close(); }
}

function plain(rows: unknown[]): Array<Record<string, unknown>> {
  return rows.map(row => ({ ...(row as Record<string, unknown>) }));
}
```

- [ ] **Step 3: Run tests and typecheck**

Run: `npm test` then `npm run typecheck`. Expected: all pass. Node prints an
`ExperimentalWarning` for `node:sqlite` on stderr; that is expected.

- [ ] **Step 4: Contract**

`apps/desktop/sidecar/src/store/contract.yaml`:

```yaml
name: sidecar-store
version: 1
status: current
source: apps/desktop/sidecar/src/store
responsibility: Keep AYLUL's laptop records in node:sqlite.
public_api:
  saveSession/getSession/listSessions: Upsert by id; list by updated_at desc, then id.
  addStep/steps: Keep the latest KEPT_STEPS per session, in insertion order (seq).
  recordDecision/decisions: Append one row per outcome, ordered by seq.
  markTerminalAllowed: Turn the latest matching answered_in_terminal into answered_in_terminal_allowed.
  addRule/rulesFor: A rule the person approved, with its backup path and approver person.
  addGrantUse/grantUses: A call a recorded rule covered (decider aylul_grant).
obligations:
  - rule: Steps are capped per session and kept in order.
    tested_by: only the latest steps of a session are kept, in order
  - rule: A terminal answer is marked allowed once, by its matching step.
    tested_by: a terminal answer is marked allowed by the matching step
  - rule: Records survive a restart.
    tested_by: the database survives reopening
gaps:
  - at: Store
    gap: No retention yet; the 90-day approval-record limit is enforced in step 2(d) or later.
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/sidecar/src/store
git commit -m "feat(sidecar): node:sqlite store for sessions, decisions and rules"
```

---

### Task 7: Installing AYLUL's hooks into Claude Code

**Files:**
- Create: `apps/desktop/sidecar/src/agents/install.ts`
- Test: `apps/desktop/sidecar/src/agents/install.test.ts`
- Modify: `apps/desktop/sidecar/src/agents/contract.yaml` (add the two functions)

**Interfaces:**
- Consumes: `planEdit`, `Plan` (Task 4).
- Produces: `HOOK_EVENTS`, `planHookInstall(settingsPath: string, hookExe: string): Plan`,
  `hooksInstalled(settingsPath: string, hookExe: string): boolean`.

- [ ] **Step 1: Write the failing tests**

`apps/desktop/sidecar/src/agents/install.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { planHookInstall, hooksInstalled, HOOK_EVENTS } from './install.ts';
import { writePlan } from './settings-file.ts';

const exe = 'C:\\Program Files\\AYLUL\\aylul-hook.exe';
const fresh = () => join(mkdtempSync(join(tmpdir(), 'aylul-claude-')), 'settings.json');
const now = new Date('2026-10-13T19:00:00.000Z');

test('every event gets one exec-form entry with its options', () => {
  const path = fresh();
  writePlan(planHookInstall(path, exe), join(path, '..', 'backups'), now);
  const hooks = JSON.parse(readFileSync(path, 'utf8')).hooks;
  for (const [event, extra] of HOOK_EVENTS) {
    assert.equal(hooks[event].length, 1, event);
    assert.deepEqual(hooks[event][0], { hooks: [{ type: 'command', command: exe, args: [], ...extra }] });
  }
  assert.equal(hooks.PostToolUse[0].hooks[0].async, true);
  assert.equal(hooks.PermissionRequest[0].hooks[0].timeout, 600);
  assert.equal(hooks.PermissionDenied[0].hooks[0].timeout, 130);
  assert.equal(hooksInstalled(path, exe), true);
});

test("the person's own hooks and settings are kept; AYLUL's are replaced, not repeated", () => {
  const path = fresh();
  const old = 'C:\\old\\aylul-hook.exe';
  writeFileSync(path, JSON.stringify({ model: 'opus', hooks: {
    Stop: [{ hooks: [{ type: 'command', command: 'mine.cmd' }, { type: 'command', command: old, args: [] }] }],
    PostToolUse: [{ matcher: 'Bash', hooks: [{ type: 'command', command: 'lint.cmd' }] }],
  } }));
  writePlan(planHookInstall(path, exe), join(path, '..', 'backups'), now);
  writePlan(planHookInstall(path, exe), join(path, '..', 'backups'), now);
  const settings = JSON.parse(readFileSync(path, 'utf8'));
  assert.equal(settings.model, 'opus');
  assert.deepEqual(settings.hooks.Stop, [
    { hooks: [{ type: 'command', command: 'mine.cmd' }] },
    { hooks: [{ type: 'command', command: exe, args: [] }] },
  ]);
  assert.equal(settings.hooks.PostToolUse.length, 2);
  assert.equal(settings.hooks.PostToolUse[0].matcher, 'Bash');
});

test('hooksInstalled is false for a missing, partial or invalid file', () => {
  const path = fresh();
  assert.equal(hooksInstalled(path, exe), false);
  writeFileSync(path, JSON.stringify({ hooks: { Stop: [{ hooks: [{ type: 'command', command: exe }] }] } }));
  assert.equal(hooksInstalled(path, exe), false);
  writeFileSync(path, '{');
  assert.equal(hooksInstalled(path, exe), false);
});

test('a hooks value of the wrong shape is refused', () => {
  const path = fresh();
  writeFileSync(path, JSON.stringify({ hooks: [] }));
  assert.throws(() => planHookInstall(path, exe), { message: 'settings_invalid' });
  writeFileSync(path, JSON.stringify({ hooks: { Stop: {} } }));
  assert.throws(() => planHookInstall(path, exe), { message: 'settings_invalid' });
});
```

Run: `npm test`. Expected: FAIL, `./install.ts` not found.

- [ ] **Step 2: Implement**

`apps/desktop/sidecar/src/agents/install.ts`:

```ts
import { existsSync, readFileSync } from 'node:fs';
import { planEdit, type Plan } from './settings-file.ts';

/** Events AYLUL listens to, with the options each entry needs (spec §2, Global Constraints). */
export const HOOK_EVENTS: ReadonlyArray<readonly [string, Record<string, unknown>]> = [
  ['SessionStart', {}],
  ['SessionEnd', {}],
  ['Stop', {}],
  ['Notification', {}],
  ['PostToolUse', { async: true }],
  ['PermissionRequest', { timeout: 600 }],
  ['PermissionDenied', { timeout: 130 }],
];

type Group = { hooks?: unknown[]; [key: string]: unknown };
const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const isAylul = (hook: unknown) =>
  isObject(hook) && typeof hook.command === 'string' && hook.command.toLowerCase().endsWith('aylul-hook.exe');

/** Replaces AYLUL's entries in ~/.claude/settings.json and keeps every other hook. */
export function planHookInstall(settingsPath: string, hookExe: string): Plan {
  return planEdit(settingsPath, settings => {
    const hooks = settings.hooks ?? {};
    if (!isObject(hooks)) throw new Error('settings_invalid');
    for (const [event, extra] of HOOK_EVENTS) {
      const groups = hooks[event] ?? [];
      if (!Array.isArray(groups)) throw new Error('settings_invalid');
      const kept: Group[] = [];
      for (const group of groups as Group[]) {
        if (!isObject(group) || !Array.isArray(group.hooks)) { kept.push(group); continue; }
        const others = group.hooks.filter(h => !isAylul(h));
        if (others.length > 0) kept.push({ ...group, hooks: others });
      }
      kept.push({ hooks: [{ type: 'command', command: hookExe, args: [], ...extra }] });
      hooks[event] = kept;
    }
    settings.hooks = hooks;
  });
}

/** True when every event has an entry running exactly this executable. */
export function hooksInstalled(settingsPath: string, hookExe: string): boolean {
  if (!existsSync(settingsPath)) return false;
  try {
    const hooks = JSON.parse(readFileSync(settingsPath, 'utf8')).hooks;
    return HOOK_EVENTS.every(([event]) => Array.isArray(hooks?.[event]) && hooks[event].some((g: Group) =>
      Array.isArray(g?.hooks) && g.hooks.some(h => isObject(h) && h.command === hookExe)));
  } catch {
    return false;
  }
}
```

- [ ] **Step 3: Run tests and typecheck**

Run: `npm test` then `npm run typecheck`. Expected: all pass.

- [ ] **Step 4: Extend the agents contract**

Add to `public_api` in `apps/desktop/sidecar/src/agents/contract.yaml`:

```yaml
  planHookInstall: Replace entries whose command ends in aylul-hook.exe with one exec-form entry per HOOK_EVENTS; keep all other hooks and settings.
  hooksInstalled: Every HOOK_EVENTS event has an entry with exactly this executable.
```

and to `obligations`:

```yaml
  - rule: The person's own hooks survive; AYLUL's entry appears once per event after repeated installs.
    tested_by: "the person's own hooks and settings are kept; AYLUL's are replaced, not repeated"
  - rule: Entries use exec form, async PostToolUse, 600 s PermissionRequest and 130 s PermissionDenied.
    tested_by: every event gets one exec-form entry with its options
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/sidecar/src/agents
git commit -m "feat(sidecar): install AYLUL's hooks into Claude Code, keeping the person's own"
```

---

### Task 8: Route, stdio RPC and the sidecar entry point

**Files:**
- Create: `apps/desktop/sidecar/src/hooks/route.ts`
- Create: `apps/desktop/sidecar/src/rpc.ts`, `apps/desktop/sidecar/src/handlers.ts`, `apps/desktop/sidecar/src/main.ts`
- Test: `apps/desktop/sidecar/src/hooks/route.test.ts`, `apps/desktop/sidecar/src/rpc.test.ts`, `apps/desktop/sidecar/src/handlers.test.ts`
- Create: `apps/desktop/sidecar/src/contract.yaml`; Modify: `apps/desktop/sidecar/src/hooks/contract.yaml` (add `makeRoute`)

**Interfaces:**
- Consumes: everything from Tasks 2–7.
- Produces:
  - `makeRoute(d: { store: Store; approvals: Approvals; now: () => Date }): Route`
  - `type Handlers = Record<string, (params: Record<string, unknown>) => unknown>`
  - `serveRpc(input: NodeJS.ReadableStream, output: NodeJS.WritableStream, handlers: Handlers, onEnd: () => void): void`
  - `ERROR_CODES` (set of stable codes)
  - `makeHandlers(d: HandlerDeps): Handlers` with methods `state`, `steps`,
    `request.answer`, `refusal.preview`, `refusal.approve`, `refusal.dismiss`,
    `hooks.preview`, `hooks.install`; `settingsFor(project: string): string`
  - Both previews return `{ path, lines, before }`, where `before` is `fingerprint` of the file
    they read; `refusal.approve` and `hooks.install` take that `before` back and refuse with
    `settings_changed` if the file moved on since
  - Wire: stdin lines `{"id": number, "method": string, "params": object}`;
    stdout lines `{"id", "result"}` or `{"id", "error": code}`. Logs go to stderr only.

- [ ] **Step 1: Write the failing route tests**

`apps/desktop/sidecar/src/hooks/route.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { makeRoute } from './route.ts';
import { Store } from '../store/store.ts';
import { Approvals } from '../approvals/approvals.ts';

function setup() {
  const store = new Store(':memory:');
  const approvals = new Approvals({ now: () => new Date('2026-10-13T19:00:00.000Z'), newId: () => 'id1',
    writeRule: () => ({ ruleId: 'r', backupPath: '' }), record: (i, o) => store.recordDecision(i, o, i.at),
    requestHoldMs: 200, refusalHoldMs: 200 });
  const route = makeRoute({ store, approvals, now: () => new Date('2026-10-13T19:00:00.000Z') });
  return { store, approvals, route };
}
const never = new AbortController().signal;

test('a PostToolUse event saves the session and a trimmed step, and answers at once', async () => {
  const { store, route } = setup();
  const reply = await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', tool_name: 'Write',
    tool_input: { file_path: 'C:\\p\\a.txt', content: 'SECRET' }, tool_use_id: 't1' }, never);
  assert.deepEqual(reply, {});
  assert.equal(store.getSession('s1')?.lastStep?.target, 'C:\\p\\a.txt');
  assert.equal(JSON.stringify(store.steps('s1')).includes('SECRET'), false);
});

test('a PermissionRequest waits in approvals', async () => {
  const { approvals, route } = setup();
  const reply = route({ session_id: 's1', hook_event_name: 'PermissionRequest', cwd: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' } }, never);
  await new Promise(r => setImmediate(r));
  approvals.answer(approvals.pending()[0].id, 'allow');
  assert.equal(((await reply).output as any).hookSpecificOutput.decision.behavior, 'allow');
});

test('a call covered by a recorded rule is noted as a grant use', async () => {
  const { store, route } = setup();
  const ruleId = store.addRule('C:\\p', 'Bash(npm test)', '', '2026-10-13T18:00:00.000Z');
  await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p\\src', project_dir: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' }, tool_use_id: 't9' }, never);
  await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', project_dir: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm run build' }, tool_use_id: 't10' }, never);
  assert.deepEqual(store.grantUses(), [{ ruleId, sessionId: 's1', toolUseId: 't9' }]);
});

test('a step after a terminal answer marks it allowed', async () => {
  const { store, route } = setup();
  const hook = new AbortController();
  const waiting = route({ session_id: 's1', hook_event_name: 'PermissionRequest', cwd: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' } }, hook.signal);
  await new Promise(r => setImmediate(r));
  hook.abort();
  await waiting;
  await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' }, tool_use_id: 't1' }, never);
  assert.equal(store.decisions()[0].outcome, 'answered_in_terminal_allowed');
});
```

Run: `npm test`. Expected: FAIL, `./route.ts` not found.

- [ ] **Step 2: Implement the route**

`apps/desktop/sidecar/src/hooks/route.ts`:

```ts
import type { HookEvent, HookReply } from '../protocol.ts';
import type { Route } from './server.ts';
import type { Store } from '../store/store.ts';
import type { Approvals } from '../approvals/approvals.ts';
import { applyEvent } from '../sessions/session.ts';
import { proposeRule } from '../agents/rules.ts';

/** Records every event, then lets approvals answer the two that wait for the person. */
export function makeRoute(d: { store: Store; approvals: Approvals; now: () => Date }): Route {
  return async (event: HookEvent, closed: AbortSignal): Promise<HookReply> => {
    const at = d.now().toISOString();
    const { session, step } = applyEvent(d.store.getSession(event.session_id), event, at);
    d.store.saveSession(session);
    if (step) {
      d.store.addStep(session.id, step);
      d.store.markTerminalAllowed(session.id, step.tool, step.target);
      noteGrantUse(d.store, event, at);
    }
    if (event.hook_event_name === 'PermissionRequest') return d.approvals.request(event, closed);
    if (event.hook_event_name === 'PermissionDenied') return d.approvals.refusal(event, closed);
    return {};
  };
}

function noteGrantUse(store: Store, event: HookEvent, at: string): void {
  const project = event.project_dir ?? '';
  const covering = proposeRule(event.tool_name ?? '', event.tool_input ?? {}, project);
  if (!covering) return;
  const rule = store.rulesFor(project).find(r => r.rule === covering);
  if (rule) store.addGrantUse(rule.id, event.session_id, event.tool_use_id ?? '', at);
}
```

Run: `npm test`. Expected: route tests pass.

- [ ] **Step 3: Write the failing RPC and handler tests**

`apps/desktop/sidecar/src/rpc.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PassThrough } from 'node:stream';
import { serveRpc } from './rpc.ts';

async function exchange(lines: string[], handlers: Parameters<typeof serveRpc>[2]) {
  const input = new PassThrough();
  const output = new PassThrough();
  let out = '';
  output.setEncoding('utf8').on('data', d => { out += d; });
  const ended = new Promise<void>(resolve => serveRpc(input, output, handlers, resolve));
  for (const line of lines) input.write(line + '\n');
  input.end();
  await ended;
  await new Promise(r => setImmediate(r));
  return out.trim().split('\n').map(l => JSON.parse(l));
}

test('a known method returns its result under the same id', async () => {
  assert.deepEqual(await exchange([JSON.stringify({ id: 1, method: 'state', params: {} })], { state: () => ({ ok: 1 }) }),
    [{ id: 1, result: { ok: 1 } }]);
});

test('unknown methods, stable errors and unexpected errors come back as codes', async () => {
  const replies = await exchange([
    JSON.stringify({ id: 1, method: 'shell', params: {} }),
    JSON.stringify({ id: 2, method: 'bad', params: {} }),
    JSON.stringify({ id: 3, method: 'oops', params: {} }),
    JSON.stringify({ id: 4, method: 'toString', params: {} }),
    'not json',
  ], { bad: () => { throw new Error('rule_too_broad'); }, oops: () => { throw new Error('C:\\secret path'); } });
  // Synchronous refusals are written before handler results, so compare by id.
  replies.sort((a, b) => String(a.id).localeCompare(String(b.id)));
  assert.deepEqual(replies, [
    { id: 1, error: 'unknown_method' },
    { id: 2, error: 'rule_too_broad' },
    { id: 3, error: 'action_failed' },
    { id: 4, error: 'unknown_method' },
    { id: null, error: 'bad_request' },
  ]);
});
```

`apps/desktop/sidecar/src/handlers.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { makeHandlers, settingsFor } from './handlers.ts';
import { Store } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { expectBefore, planAllowRule, writePlan } from './agents/settings-file.ts';

function setup(withExe = true) {
  const home = mkdtempSync(join(tmpdir(), 'aylul-h-'));
  const exe = join(home, 'aylul-hook.exe');
  if (withExe) writeFileSync(exe, '');
  const store = new Store(':memory:');
  const now = () => new Date('2026-10-13T19:00:00.000Z');
  const approvals = new Approvals({ now, newId: () => 'id1', record: () => {}, requestHoldMs: 500, refusalHoldMs: 500,
    writeRule: (project, rule, seen) => ({ ruleId: 'r',
      backupPath: writePlan(expectBefore(planAllowRule(settingsFor(project), rule), seen), join(home, 'b'), now()) }) });
  const handlers = makeHandlers({ store, approvals, claudeSettings: join(home, '.claude', 'settings.json'),
    hookExe: withExe ? exe : undefined, backupDir: join(home, 'b'), now });
  return { home, exe, approvals, handlers };
}

test('state reports hooks not installed, then installed after hooks.install', async () => {
  const { handlers, home } = setup();
  assert.equal((await handlers.state({}) as { hooksInstalled: boolean }).hooksInstalled, false);
  const preview = await handlers['hooks.preview']({}) as { path: string; lines: string[]; before: string };
  assert.equal(preview.path, join(home, '.claude', 'settings.json'));
  assert.ok(preview.lines.some(l => l.includes('aylul-hook.exe')));
  await handlers['hooks.install']({ before: preview.before });
  assert.equal((await handlers.state({}) as { hooksInstalled: boolean }).hooksInstalled, true);
});

test('without the hook executable, install is refused', () => {
  const { handlers } = setup(false);
  assert.throws(() => handlers['hooks.install']({ before: 'x' }), { message: 'hook_exe_missing' });
});

test('a refusal is previewed and approved into the project settings', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', cwd: join(project, 'src'), project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' }, reason: '[X]' }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = await handlers['refusal.preview']({ id, rule: 'Bash(npm run deploy)' }) as { lines: string[]; before: string };
  assert.ok(preview.lines.some(l => l.includes('Bash(npm run deploy)')));
  await handlers['refusal.approve']({ id, rule: 'Bash(npm run deploy)', before: preview.before });
  assert.deepEqual(JSON.parse(readFileSync(settingsFor(project), 'utf8')).permissions.allow, ['Bash(npm run deploy)']);
});

test('an approval after the settings file changed since the preview writes nothing', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project, tool_name: 'Bash',
    tool_input: { command: 'npm run deploy' }, reason: '[X]' }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = await handlers['refusal.preview']({ id, rule: 'Bash(npm run deploy)' }) as { before: string };
  mkdirSync(join(project, '.claude'), { recursive: true });
  writeFileSync(settingsFor(project), '{"model": "x"}');
  assert.throws(() => handlers['refusal.approve']({ id, rule: 'Bash(npm run deploy)', before: preview.before }),
    { message: 'settings_changed' });
  assert.equal(readFileSync(settingsFor(project), 'utf8'), '{"model": "x"}');
  assert.equal(approvals.pending().length, 1);
});

test('missing or wrong parameters are bad_params', () => {
  const { handlers } = setup();
  assert.throws(() => handlers['request.answer']({ id: 'x' }), { message: 'bad_params' });
  assert.throws(() => handlers['request.answer']({ id: 'x', behavior: 'maybe' }), { message: 'bad_params' });
  assert.throws(() => handlers.steps({}), { message: 'bad_params' });
});
```

Run: `npm test`. Expected: FAIL, `./rpc.ts` not found.

- [ ] **Step 4: Implement RPC, handlers and main**

`apps/desktop/sidecar/src/rpc.ts`:

```ts
import { createInterface } from 'node:readline';

export type Handlers = Record<string, (params: Record<string, unknown>) => unknown>;

export const ERROR_CODES = new Set(['unknown_method', 'bad_params', 'bad_request', 'not_pending', 'rule_too_broad',
  'project_unknown', 'settings_invalid', 'settings_changed', 'hook_exe_missing']);

/** One JSON line per call on input, one per answer on output. Errors leave as stable codes only. */
export function serveRpc(input: NodeJS.ReadableStream, output: NodeJS.WritableStream, handlers: Handlers, onEnd: () => void): void {
  const write = (value: object) => output.write(JSON.stringify(value) + '\n');
  const lines = createInterface({ input, crlfDelay: Infinity });
  lines.on('line', line => {
    let message: { id?: unknown; method?: unknown; params?: unknown };
    try { message = JSON.parse(line); } catch { write({ id: null, error: 'bad_request' }); return; }
    const id = message.id ?? null;
    const method = typeof message.method === 'string' && Object.hasOwn(handlers, message.method) ? handlers[message.method] : undefined;
    if (!method) { write({ id, error: 'unknown_method' }); return; }
    const params = typeof message.params === 'object' && message.params !== null ? message.params as Record<string, unknown> : {};
    Promise.resolve()
      .then(() => method(params))
      .then(result => write({ id, result: result ?? null }), error => write({ id, error: code(error) }));
  });
  lines.on('close', onEnd);
}

function code(error: unknown): string {
  const message = error instanceof Error ? error.message : '';
  if (ERROR_CODES.has(message)) return message;
  process.stderr.write(`aylul-sidecar: ${error instanceof Error ? error.stack : String(error)}\n`);
  return 'action_failed';
}
```

`apps/desktop/sidecar/src/handlers.ts`:

```ts
import { existsSync } from 'node:fs';
import { win32 } from 'node:path';
import type { Handlers } from './rpc.ts';
import type { Store } from './store/store.ts';
import type { Approvals } from './approvals/approvals.ts';
import { isNarrowRule } from './agents/rules.ts';
import { diffLines, expectBefore, fingerprint, planAllowRule, writePlan, type Plan } from './agents/settings-file.ts';
import { hooksInstalled, planHookInstall } from './agents/install.ts';

export type HandlerDeps = {
  store: Store;
  approvals: Approvals;
  claudeSettings: string;
  hookExe: string | undefined;
  backupDir: string;
  now: () => Date;
};

export const settingsFor = (project: string) => win32.join(project, '.claude', 'settings.local.json');

const text = (params: Record<string, unknown>, key: string): string => {
  const value = params[key];
  if (typeof value !== 'string' || !value) throw new Error('bad_params');
  return value;
};

/** The calls AYLUL's window can make. Each reads its parameters and calls one owner. */
export function makeHandlers(d: HandlerDeps): Handlers {
  const exe = () => {
    if (!d.hookExe || !existsSync(d.hookExe)) throw new Error('hook_exe_missing');
    return d.hookExe;
  };
  const refusal = (id: string) => {
    const item = d.approvals.pending().find(p => p.id === id && p.kind === 'refusal');
    if (!item) throw new Error('not_pending');
    if (!item.project) throw new Error('project_unknown');
    return item;
  };
  // What a preview shows, plus the fingerprint its approval must send back.
  const shown = (plan: Plan) => ({ path: plan.path, lines: diffLines(plan), before: fingerprint(plan.before) });
  return {
    state: () => ({
      sessions: d.store.listSessions(),
      pending: d.approvals.pending(),
      hooksInstalled: !!d.hookExe && hooksInstalled(d.claudeSettings, d.hookExe),
    }),
    steps: p => d.store.steps(text(p, 'sessionId')),
    'request.answer': p => {
      const behavior = p.behavior;
      if (behavior !== 'allow' && behavior !== 'deny') throw new Error('bad_params');
      d.approvals.answer(text(p, 'id'), behavior);
      return null;
    },
    'refusal.preview': p => {
      const item = refusal(text(p, 'id'));
      const rule = text(p, 'rule').trim();
      if (!isNarrowRule(rule)) throw new Error('rule_too_broad');
      return shown(planAllowRule(settingsFor(item.project), rule));
    },
    'refusal.approve': p => d.approvals.approveRule(text(p, 'id'), text(p, 'rule'), text(p, 'before')),
    'refusal.dismiss': p => { d.approvals.dismiss(text(p, 'id')); return null; },
    'hooks.preview': () => shown(planHookInstall(d.claudeSettings, exe())),
    'hooks.install': p => {
      const plan = expectBefore(planHookInstall(d.claudeSettings, exe()), text(p, 'before'));
      return { backupPath: writePlan(plan, d.backupDir, d.now()) };
    },
  };
}
```

`apps/desktop/sidecar/src/main.ts`:

```ts
// AYLUL's laptop process. Tauri starts it and talks to it over stdio; aylul-hook reaches it over the pipe.
import { homedir } from 'node:os';
import { join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { Store } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { createEndpoint, removeEndpoint } from './hooks/endpoint.ts';
import { startHookServer } from './hooks/server.ts';
import { makeRoute } from './hooks/route.ts';
import { expectBefore, planAllowRule, writePlan } from './agents/settings-file.ts';
import { makeHandlers, settingsFor } from './handlers.ts';
import { serveRpc } from './rpc.ts';

// AYLUL_DATA_DIR keeps a smoke run away from the real hook.json and database.
const dataDir = process.env.AYLUL_DATA_DIR ?? join(process.env.LOCALAPPDATA ?? join(homedir(), 'AppData', 'Local'), 'AYLUL');
const backupDir = join(dataDir, 'backups');
const claudeDir = process.env.CLAUDE_CONFIG_DIR ?? join(homedir(), '.claude');
const now = () => new Date();

const store = new Store(join(dataDir, 'aylul.db'));
const approvals = new Approvals({
  now,
  newId: randomUUID,
  record: (item, outcome) => store.recordDecision(item, outcome, now().toISOString()),
  writeRule: (project, rule, seen) => {
    const backupPath = writePlan(expectBefore(planAllowRule(settingsFor(project), rule), seen), backupDir, now());
    return { ruleId: store.addRule(project, rule, backupPath, now().toISOString()), backupPath };
  },
  requestHoldMs: 580_000,
  refusalHoldMs: 120_000,
});
const endpoint = createEndpoint(dataDir);
const server = await startHookServer(endpoint.pipe, endpoint.token, makeRoute({ store, approvals, now }));

serveRpc(process.stdin, process.stdout, makeHandlers({
  store, approvals, claudeSettings: join(claudeDir, 'settings.json'), hookExe: process.env.AYLUL_HOOK_EXE, backupDir, now,
}), () => {
  server.close();
  removeEndpoint(endpoint);
  store.close();
  process.exit(0);
});
process.stderr.write('aylul-sidecar: ready\n');
```

When Tauri exits or dies, the sidecar's stdin closes and it removes `hook.json`
and exits, so a stale pipe name is not left for the hook.

- [ ] **Step 5: Run tests, typecheck, and a start-up smoke test**

Run: `npm test` then `npm run typecheck`. Expected: all pass.

Smoke (PowerShell, in `apps/desktop/sidecar`). It uses a scratch data folder, so a
running AYLUL keeps its own `hook.json` and database:
`$env:AYLUL_DATA_DIR = "$env:TEMP\aylul-smoke"; '{"id":1,"method":"state","params":{}}' | node src/main.ts; Remove-Item Env:AYLUL_DATA_DIR`
Expected stdout: one line `{"id":1,"result":{"sessions":[],"pending":[],"hooksInstalled":...}}`;
stderr shows `aylul-sidecar: ready`; the process exits by itself when the input
ends, and `$env:TEMP\aylul-smoke\hook.json` no longer exists afterwards.
`%LOCALAPPDATA%\AYLUL\hook.json`, if AYLUL is running, is untouched.

- [ ] **Step 6: Contracts**

Add to `public_api` in `apps/desktop/sidecar/src/hooks/contract.yaml`:

```yaml
  makeRoute: Apply every event to the session and step store, note steps covered by a recorded rule, then hand PermissionRequest and PermissionDenied to approvals; {} otherwise.
```

and to its `obligations`:

```yaml
  - rule: Steps are stored trimmed; a covered call becomes a grant use; a step after a terminal answer marks it allowed.
    tested_by: a PostToolUse event saves the session and a trimmed step, and answers at once, a call covered by a recorded rule is noted as a grant use, a step after a terminal answer marks it allowed
```

`apps/desktop/sidecar/src/contract.yaml`:

```yaml
name: sidecar-composition
version: 1
status: current
source: apps/desktop/sidecar/src
responsibility: Compose the sidecar and answer the window's calls over stdio.
public_api:
  serveRpc: One JSON line in, one out with the same id; unknown methods and unexpected errors become stable codes; ends with stdin.
  makeHandlers: state, steps, request.answer, refusal.preview/approve/dismiss, hooks.preview/install; each validates its parameters; approve and install carry the previewed fingerprint.
  main: Data under AYLUL_DATA_DIR, else %LOCALAPPDATA%\AYLUL; hold 580 s requests and 120 s refusals; stop and remove hook.json when stdin ends.
obligations:
  - rule: Only own handler names are callable; error text never leaves except as a code.
    tested_by: unknown methods, stable errors and unexpected errors come back as codes
  - rule: Hook install needs the executable; parameters are checked.
    tested_by: without the hook executable, install is refused, missing or wrong parameters are bad_params
  - rule: A refusal's rule reaches the root project's settings.local.json through preview and approve, even when Claude ran in a subfolder.
    tested_by: a refusal is previewed and approved into the project settings
  - rule: An approval against a settings file that changed since its preview writes nothing.
    tested_by: an approval after the settings file changed since the preview writes nothing
gaps:
  - at: main
    gap: Node prints an ExperimentalWarning for node:sqlite on stderr at start.
```

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/sidecar/src
git commit -m "feat(sidecar): route hook events, answer the window over stdio"
```

---

### Task 9: Tauri starts the sidecar and relays calls

**Files:**
- Create: `apps/desktop/src-tauri/src/sidecar/mod.rs`, `apps/desktop/src-tauri/src/sidecar/echo-fixture.mjs`, `apps/desktop/src-tauri/src/sidecar/contract.yaml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/src/commands/mod.rs`, `apps/desktop/src-tauri/src/commands/contract.yaml`

**Interfaces:**
- Consumes: the sidecar stdio wire (Task 8).
- Produces:
  - `pub struct SidecarState(Option<Sidecar>)` with `call(&self, method: &str, params: Value) -> Result<Value, String>` and `stop(&self)`
  - Tauri command `aylul_call(method: String, params: serde_json::Value) -> Result<serde_json::Value, String>`,
    main window only. Errors: `main_required`, `unknown_method`, `sidecar_unavailable`, `sidecar_timeout`, or the sidecar's code.

- [ ] **Step 1: Write the fixture and failing tests**

`apps/desktop/src-tauri/src/sidecar/echo-fixture.mjs`:

```js
// Test fixture: answers `state` with its params, every other method with an error code.
import { createInterface } from 'node:readline';
createInterface({ input: process.stdin }).on('line', line => {
  const { id, method, params } = JSON.parse(line);
  process.stdout.write(JSON.stringify(method === 'state' ? { id, result: params } : { id, error: 'not_pending' }) + '\n');
});
```

Create `apps/desktop/src-tauri/src/sidecar/mod.rs` with only the tests module
(Step 2 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn echo() -> Sidecar {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sidecar/echo-fixture.mjs");
        Sidecar::start(Path::new("node"), &script, Path::new("unused.exe")).expect("node on PATH")
    }

    #[test]
    fn a_call_reaches_the_sidecar_and_returns_its_result() {
        let sidecar = echo();
        assert_eq!(sidecar.call("state", json!({ "x": 1 })), Ok(json!({ "x": 1 })));
        sidecar.stop();
    }

    #[test]
    fn a_sidecar_error_code_comes_back_as_the_error() {
        let sidecar = echo();
        assert_eq!(sidecar.call("request.answer", json!({})), Err("not_pending".to_string()));
        sidecar.stop();
    }

    #[test]
    fn an_unlisted_method_is_refused_before_it_reaches_the_sidecar() {
        let sidecar = echo();
        assert_eq!(sidecar.call("shell.exec", json!({})), Err("unknown_method".to_string()));
        sidecar.stop();
    }

    #[test]
    fn stop_lets_the_sidecar_exit_by_itself() {
        let sidecar = echo();
        sidecar.stop();
        // A killed process exits with code 1 on Windows; closing stdin lets it end with 0.
        let status = sidecar.child.lock().unwrap().try_wait().unwrap().expect("exited");
        assert!(status.success());
        assert_eq!(sidecar.call("state", json!({})), Err("sidecar_unavailable".to_string()));
    }

    #[test]
    fn no_sidecar_answers_unavailable() {
        let state = SidecarState(None);
        assert_eq!(state.call("state", json!({})), Err("sidecar_unavailable".to_string()));
    }
}
```

Run (from `apps/desktop`): `cargo test --manifest-path src-tauri/Cargo.toml sidecar`
Expected: FAIL to compile, `Sidecar` not found.

- [ ] **Step 2: Implement the sidecar handle**

Put above the tests module in `apps/desktop/src-tauri/src/sidecar/mod.rs`:

```rust
//! Starts AYLUL's Node sidecar, relays the window's calls over stdio, stops it on exit.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

/// The only methods the window may call (sidecar `makeHandlers`).
pub const METHODS: &[&str] = &[
    "state",
    "steps",
    "request.answer",
    "refusal.preview",
    "refusal.approve",
    "refusal.dismiss",
    "hooks.preview",
    "hooks.install",
];

type Waiting = Arc<Mutex<HashMap<u64, Sender<Value>>>>;

pub struct Sidecar {
    child: Mutex<Child>,
    /// `None` once `stop` has closed it.
    stdin: Mutex<Option<ChildStdin>>,
    waiting: Waiting,
    next: AtomicU64,
}

impl Sidecar {
    pub fn start(node: &Path, script: &Path, hook_exe: &Path) -> std::io::Result<Self> {
        let mut command = Command::new(node);
        command
            .arg(script)
            .env("AYLUL_HOOK_EXE", hook_exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or(std::io::ErrorKind::BrokenPipe)?;
        let stdout = child.stdout.take().ok_or(std::io::ErrorKind::BrokenPipe)?;
        let waiting: Waiting = Arc::default();
        let readers = waiting.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let Ok(reply) = serde_json::from_str::<Value>(&line) else { continue };
                let Some(id) = reply.get("id").and_then(Value::as_u64) else { continue };
                if let Some(sender) = readers.lock().unwrap().remove(&id) {
                    let _ = sender.send(reply);
                }
            }
        });
        Ok(Self { child: Mutex::new(child), stdin: Mutex::new(Some(stdin)), waiting, next: AtomicU64::new(1) })
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        if !METHODS.contains(&method) {
            return Err("unknown_method".into());
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = channel();
        self.waiting.lock().unwrap().insert(id, sender);
        let mut line = json!({ "id": id, "method": method, "params": params }).to_string();
        line.push('\n');
        let sent = match self.stdin.lock().unwrap().as_mut() {
            Some(stdin) => stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        if sent.is_err() {
            self.waiting.lock().unwrap().remove(&id);
            return Err("sidecar_unavailable".into());
        }
        let reply = receiver.recv_timeout(Duration::from_secs(10)).map_err(|_| {
            self.waiting.lock().unwrap().remove(&id);
            "sidecar_timeout".to_string()
        })?;
        match reply.get("error").and_then(Value::as_str) {
            Some(code) => Err(code.to_string()),
            None => Ok(reply.get("result").cloned().unwrap_or(Value::Null)),
        }
    }

    /// Closes stdin, so the sidecar removes hook.json and exits by itself;
    /// kills it only if it is still running 3 s later.
    pub fn stop(&self) {
        drop(self.stdin.lock().unwrap().take());
        let mut child = self.child.lock().unwrap();
        for _ in 0..60 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Managed state: `None` when the sidecar could not start; the app still runs.
pub struct SidecarState(pub Option<Sidecar>);

impl SidecarState {
    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        match &self.0 {
            Some(sidecar) => sidecar.call(method, params),
            None => Err("sidecar_unavailable".into()),
        }
    }

    pub fn stop(&self) {
        if let Some(sidecar) = &self.0 {
            sidecar.stop();
        }
    }
}

/// Development locations; the installed app's are OPEN in source-structure.md.
pub fn start_default() -> SidecarState {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let node = std::env::var_os("AYLUL_NODE").map(PathBuf::from).unwrap_or_else(|| "node".into());
    let script = std::env::var_os("AYLUL_SIDECAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("../sidecar/src/main.ts"));
    let hook_exe = std::env::var_os("AYLUL_HOOK_EXE").map(PathBuf::from).unwrap_or_else(|| {
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("../hook/target"));
        target.join("release").join("aylul-hook.exe")
    });
    match Sidecar::start(&node, &script, &hook_exe) {
        Ok(sidecar) => SidecarState(Some(sidecar)),
        Err(error) => {
            eprintln!("aylul: sidecar did not start: {error}");
            SidecarState(None)
        }
    }
}
```

- [ ] **Step 3: Command and composition**

Append to `apps/desktop/src-tauri/src/commands/mod.rs`:

```rust
#[tauri::command]
pub async fn aylul_call(
    window: tauri::WebviewWindow,
    sidecar: tauri::State<'_, std::sync::Arc<crate::sidecar::SidecarState>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if window.label() != "main" {
        return Err("main_required".into());
    }
    let sidecar = sidecar.inner().clone();
    tauri::async_runtime::spawn_blocking(move || sidecar.call(&method, params))
        .await
        .map_err(|_| "action_failed".to_string())?
}
```

Replace `apps/desktop/src-tauri/src/lib.rs` with:

```rust
pub mod auth;
mod commands;
pub mod native_login;
mod sidecar;
mod windows;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::hide_main,
            commands::auth_action,
            commands::cancel_login,
            commands::aylul_call
        ])
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::open_main(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = windows::open_main(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri::Manager;
            let path = app.path().app_data_dir()?.join("auth-v1.dpapi");
            app.manage(auth::AuthController::new(path));
            app.manage(std::sync::Arc::new(sidecar::start_default()));
            windows::setup(app)
        })
        .build(tauri::generate_context!())
        .expect("NYLRA native runtime failed")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(sidecar) = app.try_state::<std::sync::Arc<sidecar::SidecarState>>() {
                    sidecar.stop();
                }
            }
        });
}
```

- [ ] **Step 4: Run tests, lint and build**

Run (from `apps/desktop`):
`cargo test --manifest-path src-tauri/Cargo.toml` then
`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` then
`cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
Expected: the 5 new sidecar tests pass with the existing 17; no warnings. If
`tauri-build` rewrote `src-tauri/gen/schemas/*.json` with line endings only,
restore them: `git checkout -- src-tauri/gen`.

- [ ] **Step 5: Contracts**

`apps/desktop/src-tauri/src/sidecar/contract.yaml`:

```yaml
name: desktop-sidecar-lifecycle
version: 1
status: current
source: apps/desktop/src-tauri/src/sidecar
responsibility: Start the Node sidecar, relay allowed calls over stdio, stop it on exit.
public_api:
  Sidecar::start: Spawn node with the script, AYLUL_HOOK_EXE set, no console window; one reader thread routes replies by id.
  Sidecar::call: Listed METHODS only; 10 s reply limit; the sidecar's error code is returned as the error.
  Sidecar::stop: Close stdin so the sidecar removes hook.json and exits; kill only after 3 s.
  SidecarState: None when start failed; every call then answers sidecar_unavailable.
  start_default: AYLUL_NODE, AYLUL_SIDECAR, AYLUL_HOOK_EXE, else node on PATH, ../sidecar/src/main.ts and the cargo target's release hook.
obligations:
  - rule: A call round-trips; sidecar error codes come back as errors.
    tested_by: a_call_reaches_the_sidecar_and_returns_its_result, a_sidecar_error_code_comes_back_as_the_error
  - rule: An unlisted method never reaches the sidecar.
    tested_by: an_unlisted_method_is_refused_before_it_reaches_the_sidecar
  - rule: A failed start leaves the app running and calls unavailable.
    tested_by: no_sidecar_answers_unavailable
  - rule: Stopping lets the sidecar exit by itself, so hook.json is removed; later calls are unavailable.
    tested_by: stop_lets_the_sidecar_exit_by_itself
gaps:
  - at: start_default
    gap: Paths are development paths; how the installer ships Node and the sidecar is OPEN.
  - at: Sidecar::start
    gap: A sidecar that dies is not restarted; calls then fail until AYLUL restarts.
```

Add to `public_api` in `apps/desktop/src-tauri/src/commands/contract.yaml`:

```yaml
  aylul_call: Require the main label; relay one listed method to the sidecar on a blocking worker; return its result or a stable code.
```

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src-tauri/src
git commit -m "feat(desktop): start the sidecar and relay the window's calls to it"
```

---

### Task 10: The Sessions and Requests screens

**Files:**
- Create: `apps/desktop/src/platform/aylul.ts`
- Create: `apps/desktop/src/data/aylul-state.ts`
- Create: `apps/desktop/src/features/sessions/SessionsPanel.tsx`, `apps/desktop/src/features/approvals/ApprovalsPanel.tsx`
- Create: `apps/desktop/src/features/contract.yaml`
- Modify: `apps/desktop/src/app/main.tsx`, `apps/desktop/src/platform/contract.yaml`

**Interfaces:**
- Consumes: Tauri command `aylul_call` (Task 9); sidecar result shapes (Tasks 3, 5, 8).
- Produces: the two screens inside the main panel.

- [ ] **Step 1: Platform wrapper**

`apps/desktop/src/platform/aylul.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';

export type Step = { at: string; tool: string; target: string; summary: string };
export type Session = {
  id: string; agent: 'claude_code'; project: string; state: 'running' | 'waiting' | 'ended';
  startedAt: string; updatedAt: string; lastStep: Step | null; lastMessage: string | null;
};
export type Pending = {
  id: string; kind: 'request' | 'refusal'; sessionId: string; project: string; tool: string; target: string;
  reason: string | null; proposedRule: string | null; at: string; held: boolean;
};
export type AylulState = { sessions: Session[]; pending: Pending[]; hooksInstalled: boolean };
/** A preview; `before` goes back with the approval so a file changed since is not overwritten. */
export type Plan = { path: string; lines: string[]; before: string };

const call = <T>(method: string, params: object = {}) => invoke<T>('aylul_call', { method, params });

export const getState = () => call<AylulState>('state');
export const answerRequest = (id: string, behavior: 'allow' | 'deny') => call<null>('request.answer', { id, behavior });
export const previewRule = (id: string, rule: string) => call<Plan>('refusal.preview', { id, rule });
export const approveRule = (id: string, rule: string, before: string) =>
  call<{ backupPath: string }>('refusal.approve', { id, rule, before });
export const dismissRefusal = (id: string) => call<null>('refusal.dismiss', { id });
export const previewHooks = () => call<Plan>('hooks.preview');
export const installHooks = (before: string) => call<{ backupPath: string }>('hooks.install', { before });

const MESSAGES: Record<string, string> = {
  sidecar_unavailable: "AYLUL's local service is not running. Restart AYLUL.",
  sidecar_timeout: "AYLUL's local service did not answer. Try again.",
  not_pending: 'This item was already answered or has ended.',
  rule_too_broad: 'That rule would allow too much. Name the exact command, one file as /path from the project root, or one site.',
  project_unknown: 'AYLUL does not know this session\'s project folder, so it cannot write the rule.',
  settings_invalid: 'The settings file is not valid JSON. AYLUL did not change it.',
  settings_changed: 'The settings file changed since the preview. Preview again.',
  hook_exe_missing: 'aylul-hook.exe was not found. Build it first.',
};
export const aylulMessage = (code: string) => MESSAGES[code] ?? 'Unable to complete this action. Please retry.';
```

- [ ] **Step 2: State polling**

`apps/desktop/src/data/aylul-state.ts`:

```ts
import React from 'react';
import { getState, type AylulState } from '../platform/aylul';

/** The sidecar's state, refreshed every second while the panel is visible. */
export function useAylulState(intervalMs = 1000) {
  const [state, setState] = React.useState<AylulState | null>(null);
  const refresh = React.useCallback(async () => {
    try { setState(await getState()); } catch { setState(null); }
  }, []);
  React.useEffect(() => {
    void refresh();
    const timer = setInterval(() => { if (!document.hidden) void refresh(); }, intervalMs);
    return () => clearInterval(timer);
  }, [refresh, intervalMs]);
  return { state, refresh };
}
```

- [ ] **Step 3: Sessions screen**

`apps/desktop/src/features/sessions/SessionsPanel.tsx`:

```tsx
import React from 'react';
import { aylulMessage, installHooks, previewHooks, type AylulState, type Plan } from '../../platform/aylul';

const LABEL = { running: 'Working', waiting: 'Waiting for you', ended: 'Ended' } as const;
const projectName = (path: string) => path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;

export function SessionsPanel({ state, refresh }: { state: AylulState | null; refresh: () => Promise<void> }) {
  const [plan, setPlan] = React.useState<Plan | null>(null);
  const [note, setNote] = React.useState('');
  const fail = (e: unknown) => setNote(aylulMessage(String(e)));
  if (!state) return <section aria-labelledby="sessions"><h2 id="sessions">Sessions</h2>
    <p role="status">{aylulMessage('sidecar_unavailable')}</p></section>;
  return <section aria-labelledby="sessions"><h2 id="sessions">Sessions</h2>
    {!state.hooksInstalled && <div>
      <p>Claude Code is not connected to AYLUL yet.</p>
      {plan ? <>
        <p>AYLUL will change <code>{plan.path}</code> (a backup is kept):</p>
        <pre>{plan.lines.join('\n')}</pre>
        <button onClick={() => void installHooks(plan.before)
          .then(r => { setPlan(null); setNote(r.backupPath ? `Backup: ${r.backupPath}` : ''); return refresh(); })
          .catch(fail)}>Connect</button>
        <button onClick={() => setPlan(null)}>Cancel</button>
      </> : <button onClick={() => void previewHooks().then(setPlan).catch(fail)}>Connect Claude Code</button>}
    </div>}
    {state.sessions.length === 0 ? <p>No sessions yet.</p> : <ul>
      {state.sessions.map(s => <li key={s.id}>
        <strong>{projectName(s.project)}</strong> · {LABEL[s.state]}
        {s.lastStep && <div>Last step: {s.lastStep.summary}</div>}
        {s.lastMessage && <div>Claude said: {s.lastMessage}</div>}
      </li>)}
    </ul>}
    {note && <p role="alert">{note}</p>}
  </section>;
}
```

- [ ] **Step 4: Requests screen**

`apps/desktop/src/features/approvals/ApprovalsPanel.tsx`:

```tsx
import React from 'react';
import { aylulMessage, answerRequest, approveRule, dismissRefusal, previewRule,
  type AylulState, type Pending, type Plan } from '../../platform/aylul';

export function ApprovalsPanel({ state, refresh }: { state: AylulState | null; refresh: () => Promise<void> }) {
  if (!state) return null;
  return <section aria-labelledby="requests"><h2 id="requests">Requests</h2>
    {state.pending.length === 0 ? <p>Nothing is waiting for you.</p> : <ul>
      {state.pending.map(p => <li key={p.id}>{p.kind === 'request'
        ? <RequestItem item={p} refresh={refresh} />
        : <RefusalItem item={p} refresh={refresh} />}</li>)}
    </ul>}
  </section>;
}

function RequestItem({ item, refresh }: { item: Pending; refresh: () => Promise<void> }) {
  const [note, setNote] = React.useState('');
  const answer = (behavior: 'allow' | 'deny') =>
    void answerRequest(item.id, behavior).then(refresh).catch(e => setNote(aylulMessage(String(e))));
  return <div>
    <p>Claude Code asks to run <strong>{item.tool}</strong>: <code>{item.target}</code></p>
    <button onClick={() => answer('allow')}>Allow once</button>
    <button onClick={() => answer('deny')}>Deny</button>
    {note && <p role="alert">{note}</p>}
  </div>;
}

function RefusalItem({ item, refresh }: { item: Pending; refresh: () => Promise<void> }) {
  const [rule, setRule] = React.useState(item.proposedRule ?? '');
  const [plan, setPlan] = React.useState<Plan | null>(null);
  const [note, setNote] = React.useState('');
  const fail = (e: unknown) => setNote(aylulMessage(String(e)));
  return <div>
    <p>Auto mode refused <strong>{item.tool}</strong>: <code>{item.target}</code></p>
    {item.reason && <p>Reason: {item.reason}</p>}
    <label>Rule to allow it in this project
      <input value={rule} onChange={e => { setRule(e.target.value); setPlan(null); }} /></label>
    {!item.proposedRule && <p>AYLUL could not propose a narrow rule. Write the exact command, file or site.</p>}
    {plan ? <>
      <p>AYLUL will change <code>{plan.path}</code> (a backup is kept):</p>
      <pre>{plan.lines.join('\n') || 'The rule is already there.'}</pre>
      <button onClick={() => void approveRule(item.id, rule, plan.before).then(refresh).catch(fail)}>Approve rule</button>
    </> : <button disabled={!rule.trim()} onClick={() => void previewRule(item.id, rule).then(setPlan).catch(fail)}>Preview</button>}
    <button onClick={() => void dismissRefusal(item.id).then(refresh).catch(fail)}>Dismiss</button>
    {item.held ? <p>Claude is waiting; after approval it tries again by itself.</p>
      : <p>After approval, ask Claude to try again.</p>}
    {note && <p role="alert">{note}</p>}
  </div>;
}
```

- [ ] **Step 5: Mount both screens**

In `apps/desktop/src/app/main.tsx`:
- add imports after the existing ones:

```tsx
import { useAylulState } from '../data/aylul-state';
import { SessionsPanel } from '../features/sessions/SessionsPanel';
import { ApprovalsPanel } from '../features/approvals/ApprovalsPanel';
```

- add as the first line inside `Shell()`: `const aylul=useAylulState();`
- insert right before `<section aria-labelledby="presence">`:

```tsx
    <SessionsPanel state={aylul.state} refresh={aylul.refresh} />
    <ApprovalsPanel state={aylul.state} refresh={aylul.refresh} />
```

- [ ] **Step 6: Build**

Run (in `apps/desktop`): `npm run build`. Expected: `tsc` and `vite build` succeed.

- [ ] **Step 7: Contracts**

`apps/desktop/src/features/contract.yaml`:

```yaml
name: desktop-aylul-screens
version: 1
status: current
source: apps/desktop/src/features
responsibility: Show sessions and the items waiting for the person, and carry the person's answers.
public_api:
  SessionsPanel: Connect Claude Code after showing the settings diff; list sessions with state, last step and last message.
  ApprovalsPanel: Allow once or deny a request; for a refusal, edit the rule, preview the diff, approve or dismiss.
obligations:
  - rule: A rule is approved only after its diff was shown.
    tested_by: none
    note: Approve appears only after Preview returned a plan; checked in the acceptance journey.
gaps:
  - at: ApprovalsPanel
    gap: No component tests; the desktop app has no frontend test runner yet.
```

Add to `public_api` in `apps/desktop/src/platform/contract.yaml`:

```yaml
  getState/answerRequest/previewRule/approveRule/dismissRefusal/previewHooks/installHooks: Invoke aylul_call with one sidecar method each.
  aylulMessage: Map a stable error code to a sentence; unknown codes to a generic retry.
```

`src/data/aylul-state.ts` is covered by the features contract (polling only).

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src
git commit -m "feat(desktop): sessions and requests screens"
```

---

### Task 11: CI, code map and the acceptance journey

**Files:**
- Modify: `.github/workflows/aylul.yml`
- Modify: `docs/aylul/codebase/README.md`
- Create: `docs/evidence/nylra/<date>-aylul-hook-laptop-acceptance.md` (date of the run)

- [ ] **Step 1: CI**

In `.github/workflows/aylul.yml`, change the rust-cache `workspaces` to:

```yaml
          workspaces: |
            apps/desktop/src-tauri -> target
            apps/desktop/hook -> target
```

and add these steps after `Run tests`, each as its own step so a failure stops
the job:

```yaml
      - name: Hook format
        run: cargo fmt --manifest-path hook/Cargo.toml --check

      - name: Hook clippy
        run: cargo clippy --manifest-path hook/Cargo.toml --all-targets -- -D warnings

      - name: Hook tests
        run: cargo test --manifest-path hook/Cargo.toml

      - name: Hook release build
        run: cargo build --release --manifest-path hook/Cargo.toml

      - name: Sidecar install
        working-directory: apps/desktop/sidecar
        run: npm ci

      - name: Sidecar typecheck
        working-directory: apps/desktop/sidecar
        run: npm run typecheck

      - name: Sidecar tests
        working-directory: apps/desktop/sidecar
        env:
          AYLUL_HOOK_EXE: ${{ github.workspace }}\apps\desktop\hook\target\release\aylul-hook.exe
        run: npm test
```

- [ ] **Step 2: Code map**

Add rows to the table in `docs/aylul/codebase/README.md`:

```markdown
| `apps/desktop/hook/` | aylul-hook: carry one Claude Code hook event to the sidecar | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/hooks/` | Hook pipe endpoint, server and event route | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/sessions/` | Session state and trimmed steps from hook events | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/approvals/` | Requests and refusals held until the person answers | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/agents/` | Narrow rules and Claude Code settings changes | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/store/` | node:sqlite records | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/` (`main`, `rpc`, `handlers`) | Sidecar composition and stdio calls | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/sidecar/` | Sidecar lifecycle and call relay | Colocated `contract.yaml` |
| `apps/desktop/src/features/` | Sessions and Requests screens | Colocated `contract.yaml` |
```

and change its `Date:` line to the day of this change.

- [ ] **Step 3: Full local gate**

Run, from `apps/desktop`, with the cargo env vars from Global Constraints:
1. `npm run build`
2. `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
3. `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
4. `cargo test --manifest-path src-tauri/Cargo.toml`
5. `cargo fmt --manifest-path hook/Cargo.toml --check`, `cargo clippy --manifest-path hook/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path hook/Cargo.toml`, `cargo build --release --manifest-path hook/Cargo.toml`
6. In `sidecar`: `npm ci`, `npm run typecheck`, then with `AYLUL_HOOK_EXE` set: `npm test`
7. From the repo root: `node apps/desktop/tests/check-auth.cjs`

Expected: every command passes; report the test counts.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/aylul.yml docs/aylul/codebase/README.md
git commit -m "ci: hook and sidecar checks; code map for step 2(a)"
```

- [ ] **Step 5: Acceptance journey (with Mohammed, on his laptop)**

This needs the person; the executor prepares it and records what happened.
Start AYLUL from `apps/desktop` (`cargo run --manifest-path src-tauri/Cargo.toml`
after `npm run build`), then:

1. Sessions screen → **Connect Claude Code** → the diff of
   `~/.claude/settings.json` is shown → **Connect**. Record the backup path.
2. Start `claude` in a throwaway folder; ask it to create a file. The session
   appears with its project name, last step and, after it finishes, its last
   message.
3. In the same folder run `claude --permission-mode default` and ask for a file
   again. The request appears in AYLUL → **Allow once** → the terminal dialog
   closes and the file is written.
4. Repeat 3 and answer in the terminal instead. The request leaves AYLUL; the
   store records `answered_in_terminal_allowed`.
5. Quit AYLUL; repeat 3. Claude Code asks in the terminal as if AYLUL did not
   exist.
6. The OPEN in the spec (§5): at the first real auto-mode refusal in the
   person's work, approve the proposed rule in AYLUL; record whether Claude ran
   the call after `retry` or after the person asked again, and that the rule
   landed in the project root's `.claude/settings.local.json` even if Claude
   had moved into a subfolder.

Write `docs/evidence/nylra/<date>-aylul-hook-laptop-acceptance.md` with each step's
result and screenshots of AYLUL's window only (never the whole desktop). Do not
include session ids or local paths beyond the project folder name.

```bash
git add docs/evidence
git commit -m "docs(evidence): step 2(a) acceptance on Windows"
```
