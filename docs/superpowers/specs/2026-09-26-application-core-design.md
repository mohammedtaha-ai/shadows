# Section 14 — One Application Core (Milestone 2.5)

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there.

- **Date:** 2026-09-26
- **Status:** Designed with Mohammed on 2026-09-26, section by section. Each
  section was checked against the code, and a second model reviewed §14.5–§14.8.
  Not yet built.
- **Builds on:** Milestone 2 (§13), on `main` at `ddcaed5`.

Milestone 2.5 changes **where the code lives, not what it does.** Every
operation Shadows performs gets one home: one method on one service inside an
`AppCore`. The HTTP API, the MCP server and the daemon's startup call that
method and hold no logic of their own. No feature is added. No behaviour a client
can observe changes.

It comes before Milestone 3 (the code index, `docs/vision.md` §2.4). The index
is the first thing Shadows will serve to agents about *their* projects, and
Shadows' own code must first be organized the way the index assumes a project
is: one discoverable place for each operation.

## 14.1 Scope

In:

1. **`AppCore`** and eight services, built once at startup and passed down
   (§14.3).
2. **Thin adapters.** `protocol/`, `mcp/` and `cli/` translate a request into
   one service call and translate its result back (§14.4).
3. **Two guards against bypassing the core:** the compiler, and an architecture
   test (§14.5).
4. **A contract per service**, in the shape of Mohammed's source-contract
   template, kept current by a test (§14.6).

Out:

- Any new feature, route, MCP tool, or change to a route's JSON.
- Moving `planner/`, `workflow/`, `agent/`, `runtime/` or `process/` into
  `core/` (§14.5, OPEN).
- The web client. `api/openapi.json` does not change (§14.8).
- The deferred items in `docs/status.md`: effort at once, one lock per session.

## 14.2 Why: what the code does today

This was measured on `main` at `ddcaed5`, reading implementations rather than
comments:

- **The entry points call storage directly.** The route files in `protocol/`
  and the MCP tools in `mcp/tools.rs` call `Storage` methods themselves, and so
  do `mcp/auth.rs` and `cli/mod.rs`. No layer holds the application's
  operations.
- **One rule is checked in several places.**
  - `thread_is_busy` is called before a session is touched in
    `protocol/conversation.rs` (`start_turn`) and in `protocol/harness.rs`
    (`change_model`).
  - `turn_context` is read from `protocol/conversation.rs`, `harness.rs`,
    `thread.rs`, `sse.rs` and `planner/`.
- **One operation spans layers.** Sending a message runs through
  `protocol/conversation.rs`, `planner/spawn.rs`, `planner/turn.rs` and
  `storage/sqlite/turn.rs`. An agent has no single place to learn it from.
- **The same capability exists twice.** Reading a plan is `get_plan` in
  `protocol/workflow.rs` and `workflow_get` / `task_get` in `mcp/tools.rs`.
  Each calls `storage.get_plan` itself.
- **Two application states.** `protocol::AppState` and `mcp::McpState` each
  gather their own `Arc<Storage>`, `Arc<LiveHandles>` and UI sender.

An agent that must find "where sending a turn lives" has to read four modules,
and one that must add a plan capability has two places to put it. This is the
kind of structure that makes an agent duplicate code or assume a wrong flow
(`docs/vision.md` §1).

## 14.3 `AppCore` and its services

`AppCore` is the application. It is built once by the daemon's startup
(`AppCore::start(config)`) and shared as `Arc<AppCore>`. It is **not** a
global: no `static`, no `OnceLock`, no service locator. Every adapter receives
it explicitly.

Its fields are private. A caller reaches a service only through an accessor:
`core.plans()`, `core.turns()`, and so on. Each service is a struct with
methods and a single responsibility, and holds only what it needs.

| Service | Its one job | Methods (what they replace) |
|---|---|---|
| `Projects` | the projects and the folders a person picks them from | list, create, set modes (`protocol/project.rs`); list and create folders (`protocol/fs.rs`) |
| `Threads` | the planning threads and what they recorded | list, create, update, fork (`protocol/project.rs`, `thread.rs`); entries, operations (`conversation.rs`) |
| `Turns` | starting and stopping a Planner turn | send, stop (`protocol/conversation.rs`, `planner/spawn.rs`, `planner/turn.rs`) |
| `Harness` | the harnesses and each thread's open session | list harnesses, open session, change model, context breakdown (`protocol/harness.rs`, `planner/sessions.rs`, `settings.rs`, `context.rs`) |
| `Plans` | plan versions under §13's rules | list, get, task, approve (`protocol/workflow.rs`); prepare draft, start draft, edit, show (`mcp/tools.rs`) |
| `Grants` | MCP grants from issue to revocation | issue, list, revoke (`protocol/grants.rs`); authorize a bearer (`mcp/auth.rs`); revoke thread grants at startup (`cli/mod.rs`) |
| `Instructions` | a project's Planner instructions | current, save (`protocol/instructions.rs`, `planner/setup.rs`) |
| `Events` | what clients watch live | subscribe after a sequence, the live bus, the UI signal (`protocol/sse.rs`, `protocol/ui_signal.rs`) |

- The method names above describe the capability. The implementation plan fixes
  exact signatures, keeping the argument and result types that exist today.
- **One capability, one method.** An HTTP route and an MCP tool that do the same
  thing call the same method. The route `get_plan` and the tools `workflow_get`
  and `task_get` all end in `Plans`.
- The service is where a checked precondition lives (§14.7). Its callers never
  repeat it.
- **Who emits, who delivers.** A service emits what it causes, as the code does
  today:
  - `Turns`, through `planner/turn.rs`, sends a turn's live harness events on
    the bus;
  - `Plans` sends `plan_show`'s UI signal.

  `Events` owns only subscription and delivery. It emits nothing of its own, and
  no other service reads the bus.
- The services use the domain modules as they are today: `workflow/` for plan
  rules, `planner/` for the turn lifecycle and sessions, `agent/` for ACP,
  `runtime/` for the runtime instance, and `storage/` for persistence. They do
  not reimplement them.
- A service that crosses 300 lines follows `CLAUDE.md`'s file rule. It splits by
  responsibility inside `core/<service>/`, never into a second service with the
  same job.

## 14.4 Adapters

- **`protocol/` (HTTP API and SSE).** A route reads its path, query and body,
  makes one service call, and turns the result or the failure into a response.
  `protocol::AppState` becomes:

  ```rust
  pub struct AppState {
      pub core: Arc<AppCore>,
      pub allowed_origins: Vec<String>,   // transport: the Origin guard (§1)
      pub mcp_url: String,                // transport: what a grant's command names
      pub shutdown: watch::Receiver<bool>,// transport: ending live streams
  }
  ```

  Its `runtime`, `storage`, `handles`, `sessions`, `bus` and `ui` fields go into
  the core.
- **`mcp/` (MCP server).** A tool reads its arguments, makes one service call,
  and turns the result into a tool result (`mcp/refusal.rs` stays the owner of
  that shape). `McpState` becomes `Arc<AppCore>`. The bearer check in
  `mcp/auth.rs` calls `core.grants()`.
- **`cli/` (daemon startup).** It resolves configuration, calls
  `AppCore::start`, binds the listener, and serves both adapters.
  `AppCore::start` absorbs what `cli/mod.rs` does today before binding:
  - opening storage;
  - starting the runtime (recovery included);
  - revoking thread grants;
  - reading the harness versions;
  - building the sessions.
- **Shutdown is split the same way.** `AppCore::shut_down()` stops every
  running turn and closes every adapter. This is what `planner::shut_down` does
  today, called from `cli/mod.rs`. `cli/` keeps what belongs to the process and
  the transport:
  - catching the signal;
  - the second Ctrl+C;
  - raising `stopping` so live streams end;
  - axum's graceful shutdown.

  The order stays as it is today.
- An adapter may keep what belongs to its transport: parsing, status codes,
  `Failure` mapping, OpenAPI annotations, rmcp's schemas. It may not keep a rule.

## 14.5 Guards: nothing reaches past the core

Two layers enforce the boundary.

**1. The compiler.** The adapters' states hold `Arc<AppCore>` and nothing it
contains. `AppCore`'s fields and every service's fields are private, so a route
that writes `s.storage` or `s.sessions` does not build.

**2. `tests/architecture.rs`.** The compiler cannot see everything. A later
change could add `pub fn storage(&self)` to `AppCore`, or `use crate::storage`
in a route file. The test uses `syn`, as `tests/codemap` already does, and
fails when:

1. a file under `src/protocol/`, `src/mcp/` or `src/cli/` names
   `crate::storage`, `Storage`, `Sessions`, `LiveHandles` or `Runtime`;
2. `AppCore` or a service has a public field, or a public method whose return
   type names `Storage`, `Sessions`, `LiveHandles` or `Runtime`.

Its failure message names the rule and the service to use instead.

The modules allowed to use `storage/` are `core/`, and the domain modules
`core/` drives: `planner/` and `runtime/`.

**A shrinking allowlist.** Task 0 of the plan writes the test with an allowlist
of today's violations, file by file. From then on:

- a new violation fails at once;
- each task that moves a service removes its files from the list;
- when Milestone 2.5 ends the list is empty, and the test refuses any entry.

> **OPEN — the compiler as the only guard.** `storage/` stays `pub` because
> 22 integration test files open `Storage` directly, and moving the domain
> modules under `core/` with `pub(super)` storage would rewrite them. **Trigger:**
> the first milestone that rewrites those tests for another reason, or the first
> bypass that the architecture test misses. **Why it does not block:** the two
> layers above catch every bypass known today.

## 14.6 Contracts

Every service has one contract, `docs/codebase/contracts/<service>.yaml`, which
makes eight files. Each follows Mohammed's source-contract template, kept in
the repository as `docs/codebase/contracts/TEMPLATE.yaml`:

- **header comment:** what the service alone owns, and the trap a reader would
  otherwise fall into;
- **`name`, `version`, `status`, `source`**;
- **`functions`:** the service's public methods and their signatures, grouped
  as reads, writes and checks, or however the service reads best;
- **`obligations`:** each rule with `tested_by`, which names a test, or says
  `none` or `unknown`;
- **`not_the_caller's`:** what a caller must not do itself, with the reason;
- **`gaps`, `open_questions`, `tests`**, as the template defines them.

The template's rules hold:
- trace the implementation, never restate comments;
- a `gap` names a symbol, never a line;
- another module's types are referenced, never copied.

**One contract per service, not per file.** A hundred per-file contracts would
cost more to keep than they return. `mx` failed for that reason
(`docs/vision.md` §1.4). The service is the one entry an agent uses, so that is
where its contract belongs.

**`tests/contracts.rs` keeps them true.** It fails when:

1. a contract is not valid YAML;
2. its `source` does not exist;
3. a symbol in `functions` or in a gap's `at` does not exist in that source;
4. a name in `tested_by` or `tests` is not a test function in `tests/` or in a
   `#[cfg(test)]` module under `src/`;
5. a public method of the service has no entry in `functions`;
6. an obligation says `tested_by: none` or `unknown` without a `note` that says
   what a test would need.

Every rule listed in §14.7 must name a real test. Any other obligation may say
`none`, with its note, and the gap stays visible in the contract instead of
being hidden.

What it cannot check is whether a rule's prose is true. That stays with the
reviewer of the change.

The contracts are also the first real use of what Milestone 3 serves: a map an
agent reads instead of the code.

## 14.7 Rules that move, and must not be lost

Moving code into a service carries each rule with its reason. The rules found
so far, each of which becomes an obligation in its service's contract:

- **Busy, twice, for two reasons.** `start_turn` and `change_model` check
  `thread_is_busy` before touching the session. The reason is so that a
  model or setting is never changed under a running turn. `start_turn`'s write
  transaction checks `has_open_operation` again, inside `write_txn`
  (`BEGIN IMMEDIATE`), and that check is the atomic guarantee.

  Both stay. The early check moves into `Turns::send` and
  `Harness::change_model`, and no caller performs it.
- **Session before lease.** A turn opens the session, then leases its events
  before prompting (`protocol/conversation.rs`). A second start or a
  `/context` read cannot use the session until the turn gives it back.
- **Harness availability.** `policy::is_available` is checked before any
  session work, in `start_turn` and `change_model`.
- **Grant scope.** A tool call's grant decides which plans it sees (§13.7). The
  scope check (`plan_in_scope`, `own_thread`, `writer_of`) moves into `Plans`
  unchanged.

The implementer of each task adds any further rule it finds to that service's
contract. Dropping a rule is a defect, even when every test passes.

## 14.8 Order of work and what proves nothing broke

| # | Task | Why here |
|---|---|---|
| 0 | `tests/architecture.rs` with today's allowlist; `tests/contracts.rs` and `TEMPLATE.yaml` | The guard exists before anything moves. |
| 1 | `AppCore::start`, the eight service shells, `AppState` and `McpState` holding `Arc<AppCore>` | The frame the services go into. |
| 2 | `Plans` | The largest duplication, HTTP and MCP, so the largest gain first. |
| 3 | `Grants` | Split between the HTTP routes, the bearer check and startup. |
| 4 | `Turns` | The most delicate logic: the busy checks, the lease, Stop. |
| 5 | `Harness` | Shares the busy rule with `Turns`. |
| 6 | `Projects`, `Threads`, `Instructions` | Mostly reads and writes. |
| 7 | `Events`; the allowlist becomes empty | SSE is the most timing-sensitive path, so it goes last. |
| 8 | The eight contracts completed; `CLAUDE.md`, `docs/codebase/README.md` and this index updated | Documents what was built. |

Each task:

- ends with every existing test passing, unchanged in what it asserts: 304
  Rust, 135 web as of `ddcaed5`;
- keeps `api/openapi.json` byte-identical, which `tests/openapi.rs` already
  checks;
- regenerates the code map (`UPDATE_CODEMAP=1 cargo test --test codemap`);
- writes that service's contract.

**Risk: `Events` (task 7).** A subscription reads the durable journal after
the client's `after`, sends `caught-up`, then goes live. It re-reads the
journal each time storage's committed-sequence signal moves (§2.4, §6.18).
Moving it must keep three things:
- a frame is never lost or repeated across that switch;
- frames stay in `durable_seq` order;
- a reconnect with `after` resumes exactly where the client stopped.

Its review compares the old and new `subscribe` path by path. `tests/resync.rs`
and the SSE tests must pass unchanged.

**Risk: `Turns` (task 4).** It moves the code around Stop, the session lease
and the end of a turn. That is where Milestone 2 found its races (F1, F2), and
the tests may not cover every timing. Its review reads the old path and the new
path side by side, and the acceptance run stops a live turn.

## 14.9 Acceptance

Milestone 2.5 is done when:

1. The Rust and web gates pass on Windows and on CI's Linux job.
2. `tests/architecture.rs` passes with an empty allowlist.
3. `tests/contracts.rs` passes, with eight contracts.
4. Mohammed runs the debug build in the browser:
   1. sends a message and stops it;
   2. restarts the daemon, and the conversation is still there;
   3. edits and approves a plan;
   4. connects an external Claude Code with **Connect**, reads a plan, then
      **Revoke**s it.

   Each works as it did on `ddcaed5`.

The evidence goes in `docs/evidence/milestone2_5/WINDOWS_RUN.md`.

## 14.10 Changes to other documents

- **`CLAUDE.md`** gains two rules:
  - a new operation is a new method on one service, with a line in its contract;
  - a route or tool translates and holds no rule.

  Its ownership table gains a `core/` row: the application's operations.
- **`docs/codebase/README.md`** gains `core/` and each service. The rows of
  `protocol/` and `mcp/` change to "translating HTTP" and "translating MCP".
- **This index** gains §14.
