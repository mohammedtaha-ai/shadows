# Section 14 — A Workspace Around One Application Core (Milestone 2.5)

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there.

- **Date:** 2026-09-26, rewritten 2026-09-27.
- **Status:** Designed with Mohammed on 2026-09-26/27.
  - The first version, one crate with a facade and guard tests, was replaced
    after he asked for "clean work" and for how large Rust projects actually do
    this (§14.2).
  - Not yet built.
- **Builds on:** Milestone 2 (§13), on `main` at `ddcaed5`.

Milestone 2.5 changes **where the code lives, not what it does.** Shadows
becomes a Cargo workspace:
- one `shadows-core` crate holds the whole application behind one object,
  `AppCore`;
- small crates around it translate HTTP, MCP and the command line;
- two infrastructure crates run the harness and processes.

The compiler enforces the boundaries. No feature is added, and nothing a client
observes changes.

It comes before Milestone 3, the code index (`docs/vision.md` §2.4). The index
assumes a project where every operation has one discoverable place, and Shadows'
own code must be that project first.

## 14.1 Scope

In:

1. **A workspace** with a flat `crates/` directory (§14.3).
2. **`AppCore`** and eight services inside `shadows-core`, one folder per
   service (§14.4).
3. **Adapters that only translate:** `shadows-http`, `shadows-mcp`, and the
   `shadows` binary (§14.5).
4. **Boundaries the compiler enforces**, and the one check the compiler cannot
   make (§14.6).
5. **A contract per service**, in Mohammed's source-contract template, that a
   test keeps current (§14.7).

Out:

- Any new feature, route, MCP tool, or change to a route's JSON.
- A crate per service. Services stay folders inside `shadows-core` until their
  boundaries are proven (§14.4, OPEN).
- The web client. `api/openapi.json` does not change.
- The deferred items in `docs/status.md`: effort at once, and one lock per
  session.

## 14.2 Why, and what large Rust projects do

**What the code does today** (measured on `main` at `ddcaed5`):

- **The entry points call storage directly.** The route files in `protocol/`,
  the MCP tools in `mcp/tools.rs`, `mcp/auth.rs` and `cli/mod.rs` all call
  `Storage` methods themselves. No layer holds the application's operations.
- **One rule lives in several places.**
  - `thread_is_busy` is checked in `protocol/conversation.rs` and in
    `protocol/harness.rs`.
  - `turn_context` is read from five places.
  - Reading a plan exists twice: `get_plan` in `protocol/workflow.rs`, and
    `workflow_get` / `task_get` in `mcp/tools.rs`.
- **One capability is spread across layers.** Plans live in seven files in four
  modules: `workflow/`, `storage/sqlite/{workflow,workflow_draft,workflow_read,task,plan_view}.rs`
  and `mcp/tools.rs`. An agent has no single place to learn them from, and two
  places to add to them.
- **The boundaries are conventions.** `storage` is `pub` to the whole crate, so
  nothing but review stops a route from calling it.

**What large Rust projects do**, researched on 2026-09-27:

- **rust-analyzer** (about 40 crates) makes crates its API boundaries. It writes
  down "Architecture Invariants", and its tests go through the public API.
  Source: its `docs/book/src/contributing/architecture.md`.
- **Zed** (about 230 crates) keeps one workspace with a flat `crates/`
  directory and consistent names (`agent`, `agent_ui`, `agent_settings`).
- **Codex CLI** (100+ crates), the project closest to Shadows' domain, has one
  large `core` crate with many internal modules. Around it sit small crates:
  `protocol`, `cli`, `codex-mcp`, `config`, `state`.
- **matklad**, the author of rust-analyzer, recommends a flat `crates/`
  workspace with a virtual root manifest for anything from 10k to 1M lines:
  "tree structure tends to deteriorate over time, while flat structure doesn't
  need maintenance". Folder names equal crate names. Source: "Large Rust
  Workspaces", 2021.

The lesson: **a crate is the one boundary the compiler enforces.** Folders
inside a crate, `pub(crate)`, guard tests and allowlists are conventions that
decay. And a project starts with a large core crate, splitting it only along
boundaries that have proven themselves, as Codex does, rather than guessing
them up front.

**Why services are not crates yet.** The domain types form real cycles today:
- `thread` ↔ `workflow` and `thread` ↔ `events` import each other;
- `Threads` closes a `Harness` session, and `Harness` reads a thread's context;
- `Plans` creates a thread.

A crate per service would make the compiler refuse those cycles and force a
redesign of the types inside a milestone whose rule is "behaviour does not
change". So they stay folders in `shadows-core`, where the cycles are legal,
and the crate boundary goes where it holds today: between the application and
everything that talks to it.

## 14.3 The workspace

```text
Cargo.toml                 virtual manifest: [workspace], shared deps and lints, no package
crates/
  shadows/                 the binary: main, the CLI, startup (builds AppCore once)
  shadows-http/            the HTTP API and SSE (axum, utoipa)
  shadows-mcp/             the MCP server (rmcp)
  shadows-core/            the application: AppCore, the services, storage, runtime
  shadows-agent/           the ACP client for a harness adapter
  shadows-process/         OS processes with whole-tree containment
  fake-acp/                the fake ACP adapter tests drive (binary)
api/  web/  docs/          unchanged
```

- **Folder name equals package name.** No crate is named `core`, which would
  shadow Rust's own `core`.
- **Dependencies point one way:**

  ```text
  shadows ─┬─► shadows-http ─┐
           ├─► shadows-mcp  ─┼─► shadows-core ─► shadows-agent ─► shadows-process
           └────────────────►┘
  ```

  Cargo refuses a cycle, so `shadows-core` can never depend on an adapter.
- **Shared dependency versions and lints** are declared once, in the root
  `[workspace.dependencies]` and `[workspace.lints]`.
- `shadows-process` keeps `tree_probe` as its own test binary. Its containment
  tests name it directly; two daemon tests in `crates/shadows` (`serve_smoke`,
  `planner_mcp`) also run it as a stand-in executable, and locate it as below.
- `fake-acp` is a separate binary crate. Tests in other crates locate it, and
  `tree_probe`, through **`escargot`** (0.5, maintained by `crate-ci`). Cargo's
  `CARGO_BIN_EXE_*` only names binaries of the test's own package.
- **Where each current module goes** (§14.8 gives the order):

| Today | Goes to |
|---|---|
| `process/`, `bin/tree_probe.rs` | `shadows-process` |
| `agent/` | `shadows-agent` |
| `protocol/` | `shadows-http` |
| `mcp/` (except `grant.rs`) | `shadows-mcp` |
| `cli/`, `main.rs`, `config.rs`, `tracing.rs` | `shadows` |
| `bin/fake_acp.rs` | `fake-acp` |
| everything else: `project/`, `thread/`, `workflow/`, `planner/`, `runtime/`, `operation/`, `events/`, `command/`, `storage/`, `mcp/grant.rs`, `error.rs`, `id.rs` | `shadows-core`, arranged by service (§14.4) |
| `tests/*.rs`, `tests/fixtures/` | By what the test drives:<br>• only the core → `shadows-core/tests`, with its fixtures;<br>• HTTP, MCP or `serve` → `crates/shadows/tests`, the one crate that sees every adapter.<br>What both share, such as where the fake adapter is and a command context, is in `shadows_core::testing`. |

There is no shared test-kit crate. The core's tests would depend on a crate that
depends on the core, and Cargo would build two copies of its types.

## 14.4 `shadows-core`: `AppCore` and its services

`AppCore` is the application. The binary builds it once, with
`AppCore::start(config, mcp_url)`, and shares it as `Arc<AppCore>`. It is **not**
a global: no `static`, no `OnceLock`, no service locator. Rust's idiom is a
composition root written by hand and checked by the compiler, so no
dependency-injection library is used. `shaku` and the like add macros and
indirection and buy nothing here.

A caller reaches a service through an accessor: `core.plans()`, `core.turns()`,
and so on. Each service is a struct with methods and one job.

```text
crates/shadows-core/src/
  lib.rs            the public surface: AppCore, CoreError, ErrorCode, and the domain types adapters serialize
  app.rs            AppCore: start, the accessors, shut_down
  error.rs          CoreError, ErrorCode
  command.rs        command identity and idempotency
  db/               the SQLite pool, write transactions, migrations, the durable journal, command replay
  runtime/          the runtime instance and recovery
  projects/  threads/  turns/  harness/  plans/  grants/  instructions/  events/
    mod.rs          the service: its public methods
    model.rs        its types (no sqlx; the domain stays pure)
    rules.rs        its checks, where it has any
    store.rs        its SQLite queries
    contract.yaml   its contract (§14.7)
```

| Service | Its one job | Takes over |
|---|---|---|
| `Projects` | the projects and the folders a person picks them from | `project/`, `storage/sqlite/project.rs`, `protocol/project.rs` and `fs.rs` logic |
| `Threads` | the planning threads and what they recorded | `thread/`, `storage/sqlite/{thread,entry,fork}.rs`, the logic of `protocol/thread.rs` and the entry and operation lists |
| `Turns` | starting and stopping a Planner turn | `planner/{spawn,turn,entries,handles,shutdown}.rs`, `storage/sqlite/{turn,operation,operation_read,transition}.rs`, `operation/`, the logic of `protocol/conversation.rs` |
| `Harness` | the harnesses and each thread's open session | `planner/{sessions,settings,setup,offers,context}.rs`, `storage/sqlite/harness.rs`, the logic of `protocol/harness.rs` |
| `Plans` | plan versions under §13's rules | `workflow/`, `storage/sqlite/{workflow,workflow_draft,workflow_read,task,plan_view}.rs`, the logic of `protocol/workflow.rs` and `mcp/tools.rs` |
| `Grants` | MCP grants, from issue to revocation | `mcp/grant.rs`, `storage/sqlite/grant.rs`, the logic of `protocol/grants.rs` and `mcp/auth.rs` |
| `Instructions` | a project's Planner instructions | `storage/sqlite/instructions.rs`, the logic of `protocol/instructions.rs` |
| `Events` | what clients watch live | `events/`, `storage/sqlite/{events,events_read}.rs`, the loop of `protocol/sse.rs`, `UiSignal` |

- **One capability, one method.** An HTTP route and an MCP tool that do the same
  thing call the same method. `get_plan`, `workflow_get` and `task_get` all
  end in `Plans`.
- **A service builds its own `CommandContext`**, with the same command kinds and
  fingerprint parameters as today, so every replay still matches.
- **Who emits, who delivers.** A service emits what it causes: `Turns` sends a
  turn's live events on the bus, and `Plans` sends `plan_show`'s UI signal.
  `Events` owns subscription and delivery, and emits nothing of its own.
- **Shutdown.** `AppCore::shut_down()` stops every running turn and closes every
  adapter, which is `planner::shut_down` today. The binary keeps what belongs to
  the process and the transport:
  - catching the signal;
  - the second Ctrl+C;
  - raising `stopping` so live streams end;
  - axum's graceful shutdown.

  The order stays as it is today.
- **Startup binds first.** Today `serve` recovers and only then binds. The
  sessions need `mcp_url`, which names the address actually bound, so the binary
  binds first and then calls `AppCore::start(config, mcp_url)`. That is also
  safer. Today a second daemon started on the same database and port runs
  recovery first, marking the first daemon's live operations `Interrupted`, and
  only then fails to bind. With the bind first, it fails before it touches
  anything.
- **`CoreError`** is the one failure type services return, in words no adapter
  owns. Each adapter maps it to its own shape: `Failure` for HTTP, `Refusal` for
  MCP. Codes and messages stay exactly as today.
- **Inside the crate** a service may call another service's public methods.
  Cycles between them are legal here, and that is why the services are not
  crates yet. A service never reaches into another's `store.rs` or `model.rs`
  privates.

> **OPEN — a crate per service.** Services become their own crates once their
> types stop forming cycles (`thread` ↔ `workflow`, `thread` ↔ `events`, and
> `Threads` ↔ `Harness`). **Trigger:** the first milestone that adds a service
> with no cycle to the rest, which is likely Milestone 5's executors, or a
> `shadows-core` build time that slows work down. **Why it does not block:**
> the adapter boundary, which is the one that hurt, is enforced now.

## 14.5 Adapters

- **`shadows-http`.** A route reads its path, query and body, calls one service
  method, and turns the result or `CoreError` into a response. It keeps what
  belongs to HTTP:
  - status codes, `Failure` and `ErrorBody`;
  - the OpenAPI annotations and `api/openapi.json`;
  - the Origin guard, CORS and the trace layer;
  - SSE framing;
  - `detached`, so a client that disconnects does not cancel the work.

  Its state is `{ core: Arc<AppCore>, allowed_origins, shutdown }`.

  **`/mcp` is mounted by the HTTP router, at the same layer as today.** It goes
  inside the Origin guard, CORS and the trace layer, and outside the layer that
  turns rejections into `ErrorBody`. It arrives as a parameter,
  `shadows_http::router(state, mcp: axum::Router)`, so `shadows-http` never
  depends on `shadows-mcp`. The binary passes `shadows_mcp::service(core)`.
- **`shadows-mcp`.** A tool reads its arguments, calls one service method, and
  turns the result into a tool result. It keeps `rmcp`'s wiring, the tools'
  argument schemas and `Refusal`. The bearer check calls `core.grants()`.
- **`shadows`.** It resolves configuration, binds, calls `AppCore::start`, and
  serves both adapters. It keeps `args`, `config`, `tracing` and the signal
  handling.
- A request or response shape that is part of the HTTP contract stays in
  `shadows-http`. A shape a service computes moves to that service's `model.rs`,
  with the same name, fields and derives, so `api/openapi.json` does not change.
  `HarnessInfo` and `ContextBreakdown` are examples.

## 14.6 Boundaries

**The compiler enforces:**

1. **The adapters see only `shadows-core`'s public surface:** `AppCore`,
   `CoreError`, `ErrorCode` and the domain types they serialize. Storage,
   sessions, the live-turn registry and the runtime are private to the crate.
2. **`shadows-core` cannot import an adapter**, because Cargo refuses the cycle.
3. **`shadows-agent` and `shadows-process` know nothing of plans, threads or
   HTTP**, because they depend on nothing above them.

**Tests need more than the public surface.** The 22 storage tests open
`Storage` directly. `shadows-core` exposes it, and the fixtures they need, only
under its `test-support` feature:

```rust
#[cfg(feature = "test-support")]
pub mod testing;
```

A crate enables that feature only in its `[dev-dependencies]`. Resolver 2 does
not unify dev-dependency features into a normal build, so:

- **the gate runs `cargo check --workspace` without tests.** A non-test crate
  that reaches `shadows_core::testing` fails to build. This extends the CI check
  that already proves `test-support` never reaches the shipped binary
  (`cargo tree -e features,no-dev`).

**What the compiler cannot see:** inside `shadows-core`, one service reaching
into another's private module. There, `pub(crate)` is a convention. Two things
cover it:
- each service's `contract.yaml` names its public methods and its
  `not_the_caller's`;
- review reads it.

That is the price of keeping the services in one crate for now (§14.4, OPEN).

**Architecture Invariants** are written into `docs/codebase/README.md`, in
rust-analyzer's style, one line each:
- "`shadows-http` knows HTTP; nothing below it does."
- "`shadows-core` never imports `axum` or `rmcp`."
- "a service's `store.rs` is called only by that service."

## 14.7 Contracts

Every service has one contract, `crates/shadows-core/src/<service>/contract.yaml`,
next to its code. That makes eight contracts. Each follows Mohammed's
source-contract template, kept at `docs/codebase/contracts/TEMPLATE.yaml`:

- **header comment:** what the service alone owns, and the trap a reader would
  otherwise fall into;
- **`name`, `version`, `status`, `source`**;
- **`shapes`:** the types the service itself declares that a caller meets, with
  what a reader could misread about them; types owned elsewhere go under
  `enums` as a `source` reference only;
- **`functions`:** the service's public methods and their signatures, grouped
  as reads, writes and checks, or however the service reads best;
- **`obligations`**, each with `tested_by`;
- **`agreements`:** wherever one rule has two paths, the paths that must give
  the same answer. Shadows has these by design: busy is checked in both
  `Turns::send` and `Harness::change_model`, and one plan is read by
  `get_plan`, `workflow_get` and `task_get`;
- **`not_the_caller's`**;
- **`gaps`, `open_questions`, `tests`.**

All ten of the template's rules hold. The ones a reader breaks first:
- trace the implementation, never restate comments; a comment that disagrees
  with the code is a `gap`;
- a `gap` names a symbol, never a line; a suspicion about another service is an
  `open_question`, never a gap;
- `tested_by` is chosen by reading the test's **body**, not its name;
- another module's types are referenced, never copied;
- the sections never contradict each other.

**The contract is the entry point, and it moves with the code.** An agent reads
a service's `contract.yaml` before it changes that service. A change to a
service's behaviour, methods or tests updates its contract in the same commit,
as a code change regenerates the code map. This is what lets any agent work on
Shadows without a person or one model's memory in the loop.

**One contract per service, not per file.** A hundred per-file contracts would
cost more to keep than they return; `mx` failed for that reason
(`docs/vision.md` §1.4). The service is the one entry an agent uses.

**`crates/shadows-core/tests/contracts.rs` keeps them true.** It parses each file
with **`yaml-rust2`**, the maintained successor of the archived `serde_yaml`
line, and the sources with `syn`. It fails when:

1. a contract is not valid YAML;
2. its `source` does not exist;
3. a symbol in `functions`, or in a gap's `at`, is not in that source;
4. a name in `tested_by` or `tests` is not a test function in the workspace;
5. a public method of the service has no entry in `functions`;
6. an obligation or agreement says `tested_by: none` or `unknown` without a
   `note`;
7. a name in `shapes`, or in an agreement's `between`, is not in that source
   (a path in another service is written `harness::change_model` and looked
   up in that service's folder);
8. an agreement says `holds: false` and no `gap` names one of its symbols.

Every rule in §14.9 must name a real test. What the test cannot check is whether
a rule's prose is true, or whether a named test's body proves it; the reviewer
owns that, and reads every `tested_by` body it rules on.

## 14.8 Order of work

The tree builds and every test passes after each step.

| # | Step | What moves |
|---|---|---|
| 0 | The workspace | A virtual root manifest. The current crate moves whole into `crates/shadows`. The CI and the gate use `--workspace`. |
| 1 | `shadows-process` | `process/`, `tree_probe`, the containment tests. `escargot` is added, for the daemon tests that run `tree_probe`. |
| 2 | `shadows-agent` | `agent/`, the ACP tests |
| 3 | `shadows-core`, as it is | Every core module moves unchanged, still `pub`. `fake-acp` is created, and located through `escargot`. |
| 4 | `shadows-http`, `shadows-mcp` | The adapters leave the binary and depend on `shadows-core`, still through its `pub` internals. |
| 5 | `AppCore`, `CoreError`, `Plans` | The services begin. Plans' files move into `plans/`, its internals become private, and the routes and tools call `core.plans()`. |
| 6 | `Grants` | Plus the grant types. |
| 7 | `Turns` | **Risk:** Stop, the session lease, the end of a turn. |
| 8 | `Harness` | |
| 9 | `Projects`, `Threads`, `Instructions` | |
| 10 | `Events`; `storage` private | **Risk:** journal → caught-up → live. The last internal becomes private, and `cargo check --workspace` now proves the boundary. |
| 11 | Contracts and docs | Eight contracts pass. The code map spans the crates, and the invariants are written down. |

From step 5 onward, each step narrows `shadows-core`'s `pub` surface for the
service it moves. The compiler then refuses any adapter code still reaching past
it, which is a boundary that only tightens.

**Risk, `Turns` (step 7).** It moves the code around Stop, the session lease and
a turn's end. That is where Milestone 2 found its races (F1, F2), and the tests
may not cover every timing. Its review reads the old path and the new path side
by side, and the acceptance run stops a live turn.

**Risk, `Events` (step 10).** A subscription reads the durable journal after the
client's `after`, sends `caught-up`, then goes live. It re-reads the journal
each time storage's committed-sequence signal moves (§2.4, §6.18). Moving it
must keep three things:
- no frame is lost or repeated across that switch;
- frames stay in `durable_seq` order;
- a reconnect with `after` resumes exactly where the client stopped.

`tests/resync.rs` and the SSE tests pass unchanged.

## 14.9 Rules that move, and must not be lost

Each of these becomes an obligation in its service's contract, with a real test:

- **Busy, twice, for two reasons.** `start_turn` and `change_model` check
  `thread_is_busy` before touching the session, so that a model or setting is
  never changed under a running turn. `start_turn`'s write transaction checks
  `has_open_operation` again, inside `BEGIN IMMEDIATE`, and that check is the
  atomic guarantee.

  Both stay. The early check moves into `Turns::send` and
  `Harness::change_model`, and no caller performs it.
- **Session before lease.** A turn opens the session, then leases its events
  before prompting. Neither a second start nor a `/context` read can use the
  session until the turn gives it back.
- **Harness availability.** `policy::is_available` is checked before any session
  work.
- **Grant scope.** A tool call's grant decides which plans it sees (§13.7). The
  checks (`plan_in_scope`, `own_thread`, `writer_of`) move into `Plans`
  unchanged, refusal texts included.
- **Replay.** A retried command with the same id and the same request answers
  what it answered first. Its command kind and fingerprint parameters do not
  change.

An implementer who finds another rule adds it to the contract. Dropping a rule
is a defect even when every test passes.

## 14.10 Acceptance

Milestone 2.5 is done when:

1. **The gates pass on Windows and on CI's Linux job:**
   - `cargo fmt --check`;
   - `cargo clippy --workspace --all-targets -- -D warnings`;
   - `cargo test --workspace`, with the Rust test count at least 304;
   - `cargo check --workspace`, with no tests and so no `test-support`;
   - the web gate, still 135 tests;
   - `api/openapi.json` byte-identical.
2. `contracts.rs` passes with eight contracts.
3. **Mohammed runs the debug build in the browser:**
   1. sends a message and stops it;
   2. restarts the daemon, and the conversation is still there;
   3. edits and approves a plan;
   4. connects an external Claude Code with **Connect**, reads a plan, then
      **Revoke**s it.

   Each works as it did on `ddcaed5`. The evidence goes in
   `docs/evidence/milestone2_5/WINDOWS_RUN.md`.

## 14.11 Changes to other documents

- **§1 (`2026-09-21-architecture-design.md`):** "a single crate" becomes this
  workspace. §1 refers here for the crate list.
- **§2.9:** backend-specific SQL stays inside a service's `store.rs` and
  `shadows-core/src/db/`, never outside `shadows-core`. When PostgreSQL comes,
  it arrives as a backend module beside each store.
- **`CLAUDE.md`:**
  - "Architecture" describes the workspace;
  - the ownership table names crates;
  - two rules are added:
    - a new operation is a new method on one service, with its line in that
      service's contract;
    - an adapter translates and holds no rule.
- **`docs/codebase/`:**
  - the README maps crates and services, and holds the Architecture Invariants;
  - the generated inventory spans every crate.
- **This index** gains §14.
