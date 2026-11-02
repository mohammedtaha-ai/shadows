# Section 14 — A Workspace Around One Application Core (Milestone 2.5)

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there.

- **Date:** 2026-09-26, rewritten 2026-09-27.
- **Status:** Designed with Mohammed on 2026-09-26/27. Built; see "Delivered" below.
- **Builds on:** Milestone 2 (§13), on `main` at `ddcaed5`.

**Delivered.** Milestone 2.5 built this workspace (`008e54c` to `d51fb8a`); the
service crates of §14.12 finished in `d7a722cd`. What follows records the rules
that stay. The current service list is `AppCore`'s table in
`crates/shadows-core/src/app.rs`.

Milestone 2.5 changes **where the code lives, not what it does.** Shadows
becomes a Cargo workspace:
- one object, `AppCore`, in `shadows-core`, is the whole application's way in
  (at Milestone 2.5 the services lived in that crate; now each is a `core-<service>`
  crate);
- small crates around it translate HTTP, MCP and the command line;
- two infrastructure crates run the harness and processes.

The compiler enforces the boundaries. No feature is added, and nothing a client
observes changes.

It comes before Milestone 3, the code index (`docs/core/vision.md` §2.4). The index
assumes a project where every operation has one discoverable place, and Shadows'
own code must be that project first.

## 14.1 Scope

In:

1. **A workspace** with a flat `crates/` directory (§14.3).
2. **`AppCore`** and its services (§14.4).
3. **Adapters that only translate:** `shadows-http`, `shadows-mcp`, and the
   `shadows` binary (§14.5).
4. **Boundaries the compiler enforces**, and the one check the compiler cannot
   make (§14.6).
5. **A contract per service**, in Mohammed's source-contract template, that a
   test keeps current (§14.7).

Out:

- Any new feature, route, MCP tool, or change to a route's JSON.
- A crate per service: §14.12 did that later, finished in `d7a722cd`.
- The web client. `api/openapi.json` does not change.
- The deferred items in `docs/status.md` at `8580306`: effort at once, and one lock per
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

The lesson: **a crate makes a dependency boundary explicit.** Rust also enforces
private module and field access inside a crate; `pub(crate)` deliberately widens
access. Neither boundary assigns SQL tables to services. A project starts with
a large core crate, splitting it only along
boundaries that have proven themselves, as Codex does, rather than guessing
them up front.

**Why services were not crates at first.** In Milestone 2.5 the domain types
formed cycles (`thread` ↔ `workflow`, `Threads` ↔ `Harness`, `Plans` creating a
thread). They stayed folders until §14.12 resolved those cycles; the services
became crates in `d7a722cd`.

## 14.3 The workspace

```text
Cargo.toml                 virtual manifest: [workspace], shared deps and lints, no package
crates/
  shadows/                 the binary: main, the CLI, startup (builds AppCore once)
  shadows-http/            the HTTP API and SSE (axum, utoipa)
  shadows-mcp/             the MCP server (rmcp)
  shadows-core/            AppCore, the composition root, and CoreError
  core-db/                 the pool, the transaction, the journal
  core-<service>/          one service each: its model, private store and contract
  shadows-agent/           the ACP client for a harness adapter
  shadows-index/           a file's text in, its tags out, with tree-sitter (§15.3)
  shadows-process/         OS processes with whole-tree containment
  fake-acp/                the fake ACP adapter tests drive (binary)
api/  web/  docs/          unchanged
```

- **Folder name equals package name.** No crate is named `core`, which would
  shadow Rust's own `core`.
- **Dependencies point one way:**

  ```text
  shadows ─┬─► shadows-http ─┐                 ┌─► shadows-agent ─► shadows-process
           ├─► shadows-mcp  ─┼─► shadows-core ─┤
           └────────────────►┘                 └─► shadows-index
  ```

  Cargo refuses a cycle, so `shadows-core` can never depend on an adapter.
  `shadows-index` depends on no Shadows crate (§15.2).
- **Shared dependency versions and lints** are declared once, in the root
  `[workspace.dependencies]` and `[workspace.lints]`.
- `shadows-process` keeps `tree_probe` as its own test binary. Its containment
  tests name it directly; two daemon tests in `crates/shadows` (`serve_smoke`,
  `planner_mcp`) also run it as a stand-in executable, and locate it as below.
- `fake-acp` is a separate binary crate. Tests in other crates locate it, and
  `tree_probe`, through **`escargot`** (0.5, maintained by `crate-ci`). Cargo's
  `CARGO_BIN_EXE_*` only names binaries of the test's own package.
- **The move from one crate** to this workspace is in Git at `d51fb8a`.

There is no shared test-kit crate. The core's tests would depend on a crate that
depends on the core, and Cargo would build two copies of its types.

## 14.4 `shadows-core`: `AppCore` and its services

`AppCore` is the Shadows application. The binary builds it once, with
`AppCore::start(config, mcp_url)`, and shares it as `Arc<AppCore>`. It is **not**
a global: no `static`, no `OnceLock`, no service locator. Rust's idiom is a
composition root written by hand and checked by the compiler, so no
dependency-injection library is used. `shaku` and the like add macros and
indirection and buy nothing here.

**Scope clarification, agreed 2026-10-18, amended 2026-10-19:** one `AppCore`
means one composition root for a Shadows runtime. On 2026-10-19 Mohammed chose
to run AYLUL's conversations on this `AppCore`, inside the same product daemon
and database, with Claude Code as its brain in an isolated home; the
[AYLUL brain design](../../aylul/specs/2026-10-19-aylul-claude-code-brain-design.md)
owns that, and how it is represented here is decided in its plan. The AYLUL
hook/sidecar runtime, the native Tauri shell and the independent
code-intelligence service keep their own startup, state and shutdown, and talk
through explicit client interfaces. No common base class, shared container or
cross-runtime transaction is introduced, and the integration is not a reason to
split Projects, Plans or Turns out of this runtime. The earlier text tying AYLUL
to a remote Hermes brain is in Git at `226d0a3`.

A caller reaches a service through an accessor: `core.plans()`, `core.turns()`,
and so on. Each service is a struct with methods and one job.

The services, their one job, their crate and their contract are listed in one
place: the table on `AppCore` in `crates/shadows-core/src/app.rs`. Each service is
a `core-<service>` crate (§14.12); `shadows-core` holds `AppCore`, `CoreError` and
the test fixtures only.

- **One capability, one method.** An HTTP route and an MCP tool that do the same
  thing call the same method. `get_plan`, `workflow_get` and `task_get` all
  end in `Plans`.
- **A service builds its own `CommandContext`**, with the same command kinds and
  fingerprint parameters as today, so every replay still matches.
- **Who emits, who delivers.** A service emits what it causes: `Turns` sends a
  turn's live events on the bus, and `Plans` sends `plan_show`'s UI signal.
  `Events` owns subscription and delivery, and emits nothing of its own.
- **Shutdown.** `AppCore::shut_down()` first stops `Code`'s watchers and
  workers (§15.8), then stops every running turn and closes every adapter,
  which is `planner::shut_down` today. Only after Turns stops successfully,
  it awaits Storage closure last; a Turns error leaves storage open. The binary
  calls this same shutdown path, so `shadows serve` closes its database on exit.
  The binary keeps what belongs to
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
- **Between services** a call goes only to a dependency the service receives in
  its constructor and lists in its `Cargo.toml`. Cargo refuses a cycle, and a
  service never reaches another's store or private model (§14.12).

Extraction readiness, once OPEN here, closed when the extraction finished in
`d7a722cd` (§14.12).

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
  `HarnessInfo` is an example.

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

- **the gate runs `cargo clippy --workspace -- -D warnings` without tests.** A non-test crate
  that reaches `shadows_core::testing` fails to build. This extends the CI check
  that already proves `test-support` never reaches the shipped binary
  (`cargo tree -e features,no-dev`).

**What privacy does not establish:** Rust enforces private module access, but
`pub(crate)` permits access throughout the crate, and a shared SQL connection
can execute queries against any table regardless of the query's module. Three
things cover ownership beyond those language visibility rules:
- each service's `contract.yaml` names its public methods and its
  `not_the_caller's`;
- review reads it.
- the architecture ratchet checks cross-owner calls and direct SQL, including
  after extraction into separate crates (§14.12).

That is the price of keeping the services in one crate for now (§14.4, OPEN).

**Store functions one write shares across services** (ruled by Mohammed,
2026-09-27). One write transaction touches several services' tables: saving a
plan draft calls `insert_thread` (Threads), `check_writer` (Grants) and
`append_entry_in`. Each such function **stays in the service that owns its
table**, becomes `pub(crate)`, and is declared in that service's contract under
`shared_in_transaction`, with who calls it and why. Nothing else in a `store`
is `pub(crate)`. Moving them all into `db/` was rejected: `db/` would become a
pile of functions from every service with no owner. `contracts.rs` enforces it
(§14.7, rule 9): every `pub(crate)` function or method in a service's `store`
is declared, and every declared one exists.

**Store methods other services read or write outside their own write**
(ruling F6, Task 10). The stores' `impl Storage` methods stay `pub`: `Storage`
is private to the core, and a test reaches it only through the test-only
`testing::Storage`, so the adapter boundary is closed by the compiler. Inside
the crate, a service that calls another's store method outside a write of its
own (`turn_context`, `get_project`, `current_planner_instructions`, …) is
declared in the owning service's contract under `called_by_other_services`,
with who calls it and why; `contracts.rs` refuses a declared name that is not
in that service's folder.

The shared-store permissions above describe the migration baseline, not a new
extension point. §14.12 requires these calls to become service methods while
the services are still in `shadows-core`. The ratchet permits only enumerated
existing exceptions and removes them as their callers migrate.

**Architecture Invariants** are written into `docs/codebase/README.md`, in
rust-analyzer's style, one line each:
- "`shadows-http` knows HTTP; nothing below it does."
- "`shadows-core` never imports `axum` or `rmcp`."
- "another service calls a service's `store` only through a function or
  method that store's contract declares, under `shared_in_transaction`
  (inside one write) or `called_by_other_services`."

## 14.7 Contracts

Every service has one contract, `crates/core-<service>/contract.yaml`,
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
- **`shared_in_transaction`:** the `store` functions other services may call
  inside one write, each with its callers and why (§14.6). A section Shadows
  adds to the template;
- **`called_by_other_services`:** the `store` methods other services call
  outside a write of their own, each with its callers and why (§14.6). Also
  Shadows' own;
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
(`docs/core/vision.md` §1.4). The service is the one entry an agent uses.

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
8. an agreement says `holds: false` and no `gap` names one of its symbols;
9. a `pub(crate)` function or method in the service's `store` is missing
   from `shared_in_transaction`, or a name there is not such a function or
   method; or a name under `called_by_other_services` is not a function or
   method in the service's folder (§14.6).

Every rule in §14.9 must name a real test. What the test cannot check is whether
a rule's prose is true, or whether a named test's body proves it; the reviewer
owns that, and reads every `tested_by` body it rules on.

## 14.8 Order of work

Delivered: steps 0–11, `008e54c` to `d51fb8a`. The step table and its risk notes
are in Git at `f1459dcc`. The later extraction order is §14.12's.

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

Delivered at Milestone 2.5 (`d51fb8a`). The acceptance list is in Git at `f1459dcc`.

## 14.11 Changes to other documents

Done at Milestone 2.5; the list is in Git at `f1459dcc`.

AYLUL agent configuration is owned by `core-agents` and `core-providers`;
credentials stay with `core-secrets`. See AYLUL brain design section 11.

## 14.12 Service ownership and staged crate extraction

The selected style is a modular monolith with explicit service objects. One
`AppCore` owns construction and shutdown for one Core runtime. Its typed
accessors are the entry for adapters; services receive only their specific
dependencies through constructors. No service receives `AppCore`, a service
locator, or a callback that retrieves arbitrary services. `Arc` clones share
the same instance and state. Composition uses concrete structs; traits are
introduced only for a real interchangeable boundary, not for every class.

Each service owns its rules, model, private store and live state. Other services
call its methods. A cross-service write calls the owner's `*_in(&mut Tx, ...)`
inside the caller's single transaction, with no nested commit or premature
notification. Cross-service operations preserve atomicity, command identity,
replay/conflict behavior, ordering and cancellation recovery. `AppCore` wires
these owners; it does not accumulate their business rules.

Preparation happens in place before physical extraction:

1. Register actual owners/accessors/contracts in `AppCore` and enforce a
   shrinking list of existing exceptions with the architecture ratchet.
2. Resolve runtime, public-type, error and test dependency cycles explicitly.
   The controller records owners and the extraction order in the architecture
   plan before any crate move; the compiler is a check, not a design algorithm.
3. Replace domain `impl Storage` methods with private service stores while
   `Storage` is still local to `shadows-core`. Use one database pool/transaction
   owner. Move direct cross-owner SQL behind the owning service's method.
4. Extract the prepared database foundation, whose inputs and failures have
   no dependency on domain services. Preserve the existing SQLx transaction
   mechanism. Database-only test observation follows it under `test-support`.
5. Extract service crates in the verified order, preserving their public APIs
   and updating callers, contracts, maps, manifests and tests in the same task.
   Service-local tests exercise that service; application integration tests
   exercise `AppCore`. No lower crate requires the composition root to exist.

Rust privacy protects private stores and Cargo rejects crate cycles. Neither
prevents arbitrary SQL against another service's tables through a shared
connection. Keep SQL ownership and adapter-access checks after extraction;
moving a file cannot clear a violation. Test fixtures for the guard must include
cross-owner SQL and a service moved out of `shadows-core`.

The source-contract template remains the repository's existing template. Keep
one entry contract per service, with sections for its modules; add a subordinate
contract only when it has an independently owned boundary. Contracts describe
inspected code, reference external types by owner, and distinguish verified
`gaps` from `open_questions`. Tests and code establish truth; comments and a
passing YAML parser do not prove behavior. Preserve useful rationale when
removing obsolete comments. This amendment is a target, not evidence of delivery.

**2026-10-22, the resolved ledger.** The plan's A-8c ledger records the owners
and the acyclic extraction order. It adds four owners:
- `standards`: out of design.
- `operations`: operation rows, live handles, running-turn checks, stop and
  recovery, out of turns and runtime.
- `project-removal`: removing a project across its dependants in one
  transaction.
- The five owners out of profiles: providers, agents, extensions, usage and
  person.

Foundation inputs carry no service types:
- the journal takes string subject ids;
- `CommandContext::user` replaces `app::user_command`;
- `StorageError` keeps only database failures.

The [architecture plan](../../backend/plans/2026-10-21-architecture.md) owns the
task sequence. Product workspace moves and runtime replacement are separate
deliveries; neither is required to provide one `AppCore` or private service stores.
