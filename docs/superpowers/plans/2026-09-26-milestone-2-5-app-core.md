# Milestone 2.5 — A Workspace Around One Application Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shadows becomes a Cargo workspace:
- `shadows-core` holds the whole application behind one `AppCore`, and each of its eight services lives in one folder;
- `shadows-http`, `shadows-mcp` and the `shadows` binary only translate;
- `shadows-agent` and `shadows-process` are infrastructure.

The compiler enforces the boundaries. Nothing a client observes changes.

**Architecture:** This is spec §14. The crate moves come first and are pure moves:
- the workspace, then process, agent, core as it is, then the adapters;
- the tree builds and every test passes after each one.

Then the services go in one by one. Each task:
- moves one service's files into `crates/shadows-core/src/<service>/`;
- gives it a struct with methods;
- points the routes and tools at `core.<service>()`;
- makes that service's internals private to `shadows-core`, so the compiler refuses any adapter still reaching past it.

The boundary only ever tightens, and nothing needs an allowlist or a guard test.

**Tech Stack:** Rust 2024 edition (resolver 3), axum 0.8, utoipa, SQLx 0.9 on SQLite, `rmcp` 3.4.1. New dev-only dependencies:
- `escargot = "0.5"`, so tests in other crates can locate the `tree_probe` binary (from Task 1) and the `fake-acp` binary (from Task 3);
- `yaml-rust2 = "0.13"`, for the contracts test.

**Spec:** `docs/superpowers/specs/2026-09-26-application-core-design.md` (§14). Where this plan and §14 disagree, §14 is right and the plan is the defect: stop and report.

## Global Constraints

- **Branch** `milestone-2.5/app-core`, off `main` at `ddcaed5`.
  - No git worktrees.
  - Commit per task (a task may commit its steps separately where it says so).
  - Push only when Mohammed asks.
- **Execution:** every subagent runs on opus, stated explicitly. The reviewer is an opus agent (Codex is unavailable since 2026-09-27), which fixes what it finds, runs the gate and commits. The controller verifies the review and does not repeat it.
- **Read first:** `docs/codebase/README.md` and `docs/codebase/inventory.md`. Open only the files the task names.
- **Behaviour does not change.**
  - No route, MCP tool, JSON field, status code, error code, error message, log line, command kind or fingerprint changes.
  - `api/openapi.json` stays byte-identical, and `web/` is not touched.
  - Existing tests change only their location, construction and imports, never what they assert. If an assertion must change, stop and report.
- **Moves are moves.** Use `git mv` so history follows each file. A task that moves files changes only paths, `use` lines and visibility. Logic changes happen only in the service tasks (5–10), and only as the task describes.
  - **Store helpers that one write shares across services.** The `storage/sqlite/*.rs` files call each other's private helpers inside one `write_txn`: `classify`, `record_command` (`command.rs`), `append_event` (`events.rs`), `append_entry_in` (`entry.rs`), `insert_thread`, `load_thread` (`thread.rs`), `check_writer`, `bind_draft_ref` (`grant.rs`), `task_of`, `write_content` (`task.rs`), `remember_settings` (`harness.rs`), `has_open_operation` (`turn.rs`), and `now()` (`sqlite/mod.rs`). They are `pub(super)` or `pub(in crate::storage)` today. When a task moves a store file out of `storage/sqlite/`, each helper it still calls in a file that has not moved, or in another service's `store`, widens to `pub(crate)` — never `pub` — and keeps its body. That is a visibility change, which a move may make.
    - **The ruling (§14.6):** each helper stays in the service that owns its table. When its store moves into a service, it goes into that service's `contract.yaml` under `shared_in_transaction` (a map, name → `"<callers>: <why>"`), and `contracts.rs` rule 9 refuses any `pub(crate)` store function not declared there. Nothing else in a store is `pub(crate)`.
    - Helpers that belong to no service — `classify`, `record_command`, `append_event`, `now()` — go to `db/` (the command log, the journal and the clock), not into a service's store.
- **Writing a contract** (§14.7; the template is `docs/codebase/contracts/TEMPLATE.yaml`, read it whole first). A contract is written by tracing the service's code after its move, never from comments or this plan's prose.
  - `tested_by` names a test whose **body** you read and which proves the rule. A test that proves less goes in with a `note` saying what it misses.
  - Where one rule has two paths, write an `agreements` entry: busy in `Turns::send` and `Harness::change_model`; one plan through `get_plan`, `workflow_get` and `task_get`.
  - A comment that disagrees with the code is a `gap`. A doubt about another service is an `open_question`.
  - The reviewer checks the contract against the code and the test bodies, not only that `contracts.rs` passes.
- **Log targets.** A log line's `target` is its module path, so it moves with its file (`shadows::planner::turn` becomes `shadows_core::turns::…`). Its message and fields do not change. `tracing.rs`'s default filters (`shadows=info,warn`, `shadows=debug,…`) still select every crate's lines, because `EnvFilter` matches a target by string prefix (`tracing-subscriber` 0.3, `filter/env/directive.rs`) and every product crate's name starts with `shadows`. Do not change the filters.
- **Names:**
  - folder name equals package name (`crates/shadows-core` is the package `shadows-core`, lib `shadows_core`);
  - no package is named `core`.
- **Dependencies:**
  - every version is declared once, in the root `[workspace.dependencies]`; a crate writes `dep = { workspace = true }`;
  - lints are declared once, in `[workspace.lints]`, and every crate has `[lints] workspace = true`.
- **Files:** 300 lines needs a stated reason, and at 500 the file splits (CLAUDE.md). `planner/sessions.rs` is at 485 lines: it moves as it is, and only a task that splits it may add to it.
- **Builds** use the C: target directory, because E: fills up:
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/Temp/claude/E--Globalprojects-shadows/e86a610a-4c5b-4114-884d-faf050ce6a96/scratchpad/target CARGO_INCREMENTAL=0`.
  Stop any running `shadows.exe` preview before building.
- **The gate before every commit:**
  1. `cargo fmt --all --check`
  2. `cargo clippy --workspace --all-targets -- -D warnings`. From Task 3 on, `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`: `fake-acp`'s binary builds only with its own `test-support` (Task 3), and without the flag nothing lints it.
  3. `cargo test --workspace`
  4. From Task 10 on, also `cargo clippy --workspace -- -D warnings`, **without** `--all-targets`. This is the build without `test-support`, where the boundary is checked.

  Report the Rust test count. It is 304 at the start, and it may only grow. The web count, 135, must not move.
- **Code map:** every signature change regenerates it in the same commit: `UPDATE_CODEMAP=1 cargo test -p shadows --test codemap` from Task 0 on. From Task 0 the test reads every `crates/*/src` and the README's rows name workspace-relative paths (`crates/shadows/src/agent/`), so every task that moves a module also moves its README rows to the new path, in the same commit, or the ownership test fails. A new module gets its one-job line in `docs/codebase/README.md`.

## Review Focus

1. **A retried command after a move.** A client retries `POST /api/threads/{id}/turns`, or a Planner retries `plan_edit`, with the same `command_id`. The person expects the first answer again. The fingerprint must be built from exactly the same parameters as before. (Task 5 test `plan_commands_replay_after_the_move`; Task 7 test `turn_start_replays_after_the_move`.)
2. **A model change while a turn runs.** The person expects `THREAD_BUSY`, with the running turn's session untouched. (Task 8 test `change_model_is_refused_while_a_turn_runs`.)
3. **Stop during a turn, then an SSE reconnect with `after`.** The person expects the turn `Cancelled`, and the stream to resume without a lost or repeated frame. (Task 7 test `stop_after_the_move_records_cancelled`; Task 10 relies on `resync.rs`, which is unchanged.)
4. **An external agent reaching a plan outside its grant.** It must get the same `GRANT_SCOPE` text as today. (Task 5 test `a_project_grant_is_refused_another_projects_plan_after_the_move`.)
5. **Ctrl+C with a turn running.** The daemon records the turn `Cancelled`, then ends its live streams, in that order. (Task 5 test `shut_down_through_the_core_cancels_and_records`.)

---

## Execution map

```text
Task 0   workspace: virtual manifest; today's crate moves whole into crates/shadows
Task 1   crates/shadows-process      (process/, tree_probe, containment test)
Task 2   crates/shadows-agent        (agent/, its tests)
Task 3   crates/shadows-core as it is (every core module, still pub) + crates/fake-acp + escargot
Task 4   crates/shadows-http, crates/shadows-mcp (still reaching core's pub internals)
Task 5   AppCore, CoreError, the contracts test, and Plans            ← services begin
Task 6   Grants
Task 7   Turns                                                         ← risk: Stop, lease, turn end
Task 8   Harness
Task 9   Projects, Threads, Instructions
Task 10  Events; db/runtime private; testing module; the boundary check ← risk: journal → caught-up → live
Task 11  contracts complete; CLAUDE.md, code map, §1 and §2.9, status
Task I   (controller) whole-branch review, Mohammed's run, evidence, PR
```

---

### Task 0: The workspace

**Files:**
- Move, with `git mv`: `src/`, `tests/`, `migrations/`, `build.rs` and `Cargo.toml` into `crates/shadows/`. `api/`, `web/`, `docs/`, `harness/`, `.github/` and `Cargo.lock` stay at the root. `build.rs` (`cargo:rerun-if-changed=migrations`) must sit beside the manifest whose crate runs `sqlx::migrate!`; a virtual root manifest runs no build script.
- Create: the root `Cargo.toml`, a virtual manifest.
- Modify: `crates/shadows/Cargo.toml`, which uses `workspace = true` for version, edition, rust-version, lints and every dependency.
- Modify: every path that assumed the crate was the root:
  - `tests/openapi.rs` (`CARGO_MANIFEST_DIR` + `../../api/openapi.json`);
  - `tests/codemap/` — see "The code map spans the workspace" below;
  - `tests/project_directory.rs` (migrations stay beside the crate, so no change, but check it).
- Modify: `tests/codemap/{main,scan}.rs` and `docs/codebase/README.md` — **the code map spans the workspace from this task on**, because Tasks 1–10 each take modules out of `crates/shadows/src` and the test must keep passing after each one (§14.8):
  - the root is the workspace (`CARGO_MANIFEST_DIR/../..`), and `docs/codebase/*` is read from there;
  - the inventory scans every `crates/*/src`, sorted, with paths relative to the workspace (`crates/shadows/src/agent/acp.rs`);
  - the `newtype_id!` template is read from whichever `crates/*/src/id.rs` exists (`scan.rs`'s `newtype_id_template` answers `None` for a missing file, which would silently drop every id from the map);
  - the README's rows, and `parse_row`'s prefix check, name `crates/<crate>/src/…`; `top_level_modules` lists each `crates/*/src`'s children;
  - regenerate the inventory, and rewrite each README row's path (`src/agent/` → `crates/shadows/src/agent/`). The jobs do not change.
- Modify: `.github/workflows/ci.yml`:
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo test --workspace`;
  - `cargo fmt --all --check`;
  - the `cargo tree -e features,no-dev --package shadows` check stays as it is.
- Modify: `web/package.json` or its `gen:api` script, only if it names a path that moved. It reads `api/openapi.json`, which does not move. Check it.

**Interfaces:**
- Produces: the root manifest every later crate joins:

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94"
publish = false

[workspace.dependencies]
# every dependency and dev-dependency today's Cargo.toml lists, same versions,
# same features, same comments — moved here verbatim. The self dev-dependency
# `shadows = { path = ".", features = ["test-support"] }` becomes, in the
# package's own manifest, `shadows = { path = ".", features = ["test-support"] }`
# (a path dependency is not a workspace dependency).

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
```

Copy any `[lints]` today's `Cargo.toml` has into `[workspace.lints]`. If it has none, keep `unsafe_code = "forbid"` only if `src/` has no `unsafe` block; `grep -rn "unsafe" crates/shadows/src` decides. `[workspace.lints.clippy]` stays empty, and the gate's `-D warnings` is the policy.

- [ ] **Step 1:** `git mv src tests migrations build.rs crates/shadows/`, then `git mv Cargo.toml crates/shadows/Cargo.toml`, then write the root manifest.
- [ ] **Step 2:** Rewrite `crates/shadows/Cargo.toml` to take everything from the workspace:

```toml
[package]
name = "shadows"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
anyhow = { workspace = true }
# … every dependency, `{ workspace = true }`, with `optional = true` where it was
```

  Keep `[[bin]]`, `[features]` and the self dev-dependency in the package manifest.
- [ ] **Step 3:** Fix the paths listed under Files, and make the code map span the workspace.
- [ ] **Step 4:** Run the gate. Expect 304 Rust tests, all passing. `api/openapi.json` is unchanged (`git diff --exit-code api/`). The regenerated inventory differs from the old one only in its paths and its header's wording.
- [ ] **Step 5:** Commit: `build: a Cargo workspace; the crate moves whole into crates/shadows (§14.3)`

---

### Task 1: `shadows-process`

**Files:**
- Create: `crates/shadows-process/Cargo.toml`, with dependencies only on what `process/` uses (`process-wrap`, `tokio`, `tracing`, `thiserror`, and `libc` on unix).
- Move: `crates/shadows/src/process/mod.rs` → `crates/shadows-process/src/lib.rs`; `crates/shadows/src/bin/tree_probe.rs` → `crates/shadows-process/src/bin/tree_probe.rs`. Keep `required-features = ["test-support"]` with a `test-support` feature and a self dev-dependency, as `shadows` has.
- Move: `crates/shadows/tests/containment.rs` → `crates/shadows-process/tests/containment.rs`. Its `CARGO_BIN_EXE_tree_probe` now resolves inside the same package.
- Modify: `crates/shadows/Cargo.toml` (depends on `shadows-process = { path = "../shadows-process" }`), and every `crate::process` → `shadows_process` in `crates/shadows/src`.
- Modify: the features. `ProcessHandle::force_termination_failure` is gated on `shadows-process`'s own `test-support` now, and `planner/sessions.rs` calls it under `shadows`' `test-support`. So `shadows`' `test-support` gains `"shadows-process/test-support"`; without it the `shadows` tests that force a termination failure (`planner_turn`, `protocol`, `shutdown`) do not compile.
- Modify: the two tests that stay in `crates/shadows/tests` and run `tree_probe` as a stand-in executable: `serve_smoke.rs` (`--node`, and `--harness` for the version timeout) and `planner_mcp.rs` (`serve(…, CARGO_BIN_EXE_tree_probe)`). `CARGO_BIN_EXE_tree_probe` no longer exists in `shadows`. Add `escargot` to the root `[workspace.dependencies]` and to `shadows`' `[dev-dependencies]`, and a fixture `crates/shadows/tests/fixtures/probe.rs`:

```rust
//! Where `tree_probe` is: `shadows-process`'s test binary, which
//! `CARGO_BIN_EXE_*` names only inside that package (spec §14.3).
pub fn tree_probe_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        escargot::CargoBuild::new()
            .package("shadows-process")
            .bin("tree_probe")
            .features("test-support")
            .current_release()
            .run()
            .expect("tree_probe builds")
            .path()
            .to_path_buf()
    })
    .clone()
}
```

  No `.current_target()`: it passes `--target`, which builds into `target/<triple>/` and compiles every dependency a second time. Task 3 moves this function into `shadows_core::testing` and deletes the fixture.

**Interfaces:**
- Produces: `shadows_process::{ProcessSpec, ProcessHandle, spawn, …}`, the same items `crate::process` exports today, under the new path. Read `docs/codebase/inventory.md`'s `process` section for the list.

- [ ] **Step 1:** Create the crate and move the files.
- [ ] **Step 2:** Fix the imports. `tokio::process` stays private to this crate, now with the compiler's help: no other crate depends on `process-wrap`.
- [ ] **Step 3:** Point `serve_smoke.rs` and `planner_mcp.rs` at `probe::tree_probe_path()`. Check `escargot` 0.5's API (`features`, `current_release`, `run`, `path`) before relying on it, and adjust to it. In the README, `process/`'s row becomes `crates/shadows-process/src/lib.rs`; `crates/shadows-process/src/bin/` gets a row (reference `tree_probe.rs`); and `crates/shadows/src/bin/`'s reference file becomes `fake_acp.rs`, which stays until Task 3.
- [ ] **Step 4:** Run the gate. The count is unchanged: `containment` moved but still runs.
- [ ] **Step 5:** Commit: `build: shadows-process is its own crate (§14.3)`

---

### Task 2: `shadows-agent`

**Files:**
- Create: `crates/shadows-agent/Cargo.toml` (`agent-client-protocol`, `serde`, `serde_json`, `tokio`, `tokio-util`, `tracing`, `thiserror`, `shadows-process`, and anything else `agent/` imports).
- Move: `crates/shadows/src/agent/*` → `crates/shadows-agent/src/`, with `mod.rs` → `lib.rs`.
- Move: the tests that need only `agent` and `process`. `acp_connection.rs` needs `fake-acp`, so it moves in Task 3, when `fake-acp` exists. For the other agent-only tests, the rule is the lowest crate that can compile the test:
  - `harness_choices.rs`, `harness_observation.rs` and `sessions.rs` import `shadows::planner` or fixtures, so they stay;
  - `harness_config.rs` imports `shadows::config`, which is the binary's, so it stays.

  Move only what compiles against `shadows-agent` alone, and list what moved in the report.
- Modify: `crate::agent` → `shadows_agent` across `crates/shadows/src`.

**Interfaces:**
- Produces: `shadows_agent::{acp, choices, events, policy, breakdown, claude}`, the same modules `crate::agent` has today.

- [ ] **Step 1:** Create the crate, move the files, and fix the imports.
- [ ] **Step 2:** Check that `shadows-agent` depends on nothing but `shadows-process` among the workspace crates: `cargo tree -p shadows-agent --depth 1`.
- [ ] **Step 3:** Run the gate, then commit: `build: shadows-agent is its own crate (§14.3)`

---

### Task 3: `shadows-core` as it is, and `fake-acp`

**Files:**
- Create: `crates/shadows-core/Cargo.toml`. It takes the dependencies the moved modules use (`schemars` and `utoipa` included: the domain types derive both), plus `shadows-agent` and `shadows-process`, and has a `test-support` feature and a self dev-dependency. Every `#[cfg(feature = "test-support")]` in the moved modules now reads this crate's feature: `storage::test_support`, `newtype_id!`'s `from_literal` (`id.rs`), `planner/handles.rs`, `planner/sessions.rs` and `storage/sqlite/harness.rs`. So:
  - `shadows-core`'s `test-support = ["dep:escargot", "shadows-process/test-support"]`;
  - `shadows`' `test-support` becomes `["shadows-core/test-support", "rmcp/client", "rmcp/transport-streamable-http-client-reqwest", "dep:reqwest"]`, because its tests call `from_literal` and `force_termination_failure` too.
- Move: `crates/shadows/build.rs` → `crates/shadows-core/build.rs`, with the migrations.
- Move, unchanged except for paths:
  - `project/`, `thread/`, `workflow/`, `planner/` (with `prompt.txt`), `runtime/`, `operation/`, `events/`, `command/` and `storage/`;
  - `mcp/grant.rs`, which goes to `crates/shadows-core/src/grant/mod.rs`, because it is a domain type (§14.3's table);
  - `error.rs` and `id.rs`;
  - `migrations/`, to `crates/shadows-core/migrations/`, because `sqlx::migrate!` resolves beside the crate's manifest.
- Create: `crates/shadows-core/src/lib.rs`, with every moved module `pub mod`, exactly as `crates/shadows/src/lib.rs` has them today. **Nothing is private yet.**
- Create: `crates/fake-acp/`. It is a binary crate, `src/main.rs`, moved from `crates/shadows/src/bin/fake_acp.rs`. It uses `shadows-core` (`grant::hash_token`), `agent-client-protocol`, `tokio`, `serde_json`, and `rmcp`'s MCP client (`client`, `transport-streamable-http-client-reqwest`, with `reqwest` as `shadows` declares it). **None of that may reach a normal workspace build.** A virtual manifest's `cargo build` and the boundary check's `cargo clippy --workspace` (Task 10) build every member together, and Cargo unifies features across them: a plain `shadows-core = { features = ["test-support"] }` in `fake-acp` would switch `test-support` on for `shadows-http` as well, and Task 10's proof that `shadows_core::testing` is unreachable would pass for the wrong reason. So `fake-acp` is gated exactly as `fake_acp` was:

```toml
[[bin]]
name = "fake-acp"
path = "src/main.rs"
required-features = ["test-support"]

[features]
test-support = ["shadows-core/test-support", "rmcp/client",
                "rmcp/transport-streamable-http-client-reqwest", "dep:reqwest"]
```

  with `shadows-core` a plain dependency and `rmcp` at the workspace's server-only features. `testing::fake_acp_path()` builds it with `.features("test-support")`. Its lint is gate step 2's `--features fake-acp/test-support`. This is how §14.6's "a crate enables that feature only in its `[dev-dependencies]`" holds for a test binary that is its own package: nothing enables it except a build that asks for it.
- Create: `crates/shadows-core/src/testing.rs`, behind `#[cfg(feature = "test-support")] pub mod testing;`, with:

```rust
//! What tests in every crate share: where the fake adapter is, and the paths
//! a test needs that belong to this crate. Compiled only with `test-support`.

/// The `fake-acp` binary, built once per test process. `CARGO_BIN_EXE_*`
/// only names binaries of the test's own package, and `fake-acp` is its own
/// package, so it is located through `escargot` (spec §14.3).
pub fn fake_acp_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| built("fake-acp", "fake-acp")).clone()
}

/// `shadows-process`'s `tree_probe`, which the daemon tests run as a stand-in
/// executable (moved here from Task 1's `fixtures/probe.rs`).
pub fn tree_probe_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| built("shadows-process", "tree_probe")).clone()
}

/// No `.current_target()`: it passes `--target`, which builds into
/// `target/<triple>/` and compiles every dependency a second time.
fn built(package: &str, bin: &str) -> std::path::PathBuf {
    escargot::CargoBuild::new()
        .package(package)
        .bin(bin)
        .features("test-support")
        .current_release()
        .run()
        .unwrap_or_else(|e| panic!("{bin} builds: {e}"))
        .path()
        .to_path_buf()
}

/// The Planner's instructions, as `harness/setup` compiles them in.
pub const PROMPT: &str = include_str!("planner/prompt.txt");

/// This crate's migrations directory.
pub fn migrations_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations")
}
```

  `escargot` is an optional dependency, enabled by `test-support`. The `OnceLock` here is test apparatus, not the forbidden global `AppCore`.
- Modify: `tests/fixtures/acp.rs` (`adapter_at`'s and `fake_adapter()`'s `env!("CARGO_BIN_EXE_fake_acp")` become `shadows_core::testing::fake_acp_path()`), and every other `CARGO_BIN_EXE_fake_acp`: `acp_connection.rs`, `planner_mcp.rs`, `sessions.rs`. `serve_smoke.rs` and `planner_mcp.rs` call `shadows_core::testing::tree_probe_path()`, and `fixtures/probe.rs` is deleted. `tests/planner_mcp.rs`'s `include_str!("../src/planner/prompt.txt")` becomes `shadows_core::testing::PROMPT`, and `project_directory.rs`'s migrations path becomes `shadows_core::testing::migrations_dir()`.
- Modify: `crates/shadows/Cargo.toml` drops the `fake_acp` `[[bin]]`; the tests that stay in `crates/shadows` depend on `shadows-core` with `test-support` through `shadows`' own feature above.
- Modify: `docs/codebase/README.md`. Each moved module's row takes its `crates/shadows-core/src/…` path, `mcp/grant.rs`'s becomes `crates/shadows-core/src/grant/`, `crates/shadows/src/bin/`'s row goes, and `crates/fake-acp/src/main.rs` gets one.
- Move: the tests that compile against `shadows-core` alone, meaning no router, no `protocol`, no `mcp` service, no `config`, into `crates/shadows-core/tests/`, with the fixtures they include. These are the storage, workflow, thread, planner-session and recovery tests, plus `acp_connection.rs`. A test that includes `fixtures/app.rs`, `listening.rs` or `serve.rs` stays in `crates/shadows/tests/`. A fixture both sides include is copied, and the report names each copy. Copies are allowed because Task 10 folds shared helpers into `shadows_core::testing`.
- Modify: `crates/shadows/src/lib.rs` drops the moved modules. `crates/shadows/src/**` imports `shadows_core::…` in place of `crate::…`.

**Interfaces:**
- Produces: `shadows_core::{project, thread, workflow, planner, runtime, operation, events, command, storage, grant, error, id}`, all `pub`, with the same items as today, and `shadows_core::testing` under `test-support`.

- [ ] **Step 1:** Create `shadows-core` and move the modules. Run `cargo build -p shadows-core`.
- [ ] **Step 2:** Create `fake-acp`, and prove that escargot finds it with one moved test: `cargo test -p shadows-core --test sessions`.
- [ ] **Step 3:** Move the core-only tests, fix the fixtures' paths, and run the gate. The count is unchanged, 304.
- [ ] **Step 4:** Commit. It may be two commits, the crate and then the tests. `build: shadows-core holds the application's modules; fake-acp is its own binary (§14.3)`

---

### Task 4: `shadows-http` and `shadows-mcp`

**Files:**
- Create: `crates/shadows-http/`, holding `crates/shadows/src/protocol/*`, with `mod.rs` → `lib.rs`. It depends on `shadows-core`, `shadows-agent` for the types routes serialize today, `axum`, `tower-http`, `utoipa`, `utoipa-axum`, `serde` and `serde_json`. **It does not depend on `shadows-mcp`.**
- Create: `crates/shadows-mcp/`, holding `crates/shadows/src/mcp/*` except `grant.rs`, which moved in Task 3, with `mod.rs` → `lib.rs`. It depends on `shadows-core`, `rmcp`, `schemars` and `axum`. The `rmcp` client features and `reqwest` move to the test side (`test-support` or `[dev-dependencies]` of the crate whose tests use the MCP client).
- Move, first: `UiSignal`, from `protocol/ui_signal.rs` into `crates/shadows-core/src/events/mod.rs` (today's `events` module), unchanged, with its doc comment. `mcp/mod.rs` and `mcp/tools.rs` import `crate::protocol::UiSignal` today, so without this `shadows-mcp` would need `shadows-http`. `ui_signal.rs` is deleted, and its README row goes to the `events/` row.
- Modify: `shadows_http::router` takes the MCP router and mounts it at today's layer, so the order is unchanged (§14.5):

```rust
pub fn router(state: AppState, mcp: axum::Router) -> Router {
    // … as today, with `.merge(crate::mcp::service(…))` replaced by `.merge(mcp)`,
    // at the same position: after `rejections_as_error_bodies`, before the guard.
}
```

  `shadows_mcp::service` keeps its signature for now, `McpState` included, and the binary builds it from the same `Arc`s it builds `AppState` from.
- Modify: `crates/shadows/src/cli/mod.rs`, which serves `shadows_http::router(state, shadows_mcp::service(mcp_state))`. `crates/shadows/src/lib.rs` keeps `cli` and `tracing`, and `config.rs` stays in the binary.
- Modify: every test in `crates/shadows/tests/` and its fixtures (`app.rs`, `listening.rs`), for the new paths and the router's second argument. Tests stay in `crates/shadows/tests/`: they drive HTTP and MCP together, and this crate is the one that sees both. `openapi.rs` stays too.

- [ ] **Step 1:** Move `UiSignal` into `shadows_core::events`. Move `protocol/` into `shadows-http`. The router takes `mcp: Router`.
- [ ] **Step 2:** Move `mcp/` into `shadows-mcp`.
- [ ] **Step 3:** Update the binary and the tests, run the gate, and check that `api/openapi.json` is unchanged.
- [ ] **Step 4:** Check the direction with `cargo tree -p shadows-http --depth 1` and `cargo tree -p shadows-mcp --depth 1`. Neither names the other, and `shadows-core` names neither.
- [ ] **Step 5:** Commit: `build: the HTTP and MCP adapters are their own crates (§14.3, §14.5)`

---

### Task 5: `AppCore`, `CoreError`, the contracts test, and `Plans`

This task starts the services, so it also lays the frame the later service tasks use.

**Files:**
- Create: `crates/shadows-core/src/app.rs`, holding `AppCore`, `CoreParts`, the accessors, `start`, `assemble`, `shut_down` and `user_command`.
- Modify: `crates/shadows-core/src/error.rs`, which moved in Task 3. `CoreError` joins `ErrorCode`, which is already there.
- Create: `crates/shadows-core/src/plans/` with `mod.rs` (the `Plans` service), `model.rs`, `rules.rs` and `store.rs`.
  - `git mv` puts `workflow/mod.rs` → `plans/model.rs`, `workflow/check.rs` → `plans/rules.rs`, `workflow/ops.rs` → `plans/ops.rs` and `workflow/conversation.rs` → `plans/conversation.rs`.
  - `storage/sqlite/{workflow,workflow_draft,workflow_read,task,plan_view}.rs` → `plans/store/{edit,draft,read,task,view}.rs`, each still `impl Storage { … }` over the shared `Storage` (§14.4: the queries live with their service, and the pool stays in `db`).
- Create: `crates/shadows-core/src/plans/contract.yaml`, and `crates/shadows-core/tests/contracts.rs`.
- Modify: `crates/shadows-core/src/lib.rs`:
  - add `pub mod app;` and `pub mod plans;`, and re-export `AppCore`, `CoreParts`, `CoreError`, `UiSignal`, `StopKind`, and every plan type that appears in a public signature or a public field, because an adapter or a test names it or serializes it. Today that is all of `workflow/mod.rs`'s types, `workflow/conversation.rs`'s and `PlanOp`: `WorkflowId`, `TaskId`, `WorkflowState`, `LinkKind`, `AcceptanceItem`, `TaskContent`, `Link`, `PlanContent`, `PlanTask`, `LastEdit`, `Plan`, `EditOutcome`, `DraftStarted`, `Approved`, `PlanListing`, `PlanOp`, `Problem` (inside `StorageError::PlanInvalid`), `Focus` (in `shadows-http`'s `StartTurn`), `Place` and `PlanShown`. A type left out is unnameable outside the crate, which Task 10's `unnameable_types` refuses;
  - `workflow` stops being a public module;
  - the pure rules `apply`, `Applied`, `edit_problems` and `approval_problems` are not re-exported. `tests/workflow_rules.rs` (18 tests, `use shadows::workflow::*`) reaches them through `shadows_core::testing`, which re-exports them under `test-support`, and the other tests that named `shadows_core::workflow::…` import the root re-exports.
- Move: `harness_version` and `VERSION_BOUND` from `crates/shadows/src/cli/mod.rs`, and `adapter_version` from `crates/shadows/src/config.rs`, into `app.rs`, unchanged, with the `harness.version_timeout` and `harness.versions` log lines. `start` reads the versions, and the binary's `config.rs` keeps argument parsing only. Task 8 may move them into `harness/`.
- Modify:
  - `shadows-http`: `AppState` is `{ core: Arc<AppCore>, allowed_origins, shutdown }`, and `workflow.rs` routes are one call each;
  - `shadows-mcp`: `McpState` is replaced by `Arc<AppCore>`, `service(core: Arc<AppCore>)`, and every plan tool is one call;
  - `crates/shadows/src/cli/mod.rs`: bind, then `AppCore::start(&config, mcp_url)`, then serve;
  - `shadows-http/src/failure.rs`: `impl From<CoreError> for Failure`;
  - `shadows-mcp/src/refusal.rs`: `impl From<CoreError> for Refusal`.
- Modify: the fixtures `crates/shadows/tests/fixtures/app.rs` and `listening.rs`, and the tests that build `AppState` themselves, so that each builds `AppCore::assemble(CoreParts { … })` from the `Arc`s it holds.
- Test: `crates/shadows/tests/core.rs`, `crates/shadows/tests/core_plans.rs`.

**Interfaces:**
- Produces:

```rust
// crates/shadows-core/src/app.rs
pub type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

/// What `AppCore` is assembled from. Tests build it so the fixture keeps its
/// own handles on the same `Arc`s; production builds it in `start`. Its fields
/// are public by design: it is the one way in, and only construction uses it.
pub struct CoreParts {
    pub storage: Arc<Storage>,
    pub runtime: Arc<Runtime>,
    pub sessions: Arc<Sessions>,
    pub handles: Arc<LiveHandles>,
    pub bus: Bus,
    pub ui: tokio::sync::broadcast::Sender<UiSignal>,
    /// `http://<bound address>/mcp`: what a grant's `claude mcp add` names.
    pub mcp_url: String,
}

pub struct AppCore {
    plans: Plans,
    // … each later task adds its service here.
    // Held while adapters still reach them (Tasks 5–9); each goes when its
    // last reader moves into a service, and Task 10 removes the last.
    storage: Arc<Storage>, runtime: Arc<Runtime>, sessions: Arc<Sessions>,
    handles: Arc<LiveHandles>, bus: Bus, ui: tokio::sync::broadcast::Sender<UiSignal>, mcp_url: String,
}

/// The `CommandContext` a person's command carries: principal `User`, id
/// `local`, schema version 1, the fingerprint of `params`. The same body as
/// `shadows_http::project::ctx` today, so no fingerprint moves.
pub(crate) fn user_command(command_id: String, kind: &str, params: serde_json::Value) -> CommandContext;

impl AppCore {
    /// Opens storage, starts the runtime (recovery), revokes every thread
    /// grant, reads the harness versions, and builds the sessions on
    /// `mcp_url`. The caller binds first: `mcp_url` names the bound address,
    /// and a daemon that cannot bind must not run recovery (§14.4).
    pub async fn start(config: &StartConfig, mcp_url: String) -> anyhow::Result<Arc<AppCore>>;
    pub fn assemble(parts: CoreParts) -> Arc<AppCore>;
    pub fn plans(&self) -> &Plans;
    /// §8.5 through `planner::shut_down(runtime, handles, sessions, bound, second_signal)`, unchanged.
    pub async fn shut_down(&self, bound: Duration, second_signal: impl Future<Output = ()>)
        -> Result<StopKind, CoreError>;
}
/// What `start` needs from the binary's `Config`: the database path, node,
/// adapter and harness paths. `Config` stays in the binary, which owns argument
/// parsing, and builds this.
pub struct StartConfig { pub db_path: PathBuf, pub node_path: PathBuf, pub adapter_path: PathBuf, pub harness_path: PathBuf }
```

`planner::shut_down` returns `Result<StopKind, StorageError>`. `StopKind` is `storage::StopKind`, re-exported from `lib.rs`.

- **Transition getters.** Until each service moves, the routes and tools that have not moved read through `pub fn` getters on `AppCore`: `storage()`, `sessions()`, `handles()`, `runtime()`, `bus()`, `ui_bus()` and `mcp_url()`. They are `#[doc(hidden)]`, each with the comment `// transitional: removed by Task N`, naming the task that moves its last reader. Task 10 deletes the last one, and then the compiler refuses any adapter still reaching past a service.

```rust
// crates/shadows-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)] Storage(#[from] StorageError),
    #[error(transparent)] Start(#[from] StartError),
    #[error(transparent)] Directory(#[from] DirectoryError),
    #[error("{0}")] ProjectDirectoryUnusable(String),
    #[error("the harness could not start: {0}")] HarnessStartFailed(String),
    #[error("the {0} harness is not available yet")] HarnessUnavailable(String),
    #[error("{what} {id} is not offered")] SettingNotOffered { what: String, id: String, detail: Option<String> },
    #[error("this project does not allow the {0} mode")] ModeNotAllowed(String),
    #[error("the daemon is stopping")] RuntimeStopping,
    #[error("the turn's process tree could not be terminated")] TerminationFailed,
    /// A refusal whose code and exact text a service writes, as the MCP tools do today.
    #[error("{message}")] Refused { code: ErrorCode, message: String },
}
impl From<OpenError> for CoreError { /* Storage→Storage, Start(r)→HarnessStartFailed(r), Workspace(r)→ProjectDirectoryUnusable(r) */ }
```

```rust
// crates/shadows-core/src/plans/mod.rs
impl Plans {
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanListing>, CoreError>;
    pub async fn get(&self, workflow: &WorkflowId) -> Result<Plan, CoreError>;
    /// "PlanApprove", params { "workflow", "expected_revision" }.
    pub async fn approve(&self, command_id: String, workflow: &WorkflowId, expected_revision: i64)
        -> Result<Approved, CoreError>;
    pub async fn list_for(&self, grant: &Grant) -> Result<Vec<PlanListing>, CoreError>;
    pub async fn get_for(&self, grant: &Grant, named: Option<&WorkflowId>) -> Result<Plan, CoreError>;
    pub async fn task_for(&self, grant: &Grant, named: Option<&WorkflowId>, number: u32)
        -> Result<PlanTask, CoreError>;
    pub async fn prepare_draft(&self, grant: &Grant) -> Result<String, CoreError>;
    pub async fn start_draft(&self, grant: &Grant, args: DraftStart) -> Result<DraftStarted, CoreError>;
    pub async fn edit(&self, grant: &Grant, args: PlanEdit) -> Result<EditOutcome, CoreError>;
    /// Records the showing, then sends the UI signal unless it was a replay.
    pub async fn show(&self, grant: &Grant, args: PlanShow) -> Result<PlanShown, CoreError>;
}
pub struct DraftStart { pub title: Option<String>, pub goal: Option<String>,
                        pub from_workflow_id: Option<WorkflowId>, pub draft_ref: Option<String> }
pub struct PlanEdit { pub workflow_id: Option<WorkflowId>, pub expected_revision: i64,
                      pub ops: Vec<PlanOp>, pub command_id: Option<String> }
pub struct PlanShow { pub workflow_id: Option<WorkflowId>, pub task_number: Option<u32>, pub place: Place }
```

**How the plan logic moves:**
- `plan_in_scope`, `planner_draft`, `external_draft`, `edit`, `show`, `own_thread`, `writer_of` and `command` move from `shadows-mcp/src/tools.rs` into `Plans`, unchanged in logic and text.
- The scope flag stays private: `edit` calls `in_scope(grant, named, true)`, and `get_for` and `task_for` call it with `false`.
- `Refusal::scope(msg)` becomes `CoreError::Refused { code: ErrorCode::GrantScope, message }`, and `Refusal::new(code, msg)` becomes `CoreError::Refused { code, message }`.
- A `StorageError` a tool returns today stays a `StorageError`, carried in `CoreError::Storage`, so `Refusal`'s existing `From<StorageError>` still writes its text.
- **The fingerprints that must not move:**
  - `"PlanApprove"` { workflow, expected_revision };
  - `"DraftStart"`: { thread, title, goal } for the Planner, anchored to the running turn; { title, goal, from_workflow_id } for an external agent, anchored to the `draft_ref`;
  - `"PlanEdit"` { workflow, expected_revision, ops };
  - `"PlanShow"` { workflow, task_number, place }.

  Copy the `json!` literals; do not retype them.

**How `CoreError` maps in each adapter:**
- `Failure`: `Storage`, `Start` and `Directory` go through the existing `From<StorageError>`, `From<StartError>` and `From<DirectoryError>` for `Failure`, so their statuses, codes and texts do not move. Every other variant calls today's constructor (`project_directory_unusable`, `harness_start_failed`, `harness_unavailable`, `setting_not_offered(&what, &id, detail.as_deref())`, `mode_not_allowed`, `runtime_stopping`, `termination_failed`). `Refused` gets a new `Failure::refused(code, message)`, status 422; no HTTP route produces it in this milestone.
- `Refusal`: `Storage` goes through the existing `From`, `Refused` → `Refusal::new(code, message)`, and every other variant → `Refusal::new(<its code>, e.to_string())`.

- [ ] **Step 1: Pin the behaviour.** Write the two test files below. They pass against the code before this task, and they pin it across the move.

```rust
// crates/shadows/tests/core_plans.rs
//! Spec §14.9: plan operations answer the same after moving into `Plans`.
#[path = "fixtures/acp.rs"] mod acp;
#[path = "fixtures/app.rs"] mod app;
#[path = "fixtures/listening.rs"] mod listening;
#[path = "fixtures/plan.rs"] mod plan;

use app::other_project;
use listening::{listening_app, ok, project_client, refused};
use plan::{add, draft_on};
use serde_json::json;

#[tokio::test]
async fn plan_commands_replay_after_the_move() {
    let l = listening_app().await;
    let (_grant, client) = project_client(&l).await;
    let r = ok(&client, "draft_prepare", json!({})).await;
    let args = json!({ "title": "T", "goal": "G", "draft_ref": r["draft_ref"] });
    let first = ok(&client, "draft_start", args.clone()).await;
    let again = ok(&client, "draft_start", args).await;
    assert_eq!(first["workflow_id"], again["workflow_id"]);
    let edit = json!({ "workflow_id": first["workflow_id"], "expected_revision": 0,
        "ops": [add(1)], "command_id": "e1" });
    let e1 = ok(&client, "plan_edit", edit.clone()).await;
    let e2 = ok(&client, "plan_edit", edit).await;
    assert_eq!(e1, e2);
    assert_eq!(e1["revision"], 1, "the replay answered the first edit, not a second one");
}

#[tokio::test]
async fn a_project_grant_is_refused_another_projects_plan_after_the_move() {
    let l = listening_app().await;
    let (_g, client) = project_client(&l).await;
    let (_, their_thread) = other_project(&l.app).await;
    let other = draft_on(&l.app, &their_thread, "their-start").await.workflow_id;
    let text = refused(&client, "workflow_get", json!({ "workflow_id": other })).await;
    assert!(text.starts_with("GRANT_SCOPE: that plan is not in this grant's project"), "{text}");
}
```

```rust
// crates/shadows/tests/core.rs
//! Spec §14.4: the application is one `AppCore`; adapters and shutdown reach it through that.
#[path = "fixtures/acp.rs"] mod acp;
#[path = "fixtures/app.rs"] mod app;

use app::{call, default_settings, names, start_on, test_app, wait_terminal};

#[tokio::test]
async fn the_router_is_built_from_the_core() {
    let app = test_app().await;
    let (status, list) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200);
    assert_eq!(names(&list), vec!["Demo".to_string()]);
}

#[tokio::test]
async fn shut_down_through_the_core_cancels_and_records() {
    let app = test_app().await;
    let op = start_on(&app, app.thread.as_str(), "hang", default_settings()).await;
    let kind = app.core.shut_down(std::time::Duration::from_secs(10), std::future::pending()).await.unwrap();
    assert_eq!(kind, shadows_core::StopKind::Graceful);
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
}
```

Match the `use` lines to what the fixtures need; `listening.rs` names `super::acp`, `super::app` and `super::plan`. Read `tests/mcp_tools.rs` first. `core.rs` needs `App.core`, which Step 3 adds; until then it is expected not to compile.

- [ ] **Step 2: Add the contracts test.** The template is already in the repo at `docs/codebase/contracts/TEMPLATE.yaml`. `crates/shadows-core/tests/contracts.rs`:
  - finds every `crates/shadows-core/src/*/contract.yaml`;
  - parses it with `yaml_rust2::YamlLoader::load_from_str`;
  - collects the source's declared symbols and the service's public methods with `syn`, and test names from every `#[test]` / `#[tokio::test]` under `crates/*/tests` and `crates/*/src`;
  - fails on §14.7's six rules.

  `syn` (with `full`) and `yaml-rust2` join `shadows-core`'s `[dev-dependencies]`. Check `yaml-rust2` 0.13's API (`YamlLoader::load_from_str`, `Yaml::Hash`, `as_vec`) before relying on it, and adjust to it. A contract nests the template's top-level `reads:` / `writes:` / `checks:` groups under one `functions:` key, as §14.7 says; the test reads only `functions`. Its code is the one below, with the paths changed.

```rust
//! Spec §14.7: a service contract that no longer matches its code fails here.
//! It cannot check that a rule's prose is true; the reviewer owns that.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use yaml_rust2::{Yaml, YamlLoader};

fn workspace() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..") }

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for e in read { let p = e.unwrap().path();
        if p.is_dir() { rs_files(&p, out) } else if p.extension().is_some_and(|x| x == "rs") { out.push(p) } }
}

fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| { let p = a.path();
        p.is_ident("test") || (p.segments.len() == 2 && p.segments[1].ident == "test") })
}

fn collect_tests(items: &[syn::Item], out: &mut BTreeSet<String>) {
    for item in items { match item {
        syn::Item::Fn(f) if is_test(&f.attrs) => { out.insert(f.sig.ident.to_string()); }
        syn::Item::Mod(m) => if let Some((_, inner)) = &m.content { collect_tests(inner, out) },
        _ => {} } }
}

fn all_tests() -> BTreeSet<String> {
    let mut files = Vec::new();
    for c in std::fs::read_dir(workspace().join("crates")).unwrap() {
        let c = c.unwrap().path(); rs_files(&c.join("tests"), &mut files); rs_files(&c.join("src"), &mut files); }
    let mut out = BTreeSet::new();
    for f in files { if let Ok(file) = syn::parse_file(&std::fs::read_to_string(&f).unwrap()) { collect_tests(&file.items, &mut out) } }
    out
}

fn symbols(dir: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut all, mut public) = (BTreeSet::new(), BTreeSet::new());
    let mut files = Vec::new(); rs_files(dir, &mut files);
    for f in files {
        let Ok(file) = syn::parse_file(&std::fs::read_to_string(&f).unwrap()) else { continue };
        for item in &file.items { match item {
            syn::Item::Fn(x) => { all.insert(x.sig.ident.to_string()); }
            syn::Item::Struct(x) => { all.insert(x.ident.to_string()); }
            syn::Item::Enum(x) => { all.insert(x.ident.to_string()); }
            syn::Item::Const(x) => { all.insert(x.ident.to_string()); }
            syn::Item::Type(x) => { all.insert(x.ident.to_string()); }
            syn::Item::Impl(i) => for it in &i.items { if let syn::ImplItem::Fn(m) = it {
                all.insert(m.sig.ident.to_string());
                // The service's own `mod.rs` only: `Path::ends_with` would also
                // take `store/mod.rs`, whose `impl Storage` methods are not the service's.
                if matches!(m.vis, syn::Visibility::Public(_)) && i.trait_.is_none() && f == dir.join("mod.rs") {
                    public.insert(m.sig.ident.to_string()); } } },
            _ => {} } }
    }
    (all, public)
}

/// Rule 9: the `pub(crate)` free functions of a service's `store` (`store.rs` or `store/`),
/// which are exactly the ones another service may call inside one write (§14.6).
fn shared(dir: &Path) -> BTreeSet<String> {
    let mut files = Vec::new(); rs_files(&dir.join("store"), &mut files);
    if dir.join("store.rs").exists() { files.push(dir.join("store.rs")) }
    let mut out = BTreeSet::new();
    for f in files {
        let Ok(file) = syn::parse_file(&std::fs::read_to_string(&f).unwrap()) else { continue };
        for item in &file.items { if let syn::Item::Fn(x) = item {
            if let syn::Visibility::Restricted(r) = &x.vis { if r.in_token.is_none() && r.path.is_ident("crate") {
                out.insert(x.sig.ident.to_string()); } } } }
    }
    out
}

fn listed(y: &Yaml, out: &mut BTreeSet<String>) {
    if let Yaml::Hash(h) = y { for (k, v) in h {
        let named = matches!(v, Yaml::String(_))
            || matches!(v, Yaml::Hash(inner) if inner.contains_key(&Yaml::String("signature".into())));
        if named { if let Yaml::String(k) = k { out.insert(k.clone()); } } else { listed(v, out) } } }
}

#[test]
fn every_contract_matches_its_service() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let tests = all_tests();
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(&src).unwrap() {
        let dir = entry.unwrap().path();
        let path = dir.join("contract.yaml");
        if !path.exists() { continue; }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let docs = match YamlLoader::load_from_str(&std::fs::read_to_string(&path).unwrap()) {
            Ok(d) => d, Err(e) => { problems.push(format!("{name}: not valid YAML: {e}")); continue } };
        let doc = &docs[0];
        let Some(source) = doc["source"].as_str() else { problems.push(format!("{name}: no source")); continue };
        let source_dir = workspace().join(source);
        if !source_dir.exists() { problems.push(format!("{name}: source {source} does not exist")); continue }
        let (declared, public) = symbols(&source_dir);
        let mut fns = BTreeSet::new(); listed(&doc["functions"], &mut fns);
        for f in &fns { if !declared.contains(f) { problems.push(format!("{name}: `{f}` is not in {source}")) } }
        let mut gap_at = BTreeSet::new();
        if let Some(gaps) = doc["gaps"].as_vec() { for g in gaps { if let Some(at) = g["at"].as_str() {
            gap_at.insert(at.to_string());
            if !declared.contains(at) { problems.push(format!("{name}: gap at `{at}` is not in {source}")) } } } }
        // Rule 7: a shape is a type this service declares.
        if let Yaml::Hash(h) = &doc["shapes"] { for k in h.keys() { if let Some(k) = k.as_str() {
            if !declared.contains(k) { problems.push(format!("{name}: shape `{k}` is not in {source}")) } } } }
        // Rules 7 and 8: an agreement names real symbols, and a broken one is also a gap.
        if let Some(ags) = doc["agreements"].as_vec() { for a in ags {
            let between: Vec<&str> = a["between"].as_vec().map(|v| v.iter().filter_map(Yaml::as_str).collect()).unwrap_or_default();
            if between.len() < 2 { problems.push(format!("{name}: an agreement names fewer than two paths")) }
            // A path in another service is written `harness::change_model` and looked up in that folder.
            for s in &between { let found = match s.split_once("::") {
                    Some((svc, sym)) => symbols(&src.join(svc)).0.contains(sym),
                    None => declared.contains(*s) };
                if !found { problems.push(format!("{name}: agreement path `{s}` does not exist")) } }
            if a["holds"].as_bool() == Some(false) && !between.iter().any(|s| gap_at.contains(*s)) {
                problems.push(format!("{name}: agreement {between:?} does not hold and no gap names it")) } } }
        let mut named = Vec::new();
        // The template's `agreements` carry `tested_by` too; rule 4 covers every one.
        for section in ["obligations", "agreements"] {
        if let Some(obs) = doc[section].as_vec() { for o in obs {
            match &o["tested_by"] { Yaml::String(s) => named.push(s.clone()),
                Yaml::Array(a) => named.extend(a.iter().filter_map(|t| t.as_str().map(str::to_string))), _ => {} }
            let t = o["tested_by"].as_str().unwrap_or("");
            if (t == "none" || t == "unknown") && o["note"].as_str().is_none() {
                problems.push(format!("{name}: an entry of {section} says tested_by: {t} without a note")) } } } }
        if let Some(ts) = doc["tests"].as_vec() { for t in ts { if let Some(n) = t["name"].as_str() { named.push(n.to_string()) } } }
        for n in named { let bare = n.trim_start_matches("integration: ").to_string();
            if bare != "none" && bare != "unknown" && !tests.contains(&bare) { problems.push(format!("{name}: test `{bare}` does not exist")) } }
        for m in &public { if !fns.contains(m) { problems.push(format!("{name}: public method `{m}` has no entry in functions")) } }
        // Rule 9: what a store shares inside one write is declared, and only that.
        let actual = shared(&source_dir);
        let mut declared_shared = BTreeSet::new(); listed(&doc["shared_in_transaction"], &mut declared_shared);
        for s in actual.difference(&declared_shared) { problems.push(format!("{name}: store function `{s}` is pub(crate) but not in shared_in_transaction")) }
        for s in declared_shared.difference(&actual) { problems.push(format!("{name}: shared_in_transaction names `{s}`, which is not a pub(crate) store function")) }
    }
    assert!(problems.is_empty(), "contracts out of date (spec §14.7):\n{}", problems.join("\n"));
}
```

  `source` in a contract is the service's folder relative to the workspace, for example `crates/shadows-core/src/plans`. The public methods counted are those in `impl` blocks of the folder's `mod.rs`. Prove the test bites: write a scratch `crates/shadows-core/src/scratch/contract.yaml` with `source: crates/shadows-core/src/plans` and `functions: { nope: "() -> ()" }`, see it fail naming `nope`. Then, in the same scratch file, replace `functions` with `agreements: [{ between: [nope, also_nope], holds: false, tested_by: none, note: x }]` and see it fail three ways: two missing paths, and no gap. Then delete it.
- [ ] **Step 3: Build `AppCore`, `CoreError` and `Plans`, and move plans' files into `plans/`** (see Files). Point the routes and tools at `core.plans()`:

```rust
// shadows-http/src/workflow.rs
pub(super) async fn approve_plan(State(s): State<AppState>, Path(workflow): Path<WorkflowId>,
    Json(body): Json<ApprovePlan>) -> Result<Json<Approved>, Failure> {
    Ok(Json(s.core.plans().approve(body.command_id, &workflow, body.expected_revision).await?))
}
// shadows-mcp/src/tools.rs
async fn workflow_get(&self, Extension(grant): Extension<Grant>,
    Parameters(args): Parameters<PlanArgs>) -> CallToolResult {
    answer(self.core.plans().get_for(&grant, args.workflow_id.as_ref()).await.map_err(Refusal::from))
}
```

  The binary keeps signal handling and the order: `core.shut_down(CONFIRMATION_BOUND, second_signal)`, then `stopping.send_replace(true)`, with every log line kept.
- [ ] **Step 4: Make plans' internals private.**
  - In `shadows-core/src/lib.rs`, `plans`' submodules `model`, `rules`, `ops`, `conversation` and `store` are private.
  - `lib.rs` re-exports only the types adapters serialize, plus `Plans` and its argument structs.
  - Build the workspace. An adapter that still names a plan internal fails to compile, and that is the check. Fix the adapter; never widen the visibility.
- [ ] **Step 5: Write `plans/contract.yaml`.** Its obligations:
  - grant scope, with `tested_by` the scope tests in `plan_grants.rs`;
  - replay: `plan_commands_replay_after_the_move`;
  - the UI signal sent only when not replayed, with `tested_by` the `plan_show` test in `plan_in_conversation.rs`.

  Its `agreements`: `between: [get, get_for]` — for a grant whose scope covers the plan, both return the same `Plan` — and `between: [get_for, task_for]` — a task read through `task_for` is the task inside `get_for`'s plan. Name a test whose body compares them, or `tested_by: none` with a `note` naming the test that would.

  `not_the_caller's`: "check a grant's scope before calling a `*_for` method".
- [ ] **Step 6: Run the gate.** Expect 309 Rust: 304, plus 2 in `core_plans`, plus 2 in `core`, plus 1 contracts test. Report the real number. `api/openapi.json` is unchanged.
- [ ] **Step 7: Commit.** `feat(core): AppCore, CoreError and Plans; plans live in one folder (§14.4)`

---

### Task 6: `Grants`

**Files:**
- Create: `crates/shadows-core/src/grants/`. `git mv grant/mod.rs` → `grants/model.rs`, and `storage/sqlite/grant.rs` → `grants/store.rs`. Add `mod.rs` with the service and `contract.yaml`.
- Modify: `shadows-http/src/grants.rs` and `shadows-mcp/src/auth.rs`, one call each. `AppCore::start` calls `grants().revoke_thread_grants()`, keeping the `recovery.thread_grants_revoked` log line. The `mcp_url()` getter is deleted.
- Test: extend `crates/shadows/tests/core.rs`.

**Interfaces:**

```rust
impl Grants {
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<Grant>, CoreError>;
    /// "McpGrantIssue", params { "project" }. `command` is the
    /// `claude mcp add --transport http shadows <mcp_url> --header "Authorization: Bearer <token>"`
    /// line, built from the core's mcp_url — present only when the token is.
    pub async fn issue(&self, command_id: String, project: &ProjectId) -> Result<IssuedView, CoreError>;
    /// "McpGrantRevoke", params { "grant" }.
    pub async fn revoke(&self, command_id: String, grant: &GrantId) -> Result<Grant, CoreError>;
    /// The bearer check: `Ok(None)` for an unknown or revoked token.
    pub async fn authorize(&self, token: &str) -> Result<Option<Grant>, CoreError>;
    /// Startup (§13.7): every thread grant of an earlier daemon.
    pub async fn revoke_thread_grants(&self) -> Result<u64, CoreError>;
}
pub struct IssuedView { pub grant: Grant, pub token: Option<String>, pub command: Option<String> }
```

`IssuedGrantBody` in `shadows-http` keeps its name and fields, and is built from `IssuedView`. `require_grant` keeps its three answers (401, 401, 503) and its log lines.

- [ ] **Step 1: Pin.** `revoked_grant_is_refused_after_the_move`:
  - issue a grant through the HTTP route, and take the token;
  - `listening::mcp_client(&l.base, &token)` → `workflow_list` succeeds;
  - revoke it through the `DELETE` route, with `command_id` in the query;
  - a raw `POST {base}/mcp` with that bearer answers 401. Follow `mcp_server.rs::a_revoked_token_is_401`. `mcp_client` panics on a failed connection, so it cannot be used for this step.

  It passes before the move.
- [ ] **Step 2: Move and narrow.** `grants`' submodules become private. `Grant`, `GrantId`, `GrantKind` and `IssuedView` are re-exported, because adapters and `fake-acp` name them. `hash_token` is re-exported only under `test-support`, for `fake-acp`.
- [ ] **Step 3: Write `grants/contract.yaml`, and run the gate.** Its obligations:
  - only the token's hash is stored;
  - a replayed issue returns no token;
  - thread grants die at startup.
- [ ] **Step 4: Commit.** `refactor(core): Grants owns issue, revoke and the bearer check (§14.4)`

---

### Task 7: `Turns` (the risk task)

**Files:**
- Create: `crates/shadows-core/src/turns/`.
  - `git mv` puts `planner/{spawn,turn,entries,handles,shutdown}.rs` and `operation/mod.rs` → `turns/`;
  - `storage/sqlite/{turn,operation,operation_read,transition}.rs` → `turns/store/`.
  - Add `mod.rs` with the service and `contract.yaml`.
- Modify: `shadows-http/src/conversation.rs`. `start_turn` and `stop_turn` each become `detached(one call)`. `start`, `Turn`, `record`, `offer_for_model` and the `focus_block` call move into `Turns::send`. The list routes stay until Task 9.
- Test: `crates/shadows/tests/core_turns.rs`.

**Interfaces:**

```rust
pub struct SendTurn {
    pub command_id: String, pub prompt: String, pub model: String, pub mode: String,
    pub effort: Option<String>, pub focus: Option<Focus>, pub client_tab: Option<String>,
}
impl Turns {
    /// §12.3, §13.9 — in this exact order, as today:
    /// 1. build the "turn.start" command (params thread_id, prompt, model, mode,
    ///    effort, and focus when given), and answer a replay;
    /// 2. refuse when the registry is closed (RuntimeStopping);
    /// 3. read turn_context, and refuse an unavailable harness;
    /// 4. **refuse THREAD_BUSY before the session is touched**;
    /// 5. open the session, and lease its events (Busy → THREAD_BUSY,
    ///    Closed → HARNESS_START_FAILED);
    /// 6. set the model if needed, then refuse an unoffered setting or a mode
    ///    the project does not allow;
    /// 7. record the turn (TransitionConflict while the registry is closed →
    ///    RuntimeStopping), giving the events back on a replay or a failure;
    /// 8. start the PlannerTurn with the bus.
    pub async fn send(&self, thread: ThreadId, turn: SendTurn) -> Result<OperationId, CoreError>;
    /// PlannerTurn::stop as the local user; TerminationFailed → CoreError::TerminationFailed;
    /// then the operation as recorded.
    pub async fn stop(&self, op: &OperationId) -> Result<Operation, CoreError>;
}
```

The route clones the core into the detached task:

```rust
let core = s.core.clone();
detached(async move { core.turns().send(thread_id, turn).await.map_err(Failure::from) })
```

- [ ] **Step 1: Pin.** These pass before the move:
  - `turn_start_replays_after_the_move`: the same `command_id` twice gives one operation id, 202 both times;
  - `stop_after_the_move_records_cancelled`: start `hang`, then `POST /api/operations/{id}/stop`, and the status is `Cancelled`;
  - `a_second_start_while_busy_is_thread_busy`: start `hang`, and a second start with a new `command_id` gets 409 `THREAD_BUSY`.
- [ ] **Step 2: Move.** Read the old `start` and the new `send` side by side, statement by statement. Nothing reorders.
- [ ] **Step 3: Narrow.** `turns`' submodules become private. `OperationId`, `Operation`, `StopKind` and `SendTurn` are re-exported. Delete the getters no adapter reads.
- [ ] **Step 4: Gate.** These pass unchanged: `planner_turn`, `turn_command`, `operation_lifecycle`, `shutdown`, `disconnect`, `recovery` and `plan_in_conversation`.
- [ ] **Step 5: Write `turns/contract.yaml`.** Its obligations are §14.9's busy, session-before-lease, availability and replay rules, each with `tested_by`. `not_the_caller's`: "check `thread_is_busy`, or open or lease a session, before `send`: `send` does all three, in the only safe order".
- [ ] **Step 6: Commit.** `refactor(core): Turns owns starting and stopping a turn (§14.4, §14.9)`

---

### Task 8: `Harness`

**Files:**
- Create: `crates/shadows-core/src/harness/`. `git mv` puts `planner/{sessions,settings,setup,offers,context}.rs` and `prompt.txt` → `harness/`, and `storage/sqlite/harness.rs` → `harness/store.rs`. Update `testing::PROMPT`'s path.
- Move: `HarnessInfo`, `RememberedSettings`, `ContextBreakdown` and `label()` from `shadows-http/src/harness.rs` to `harness/model.rs`. Their names, fields and derives stay the same, so `api/openapi.json` is unchanged.
- Modify: `shadows-http/src/harness.rs`, one call each. `shadows-http/src/sse.rs`'s options frame calls `core.harness().choices(…)`.

**Interfaces:**

```rust
impl Harness {
    pub async fn list(&self) -> Result<Vec<HarnessInfo>, CoreError>;
    pub async fn open_session(&self, thread: &ThreadId) -> Result<SessionChoices, CoreError>;
    /// Early THREAD_BUSY before the session is touched (§14.9), then change_model;
    /// ModelRefused maps exactly as the route maps it today.
    pub async fn change_model(&self, thread: &ThreadId, model: &str) -> Result<SessionChoices, CoreError>;
    pub async fn context(&self, thread: &ThreadId) -> Result<ContextBreakdown, CoreError>;
    /// Also used by Events (Task 10) for the options frame.
    pub async fn choices(&self, thread: &ThreadId, offered: &Offered) -> Result<SessionChoices, CoreError>;
}
```

- [ ] **Step 1: Pin.** `change_model_is_refused_while_a_turn_runs`: start `hang`, then `PUT` the session model (read the path in `harness.rs`'s `#[utoipa::path]`) and get 409 `THREAD_BUSY`. Stopping the turn then gives `Cancelled`. It passes before the move.
- [ ] **Step 2: Move and narrow.**
  - If `sessions.rs` grows past 500 lines while moving, split it by responsibility inside `harness/`, and state each file's job.
  - Gate.
  - Write `harness/contract.yaml`: the busy rule shared with `Turns`, and the availability rule. The busy rule is an agreement: `between: [change_model, turns::send]`, both refuse while the thread has an open operation, with `tested_by` `change_model_is_refused_while_a_turn_runs` and the busy test of `send`.
- [ ] **Step 3: Commit.** `refactor(core): Harness owns the harness list, sessions and model changes (§14.4)`

---

### Task 9: `Projects`, `Threads`, `Instructions`

**Files:**
- Create:
  - `crates/shadows-core/src/projects/`, from `project/` and `storage/sqlite/project.rs`, plus the browse helpers from `shadows-http/src/fs.rs`, including its `blocking`;
  - `threads/`, from `thread/` and `storage/sqlite/{thread,entry,fork}.rs`;
  - `instructions/`, from `storage/sqlite/instructions.rs`.

  Each gets a `contract.yaml`.
- Modify: `shadows-http/src/{project,fs,thread,instructions}.rs`, and `conversation.rs`'s `list_entries` and `list_operations`, to one call each. `shadows_http::project::ctx` is deleted once its last caller is gone.

**Interfaces:**

```rust
impl Projects {
    pub async fn list(&self) -> Result<Vec<Project>, CoreError>;
    /// Resolves the directory before the fingerprint; "project.create", params { slug, name, directory }.
    pub async fn create(&self, command_id: String, slug: &str, name: &str, directory: &str) -> Result<Project, CoreError>;
    /// Validates every harness and mode against policy, dedups and sorts; "project.modes", params { project, allowed_modes }.
    pub async fn set_modes(&self, command_id: String, project: &ProjectId, allowed_modes: BTreeMap<String, Vec<String>>) -> Result<Project, CoreError>;
    pub async fn list_dirs(&self, path: Option<String>) -> Result<DirectoryListing, CoreError>;
    pub async fn create_dir(&self, parent: String, name: String) -> Result<DirectoryEntry, CoreError>;
}
impl Threads {
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanningThread>, CoreError>;
    /// Harness defaults to claude-code; unknown → SettingNotOffered("harness"); "thread.create", params { project, title, harness }.
    pub async fn create(&self, command_id: String, project: &ProjectId, title: &str, harness: Option<&str>) -> Result<PlanningThread, CoreError>;
    /// "thread.harness"; closes the session when the harness changed (logs, never fails, on close error).
    pub async fn set_harness(&self, command_id: String, thread: &ThreadId, harness: &str) -> Result<PlanningThread, CoreError>;
    /// "thread.fork", params { thread_id, at_entry_id }.
    pub async fn fork(&self, command_id: String, thread: &ThreadId, at: &ThreadEntryId) -> Result<PlanningThread, CoreError>;
    pub async fn entries(&self, thread: &ThreadId) -> Result<Vec<ThreadEntry>, CoreError>;
    pub async fn operations(&self, thread: &ThreadId) -> Result<Vec<Operation>, CoreError>;
}
impl Instructions {
    pub async fn current(&self, project: &ProjectId) -> Result<Option<InstructionsVersion>, CoreError>;
    /// "PlannerInstructionsSave", params { project, body }.
    pub async fn save(&self, command_id: String, project: &ProjectId, body: &str) -> Result<InstructionsVersion, CoreError>;
}
```

`known_harness` becomes one function in `threads/rules.rs`, which `Projects` also calls, so there is one rule in one place.

- [ ] **Step 1: Pin.** The existing tests cover these routes: `protocol`, `thread_contract`, `thread_routes`, `fork`, `fs_routes`, `project_directory`, `project_contract`, `planner_instructions` and `storage_contract`. Add `project_create_replays_after_the_move` to `core.rs`.
- [ ] **Step 2: Move, narrow, gate, and write the three contracts.**
- [ ] **Step 3: Commit.** `refactor(core): Projects, Threads and Instructions own their operations (§14.4)`

---

### Task 10: `Events`, and the boundary closes

**Files:**
- Create: `crates/shadows-core/src/events/`, holding today's `events/`, `UiSignal`, `storage/sqlite/events_read.rs` → `events/store.rs`, and the journal-then-live loop from `shadows-http/src/sse.rs`.
- Create: `crates/shadows-core/src/db/`, holding what is left of `storage/`:
  - `mod.rs`, which is the `Storage` pool and `write_txn`;
  - `command.rs`, for idempotency;
  - `journal.rs`, from `storage/sqlite/events.rs`: `append_event`, which every service's write calls inside its own transaction. §14.4 puts "the durable journal" in `db/`, and "Events emits nothing of its own": appending is the writer's act, reading and delivering is Events';
  - `storage::test_support`, which appends through `journal.rs`;
  - migrations, which run from here;
  - `runtime.rs`, which moves to `crates/shadows-core/src/runtime/` with the runtime.

  `db` and `runtime` are private modules.
- Modify: `shadows-http/src/sse.rs`. It keeps the framing: event names, JSON bodies, `lagged`, `fatal`, the bounded channel and the `sse.*` log lines. It turns each `Delivery` into today's frame.
- Modify: `crates/shadows-core/src/testing.rs`. It exposes what the core's own tests need and nothing more: `pub use crate::db::Storage;`, the fixtures' helpers, `fake_acp_path`, `tree_probe_path`, and Task 5's plan-rule re-exports. Fold in the helper copies Task 3 made.
- Modify: the root `Cargo.toml`, with `[workspace.lints.rust] unnameable_types = "warn"`. A public method that returns a type from a private module is then refused under `-D warnings`.
- Modify: `.github/workflows/ci.yml`, which adds `cargo clippy --workspace -- -D warnings` (no `--all-targets`).

**Interfaces:**

```rust
pub enum Delivery {
    Durable(StoredEvent),
    CaughtUp { seq: i64 },
    Delta { op: OperationId, text: String },
    TurnEnd { op: OperationId, subtype: &'static str, stop_reason: Option<String> },
    Usage { thread: ThreadId, used: u64, size: u64, limits: Option<AccountLimits> },
    Options { thread: ThreadId, choices: SessionChoices },
    PlanShow(UiSignal),
    Lagged,
    Fatal(String),
}
impl Events {
    /// Takes the committed-sequence watch, the bus, the options watch and the
    /// UI receiver before anything is read, as `subscribe` does today, then
    /// replay, CaughtUp, live (§2.4, §6.18).
    pub fn subscribe(&self, thread: ThreadId, after: i64) -> Subscription;
}
impl Subscription {
    /// The next frame, or `Err(why)` once over, `why` being the text `sse.closed` logs today.
    pub async fn next(&mut self) -> Result<Delivery, &'static str>;
}
```

**What must not change:**
- `shutdown` stays the first `biased` arm, in `sse.rs`:
  `select! { biased; _ = shutdown.wait_for(..) => …, d = sub.next() => … }`;
- `next` keeps the other arms in today's order: committed, bus, options, signals;
- the journal is re-read whenever the committed-sequence signal moves;
- the receivers are taken before the replay is read.

The journal type is `StoredEvent`, which `read_events_after` returns.

- [ ] **Step 1: Pin.** `resync`, `disconnect`, `stream_frames` and every `subscribe` test in `protocol` are the pins. Read their bodies first.
- [ ] **Step 2: Move the loop.** Read the old and new paths side by side (§14.8's risk). Delete the last transition getter.
- [ ] **Step 3: Close the boundary.**
  - `db` and `runtime` become private.
  - `lib.rs`'s public surface is `AppCore`, `CoreParts`, `StartConfig`, `CoreError`, `ErrorCode`, the services and the domain types adapters serialize, plus `testing` under `test-support`.
  - Run `cargo clippy --workspace -- -D warnings` without `--all-targets`, and fix every adapter that reached past a service by calling the service. Never widen a visibility.
- [ ] **Step 4: Prove the boundary bites.**
  - Add `use shadows_core::testing;` to `shadows-http/src/lib.rs`, and see `cargo clippy --workspace -- -D warnings` fail. Revert. If it does not fail, some member enables `shadows-core/test-support` outside `[dev-dependencies]` (check `fake-acp`, Task 3): fix that, never the check.
  - Add `pub fn storage(&self) -> &Storage` to `AppCore`, and see `unnameable_types` fail. Revert.
- [ ] **Step 5: Gate, and write `events/contract.yaml`.** Its obligations:
  - no frame lost or repeated across journal → live;
  - `durable_seq` order;
  - `after` resumes exactly;
  - Events emits nothing of its own.

  Each has `tested_by` from `resync.rs` or `protocol.rs`.
- [ ] **Step 6: Commit.** `refactor(core): Events owns delivery; storage and runtime are private to the core (§14.6)`

---

### Task 11: Contracts complete, and the documents

**Files:**
- Check: eight `contract.yaml` files exist, and `contracts.rs` passes.
- Modify: `docs/superpowers/specs/2026-09-21-architecture-design.md` (§1). "A single crate" becomes the workspace, and §1 refers to §14.3 for the crate list.
- Modify: `docs/superpowers/specs/` §2.9's owner. Backend-specific SQL lives in a service's `store.rs` and in `shadows-core/src/db/`, never outside `shadows-core`.
- Modify: `CLAUDE.md`.
  - "Architecture" describes the workspace, with the crate list and the dependency direction.
  - The ownership table names crates.
  - "Rules (project-specific)" gains two rules:
    - a new operation is a new method on one service, with its line in that service's contract;
    - an adapter translates and holds no rule.
  - "How agents work here" gains, right after the code-map rule:
    - **the contract is the entry point for a service.** Read `crates/shadows-core/src/<service>/contract.yaml` before changing that service; open its code for what the contract points to;
    - **a change to a service updates its contract in the same commit**: its methods, obligations, agreements and tests, following `docs/codebase/contracts/TEMPLATE.yaml`. `contracts.rs` catches a missing name; the reviewer catches a false rule.
  - The "AI Start Here" table gains a row for the contracts: one per service, next to its code, and the template.
- Modify: `docs/codebase/README.md`.
  - One row per crate, and one per service folder, each job without "and".
  - An **Architecture Invariants** section, rust-analyzer style:
    - "`shadows-http` knows HTTP; nothing below it does";
    - "`shadows-core` never imports `axum` or `rmcp`";
    - "a service's `store` is called by another service only through a function its contract declares under `shared_in_transaction`, inside one write" (§14.6, ruled 2026-09-27).
- Modify: the code map test, so the inventory groups its files by crate (it has walked every `crates/*/src` since Task 0).
- Modify: `docs/superpowers/specs/2026-09-26-application-core-design.md` (status: Built, with the commit range), and `docs/status.md`.

- [ ] **Step 1:** Read each contract once against its service, by the template's ten rules: are `shapes` and `functions` complete; does every `tested_by` body prove its rule; is every place one rule has two paths an `agreements` entry; is every `not_the_caller's` true; do the sections contradict each other anywhere?
- [ ] **Step 2:** Update the documents, regenerate the code map, and run the gate.
- [ ] **Step 3:** Commit: `docs: the workspace and the application core in CLAUDE.md, the code map, §1, §2.9 and status (§14.11)`

---

### Task I: Whole-branch review, Mohammed's run, merge (controller)

1. **Whole-branch review.** An opus agent reads `main..milestone-2.5/app-core`, fixes what it finds, and reports. It checks:
   - every §14.9 rule is in its service;
   - no fingerprint changed;
   - `detached` wraps the same calls;
   - no service returns an adapter type;
   - `git log --follow` works across the moves;
   - each of the eight contracts is true against its code: it picks three obligations per contract and reads the `tested_by` bodies.
2. **Mohammed's run on the debug build** (§14.10):
   1. send a message and stop it;
   2. restart the daemon, and the conversation is still there;
   3. edit and approve a plan;
   4. Connect an external Claude Code, read a plan, then Revoke.

   Write `docs/evidence/milestone2_5/WINDOWS_RUN.md`. `.claude/launch.json` points the daemon at the new binary path in the target directory.
3. **The PR,** when Mohammed says. After merge, delete the branch locally and on GitHub.
