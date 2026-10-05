# The `/` Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typing `/` at the start of the composer opens the harness's commands and skills, filtered as the person types; picking one writes `/<name> ` into the box.

**Architecture:** `shadows-agent` maps ACP's `available_commands_update` to `HarnessEvent::Commands`. A new `harness/commands.rs` keeps each open session's latest list in memory and broadcasts changes. The thread stream sends a transient `commands` frame after `caught-up` and on every change. The web client keeps the last list per thread in the query cache, and the composer shows a menu built from it.

**Tech Stack:** Rust (tokio, agent-client-protocol 2.2.0, axum SSE, utoipa), React + TanStack Query + Vitest in `web/`.

**Spec:** `docs/superpowers/specs/2026-10-05-slash-menu-design.md` (§21). Read it first; this plan argues from it.

## Global Constraints

- Branch `next/slash-menu`, already checked out. No worktrees. Do not push.
- Build with `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0` in front of every cargo command.
- No Rust line over 100 columns. Run `cargo fmt --all` before each commit (it is the project's formatter; it is not prettier).
- Never run prettier or any repo-wide formatter in `web/`. Do not use Python or `sed` to edit files; use the editor tools.
- No SQLite, no migration: the list lives only in daemon memory (§21.2).
- The menu shows only the harness's list; Shadows adds no commands (§21 intro).
- Targeted tests while working. The full gate runs once, at the end of the branch, not per task.
- A change to a service updates its `contract.yaml` in the same commit; a new module gets its row in `docs/codebase/README.md` (`cargo test -p shadows --test codemap` checks it).
- A file passing 300 lines states its one job in the commit message; 500 splits.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **The person switches conversation while the menu is open.** The menu must show the new thread's list (or close), never the old one's. Pinned in Task 2 (`use-commands.test.ts`: the cache is per thread, `useCommands(threadId)` reads only its own) and by construction in Task 3 (`commands` is `useConversation`'s, keyed by the open thread).
2. **The list changes while the menu is open** (the adapter sends a new one mid-session) and the highlighted index is past its end. The highlight clamps; Enter never picks `undefined`. Pinned in Task 3.
3. **Enter while the menu is open and a turn runs or the session is not ready.** It picks; it never sends or queues. Pinned in Task 3.
4. **Pasted text such as `/compact\nmore`.** A newline closes the menu; Enter sends as usual. Pinned in Task 3.
5. **A 144-entry list.** The menu scrolls, and the highlighted entry stays in view while arrowing. Pinned in Task 3 (`scrollIntoView` called on highlight change).

---

### Task 1: The daemon carries the list to the thread stream

**Files:**
- Modify: `crates/shadows-agent/src/events.rs` (add `SlashCommand`, `HarnessEvent::Commands`)
- Modify: `crates/shadows-agent/src/acp.rs` (fn `forward`, ~line 351)
- Create: `crates/shadows-core/src/harness/commands.rs`
- Modify: `crates/shadows-core/src/harness/mod.rs` (module list, doc comment line ~10, `watch_commands`, `commands_of`)
- Modify: `crates/shadows-core/src/harness/sessions.rs` (field, constructor, one `forget`, event chain in `open_live`)
- Modify: `crates/shadows-core/src/turns/entries.rs:66-69` (ignore `Commands` like `Options`)
- Modify: `crates/shadows-core/src/events/mod.rs` (take the commands receiver in both `Live` builders)
- Modify: `crates/shadows-core/src/events/subscription.rs` (`Delivery::Commands`, `Live.commands`, after-caught-up send, select branch)
- Modify: `crates/shadows-http/src/sse.rs` (frame `commands`, frames doc text ~line 193, `frame` doc ~line 231)
- Modify: `crates/fake-acp/src/main.rs` (send the update after `session/new` and `session/resume`)
- Modify: `crates/shadows-core/src/harness/contract.yaml`, `crates/shadows-core/src/events/contract.yaml`, `docs/codebase/README.md`
- Modify: `api/openapi.json` (regenerated, the SSE description changed)
- Test: `crates/shadows/tests/slash_commands.rs` (new)

**Interfaces:**
- Produces (Rust): `shadows_agent::events::SlashCommand { pub name: String, pub description: String, pub hint: Option<String> }` deriving `Debug, Clone, PartialEq, serde::Serialize`; `HarnessEvent::Commands(Vec<SlashCommand>)`.
- Produces (SSE): frame `commands`, data `{"thread_id": "<id>", "commands": [{"name": "...", "description": "...", "hint": "..." | null}]}`. Task 2 parses exactly this.

- [ ] **Step 1: Read first.** Read the spec, `docs/codebase/README.md` rows for `harness` and `events`, and `crates/shadows-core/src/harness/contract.yaml`. Read `harness/offers.rs` (78 lines) and `harness/titles.rs` (43 lines) whole: `commands.rs` copies their shape. Check the exact ACP 2.2.0 types in the registry source: `grep -rn "pub struct AvailableCommand\b\|pub enum AvailableCommandInput\|pub struct UnstructuredCommandInput\|AvailableCommandsUpdate" ~/.cargo/registry/src/*/agent-client-protocol-schema*/src/` (the schema crate may be named differently; find it with `ls ~/.cargo/registry/src/*/ | grep agent-client`). Note the constructors (`::new(..)`) and the hint field path; the code below assumes `AvailableCommandInput::Unstructured(u)` with `u.hint`, adjust to what the source says.

- [ ] **Step 2: Write the failing test** `crates/shadows/tests/slash_commands.rs`:

```rust
//! The `/` menu's list (spec §21): the harness's `available_commands_update`
//! reaches the thread stream as a transient `commands` frame, live and again
//! after `caught-up` for a stream opened later.

use serde_json::{Value, json};
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;

use app::{post, test_app};

fn names(frame: &Value) -> Vec<String> {
    frame["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn opening_a_session_sends_its_commands_to_a_subscriber() {
    let app = test_app().await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    post(&app, &format!("/api/threads/{}/session", app.thread), json!({})).await;
    let frame = app::next_frame_named(&mut sub, "commands").await;
    assert_eq!(frame["thread_id"], app.thread.as_str());
    assert_eq!(names(&frame), ["compact", "superpowers:brainstorming"]);
    assert_eq!(frame["commands"][0]["hint"], Value::Null);
    assert_eq!(frame["commands"][1]["hint"], "[topic]");
}

#[tokio::test]
async fn a_stream_opened_after_the_list_gets_it_after_caught_up() {
    let app = test_app().await;
    let mut first = app::subscribe(&app, &app.thread).await;
    post(&app, &format!("/api/threads/{}/session", app.thread), json!({})).await;
    app::next_frame_named(&mut first, "commands").await;
    let mut later = app::subscribe(&app, &app.thread).await;
    let frame = app::next_frame_named(&mut later, "commands").await;
    assert_eq!(names(&frame), ["compact", "superpowers:brainstorming"]);
}
```

The `use shadows_core::testing::acp;` line is needed: `fixtures/app.rs` does `use super::acp;` (as `harness_choices.rs` does). `subscribe` (waits for `caught-up`), `next_frame_named`, `post` and `test_app` exist in `fixtures/app.rs` with these shapes (`app.thread` is a `ThreadId`).

- [ ] **Step 3: Run it to see it fail.**

Run: `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0 cargo test -p shadows --test slash_commands`
(No feature flag: `shadows` enables `test-support` through its self dev-dependency, and `fake-acp` is built by the test apparatus. It is the same invocation as `harness_choices`.)
Expected: FAIL, panicking `no frame in time` waiting for a `commands` frame (10 s).

- [ ] **Step 4: The fake adapter sends a list.** In `crates/fake-acp/src/main.rs`, the `NewSessionRequest` handler's `_cx` becomes `cx`; it responds, then sends the update. Do the same in the `ResumeSessionRequest` handler (the real adapter sends after new, resume and load, not fork, §21.1; the fake's fork has its own handler, which stays as it is). The handlers keep `st` (a std `MutexGuard`) to their end: `drop(st)` before sending, and clone `id` where it is both inserted and used. Add the list builder to `crates/fake-acp/src/session.rs` (import `AvailableCommand`, `AvailableCommandInput`, `UnstructuredCommandInput` from `agent_client_protocol::schema::v1` there, and `AvailableCommandsUpdate`, `SessionUpdate` in `main.rs`):

```rust
/// The `/` list the fake offers after `session/new` and `session/resume`
/// (§21.1): a built-in command without a hint and a skill with one.
pub(crate) fn commands() -> Vec<AvailableCommand> {
    vec![
        AvailableCommand::new("compact", "Clear history but keep a summary"),
        AvailableCommand::new("superpowers:brainstorming", "Explore intent before building")
            .input(AvailableCommandInput::Unstructured(UnstructuredCommandInput::new("[topic]"))),
    ]
}
```

and in the `new` handler (the `resume` one is the same with `ResumeSessionResponse::new().config_options(opts)` and `id` already in hand):

```rust
let opts = options(&s);
st.sessions.insert(id.clone(), s);
drop(st);
responder.respond(NewSessionResponse::new(id.clone()).config_options(opts))?;
let list = AvailableCommandsUpdate::new(session::commands());
update(&cx, &id, SessionUpdate::AvailableCommandsUpdate(list))
```

(`update` is already imported in `main.rs` from `prompts`, `pub(crate)` at `prompts.rs:30`. The constructors above are checked against the schema crate 1.9.1's `v1` module that `agent_client_protocol::schema::v1` re-exports: `AvailableCommand::new(name, description).input(..)`, `AvailableCommandInput::Unstructured(UnstructuredCommandInput::new(hint))`, `AvailableCommandsUpdate::new(vec)`.)

- [ ] **Step 5: `shadows-agent` forwards it.** In `events.rs`, beside `LimitWindow`:

```rust
/// One entry of the harness's `/` list (spec §21.1): a skill, a plugin
/// command or a built-in command, with no field saying which.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    /// What the command takes after its name, e.g. `[topic]`.
    pub hint: Option<String>,
}
```

Add to `HarnessEvent`, after `Options(Value)`:

```rust
    /// The harness's complete `/` list (§21), sent after the session opens
    /// and whenever it changes, between turns too.
    Commands(Vec<SlashCommand>),
```

In `acp.rs`, add `AvailableCommandInput` to the `v1::{…}` import list and change `use super::events::HarnessEvent;` to `use super::events::{HarnessEvent, SlashCommand};`. In `forward`, before the `other =>` arm (after `SessionInfoUpdate`):

```rust
        SessionUpdate::AvailableCommandsUpdate(u) => Some(HarnessEvent::Commands(
            u.available_commands
                .into_iter()
                .map(|c| SlashCommand {
                    hint: match c.input {
                        Some(AvailableCommandInput::Unstructured(i)) => Some(i.hint),
                        _ => None,
                    },
                    name: c.name,
                    description: c.description,
                })
                .collect(),
        )),
```

In `turns/entries.rs:66-69` add `| HarnessEvent::Commands(_)` to the arm that does nothing.

- [ ] **Step 6: Create `crates/shadows-core/src/harness/commands.rs`:**

```rust
//! One job: the latest `/` list each open session sent (spec §21.2).
//!
//! Every `available_commands_update` is the complete list, so each one
//! replaces the thread's entry here and is published to whoever watches (the
//! SSE `commands` frame). Memory only: a restarted daemon has none until the
//! session opens again and the adapter sends it again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::threads::ThreadId;
use shadows_agent::events::{HarnessEvent, SlashCommand};

pub(super) struct Commands {
    latest: Mutex<HashMap<ThreadId, Vec<SlashCommand>>>,
    changed: broadcast::Sender<(ThreadId, Vec<SlashCommand>)>,
}

impl Commands {
    pub(super) fn new() -> Self {
        Self {
            latest: Mutex::new(HashMap::new()),
            changed: broadcast::channel(256).0,
        }
    }

    fn record(&self, thread: &ThreadId, list: Vec<SlashCommand>) {
        self.latest
            .lock()
            .expect("commands lock")
            .insert(thread.clone(), list.clone());
        // Nobody watching is not a failure: the next stream sends it.
        let _ = self.changed.send((thread.clone(), list));
    }

    pub(super) fn get(&self, thread: &ThreadId) -> Option<Vec<SlashCommand>> {
        self.latest.lock().expect("commands lock").get(thread).cloned()
    }

    /// Publishes nothing (§21.2): a client keeps its last list.
    pub(super) fn forget(&self, thread: &ThreadId) {
        self.latest.lock().expect("commands lock").remove(thread);
    }

    pub(super) fn subscribe(&self) -> broadcast::Receiver<(ThreadId, Vec<SlashCommand>)> {
        self.changed.subscribe()
    }
}

/// Takes the `/` list off the connection, in its own dispatch, and hands
/// every other event on to `next`. It arrives between turns too, when nobody
/// reads the thread's events.
pub(super) fn keep_commands(
    commands: Arc<Commands>,
    thread: ThreadId,
    next: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
) -> impl Fn(HarnessEvent) + Clone + Send + Sync + 'static {
    move |event| match event {
        HarnessEvent::Commands(list) => commands.record(&thread, list),
        other => next(other),
    }
}
```

Add a unit test module at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn one(name: &str) -> SlashCommand {
        SlashCommand { name: name.into(), description: String::new(), hint: None }
    }

    #[test]
    fn a_new_list_replaces_the_old_and_forget_drops_it() {
        let commands = Commands::new();
        let thread = ThreadId::generate();
        let mut rx = commands.subscribe();
        commands.record(&thread, vec![one("a")]);
        commands.record(&thread, vec![one("b")]);
        assert_eq!(commands.get(&thread), Some(vec![one("b")]));
        assert_eq!(rx.try_recv().unwrap().1, vec![one("a")]);
        commands.forget(&thread);
        assert_eq!(commands.get(&thread), None);
    }
}
```

(`ThreadId` has no `From<String>`; `ThreadId::generate()` is the public constructor.)

- [ ] **Step 7: Wire it into `Sessions` and `Harness`.**
  - `harness/mod.rs`: add `mod commands;` beside `mod context;` (alphabetical), and name `commands` in the module doc list (line ~10: "`commands` the latest `/` list each session sent").
  - `sessions.rs`: add `use super::commands::{Commands, keep_commands};` beside the other `super::` imports (line ~21), field `pub(super) commands: Arc<Commands>` beside `offers` (line ~126), built with `Arc::new(Commands::new())` (line ~148).
  - Replace every `self.offers.forget(x); self.setups.forget(x).await;` pair (lines ~186, ~241, ~357, ~375, ~389, ~418) with `self.forget(x).await;` and add:

```rust
    /// Drops what a closed or failed session left: its offer, its `/` list
    /// and its grant.
    async fn forget(&self, thread: &ThreadId) {
        self.offers.forget(thread);
        self.commands.forget(thread);
        self.setups.forget(thread).await;
    }
```

  Line ~209 forgets only setups (spawn failed, nothing recorded yet): leave it.
  - In `open_live`, after `let events = keep_titles(...)` (line ~215): `let events = keep_commands(self.commands.clone(), thread.clone(), events);`
  - In `harness/mod.rs`, beside `watch_options` (line ~176), add `pub(crate)` methods that read `Sessions.commands` directly (it is `pub(super)`, and `Commands::get` / `subscribe` are visible to the `harness` module), so `sessions.rs` gains no pass-through:

```rust
    /// The thread's latest `/` list, if its open session sent one (§21.2).
    pub(crate) fn commands_of(&self, thread: &ThreadId) -> Option<Vec<SlashCommand>> {
        self.sessions.commands.get(thread)
    }

    /// Every change to any thread's `/` list, as it happens. `Events` takes
    /// it for each subscriber, before the journal is read.
    pub(crate) fn watch_commands(&self) -> broadcast::Receiver<(ThreadId, Vec<SlashCommand>)> {
        self.sessions.commands.subscribe()
    }
```

  (import `SlashCommand` beside `AccountLimits` in `harness/mod.rs`.)
  - `sessions.rs` is at 500 lines and nets about +6 here (six pairs shrink by 6; `forget`, the field, its constructor, the import and the chain line add about 12). It passes 500 with one responsibility (the live adapter each open thread has): say so in the commit message. Do not move code out to compensate.

- [ ] **Step 8: The stream delivers it.** In `events/subscription.rs`:
  - Add to `Delivery`, after `Options`:

```rust
    /// The harness's `/` list (§21.3): after `caught-up` when the thread has
    /// one, and on every change. Transient.
    Commands {
        thread: ThreadId,
        commands: Vec<SlashCommand>,
    },
```

  - Add `pub(super) commands: broadcast::Receiver<(ThreadId, Vec<SlashCommand>)>` to `Live`, and in `events/mod.rs` both builders take `commands: self.harness.watch_commands(),` right after `options:`.
  - Add a field `commands_due: bool` (false in `new`). Where `next` returns `CaughtUp`, set `self.commands_due = matches!(self.scope, Scope::Thread(_));` before returning. At the top of the loop, after the `self.reading` block and before `self.live()`:

```rust
            if self.commands_due {
                self.commands_due = false;
                if let Scope::Thread(thread) = &self.scope
                    && let Some(commands) = self.harness.commands_of(thread)
                {
                    let thread = thread.clone();
                    return Ok(Delivery::Commands { thread, commands });
                }
            }
```

  - In `live()`, add a select branch after `options`: `listed = self.live.commands.recv() => Woke::Commands(listed),`, a `Woke::Commands(Result<(ThreadId, Vec<SlashCommand>), RecvError>)` variant, and arms copying the `Options` ones: own thread → `Some(Ok(Delivery::Commands { thread, commands }))`; other thread → `None`; `Lagged` → `Delivery::Lagged`; `Closed` → `self.end("shutdown: commands closed")`.
  - Update the module doc's "beside it the session's options" line to "the session's options and `/` list". The file passes 300 lines: its one job is unchanged (one subscriber's replay, handoff and live phase); say so in the commit message.

- [ ] **Step 9: The SSE frame.** In `crates/shadows-http/src/sse.rs` `frame()`:

```rust
        // The harness's `/` list (§21.3).
        Delivery::Commands { thread, commands } => Event::default()
            .event("commands")
            .data(serde_json::json!({ "thread_id": thread, "commands": commands }).to_string()),
```

Add to the frames description, after the `options` line:
`- \`commands\` — \`{thread_id, commands}\`: the harness's \`/\` list, each \`{name, description, hint}\`; sent after \`caught-up\` when there is one, and on every change. Transient.\n\`
and add `commands` to `frame`'s doc list. Regenerate: `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` (with the target-dir prefix).

- [ ] **Step 10: Contracts and code map.** `harness/contract.yaml`: under `for_events`, add `watch_commands` and `commands_of` with signatures and one-line comments like `watch_options`; under obligations/agreements, one line: the list is memory only and forgotten with the offer. `events/contract.yaml`: where it says `Events::subscribe` takes `harness.watch_options()` (line ~118), add `harness.watch_commands()`, and that a thread stream sends the list right after `caught-up`. `docs/codebase/README.md`: a row for `crates/shadows-core/src/harness/commands.rs`, job "the latest `/` list each open session sent" (no "and").

- [ ] **Step 11: Run the targeted tests.**

Run (prefix each with the target-dir env): `cargo test -p shadows --test slash_commands`, `cargo test -p shadows --test harness_choices`, `cargo test -p shadows --test stream_frames`, `cargo test -p shadows-core --lib harness::commands`, `cargo test -p shadows-core --test acp_connection` and `--test thread_session` (the fake now sends a list they must tolerate), `cargo test -p shadows --test message_queue`, `cargo test -p shadows --test codemap`, `cargo test -p shadows --test openapi`, and the contracts test (`grep -rln "contract.yaml" crates/*/tests | head` finds it).
Expected: all PASS.

- [ ] **Step 12: Commit.**

```bash
cargo fmt --all
git add -A crates api docs/codebase
git commit -m "§21 daemon: the harness's / list reaches the thread stream

subscription.rs passes 300 lines: still one job, one subscriber's replay,
handoff and live phase; the list is one more live source.
sessions.rs passes 500: still one job, the live adapter each open thread
has; the six offer-and-setup forgets became one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The web client keeps each thread's list

**Files:**
- Modify: `web/src/stream/frames.ts` (types + `parseCommands`)
- Modify: `web/src/stream/thread-stream.ts` (`Notice` gains `commands`; `on('commands', …)`)
- Create: `web/src/app/conversation/use-commands.ts`
- Modify: `web/src/app/conversation/use-conversation.ts` (store the notice; return the list)
- Modify: `web/src/app/conversation/use-session.ts` (`reopenSession`, defined at line 51 and called only by `cli-picker.tsx:50`: it also calls `clearCommands`, so the one call site needs no edit)
- Test: `web/src/stream/frames.test.ts`, `web/src/stream/thread-stream.test.ts`, `web/src/app/conversation/use-commands.test.ts` (new)

**Interfaces:**
- Consumes: SSE frame `commands` `{thread_id, commands: [{name, description, hint|null}]}` (Task 1).
- Produces: `export type SlashCommand = { name: string; description: string; hint: string | null }` in `frames.ts`; `Notice` variant `{ type: 'commands'; commands: readonly SlashCommand[] }`; in `use-commands.ts`: `commandsKey(threadId)`, `replaceCommands(queryClient, threadId, list)`, `clearCommands(queryClient, threadId)`, `useCommands(threadId: string | null): readonly SlashCommand[]` (empty for `null` or none yet). `useConversation` exposes `commands` from `useCommands(threadId)`.

- [ ] **Step 1: Failing tests.** In `frames.test.ts`, following the file's existing `parseOptions` tests:

```ts
describe('parseCommands', () => {
  it('reads the list with and without a hint', () => {
    const data = JSON.stringify({
      thread_id: 't1',
      commands: [
        { name: 'compact', description: 'Clear history', hint: null },
        { name: 'superpowers:brainstorming', description: 'Explore', hint: '[topic]' },
      ],
    })
    expect(parseCommands(data)).toEqual({
      threadId: 't1',
      commands: [
        { name: 'compact', description: 'Clear history', hint: null },
        { name: 'superpowers:brainstorming', description: 'Explore', hint: '[topic]' },
      ],
    })
  })

  it('refuses an entry without a name', () => {
    const data = JSON.stringify({ thread_id: 't1', commands: [{ description: 'x', hint: null }] })
    expect(() => parseCommands(data)).toThrow(FrameError)
  })
})
```

In `thread-stream.test.ts`, following its `hands on usage and options frames as notices` test (line ~204, `harness({ onNotice })`, `current().emit(...)`): a `commands` frame calls `onNotice` with `{ type: 'commands', commands: [...] }`:

```ts
  it('hands on a commands frame as a notice', () => {
    const notices: Notice[] = []
    const { stream, current } = harness({ onNotice: (n) => notices.push(n) })
    stream.start()
    const commands = [{ name: 'compact', description: 'Clear history', hint: null }]
    current().emit('commands', JSON.stringify({ thread_id: 't', commands }))
    expect(notices).toEqual([{ type: 'commands', commands }])
  })
```

Add `parseCommands` to the import list of `frames.test.ts` (it already imports `FrameError`).

`use-commands.test.ts`:

```ts
import { QueryClient } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { clearCommands, commandsKey, replaceCommands } from './use-commands'

describe('the thread command cache', () => {
  it('keeps the last list per thread and clears on a harness switch', () => {
    const qc = new QueryClient()
    const list = [{ name: 'compact', description: '', hint: null }]
    replaceCommands(qc, 't1', list)
    expect(qc.getQueryData(commandsKey('t1'))).toEqual(list)
    expect(qc.getQueryData(commandsKey('t2'))).toBeUndefined()
    clearCommands(qc, 't1')
    expect(qc.getQueryData(commandsKey('t1'))).toEqual([])
  })
})
```

- [ ] **Step 2: Run to see them fail.** `cd web && npx vitest run src/stream/frames.test.ts src/stream/thread-stream.test.ts src/app/conversation/use-commands.test.ts` → FAIL (missing exports).

- [ ] **Step 3: Implement.** `frames.ts`, beside `parseOptions`, using the file's own `object` and `FrameError` helpers:

```ts
export type SlashCommand = { name: string; description: string; hint: string | null }
export type CommandsFrame = { threadId: string; commands: readonly SlashCommand[] }

function isSlashCommand(value: unknown): value is SlashCommand {
  if (typeof value !== 'object' || value === null) return false
  const c = value as Record<string, unknown>
  return (
    typeof c.name === 'string' &&
    typeof c.description === 'string' &&
    (c.hint === null || typeof c.hint === 'string')
  )
}

export function parseCommands(data: string): CommandsFrame {
  const frame = object('commands', data)
  const threadId = frame.thread_id
  const commands = frame.commands
  if (typeof threadId !== 'string' || !Array.isArray(commands) || !commands.every(isSlashCommand)) {
    throw new FrameError('commands', data)
  }
  return { threadId, commands }
}
```

`thread-stream.ts`: add `| { type: 'commands'; commands: readonly SlashCommand[] }` to `Notice` (and to its doc comment: "the harness's `/` list (`commands`, §21)"), import `parseCommands`, and beside `on('options', …)`:

```ts
    on('commands', (data) => {
      this.#options.onNotice?.({ type: 'commands', commands: parseCommands(data).commands })
    })
```

`use-commands.ts`:

```ts
// One job: the `/` list each conversation's harness last sent (spec §21.3),
// kept per thread in the query cache. Only the stream fills it; nothing
// fetches it, so it is never stale and never refetched.

import { type QueryClient, useQuery } from '@tanstack/react-query'
import type { SlashCommand } from '@/stream/frames'

const NONE: readonly SlashCommand[] = []

export function commandsKey(threadId: string) {
  return ['threads', threadId, 'commands'] as const
}

export function replaceCommands(qc: QueryClient, threadId: string, list: readonly SlashCommand[]) {
  qc.setQueryData(commandsKey(threadId), list)
}

/** The thread's harness changed (§12.6): the old harness's list is not the new one's. */
export function clearCommands(qc: QueryClient, threadId: string) {
  qc.setQueryData(commandsKey(threadId), NONE)
}

export function useCommands(threadId: string | null): readonly SlashCommand[] {
  const { data } = useQuery({
    queryKey: commandsKey(threadId ?? ''),
    queryFn: () => NONE,
    enabled: false,
    staleTime: Infinity,
  })
  return threadId === null ? NONE : (data ?? NONE)
}
```

`use-conversation.ts`: in the notice handler (line ~63, beside `if (notice.type === 'options') replaceChoices(...)`) add `if (notice.type === 'commands') replaceCommands(queryClient, threadId, notice.commands)`, and return `commands: useCommands(threadId)` in the returned object (line ~106). `use-session.ts`: `reopenSession` becomes

```ts
export function reopenSession(queryClient: QueryClient, threadId: string) {
  clearCommands(queryClient, threadId)
  void queryClient.resetQueries({ queryKey: sessionKey(threadId) })
}
```

(import `clearCommands` from `./use-commands`; `use-commands.ts` imports nothing from `use-session.ts`, so there is no cycle.) The `isSlashCommand` guard's `value is SlashCommand` is fine for oxlint; keep the long `if` in `parseCommands` split over lines if it passes 100 columns.

- [ ] **Step 4: Run the tests.** Same command as Step 2, plus `npx vitest run src/app/conversation` → PASS. Then `npm run typecheck` (`tsc -b`) and `npm run lint` (`oxlint --deny-warnings`) → no errors.

- [ ] **Step 5: Commit.**

```bash
git add web/src
git commit -m "§21 web: keep each thread's / list from the stream

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The composer's `/` menu

**Files:**
- Create: `web/src/app/conversation/slash-filter.ts` (pure: when the menu applies, what it lists)
- Create: `web/src/app/conversation/slash-menu.tsx` (the list and the description panel)
- Modify: `web/src/app/conversation/composer.tsx` (prop `commands`; keys; render; hint overlay)
- Modify: `web/src/app/conversation/conversation.tsx` (pass `commands={c.commands}` from `useConversation`); the other call site, `draft.tsx:120` (no thread), is left alone: the prop is optional and defaults to no commands
- Test: `web/src/app/conversation/slash-filter.test.ts`, `web/src/app/conversation/slash-menu.test.tsx` (new)

**Interfaces:**
- Consumes: `SlashCommand` from `@/stream/frames`; `commands: readonly SlashCommand[]` from `useConversation` (Task 2).
- Produces: `slashMatches(commands: readonly SlashCommand[], text: string): readonly SlashCommand[] | null` (`null` = the menu does not apply to this text); `<SlashMenu items highlighted onPick onHighlight />`; `Composer` optional prop `commands`.

- [ ] **Step 1: Failing filter tests** `slash-filter.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { slashMatches } from './slash-filter'

const c = (name: string) => ({ name, description: '', hint: null })
const list = [c('review'), c('superpowers:brainstorming'), c('init'), c('codex:review'), c('brief')]
const names = (r: ReturnType<typeof slashMatches>) => r?.map((x) => x.name) ?? null

describe('slashMatches', () => {
  it('applies only to a whole text of / and no whitespace', () => {
    expect(names(slashMatches(list, '/'))).toEqual(list.map((x) => x.name))
    expect(slashMatches(list, 'hi /br')).toBeNull()
    expect(slashMatches(list, '/br ')).toBeNull()
    expect(slashMatches(list, '/br\nx')).toBeNull()
    expect(slashMatches(list, '')).toBeNull()
  })

  it('orders name prefix, then the part after the last colon, then contains', () => {
    expect(names(slashMatches(list, '/br'))).toEqual(['brief', 'superpowers:brainstorming'])
    expect(names(slashMatches(list, '/rev'))).toEqual(['review', 'codex:review'])
    expect(names(slashMatches(list, '/storm'))).toEqual(['superpowers:brainstorming'])
  })

  it('ignores case and keeps the adapter order within a group', () => {
    expect(names(slashMatches(list, '/BR'))).toEqual(['brief', 'superpowers:brainstorming'])
  })

  it('applies with no match, listing nothing', () => {
    expect(slashMatches(list, '/zzz')).toEqual([])
  })
})
```

- [ ] **Step 2: Run → FAIL.** `cd web && npx vitest run src/app/conversation/slash-filter.test.ts`

- [ ] **Step 3: Implement `slash-filter.ts`:**

```ts
// One job: which `/` entries a composer text lists (spec §21.4). `null` when
// the menu does not apply: the whole text must be `/` and no whitespace.

import type { SlashCommand } from '@/stream/frames'

export function slashMatches(
  commands: readonly SlashCommand[],
  text: string,
): readonly SlashCommand[] | null {
  if (!/^\/\S*$/.test(text)) return null
  const typed = text.slice(1).toLowerCase()
  const starts: SlashCommand[] = []
  const afterColon: SlashCommand[] = []
  const contains: SlashCommand[] = []
  for (const command of commands) {
    const name = command.name.toLowerCase()
    if (name.startsWith(typed)) starts.push(command)
    else if (name.slice(name.lastIndexOf(':') + 1).startsWith(typed)) afterColon.push(command)
    else if (name.includes(typed)) contains.push(command)
  }
  return [...starts, ...afterColon, ...contains]
}
```

Run Step 2's command → PASS.

- [ ] **Step 4: Failing composer tests** `slash-menu.test.tsx`. The web tests here run the whole app over a faked daemon (`startApp`, `answers`, `typeInto`, `until` from `../test-app` and `@/test/fake-daemon`; `app.pushFrame` delivers an SSE frame), in happy-dom, with no React Testing Library and no `userEvent`: model the file on `waiting-messages.test.tsx` (Enter is `box.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))` inside `act`). The list arrives through a real `commands` frame, so these tests also cover Task 2's path.

```tsx
// @vitest-environment happy-dom
//
// The composer's `/` menu (§21.4) over a faked daemon: the harness's list
// arrives as a `commands` frame; the menu opens, filters, picks and closes.

import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import { runningOperation } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

const COMMANDS = [
  { name: 'compact', description: 'Clear history', hint: null },
  { name: 'superpowers:brainstorming', description: 'Explore intent', hint: '[topic]' },
]

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  vi.restoreAllMocks()
})

/** The conversation at t1, its stream caught up, the list delivered and the
 * session ready (Send acts), or a turn running (Enter would queue). */
async function open(running = false) {
  const daemon = {
    ...answers(running ? { operations: [runningOperation()] } : {}),
    'GET /api/threads/t1/queue': [],
    'POST /api/threads/t1/queue': () => new Response('refused', { status: 500 }),
  }
  const a = (app = await startApp('/projects/p1/threads/t1', daemon))
  await until(() => a.sources.length > 0 && a.container.querySelector('textarea') !== null)
  act(() => a.pushFrame('caught-up', { seq: 0 }))
  act(() => a.pushFrame('commands', { thread_id: 't1', commands: COMMANDS }))
  await until(() => a.button(running ? 'Stop' : 'fake-large') !== undefined)
  const box = a.container.querySelector('textarea')
  if (box === null) throw new Error('no message box')
  return { a, box }
}

const press = (box: HTMLTextAreaElement, key: string) =>
  act(() => {
    box.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }))
  })
const menu = () => document.querySelector('[role="listbox"]')
const names = () =>
  [...document.querySelectorAll('[role="option"]')].map((o) => o.getAttribute('data-name'))
const sentOrQueued = (a: TestApp) =>
  a.calls.filter((c) => c === 'POST /api/threads/t1/turns' || c === 'POST /api/threads/t1/queue')

it('opens on / and filters as the person types', async () => {
  const { box } = await open()
  typeInto(box, '/')
  expect(names()).toEqual(['compact', 'superpowers:brainstorming'])
  typeInto(box, '/br')
  expect(names()).toEqual(['superpowers:brainstorming'])
  typeInto(box, 'hi /br')
  expect(menu()).toBeNull()
})

it('Enter picks without sending, writes /name and shows the hint', async () => {
  const { a, box } = await open()
  typeInto(box, '/br')
  press(box, 'Enter')
  expect(box.value).toBe('/superpowers:brainstorming ')
  expect(a.text()).toContain('[topic]')
  expect(menu()).toBeNull()
  expect(sentOrQueued(a)).toEqual([])
})

it('the hint goes once the person types', async () => {
  const { a, box } = await open()
  typeInto(box, '/br')
  press(box, 'Enter')
  typeInto(box, '/superpowers:brainstorming x')
  expect(a.text()).not.toContain('[topic]')
})

it('arrows move, Tab picks, Escape closes and keeps the text', async () => {
  const { box } = await open()
  typeInto(box, '/')
  press(box, 'ArrowDown')
  press(box, 'Tab')
  expect(box.value).toBe('/superpowers:brainstorming ')
  typeInto(box, '/co')
  press(box, 'Escape')
  expect(menu()).toBeNull()
  expect(box.value).toBe('/co')
  typeInto(box, '/com')
  expect(names()).toEqual(['compact'])
})

it('a click picks', async () => {
  const { box } = await open()
  typeInto(box, '/')
  const option = document.querySelector('[data-name="compact"]')
  act(() => option?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true })))
  expect(box.value).toBe('/compact ')
})

it('with no match the menu closes and Enter sends', async () => {
  const { a, box } = await open()
  typeInto(box, '/zzz')
  expect(menu()).toBeNull()
  press(box, 'Enter')
  await until(() => a.calls.includes('POST /api/threads/t1/turns'))
})

it('an empty list opens nothing', async () => {
  const { a, box } = await open()
  act(() => a.pushFrame('commands', { thread_id: 't1', commands: [] }))
  typeInto(box, '/')
  expect(menu()).toBeNull()
})

it('Enter picks while a turn runs; it does not queue', async () => {
  const { a, box } = await open(true)
  typeInto(box, '/co')
  press(box, 'Enter')
  expect(box.value).toBe('/compact ')
  expect(sentOrQueued(a)).toEqual([])
})

it('a newline closes the menu', async () => {
  const { box } = await open()
  typeInto(box, '/co\nmore')
  expect(menu()).toBeNull()
})

it('a shorter new list clamps the highlight', async () => {
  const { a, box } = await open()
  typeInto(box, '/')
  press(box, 'ArrowDown')
  act(() => a.pushFrame('commands', { thread_id: 't1', commands: [COMMANDS[0]] }))
  expect(names()).toEqual(['compact'])
  press(box, 'Enter')
  expect(box.value).toBe('/compact ')
})

it('the highlighted entry is scrolled into view', async () => {
  const scroll = vi.spyOn(Element.prototype, 'scrollIntoView').mockImplementation(() => {})
  const { box } = await open()
  typeInto(box, '/')
  press(box, 'ArrowDown')
  expect(scroll).toHaveBeenCalled()
})
```

`open()` waits for the `fake-large` button (the model menu, shown once the session answered) so Enter-sends is enabled. If a route a test needs is missing from the daemon, the request answers 404 and `a.calls` shows it: add it to `daemon` in `open`.

- [ ] **Step 5: Run → FAIL.** `npx vitest run src/app/conversation/slash-menu.test.tsx`

- [ ] **Step 6: Implement `slash-menu.tsx`:**

```tsx
// One job: drawing the `/` menu (spec §21.4): the entries, the highlighted
// one, and its description beside the list. Keys are the composer's.

import { useEffect, useRef } from 'react'
import type { SlashCommand } from '@/stream/frames'

export function SlashMenu({
  items,
  highlighted,
  onPick,
  onHighlight,
}: {
  items: readonly SlashCommand[]
  highlighted: number
  onPick: (command: SlashCommand) => void
  onHighlight: (index: number) => void
}) {
  const active = useRef<HTMLLIElement | null>(null)
  useEffect(() => active.current?.scrollIntoView({ block: 'nearest' }), [highlighted])
  const current = items[highlighted]
  return (
    <div className="absolute bottom-full left-0 mb-2 flex items-start gap-2">
      <ul
        role="listbox"
        aria-label="Commands"
        dir="ltr"
        className="max-h-80 w-72 overflow-y-auto rounded-lg border border-border bg-popover p-1 text-sm shadow-md"
      >
        {items.map((command, i) => (
          <li
            key={command.name}
            ref={i === highlighted ? active : undefined}
            role="option"
            aria-selected={i === highlighted}
            data-name={command.name}
            onMouseEnter={() => onHighlight(i)}
            onMouseDown={(e) => {
              e.preventDefault() // keep the textarea's focus
              onPick(command)
            }}
            className={`cursor-pointer truncate rounded px-2 py-1 ${i === highlighted ? 'bg-accent text-accent-foreground' : 'text-foreground'}`}
          >
            {command.name}
          </li>
        ))}
      </ul>
      {current !== undefined && current.description !== '' && (
        <p dir="auto" className="max-w-xs rounded-lg bg-popover p-2 text-xs text-muted-foreground shadow-md">
          {current.description}
        </p>
      )}
    </div>
  )
}
```

Use the colour tokens the composer bar's menus use if `bg-popover` / `bg-accent` do not exist (`grep -n "bg-" web/src/app/conversation/cli-picker.tsx`).

- [ ] **Step 7: Wire the composer.** In `composer.tsx`:
  - New optional prop `commands?: readonly SlashCommand[]` (default `NONE`, a module constant `[]`) with a doc line (the harness's `/` list, §21).
  - State: `const [highlighted, setHighlighted] = useState(0)`, `const [dismissed, setDismissed] = useState<string | null>(null)` (the text Escape closed the menu on), `const [hint, setHint] = useState<{ text: string; hint: string } | null>(null)`.
  - Derived: `const matches = slashMatches(commands, prompt)`, `const menu = matches !== null && matches.length > 0 && dismissed !== prompt ? matches : null`, `const at = Math.min(highlighted, (menu?.length ?? 1) - 1)` (the clamp).
  - In `onChange`: `setHighlighted(0)`, `setDismissed(null)` (any change reopens if it still qualifies), `setHint(null)` unless the new value equals `hint.text` (it does not when the person typed).
  - `pick(command)`: `const text = '/' + command.name + ' '`; `setPrompt(text)`; `pending.current = null`; `setHint(command.hint === null ? null : { text, hint: command.hint })`; `setHighlighted(0)`.
  - `onKeyDown`, before the existing Enter branch, and only when `menu !== null && !e.nativeEvent.isComposing`: `ArrowDown` → `setHighlighted((at + 1) % menu.length)`; `ArrowUp` → `setHighlighted((at - 1 + menu.length) % menu.length)`; `Enter` without Shift or `Tab` → `pick(menu[at])`; `Escape` → `setDismissed(prompt)`. Each of these calls `e.preventDefault()` and returns. Shift+Enter is left alone (it inserts a newline, which closes the menu).
  - Render: wrap the `<textarea>` in `<div className="relative flex-1">`; move `flex-1` there and give the textarea `w-full`. Inside the wrapper, when `menu !== null`, `<SlashMenu items={menu} highlighted={at} onPick={pick} onHighlight={setHighlighted} />`. When `hint !== null && hint.text === prompt`, an overlay with the textarea's padding and font that does not take pointer events:

```tsx
<div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden whitespace-pre-wrap px-2 py-1 text-sm">
  <span className="invisible">{prompt}</span>
  <span className="text-faint-foreground">{hint.hint}</span>
</div>
```

  - `composer.tsx` is 347 lines; keep the addition small (the filter and the menu live in their own files). State its one job in the commit message if it grows past its current size by much: "the box where a message is written and sent".
  - Pass `commands` from `conversation.tsx` (`useConversation`'s new `commands`), and leave `draft.tsx` as it is (optional prop, default `[]`)

- [ ] **Step 8: Run the tests.** `npx vitest run src/app/conversation` → PASS (all, including the existing composer tests). `npm run typecheck` and `npm run lint` (oxlint) → no errors.

- [ ] **Step 9: Commit.**

```bash
git add web/src
git commit -m "§21 web: the composer's / menu

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Status, the full gate, the browser run (controller)

**Files:**
- Modify: `docs/status.md` (a §21 paragraph beside §20's), `docs/superpowers/specs/README.md` (§21 row: built), the spec's Status line, and the spec's OPEN block (closed with what the run found).

- [ ] **Step 1: The full gate, once,** from the root with the target-dir prefix: steps 1–6 of `CLAUDE.md`'s gate, then `cd web && npx vitest run`.
- [ ] **Step 2: The browser run** on a copy of the real database (scratchpad), against the real adapter: `/` opens the menu; `/br` finds brainstorming; picking writes it with its hint; a picked `/compact` sent runs; the menu still opens after the page is hidden and shown; the OPEN block: `/model <name>` sent — does the model picker follow; `/context` queued during a turn and sent with Send now — is it run as a command. Read the daemon log for errors.
- [ ] **Step 3:** Amend the spec's OPEN block with the answers (close it, or amend §20.4 as the block says), update status and the README row, commit.
- [ ] **Step 4:** One whole-branch review before the PR.
