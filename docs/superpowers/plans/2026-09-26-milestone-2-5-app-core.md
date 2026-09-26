# Milestone 2.5 — One Application Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every operation Shadows performs gets one home, a method on one service inside an `AppCore`. The HTTP routes, the MCP tools and the daemon's startup call that method and hold no rule. No behaviour a client can observe changes.

**Architecture:** A new `src/core/` module holds `AppCore` and eight services, plus a `CoreError` that knows nothing of HTTP or MCP. `AppCore` is built once by `cli::serve` through `AppCore::start`, and by the tests through `AppCore::assemble`. It is shared as `Arc<AppCore>` and never as a global. `protocol::AppState` and `mcp::McpState` hold that `Arc` plus transport-only fields. Each adapter maps `CoreError` to its own failure shape. Two guards keep it that way: the compiler, and `tests/architecture.rs` with an allowlist that shrinks to empty. Each service gets a YAML contract that `tests/contracts.rs` keeps current.

**Tech Stack:** Rust (axum 0.8, utoipa, SQLx 0.9 on SQLite, `rmcp` 3.4.1). New dev-dependency: `yaml-rust2 = "0.13"`, for the contracts test only. `syn` and `proc-macro2` are already dev-dependencies.

**Spec:** `docs/superpowers/specs/2026-09-26-application-core-design.md` (§14). Every task's requirements include the spec sections it names. Where this plan and §14 disagree, §14 is right and the plan is the defect: stop and report.

## Global Constraints

- Branch `milestone-2.5/app-core`, already created off `main` at `ddcaed5`. It holds the spec commits.
  - **No git worktrees** (Mohammed's rule).
  - Commit per task. Push only when Mohammed asks.
- Every subagent runs on opus, stated explicitly in the dispatch.
- The reviewer is Codex `gpt-6-sol`, which fixes what it finds. The controller verifies the review and does not repeat it (CLAUDE.md "How agents work here").
- Read `docs/codebase/README.md` and `docs/codebase/inventory.md` first. Open only the files the task names.
- **Behaviour does not change.**
  - No route, MCP tool, JSON field, status code, error code or error message changes.
  - `api/openapi.json` stays byte-identical, which `tests/openapi.rs` checks. So `web/` is not touched.
  - Existing tests change only their construction and imports, never what they assert. If an assertion must change, stop and report.
- **The dependency direction** is `cli/`, `protocol/`, `mcp/` → `core/` → the domain modules (`planner/`, `workflow/`, `agent/`, `runtime/`, `project/`, `thread/`, `grant/`) → `storage/`. `core/` never imports `protocol/` or `mcp/`, and the domain modules never import `core/`.
- **Idempotency moves with the operation.** A service builds its own `CommandContext`, with the same `command_kind` strings and the same fingerprint parameters the route or tool builds today. A changed fingerprint breaks replay, and the tests catch it.
- **Detachment stays in the HTTP adapter.** `protocol/conversation.rs::detached` spawns the work, so a client that disconnects does not cancel it. A route that calls it today still calls it, around the service call.
- **Files:** 300 lines needs a stated reason, and at 500 the file splits (CLAUDE.md). A service that grows past 300 splits by responsibility into `core/<service>/`.
- Every signature change regenerates the code map in the same commit: `UPDATE_CODEMAP=1 cargo test --test codemap`. A new module gets its one-job line in `docs/codebase/README.md`.
- **The gate before every commit:**
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - Report the test counts. At the start: 304 Rust and 135 web; the web count must not move.
- **Builds use the C: target directory:**
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/Temp/claude/E--Globalprojects-shadows/e86a610a-4c5b-4114-884d-faf050ce6a96/scratchpad/target CARGO_INCREMENTAL=0`,
  because E: fills up. Stop any running `shadows.exe` preview before `cargo test`, or it cannot overwrite the binary.
- **Each task writes or extends its service's contract**, `docs/codebase/contracts/<service>.yaml`, from `TEMPLATE.yaml`. It records every rule the task moved (§14.7) with `tested_by`. §14.7's rules need a real test; any other `none` or `unknown` needs a `note`.

## Review Focus

1. **A retried command after the move.** A client retries `POST /api/threads/{id}/turns`, or a Planner retries `plan_edit`, with the same `command_id`. The person expects the first answer again, not a `COMMAND_CONFLICT` or a second turn. The fingerprint must be built from exactly the same parameters as before. (Task 2 test `plan_commands_replay_after_the_move`; Task 4 test `turn_start_replays_after_the_move`.)
2. **A model change while a turn runs.** The person expects `THREAD_BUSY`, with the running turn's session untouched. The early busy check must survive its move into `Harness::change_model`. (Task 5 test `change_model_is_refused_while_a_turn_runs`.)
3. **Stop during a turn, then an SSE reconnect with `after`.** The person expects the turn `Cancelled`, and the stream to resume without a lost or repeated frame. (Task 4 test `stop_after_the_move_records_cancelled`; Task 7 relies on `tests/resync.rs`, which is unchanged.)
4. **An external agent reaching a plan outside its grant.** It must get the same `GRANT_SCOPE` text as today, now produced by `Plans`. (Task 2 test `a_project_grant_is_refused_another_projects_plan_after_the_move`.)
5. **Ctrl+C with a turn running.** The daemon records the turn `Cancelled`, then ends its live streams, in that order. (Task 1 test `shut_down_through_the_core_cancels_and_records`.)

---

## Execution map

```text
Task 0  guards: tests/architecture.rs (allowlist = today) + tests/contracts.rs + TEMPLATE.yaml
Task 1  core/: AppCore, CoreParts, CoreError, service shells; AppState/McpState hold Arc<AppCore>; shut_down
Task 2  Plans          (protocol/workflow.rs, mcp/tools.rs)
Task 3  Grants         (protocol/grants.rs, mcp/auth.rs, cli startup) + grant types move to src/grant/
Task 4  Turns          (protocol/conversation.rs start/stop)             ← risk: Stop, lease, turn end
Task 5  Harness        (protocol/harness.rs)
Task 6  Projects, Threads, Instructions (protocol/project.rs, fs.rs, thread.rs, instructions.rs, conversation.rs lists)
Task 7  Events         (protocol/sse.rs, ui_signal.rs); allowlist empty  ← risk: journal → caught-up → live
Task 8  contracts complete; CLAUDE.md, docs/codebase/README.md, specs README; status.md
Task I  (controller) whole-branch review, Mohammed's run, evidence, PR
```

Tasks run in order, each with one implementer and then one review. Every task leaves the tree building and every test passing.

---

### Task 0: The guards

**Files:**
- Create: `tests/architecture.rs`
- Create: `tests/contracts.rs`
- Create: `docs/codebase/contracts/TEMPLATE.yaml`: Mohammed's template, copied verbatim from `C:\Users\Mohammed\Downloads\Telegram Desktop\GENERIC_SOURCE_CONTRACT_TEMPLATE.yaml`
- Modify: `Cargo.toml` (`[dev-dependencies]`: `yaml-rust2 = "0.13"`)

**Interfaces:**
- Produces: `ALLOWLIST` in `tests/architecture.rs`, which later tasks shrink. And the contract rules later tasks' contracts must pass.

- [ ] **Step 1: Write `tests/architecture.rs`.** It walks `src/protocol/`, `src/mcp/` and `src/cli/`, tokenizes each file with `proc_macro2`, and fails on a forbidden identifier. Comments are not tokens, and doc comments become string literals, so prose never trips it.

```rust
//! Spec §14.5: the adapters (`protocol/`, `mcp/`, `cli/`) reach the
//! application only through `core::AppCore`. A file here that names storage,
//! the Planner's live registries or the runtime has bypassed the core.
//!
//! `ALLOWLIST` holds the files that still do, as of Milestone 2.5's start. Each
//! task that moves a service removes its files; Task 7 empties it. A file on
//! the list that no longer violates fails too, so the list cannot go stale.

use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};

const ADAPTERS: &[&str] = &["src/protocol", "src/mcp", "src/cli"];
/// The types, and the names of Task 1's temporary `AppCore` getters, so a
/// route that reads through a getter is still a violation until its service
/// moves.
const FORBIDDEN: &[&str] = &[
    "storage", "Storage", "sessions", "Sessions", "handles", "LiveHandles",
    "runtime", "Runtime", "bus", "ui_bus",
];

/// Files allowed to violate, until their service moves. Paths use `/`.
const ALLOWLIST: &[&str] = &[
    // Filled in Step 2 from the first run's output.
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn idents(tokens: TokenStream, found: &mut Vec<String>) {
    for tree in tokens {
        match tree {
            TokenTree::Ident(i) => found.push(i.to_string()),
            TokenTree::Group(g) => idents(g.stream(), found),
            _ => {}
        }
    }
}

fn violations() -> Vec<(String, Vec<String>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ADAPTERS {
        rust_files(&root.join(dir), &mut files);
    }
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("readable source");
        let tokens: TokenStream = text.parse().expect("tokenizable source");
        let mut found = Vec::new();
        idents(tokens, &mut found);
        let mut hits: Vec<String> = found
            .into_iter()
            .filter(|i| FORBIDDEN.contains(&i.as_str()))
            .collect();
        hits.sort();
        hits.dedup();
        if !hits.is_empty() {
            let rel = file.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((rel, hits));
        }
    }
    out
}

#[test]
fn adapters_reach_the_application_only_through_the_core() {
    let found = violations();
    let new: Vec<_> = found
        .iter()
        .filter(|(file, _)| !ALLOWLIST.contains(&file.as_str()))
        .collect();
    assert!(
        new.is_empty(),
        "these files bypass core::AppCore (spec §14.5). Call the service instead \
         (core.plans(), core.turns(), core.grants(), …), and move any rule into it:\n{new:#?}"
    );
    let stale: Vec<_> = ALLOWLIST
        .iter()
        .filter(|allowed| !found.iter().any(|(file, _)| file == *allowed))
        .collect();
    assert!(
        stale.is_empty(),
        "these files no longer bypass the core; remove them from ALLOWLIST: {stale:?}"
    );
}
```

- [ ] **Step 2: Fill the allowlist from reality.** Run `cargo test --test architecture`. It fails and prints every violating file. Copy exactly those paths into `ALLOWLIST`, one per line with a comment naming the task that removes it:

| File | Removed by |
|---|---|
| `src/protocol/workflow.rs`, `src/mcp/tools.rs`, `src/mcp/server.rs` | Task 2 |
| `src/protocol/grants.rs`, `src/mcp/auth.rs` | Task 3 |
| `src/protocol/conversation.rs` | Task 6. Task 4 moves start and stop, and Task 6 moves the lists. |
| `src/protocol/harness.rs` | Task 5 |
| `src/protocol/project.rs`, `src/protocol/thread.rs`, `src/protocol/instructions.rs` | Task 6 |
| `src/protocol/sse.rs` | Task 7 |
| `src/protocol/mod.rs`, `src/mcp/mod.rs`, `src/cli/mod.rs` | Task 1 |

Run again. Expected: PASS. A path the run printed that is not in the table goes to the task whose service owns it, and the report says so.

- [ ] **Step 3: Write `tests/contracts.rs`.** It loads every `docs/codebase/contracts/*.yaml` except `TEMPLATE.yaml`, parses it with `yaml_rust2::YamlLoader::load_from_str`, and checks §14.6's six rules. The service source is the file named by `source`, and the public methods are collected with `syn`, from `pub fn` and `pub async fn` in that file's `impl` blocks. Test names are collected with `syn` from every `#[test]` or `#[tokio::test]` function in `tests/**/*.rs` and in `src/**/*.rs`.

```rust
//! Spec §14.6: a service contract that no longer matches its code fails here.
//! It cannot check that a rule's prose is true; the reviewer owns that.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use yaml_rust2::{Yaml, YamlLoader};

fn root() -> &'static Path { Path::new(env!("CARGO_MANIFEST_DIR")) }

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for entry in read {
        let path = entry.unwrap().path();
        if path.is_dir() { rs_files(&path, out) }
        else if path.extension().is_some_and(|e| e == "rs") { out.push(path) }
    }
}

fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let p = a.path();
        p.is_ident("test") || (p.segments.len() == 2 && p.segments[1].ident == "test")
    })
}

fn collect_tests(items: &[syn::Item], out: &mut BTreeSet<String>) {
    for item in items {
        match item {
            syn::Item::Fn(f) if is_test(&f.attrs) => { out.insert(f.sig.ident.to_string()); }
            syn::Item::Mod(m) => if let Some((_, inner)) = &m.content { collect_tests(inner, out) },
            _ => {}
        }
    }
}

fn all_tests() -> BTreeSet<String> {
    let mut files = Vec::new();
    rs_files(&root().join("tests"), &mut files);
    rs_files(&root().join("src"), &mut files);
    let mut out = BTreeSet::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        if let Ok(file) = syn::parse_file(&text) { collect_tests(&file.items, &mut out) }
    }
    out
}

/// Every symbol `source` declares at any depth: fns, methods, types, consts.
fn symbols(file: &syn::File) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut all, mut public_methods) = (BTreeSet::new(), BTreeSet::new());
    for item in &file.items {
        match item {
            syn::Item::Fn(f) => { all.insert(f.sig.ident.to_string()); }
            syn::Item::Struct(s) => { all.insert(s.ident.to_string()); }
            syn::Item::Enum(e) => { all.insert(e.ident.to_string()); }
            syn::Item::Const(c) => { all.insert(c.ident.to_string()); }
            syn::Item::Type(t) => { all.insert(t.ident.to_string()); }
            syn::Item::Impl(i) => for it in &i.items {
                if let syn::ImplItem::Fn(m) = it {
                    all.insert(m.sig.ident.to_string());
                    if matches!(m.vis, syn::Visibility::Public(_)) && i.trait_.is_none() {
                        public_methods.insert(m.sig.ident.to_string());
                    }
                }
            },
            _ => {}
        }
    }
    (all, public_methods)
}

/// The function names a contract lists: the keys of every mapping under
/// `functions`, at any depth, whose value is a string or has `signature`.
fn listed_functions(y: &Yaml, out: &mut BTreeSet<String>) {
    if let Yaml::Hash(h) = y {
        for (k, v) in h {
            let named = matches!(v, Yaml::String(_))
                || matches!(v, Yaml::Hash(inner) if inner.contains_key(&Yaml::String("signature".into())));
            if named { if let Yaml::String(k) = k { out.insert(k.clone()); } }
            else { listed_functions(v, out) }
        }
    }
}

#[test]
fn every_contract_matches_its_service() {
    let dir = root().join("docs/codebase/contracts");
    let tests = all_tests();
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.ends_with(".yaml") || name == "TEMPLATE.yaml" { continue; }
        let text = std::fs::read_to_string(&path).unwrap();
        // Rule 1: valid YAML.
        let docs = match YamlLoader::load_from_str(&text) {
            Ok(d) => d, Err(e) => { problems.push(format!("{name}: not valid YAML: {e}")); continue; }
        };
        let doc = &docs[0];
        // Rule 2: `source` exists.
        let Some(source) = doc["source"].as_str() else { problems.push(format!("{name}: no source")); continue };
        let Ok(src) = std::fs::read_to_string(root().join(source)) else {
            problems.push(format!("{name}: source {source} does not exist")); continue };
        let file = syn::parse_file(&src).unwrap();
        let (declared, public_methods) = symbols(&file);
        // Rule 3: listed functions and gap `at`s exist in the source.
        let mut listed = BTreeSet::new();
        listed_functions(&doc["functions"], &mut listed);
        for f in &listed { if !declared.contains(f) { problems.push(format!("{name}: `{f}` is not in {source}")) } }
        if let Some(gaps) = doc["gaps"].as_vec() { for g in gaps {
            if let Some(at) = g["at"].as_str() { if !declared.contains(at) {
                problems.push(format!("{name}: gap at `{at}` is not in {source}")) } } } }
        // Rule 4: named tests exist.
        let mut named = Vec::new();
        if let Some(obs) = doc["obligations"].as_vec() { for o in obs {
            match &o["tested_by"] { Yaml::String(s) => named.push(s.clone()),
                Yaml::Array(a) => named.extend(a.iter().filter_map(|t| t.as_str().map(str::to_string))), _ => {} }
            // Rule 6: `none` / `unknown` carries a note.
            let t = o["tested_by"].as_str().unwrap_or("");
            if (t == "none" || t == "unknown") && o["note"].as_str().is_none() {
                problems.push(format!("{name}: an obligation says tested_by: {t} without a note")) }
        } }
        if let Some(ts) = doc["tests"].as_vec() { for t in ts { if let Some(n) = t["name"].as_str() { named.push(n.to_string()) } } }
        for n in named {
            let bare = n.trim_start_matches("integration: ").to_string();
            if bare != "none" && bare != "unknown" && !tests.contains(&bare) {
                problems.push(format!("{name}: test `{bare}` does not exist")) }
        }
        // Rule 5: every public method is listed.
        for m in &public_methods { if !listed.contains(m) {
            problems.push(format!("{name}: public method `{m}` of {source} has no entry in functions")) } }
    }
    assert!(problems.is_empty(), "contracts out of date (spec §14.6):\n{}", problems.join("\n"));
}
```

- [ ] **Step 4: Prove each guard bites.**
  - Add `use crate::storage::Storage;` to a file not on the allowlist, such as `src/protocol/failure.rs`. Run `cargo test --test architecture` and expect FAIL. Revert.
  - Write a scratch `docs/codebase/contracts/scratch.yaml` with `source: src/config.rs` and `functions: { nope: "() -> ()" }`. Run `cargo test --test contracts` and expect FAIL naming `nope`. Delete the scratch file.
- [ ] **Step 5: Copy the template, then run the gate.** Expect 306 Rust (304 + 2), all passing.
- [ ] **Step 6: Commit.** `test(architecture): guard the adapters and the service contracts (§14.5, §14.6)`

---

### Task 1: `AppCore`, `CoreError`, and the adapters holding it

**Files:**
- Create: `src/core/mod.rs`: `AppCore`, `CoreParts`, accessors, `start`, `assemble`, `shut_down`
- Create: `src/core/error.rs`: `CoreError`
- Create: `src/core/{projects,threads,turns,harness,plans,grants,instructions,events}.rs`: each a struct holding the `Arc`s it needs, with no methods yet
- Modify: `src/lib.rs` (`pub mod core;`)
- Modify: `src/protocol/mod.rs` (`AppState`), `src/mcp/mod.rs` (`McpState` → `Arc<AppCore>`), `src/cli/mod.rs`
- Modify: `src/protocol/failure.rs` (`impl From<CoreError> for Failure`), `src/mcp/refusal.rs` (`impl From<CoreError> for Refusal`)
- Move: `UiSignal`, from `src/protocol/ui_signal.rs` to `src/core/events.rs`
- Modify: `tests/fixtures/app.rs`, `tests/fixtures/listening.rs`, and the nine tests that build `AppState` themselves (`cors`, `debug_log`, `disconnect`, `fs_routes`, `openapi`, `planner_turn`, `project_directory`, `protocol`, `resync`). Only their construction changes.
- Test: `tests/core.rs` (new)

**Interfaces:**
- Produces:

```rust
// src/core/mod.rs
pub type Bus = tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>;

/// What `AppCore` is assembled from. Tests build it so the fixture keeps its
/// own handles on the same `Arc`s; production builds it in `start`.
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
    projects: Projects, threads: Threads, turns: Turns, harness: Harness,
    plans: Plans, grants: Grants, instructions: Instructions, events: Events,
    // Held for shut_down only; never returned.
    runtime: Arc<Runtime>, handles: Arc<LiveHandles>, sessions: Arc<Sessions>,
}

impl AppCore {
    /// Production startup, in today's order: open storage, start the runtime
    /// (recovery), revoke every thread grant, read the harness versions, build
    /// the sessions on `mcp_url`. The listener is bound by the caller first,
    /// because `mcp_url` names the address actually bound.
    pub async fn start(config: &Config, mcp_url: String) -> anyhow::Result<Arc<AppCore>>;
    pub fn assemble(parts: CoreParts) -> Arc<AppCore>;
    pub fn projects(&self) -> &Projects;
    pub fn threads(&self) -> &Threads;
    pub fn turns(&self) -> &Turns;
    pub fn harness(&self) -> &Harness;
    pub fn plans(&self) -> &Plans;
    pub fn grants(&self) -> &Grants;
    pub fn instructions(&self) -> &Instructions;
    pub fn events(&self) -> &Events;
    /// Spec §8.5 through `planner::shut_down`, unchanged: stop every running
    /// turn, close every adapter, record the stop.
    pub async fn shut_down(
        &self,
        bound: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> Result<StopKind, ShutdownError>;
}
```

```rust
// src/core/error.rs — every way a service call fails, in words no adapter owns.
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
    /// A refusal whose code and exact text a service writes, as the MCP tools
    /// do today (`GRANT_SCOPE`, `INVALID_COMMAND` …).
    #[error("{message}")] Refused { code: ErrorCode, message: String },
}
impl From<OpenError> for CoreError { /* Storage→Storage, Start(r)→HarnessStartFailed(r), Workspace(r)→ProjectDirectoryUnusable(r) */ }
```

- `StopKind` and `ShutdownError` are whatever `planner::shut_down` returns today. Name them exactly as `src/planner/shutdown.rs` declares them.
- `AppState` becomes `{ core: Arc<AppCore>, allowed_origins: Vec<String>, shutdown: watch::Receiver<bool> }`. `mcp_url` moves into `CoreParts`, because `Grants::issue` builds the command from it (Task 3). This amends §14.4's `AppState` sketch, and Step 7 amends the spec text to match.
- `McpState` is replaced by `Arc<AppCore>`. `mcp::service(core: Arc<AppCore>) -> Router`.

- [ ] **Step 1: Write the failing test `tests/core.rs`.**

```rust
mod fixtures;
use fixtures::app::*;

#[tokio::test]
async fn the_app_state_reaches_services_only_through_the_core() {
    let app = test_app().await;
    // The router still answers, now built from AppCore.
    let (status, list) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200);
    assert_eq!(names(&list), vec!["Demo".to_string()]);
}

#[tokio::test]
async fn shut_down_through_the_core_cancels_and_records() {
    let app = test_app().await;
    let op = start_on(&app, app.thread.as_str(), "hang", default_settings()).await;
    app.core.shut_down(std::time::Duration::from_secs(10), std::future::pending()).await.unwrap();
    let done = wait_terminal(&app, &op).await;
    assert_eq!(done.state.as_str(), "Cancelled");
}
```

The fixture's `App` gains `pub core: Arc<AppCore>`, built with `AppCore::assemble(CoreParts { … })` from the same `Arc`s it already holds. Every other field of `App` stays, so the 363 existing uses of `app.storage` and the rest compile unchanged. If `Operation.state` is not a string-like enum with `as_str`, assert the way `tests/shutdown.rs` asserts a cancelled operation. Read it first.

- [ ] **Step 2: Run it.** `cargo test --test core`. Expected: FAIL to compile (`shadows::core` does not exist).
- [ ] **Step 3: Implement `src/core/`.**
  - The shells hold, and only hold, what their later task needs:
    - `Plans`: storage, handles, ui
    - `Grants`: storage, mcp_url
    - `Turns`: storage, runtime, handles, sessions, bus
    - `Harness`: storage, sessions
    - `Projects`: storage
    - `Threads`: storage, sessions
    - `Instructions`: storage
    - `Events`: storage, sessions, bus, ui
  - `start` moves `serve`'s pre-bind body out of `cli/mod.rs`, including `harness_version`. Keep it in `src/core/mod.rs`, or in `src/core/start.rs` if `mod.rs` passes 300 lines.
  - `serve` becomes: bind, then `AppCore::start(&config, mcp_url)`, print the address, and serve `protocol::router(AppState { core, allowed_origins, shutdown })`. The graceful-shutdown future calls `core.shut_down(CONFIRMATION_BOUND, second_signal)` and then `stopping.send_replace(true)`, in today's order, keeping every log line (`shutdown.recorded`, `shutdown.unrecorded`, `shutdown.signal_unavailable`).
  - Handlers keep working this task by reading through the core. Add temporary `pub(crate)` getters on `AppCore` with exactly these names:
    - `storage()`, `sessions()`, `handles()`, `runtime()` and `bus()`, each returning its `&Arc<…>`;
    - `ui_bus()` for the UI sender.

    Change `s.storage` to `s.core.storage()` in each handler. **These getters are the allowlist's shadow:**
    - every handler that uses one still names a `FORBIDDEN` identifier, so its file stays on `ALLOWLIST`;
    - each later task deletes the getters its routes no longer need;
    - Task 7 deletes the last one, and `the_core_exposes_no_storage` (Task 7) proves they are gone.
  - `UiSignal` moves now from `src/protocol/ui_signal.rs` to `src/core/events.rs`, with its fields and derives unchanged, because `CoreParts` names it and `core/` must not import `protocol/`. Update its imports in `mcp/tools.rs`, `protocol/sse.rs` and `tests/fixtures/app.rs`. `protocol/ui_signal.rs` keeps whatever else it holds, or is deleted if the struct was all.
- [ ] **Step 4: Map `CoreError` in each adapter.**
  - `impl From<CoreError> for Failure` calls the constructors that exist today:
    - `Storage`, `Start` and `Directory` go through the existing `From`s;
    - `ProjectDirectoryUnusable` → `project_directory_unusable`;
    - `HarnessStartFailed` → `harness_start_failed`;
    - `HarnessUnavailable` → `harness_unavailable`;
    - `SettingNotOffered` → `setting_not_offered(&what, &id, detail.as_deref())`;
    - `ModeNotAllowed` → `mode_not_allowed`;
    - `RuntimeStopping` → `runtime_stopping()`;
    - `TerminationFailed` → `termination_failed()`;
    - `Refused { code, message }` → a new `Failure::refused(code, message)` with status 422. No HTTP route produces `Refused` in this milestone.
  - `impl From<CoreError> for Refusal`:
    - `Storage` goes through the existing `From`;
    - `Refused` → `Refusal::new(code, message)`;
    - every other variant → `Refusal::new(<its ErrorCode>, e.to_string())`.
- [ ] **Step 5: Update the fixtures and the nine test files' `AppState` literals.** No assertion changes.
- [ ] **Step 6: Run the gate.** Expect 308 Rust. `tests/architecture.rs` must pass: `protocol/mod.rs`, `mcp/mod.rs` and `cli/mod.rs` now name only `core` types, so remove them from `ALLOWLIST`.
- [ ] **Step 7: Amend §14.4** so the `AppState` sketch shows `mcp_url` in the core, not in `AppState`. Record `CoreError` in §14.3 in one sentence.
- [ ] **Step 8: Commit.** `feat(core): AppCore holds the application; adapters hold only Arc<AppCore> (§14.3, §14.4)`

---

### Task 2: `Plans`

**Files:**
- Modify: `src/core/plans.rs`
- Modify: `src/protocol/workflow.rs` (`list_plans`, `get_plan`, `approve_plan` → one call each)
- Modify: `src/mcp/tools.rs` (every tool → one call). `plan_in_scope`, `planner_draft`, `external_draft`, `edit`, `show`, `own_thread` and `writer_of` move into `Plans` unchanged in logic and text.
- Modify: `src/mcp/server.rs` (`Shadows` holds `Arc<AppCore>`)
- Create: `docs/codebase/contracts/plans.yaml`
- Test: `tests/core_plans.rs` (new)

**Interfaces:**
- Consumes: `CoreError::Refused` (Task 1), `Grant` and `GrantKind` from `crate::mcp::grant` (they move in Task 3; import them from where they are).
- Produces:

```rust
impl Plans {
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanListing>, CoreError>;
    pub async fn get(&self, workflow: &WorkflowId) -> Result<Plan, CoreError>;
    /// Person's approval (§13.2). command kind "PlanApprove", params
    /// { "workflow", "expected_revision" }, principal User/local — as today.
    pub async fn approve(&self, command_id: String, workflow: &WorkflowId, expected_revision: i64)
        -> Result<Approved, CoreError>;
    // Through a grant (§13.6, §13.7): the scope check lives here, once.
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
/// The tools' arguments, as plain data (no schemars here; the tool's own
/// `*Args` struct in mcp/tools.rs keeps the JSON schema and converts).
pub struct DraftStart { pub title: Option<String>, pub goal: Option<String>,
                        pub from_workflow_id: Option<WorkflowId>, pub draft_ref: Option<String> }
pub struct PlanEdit { pub workflow_id: Option<WorkflowId>, pub expected_revision: i64,
                      pub ops: Vec<PlanOp>, pub command_id: Option<String> }
pub struct PlanShow { pub workflow_id: Option<WorkflowId>, pub task_number: Option<u32>, pub place: Place }
```

`get_for`'s old `latest_only` flag stays private: `edit` calls the private `in_scope(grant, named, true)`, and `get_for` and `task_for` call it with `false`. Every refusal text moves verbatim. `Refusal::scope(msg)` becomes `CoreError::Refused { code: ErrorCode::GrantScope, message }`, and `Refusal::new(code, msg)` becomes `CoreError::Refused { code, message }`.

- [ ] **Step 1: Write the failing tests `tests/core_plans.rs`.** They pin the moved behaviour through the adapters, which is what a client sees. Use the existing helpers in `tests/fixtures/plan.rs` and `tests/fixtures/listening.rs`.

```rust
mod fixtures;
use fixtures::listening::*;
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
    let edit = json!({ "workflow_id": first["workflow_id"], "expected_revision": first["revision"],
        "ops": [{ "op": "task_add", "number": 1, "title": "A" }], "command_id": "e1" });
    let e1 = ok(&client, "plan_edit", edit.clone()).await;
    let e2 = ok(&client, "plan_edit", edit).await;
    assert_eq!(e1, e2);
}

#[tokio::test]
async fn a_project_grant_is_refused_another_projects_plan_after_the_move() {
    let l = listening_app().await;
    let (_g, client) = project_client(&l).await;
    let other = fixtures::plan::plan_in_other_project(&l).await;
    let text = refused(&client, "workflow_get", json!({ "workflow_id": other })).await;
    assert!(text.starts_with("GRANT_SCOPE: that plan is not in this grant's project"), "{text}");
}
```

Adjust the tool arguments' field names to what `DraftStarted` and `EditOutcome` serialize as today. Read `src/workflow/mod.rs`. If `plan_in_other_project` does not exist in `tests/fixtures/plan.rs`, add it there, using the same pattern `tests/plan_grants.rs` uses for a second project's plan.

- [ ] **Step 2: Run them.** `cargo test --test core_plans`. They pass against the old code; that is expected, because these pin behaviour across the move. Record that they passed before the move.
- [ ] **Step 3: Move the logic into `Plans`.** The routes and tools become one call plus a conversion:

```rust
// src/protocol/workflow.rs
pub(super) async fn approve_plan(State(s): State<AppState>, Path(workflow): Path<WorkflowId>,
    Json(body): Json<ApprovePlan>) -> Result<Json<Approved>, Failure> {
    Ok(Json(s.core.plans().approve(body.command_id, &workflow, body.expected_revision).await?))
}
// src/mcp/tools.rs
async fn workflow_get(&self, Extension(grant): Extension<Grant>,
    Parameters(args): Parameters<PlanArgs>) -> CallToolResult {
    answer(self.core.plans().get_for(&grant, args.workflow_id.as_ref()).await.map_err(Refusal::from))
}
```

  Delete the `AppCore` getters no route or tool uses any more.
- [ ] **Step 4: Remove `src/protocol/workflow.rs`, `src/mcp/tools.rs` and `src/mcp/server.rs` from `ALLOWLIST`, then run the gate.** Expect 310 Rust. Rerun `tests/core_plans.rs` and the existing `tests/mcp_tools.rs`, `tests/plan_grants.rs`, `tests/plan_storage.rs`, `tests/planner_mcp.rs` and `tests/plan_in_conversation.rs`, all unchanged.
- [ ] **Step 5: Write `docs/codebase/contracts/plans.yaml`** from the template. Its obligations include:
  - grant scope, with `tested_by` the scope tests in `tests/plan_grants.rs`;
  - replay;
  - the UI signal sent only when not replayed, with `tested_by` the `plan_show` test in `tests/plan_in_conversation.rs`.

  `not_the_caller's` includes: "check a grant's scope before calling a `*_for` method".
- [ ] **Step 6: Commit.** `refactor(core): Plans owns every plan operation, HTTP and MCP alike (§14.3)`

---

### Task 3: `Grants`, and the grant types move to `src/grant/`

**Files:**
- Move: `src/mcp/grant.rs` → `src/grant/mod.rs`. `Grant`, `GrantId`, `GrantKind`, `IssuedGrant`, `Token` and `hash_token` are domain types, used by storage and `command/`. Update every `crate::mcp::grant` import, and the four test and bin imports: `tests/fixtures/listening.rs`, `tests/fixtures/plan.rs`, `tests/grants.rs`, `src/bin/fake_acp.rs`.
- Modify: `src/core/grants.rs`, `src/protocol/grants.rs`, `src/mcp/auth.rs`, `src/core/mod.rs` (`start` calls `grants.revoke_thread_grants()`)
- Create: `docs/codebase/contracts/grants.yaml`
- Test: extend `tests/core.rs`

**Interfaces:**

```rust
impl Grants {
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<Grant>, CoreError>;
    /// "McpGrantIssue", params { "project" }. `command` is the
    /// `claude mcp add --transport http shadows <mcp_url> --header "Authorization: Bearer <token>"`
    /// line, built here from the core's mcp_url — present only when the token is.
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

`protocol/grants.rs`'s `IssuedGrantBody` keeps its name and fields, so OpenAPI is unchanged, and it is built from `IssuedView`. `mcp/auth.rs::require_grant` takes `State<Arc<AppCore>>` and calls `core.grants().authorize(token)`, keeping its three answers (401, 401, 503) and its log lines.

- [ ] **Step 1: Failing test.** `revoked_grant_is_refused_after_the_move`:
  - issue a grant through `POST /api/projects/{id}/mcp-grants`;
  - connect an MCP client with its token and call `workflow_list`, which succeeds;
  - `DELETE` the grant;
  - the next `mcp_client` connection with that token fails with 401. Use `listening.rs`'s pattern from `tests/grants.rs`.

  Run it; it passes before the move, and this pins it.
- [ ] **Step 2: Move the files and the logic.** Delete the getters no longer needed. Remove `src/protocol/grants.rs` and `src/mcp/auth.rs` from `ALLOWLIST`.
- [ ] **Step 3: Run the gate.** `tests/grants.rs` and `tests/plan_grants.rs` pass unchanged, except for their `use` lines.
- [ ] **Step 4: Write `grants.yaml`.** Among its obligations:
  - the token is stored only as its hash;
  - a replayed issue returns no token;
  - thread grants die at startup.
- [ ] **Step 5: Commit.** `refactor(core): Grants owns issue, revoke and the bearer check; grant types become src/grant (§14.3)`

---

### Task 4: `Turns` (the risk task)

**Files:**
- Modify: `src/core/turns.rs`. `start`, `record`, `offer_for_model` and `focus_block`'s call move from `protocol/conversation.rs`, and `stop_turn`'s body moves too.
- Modify: `src/protocol/conversation.rs`. `start_turn` and `stop_turn` each become `detached(one call)`. The list routes stay until Task 6.
- Create: `docs/codebase/contracts/turns.yaml`
- Test: `tests/core_turns.rs` (new)

**Interfaces:**

```rust
/// A turn request as plain data; `StartTurn` in protocol keeps the JSON schema.
pub struct SendTurn {
    pub command_id: String, pub prompt: String, pub model: String, pub mode: String,
    pub effort: Option<String>, pub focus: Option<Focus>, pub client_tab: Option<String>,
}
impl Turns {
    /// §12.3, §13.9. In this exact order, as today: build the "turn.start"
    /// command (params thread_id, prompt, model, mode, effort, and focus when
    /// given); answer a replay; refuse when the registry is closed
    /// (RuntimeStopping); read turn_context; refuse an unavailable harness;
    /// **refuse THREAD_BUSY before the session is touched**; open the session;
    /// lease its events (Busy → THREAD_BUSY, Closed → HARNESS_START_FAILED);
    /// set the model if needed; refuse an unoffered setting; refuse a mode the
    /// project does not allow; record the turn (TransitionConflict while the
    /// registry is closed → RuntimeStopping); on a replay or failure give the
    /// events back; start the PlannerTurn with the bus.
    pub async fn send(&self, thread: ThreadId, turn: SendTurn) -> Result<OperationId, CoreError>;
    /// §12.3: PlannerTurn::stop as the local user; TerminationFailed → CoreError::TerminationFailed;
    /// then the operation as recorded.
    pub async fn stop(&self, op: &OperationId) -> Result<Operation, CoreError>;
}
```

`Turns` holds `Arc`s and is `Clone`, so the route does `let core = s.core.clone(); detached(async move { core.turns().send(thread_id, turn).await.map_err(Failure::from) })`.

- [ ] **Step 1: Failing tests `tests/core_turns.rs`.** They pin behaviour and pass before the move.
  - `turn_start_replays_after_the_move`: the same `command_id` posted twice gives one operation id, `202` both times.
  - `stop_after_the_move_records_cancelled`: start with prompt `hang`, stop it, and the operation is `Cancelled`.
  - `a_second_start_while_busy_is_thread_busy`: start `hang`, and a second start with a new `command_id` gets 409 `THREAD_BUSY`.

  Use `start_on`, `http_start`, `wait_terminal` and `fresh_command` from `tests/fixtures/app.rs`.
- [ ] **Step 2: Move the logic.** Read the old `start` and the new `send` side by side, statement by statement. Nothing reorders.
- [ ] **Step 3: Run the gate.** These pass unchanged: `tests/planner_turn.rs`, `turn_command.rs`, `operation_lifecycle.rs`, `shutdown.rs`, `disconnect.rs`, `recovery.rs`, `containment.rs` and `plan_in_conversation.rs`.
- [ ] **Step 4: Write `turns.yaml`.** Its obligations are §14.7's first three rules, each with `tested_by`:
  - busy, twice, for two reasons;
  - session before lease;
  - harness availability.

  `not_the_caller's`: "check `thread_is_busy`, open or lease a session before `send`: `send` does all three, in the only safe order".
- [ ] **Step 5: Commit.** `refactor(core): Turns owns starting and stopping a turn (§14.3, §14.7)`

---

### Task 5: `Harness`

**Files:**
- Modify: `src/core/harness.rs`, `src/protocol/harness.rs`
- Create: `docs/codebase/contracts/harness.yaml`
- Test: extend `tests/core_turns.rs`

**Interfaces:**

```rust
impl Harness {
    pub async fn list(&self) -> Result<Vec<HarnessInfo>, CoreError>;
    pub async fn open_session(&self, thread: &ThreadId) -> Result<SessionChoices, CoreError>;
    /// Early THREAD_BUSY before the session is touched (§14.7), then
    /// change_model; ModelRefused maps exactly as the route maps it today.
    pub async fn change_model(&self, thread: &ThreadId, model: &str) -> Result<SessionChoices, CoreError>;
    pub async fn context(&self, thread: &ThreadId) -> Result<ContextBreakdown, CoreError>;
    /// Used by Events (Task 7) for the options frame.
    pub async fn choices(&self, thread: &ThreadId, offered: &Offered) -> Result<SessionChoices, CoreError>;
}
```

`HarnessInfo`, `RememberedSettings` and `ContextBreakdown` move from `protocol/harness.rs` to `src/core/harness.rs`, with the same names, fields and derives (`serde::Serialize`, `utoipa::ToSchema`). `label()` moves with them, so `api/openapi.json` is unchanged. `protocol/openapi.rs` imports them from their new path. `choices_for` becomes `Harness::choices`, and `open_failure` becomes `From<OpenError> for CoreError`.

- [ ] **Step 1: Failing test.** `change_model_is_refused_while_a_turn_runs`: start `hang`, then `PUT /api/threads/{id}/session/model` gives 409 `THREAD_BUSY`. Read the exact path in `protocol/mod.rs`'s route table or `api/openapi.json`. It passes before the move.
- [ ] **Step 2: Move, remove `src/protocol/harness.rs` from `ALLOWLIST`, then run the gate.**
- [ ] **Step 3: Write `harness.yaml`.** Its obligations include the busy rule, shared with `Turns`, and the availability rule.
- [ ] **Step 4: Commit.** `refactor(core): Harness owns the harness list, sessions and model changes (§14.3)`

---

### Task 6: `Projects`, `Threads`, `Instructions`

**Files:**
- Modify: `src/core/{projects,threads,instructions}.rs`
- Modify: `src/protocol/{project,fs,thread,instructions}.rs`, and `src/protocol/conversation.rs` for `list_entries` and `list_operations`
- Modify: `src/planner/setup.rs` only if it reads instructions through a path the move breaks. It is a domain module, and it may keep calling storage.
- Create: `docs/codebase/contracts/{projects,threads,instructions}.yaml`

**Interfaces:**

```rust
impl Projects {
    pub async fn list(&self) -> Result<Vec<Project>, CoreError>;
    /// Resolves the directory before the fingerprint (two spellings, one
    /// request); "project.create", params { slug, name, directory }.
    pub async fn create(&self, command_id: String, slug: &str, name: &str, directory: &str)
        -> Result<Project, CoreError>;
    /// Validates every harness and mode against policy, dedups and sorts;
    /// "project.modes", params { project, allowed_modes }.
    pub async fn set_modes(&self, command_id: String, project: &ProjectId,
        allowed_modes: BTreeMap<String, Vec<String>>) -> Result<Project, CoreError>;
    pub async fn list_dirs(&self, path: Option<String>) -> Result<DirectoryListing, CoreError>;
    pub async fn create_dir(&self, parent: String, name: String) -> Result<DirectoryEntry, CoreError>;
}
impl Threads {
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanningThread>, CoreError>;
    /// Harness defaults to claude-code; unknown → SettingNotOffered("harness").
    /// "thread.create", params { project, title, harness }.
    pub async fn create(&self, command_id: String, project: &ProjectId, title: &str,
        harness: Option<&str>) -> Result<PlanningThread, CoreError>;
    /// "thread.harness"; closes the session when the harness changed (logs, never fails, on close error).
    pub async fn set_harness(&self, command_id: String, thread: &ThreadId, harness: &str)
        -> Result<PlanningThread, CoreError>;
    /// "thread.fork", params { thread_id, at_entry_id }.
    pub async fn fork(&self, command_id: String, thread: &ThreadId, at: &ThreadEntryId)
        -> Result<PlanningThread, CoreError>;
    pub async fn entries(&self, thread: &ThreadId) -> Result<Vec<ThreadEntry>, CoreError>;
    pub async fn operations(&self, thread: &ThreadId) -> Result<Vec<Operation>, CoreError>;
}
impl Instructions {
    pub async fn current(&self, project: &ProjectId) -> Result<Option<InstructionsVersion>, CoreError>;
    /// "PlannerInstructionsSave", params { project, body }.
    pub async fn save(&self, command_id: String, project: &ProjectId, body: &str)
        -> Result<InstructionsVersion, CoreError>;
}
```

`fs.rs`'s `blocking` helper moves with `list_dirs` and `create_dir` into `Projects`, because it is how the browse calls run and not a transport detail. `known_harness` becomes a private check in `Threads` and `Projects`, and returns `CoreError::SettingNotOffered { what: "harness" … }`.

- [ ] **Step 1: Pin the tests.** The existing tests `tests/protocol.rs`, `thread_contract.rs`, `fs_routes.rs`, `project_directory.rs` and `storage_contract.rs`, plus the instructions tests in `tests/planner_mcp.rs`, cover these routes. Add `project_create_replays_after_the_move` to `tests/core.rs`: the same `command_id` posted twice gives one project.
- [ ] **Step 2: Move the logic, then remove `project.rs`, `thread.rs`, `instructions.rs` and `conversation.rs` from `ALLOWLIST`.** `fs.rs` was never on it.
- [ ] **Step 3: Run the gate, then write the three contracts.**
- [ ] **Step 4: Commit.** `refactor(core): Projects, Threads and Instructions own their operations (§14.3)`

---

### Task 7: `Events` (the second risk task)

**Files:**
- Modify: `src/core/events.rs`, which already holds `UiSignal` (Task 1). The journal-then-live loop moves here from `src/protocol/sse.rs`.
- Modify: `src/protocol/sse.rs`. It keeps the SSE framing: event names, `id`s, JSON bodies and keep-alive, and turns each `Delivery` into the frame it sends today.
- Modify: `tests/architecture.rs`. It gains `the_core_exposes_no_storage`, §14.5's second rule.
- Create: `docs/codebase/contracts/events.yaml`

**Interfaces:**

```rust
/// What a subscription yields, in order, as today's stream sends it.
pub enum Delivery {
    Journal(Vec<DurableEvent>),          // the events after `after`, in durable_seq order
    CaughtUp { last_seq: i64 },
    Live(ThreadId, OperationId, HarnessEvent),
    Options(SessionChoices),
    Ui(UiSignal),
}
impl Events {
    /// Replay after `after`, then `CaughtUp`, then live; re-reads the journal
    /// whenever storage's committed-sequence signal moves (§2.4, §6.18).
    pub async fn subscribe(&self, thread: ThreadId, after: i64)
        -> Result<impl futures::Stream<Item = Result<Delivery, CoreError>> + Send, CoreError>;
}
```

`DurableEvent` is the type `send_journal_after` reads today; use its real name from `src/protocol/sse.rs`. If the loop's shape does not fit a `Stream`, `subscribe` may return a struct with an `async fn next(&mut self) -> Option<Result<Delivery, CoreError>>` instead. Name it in the report. **What must not change:**
- the order of `select!` arms;
- the re-read on the committed-sequence signal;
- the `stopping` check. It stays in `sse.rs`, because it is the transport's.

- [ ] **Step 1: Pin the tests.** `tests/resync.rs`, `tests/disconnect.rs` and every `subscribe` test in `tests/protocol.rs` are the pins. Read their bodies before moving anything.
- [ ] **Step 2: Move the loop.** Read the old and new paths side by side, as §14.8's risk note requires. Delete the last `AppCore` getters. Remove `src/protocol/sse.rs` from `ALLOWLIST`: **it is now empty.** Change the test so that a non-empty `ALLOWLIST` itself fails, by adding `assert!(ALLOWLIST.is_empty())` at the end. Then add §14.5's second rule to `tests/architecture.rs`:

```rust
/// Spec §14.5 rule 2: `AppCore` and its services expose nothing an adapter
/// could use to bypass them — no public or `pub(crate)` field, and no
/// public or `pub(crate)` method whose return type names a FORBIDDEN type.
#[test]
fn the_core_exposes_no_storage() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src/core"), &mut files);
    let types = ["Storage", "Sessions", "LiveHandles", "Runtime"];
    let mut problems = Vec::new();
    for file in files {
        let parsed = syn::parse_file(&std::fs::read_to_string(&file).unwrap()).unwrap();
        for item in &parsed.items {
            match item {
                syn::Item::Struct(s) => for f in &s.fields {
                    if !matches!(f.vis, syn::Visibility::Inherited) {
                        problems.push(format!("{}: {} has a visible field", file.display(), s.ident));
                    }
                },
                syn::Item::Impl(i) => for it in &i.items {
                    if let syn::ImplItem::Fn(m) = it
                        && !matches!(m.vis, syn::Visibility::Inherited)
                    {
                        let ret = quote_ret(&m.sig.output);
                        if types.iter().any(|t| ret.contains(t)) {
                            problems.push(format!("{}: {} returns {ret}", file.display(), m.sig.ident));
                        }
                    }
                },
                _ => {}
            }
        }
    }
    assert!(problems.is_empty(), "the core exposes what adapters must not reach (§14.5):\n{problems:#?}");
}

fn quote_ret(out: &syn::ReturnType) -> String {
    match out {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => {
            let mut s = proc_macro2::TokenStream::new();
            syn::__private::ToTokens::to_tokens(ty.as_ref(), &mut s);
            s.to_string()
        }
    }
}
```

`CoreParts` is the one struct whose fields are public by design, because tests assemble it. Exempt it by name (`s.ident == "CoreParts"`) and say so in a comment. If `syn::__private::ToTokens` is not reachable, use `prettyplease`, or add `quote = "1"` to `[dev-dependencies]`, which is already in the graph through `syn`, and say which in the report. Break the test once, by making a service field `pub`, to prove it bites.
- [ ] **Step 3: Run the gate.** The pins pass unchanged.
- [ ] **Step 4: Write `events.yaml`.** Among its obligations:
  - no frame is lost or repeated across the switch from journal to live;
  - frames follow `durable_seq` order;
  - `after` resumes exactly;
  - Events emits nothing of its own.

  Each has `tested_by` naming a test from `tests/resync.rs` or `tests/protocol.rs`.
- [ ] **Step 5: Commit.** `refactor(core): Events owns subscription and delivery; the adapters no longer reach past the core (§14.5)`

---

### Task 8: Contracts complete, and the documents that describe the core

**Files:**
- Review: all eight `docs/codebase/contracts/*.yaml` exist and `tests/contracts.rs` passes.
- Modify: `CLAUDE.md`.
  - The ownership table gains `core/`: "the application's operations, one method per operation".
  - Two rules join "Rules (project-specific)":
    - a new operation is a new method on one service, with its line in that service's contract;
    - a route or tool translates and holds no rule.
- Modify: `docs/codebase/README.md`.
  - `src/core/` and each service get their rows.
  - `src/protocol/` becomes "translating HTTP and SSE to core calls" (no "and": write "the HTTP surface over the core"), and `src/mcp/` becomes "the MCP surface over the core".
  - `src/mcp/tools.rs` becomes "each MCP tool's call into the core".
- Modify: `docs/superpowers/specs/2026-09-26-application-core-design.md`. Status: "Built" with the commit range.
- Modify: `docs/status.md`. Milestone 2.5 is built, and the Windows run is next.

- [ ] **Step 1: Read each contract against its service** once, to check that `functions` are complete and every `not_the_caller's` is true.
- [ ] **Step 2: Update the documents above, regenerate the code map, and run the gate.**
- [ ] **Step 3: Commit.** `docs: the application core in CLAUDE.md, the code map and status (§14.10)`

---

### Task I: Whole-branch review, Mohammed's run, merge (controller)

1. **Whole-branch review.** Codex `gpt-6-sol` reads `main..milestone-2.5/app-core`, fixes what it finds, and reports. It checks:
   - every §14.7 rule is present in its service;
   - no fingerprint changed;
   - `detached` wraps the same calls as before;
   - no service returns an adapter type.
2. **Mohammed's run on the debug build**, §14.9:
   1. send a message and stop it;
   2. restart the daemon, and the conversation is still there;
   3. edit and approve a plan;
   4. Connect an external Claude Code, read a plan, then Revoke.

   Write `docs/evidence/milestone2_5/WINDOWS_RUN.md`.
3. **The PR,** when Mohammed says. After merge, delete the branch locally and on GitHub.
