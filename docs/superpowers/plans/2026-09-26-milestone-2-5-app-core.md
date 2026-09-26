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
- **The dependency direction** is `cli/`, `protocol/`, `mcp/` → `core/` → the domain modules (`planner/`, `workflow/`, `agent/`, `runtime/`, `project/`, `thread/`, `grant/`) → `storage/`. `core/` never imports `protocol/` or `mcp/`, and the domain modules never import `core/`. (`grant/` does not exist today: the grant types live in `src/mcp/grant.rs`, and Task 2 Step 0 moves them, before `core/plans.rs` first needs them.)
- **The module is named `core`, like Rust's own `core` crate.** Inside `src/`, always write `crate::core::…`, never a bare `core::…` path, and let an adapter's `use` say `crate::core::AppCore`. Tests reach it as `shadows::core::…`.
- **An adapter never names `crate::storage`, not even for `StorageError`.** `src/core/error.rs` re-exports it (`pub use crate::storage::StorageError;`, re-exported from `core/mod.rs`), because `CoreError::Storage` carries it. An adapter that maps it (`failure.rs`, `refusal.rs`) imports `crate::core::StorageError`.
- **Idempotency moves with the operation.** A service builds its own `CommandContext`, with the same `command_kind` strings and the same fingerprint parameters the route or tool builds today. A changed fingerprint breaks replay, and the tests catch it.
- **Detachment stays in the HTTP adapter.** `protocol/conversation.rs::detached` spawns the work, so a client that disconnects does not cancel it. A route that calls it today still calls it, around the service call.
- **Files:** 300 lines needs a stated reason, and at 500 the file splits (CLAUDE.md). A service that grows past 300 splits by responsibility into `core/<service>/`.
- Every signature change regenerates the code map in the same commit: `UPDATE_CODEMAP=1 cargo test --test codemap`. A new module gets its one-job line in `docs/codebase/README.md`. `the_ownership_map_accounts_for_every_module` fails on a top-level module with no row (`src/core/`, `src/grant/`), on a row whose module or reference file no longer exists (`src/protocol/ui_signal.rs` after Task 1, `src/mcp/grant.rs` after Task 2), and on a job stated with " and ". So a task that creates, moves or deletes a file fixes its README row in the same commit.
- **The gate before every commit:**
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - Report the test counts. At the start: 304 Rust and 135 web; the web count must not move.
- **Builds use the C: target directory:**
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/Temp/claude/E--Globalprojects-shadows/e86a610a-4c5b-4114-884d-faf050ce6a96/scratchpad/target CARGO_INCREMENTAL=0`,
  because E: fills up. Stop any running `shadows.exe` preview before `cargo test`, or it cannot overwrite the binary.
- **Each task writes or extends its service's contract**, `docs/codebase/contracts/<service>.yaml`, from `TEMPLATE.yaml`. It records every rule the task moved (§14.7) with `tested_by`. §14.7's rules need a real test; any other `none` or `unknown` needs a `note`.
  - **One deviation from the template's shape, which §14.6 makes:** the template puts its function groups (`reads:`, `writes:`, `checks:`) at the top level; a Shadows contract nests them under one `functions:` key, because that is the key `tests/contracts.rs` reads. A group placed at the top level is invisible to rules 3 and 5.
  - `source` names the service's file (`src/core/plans.rs`). If a service splits into `src/core/<service>/`, `source` names the file that holds its `impl` with the public methods.

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
Task 2  grant types move to src/grant/; Plans (protocol/workflow.rs, mcp/tools.rs)
Task 3  Grants         (protocol/grants.rs, mcp/auth.rs, core startup)
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
- Modify: `Cargo.toml` (`[dev-dependencies]`: `yaml-rust2 = "0.13"`, with a comment naming `tests/contracts.rs` as its one user, as the file's other dev-dependencies have). If `0.13` does not resolve, take the newest `yaml-rust2` that does and name it in the report.

**Interfaces:**
- Produces: `ALLOWLIST` in `tests/architecture.rs`, which later tasks shrink. And the contract rules later tasks' contracts must pass.

- [ ] **Step 1: Write `tests/architecture.rs`.** It walks `src/protocol/`, `src/mcp/` and `src/cli/`, tokenizes each file with `proc_macro2`, and fails on a forbidden identifier. Comments are not tokens, and doc comments become string literals, so prose never trips it. One identifier is not a bypass: an error enum's variant (`CoreError::Storage`, `StartError::Storage`, `OpenError::Storage`), which names a failure the adapter maps, not a way past the core. Without that exemption `failure.rs` could never leave the allowlist, because it must map `CoreError::Storage`.

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

/// Every identifier in `tokens`, except an error enum's variant
/// (`CoreError::Storage`, `StartError::Storage`): that names a failure the
/// adapter maps, not a way past the core.
fn idents(tokens: TokenStream, found: &mut Vec<String>) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let colon = |t: &TokenTree| matches!(t, TokenTree::Punct(p) if p.as_char() == ':');
    for (n, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(i) => {
                let error_variant = n >= 3
                    && colon(&trees[n - 1])
                    && colon(&trees[n - 2])
                    && matches!(&trees[n - 3], TokenTree::Ident(e) if e.to_string().ends_with("Error"));
                if !error_variant {
                    found.push(i.to_string());
                }
            }
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

The table below was read from `main` at `ddcaed5` with this test's rule applied (fifteen files):

| File | Removed by |
|---|---|
| `src/protocol/mod.rs`, `src/mcp/mod.rs`, `src/cli/mod.rs` | Task 1 |
| `src/protocol/failure.rs`, `src/mcp/refusal.rs` (each imports `crate::storage::StorageError`) | Task 1, which switches them to `crate::core::StorageError` |
| `src/protocol/workflow.rs`, `src/mcp/tools.rs` | Task 2 |
| `src/protocol/grants.rs`, `src/mcp/auth.rs` | Task 3 |
| `src/protocol/harness.rs` | Task 5 |
| `src/protocol/conversation.rs` | Task 6. Task 4 moves start and stop, and Task 6 moves the lists. |
| `src/protocol/project.rs`, `src/protocol/thread.rs`, `src/protocol/instructions.rs` | Task 6 |
| `src/protocol/sse.rs` | Task 7 |

`src/mcp/server.rs`, `src/protocol/fs.rs`, `guard.rs`, `openapi.rs`, `ui_signal.rs` and `src/cli/args.rs` name nothing forbidden and are not on the list. Run again. Expected: PASS. A path the run printed that is not in the table goes to the task whose service owns it, and the report says so.

- [ ] **Step 3: Copy the template to `docs/codebase/contracts/TEMPLATE.yaml` first** (the test's `read_dir` panics on a missing directory), **then write `tests/contracts.rs`.** It loads every `docs/codebase/contracts/*.yaml` except `TEMPLATE.yaml`, parses it with `yaml_rust2::YamlLoader::load_from_str`, and checks §14.6's six rules. The service source is the file named by `source`, and the public methods are collected with `syn`, from `pub fn` and `pub async fn` in that file's `impl` blocks. Test names are collected with `syn` from every `#[test]` or `#[tokio::test]` function in `tests/**/*.rs` and in `src/**/*.rs`.

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
        if let Some(ags) = doc["agreements"].as_vec() { for a in ags {
            if let Some(t) = a["tested_by"].as_str() { named.push(t.to_string()) } } }
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
  - Add `use crate::storage::Storage;` to a file not on the allowlist: `src/protocol/guard.rs` (`failure.rs` is on the list, so it would prove nothing). Run `cargo test --test architecture` and expect FAIL naming `guard.rs`. Revert.
  - Replace that line with `const _X: &str = stringify!(CoreError::Storage);` (`stringify!` compiles whatever tokens it is given, and `CoreError` does not exist yet) and run again: expect PASS, which shows the error-variant exemption holds. Revert.
  - Write a scratch `docs/codebase/contracts/scratch.yaml` with `source: src/config.rs`, `functions: { nope: "() -> ()" }` and one obligation `{ rule: x, tested_by: no_such_test }`. Run `cargo test --test contracts` and expect FAIL naming both `nope` and `no_such_test`. Delete the scratch file.
- [ ] **Step 5: Run the gate.** Expect 306 Rust (304 + 2), all passing.
- [ ] **Step 6: Commit.** `test(architecture): guard the adapters and the service contracts (§14.5, §14.6)`

---

### Task 1: `AppCore`, `CoreError`, and the adapters holding it

**Files:**
- Create: `src/core/mod.rs`: `AppCore`, `CoreParts`, accessors, `start`, `assemble`, `shut_down`
- Create: `src/core/error.rs`: `CoreError`
- Create: `src/core/{projects,threads,turns,harness,plans,grants,instructions,events}.rs`: each an empty struct with no fields and no methods yet (see Step 3 for why empty)
- Modify: `src/lib.rs` (`pub mod core;`)
- Modify: `src/protocol/mod.rs` (`AppState`; `router` mounts `crate::mcp::service(state.core.clone())`), `src/mcp/mod.rs` (`McpState` → `Arc<AppCore>`), `src/mcp/server.rs` (`Shadows` holds `core: Arc<AppCore>`, because `McpState` is gone), `src/mcp/tools.rs` (`self.state.storage` → `self.core.storage()`, and so on), `src/mcp/auth.rs` (`require_grant` takes `State<Arc<AppCore>>` and reads `core.storage()`, so `mcp/mod.rs` names no storage), `src/cli/mod.rs`, and every `protocol/` handler that reads a moved field
- Modify: `src/protocol/failure.rs` (`impl From<CoreError> for Failure`), `src/mcp/refusal.rs` (`impl From<CoreError> for Refusal`); both import `crate::core::StorageError`, no longer `crate::storage::StorageError`
- Move: `UiSignal`, from `src/protocol/ui_signal.rs` to `src/core/events.rs`. `ui_signal.rs` holds nothing else, so it is deleted, with `mod ui_signal;` and `pub use ui_signal::UiSignal;` in `protocol/mod.rs`.
- Modify: `docs/codebase/README.md`: add `src/core/` ("the application's operations, one method per operation", reference `src/core/mod.rs`); delete the `src/protocol/ui_signal.rs` row; add `src/core/events.rs` ("what clients watch live", reference itself), which now holds `UiSignal`.
- Modify: `tests/fixtures/app.rs` and the twelve tests that build `AppState` themselves (`cors`, `debug_log`, `disconnect`, `fs_routes`, `openapi`, `planner_turn`, `project_directory`, `protocol`, `resync`, `shutdown`, `stream_frames`, `thread_routes`). Only their construction and imports change. `tests/fixtures/listening.rs` builds through `test_app_with` and needs no change.
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
    // Held for shut_down; never returned once Task 7 deletes the getters.
    runtime: Arc<Runtime>, handles: Arc<LiveHandles>, sessions: Arc<Sessions>,
    // Held only for Task 1's temporary getters. Each goes with its last getter
    // (an unread private field fails clippy), and Task 7 removes the last.
    storage: Arc<Storage>, bus: Bus, ui: tokio::sync::broadcast::Sender<UiSignal>, mcp_url: String,
}

/// The one `CommandContext` a person's command carries: principal `User`,
/// id `local`, schema version 1, the fingerprint of `params`. The same body
/// as `protocol::project::ctx` today, so no fingerprint moves. Every service
/// that takes a person's `command_id` builds its context here.
pub(crate) fn user_command(command_id: String, kind: &str, params: serde_json::Value) -> CommandContext;

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
    /// Spec §8.5 through `planner::shut_down(runtime, handles, sessions,
    /// confirm_within, escalate)`, unchanged: stop every running turn, close
    /// every adapter, record the stop.
    pub async fn shut_down(
        &self,
        bound: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> Result<StopKind, CoreError>;
}
```

`planner::shut_down` returns `Result<StopKind, StorageError>` (`src/planner/shutdown.rs`); `StopKind` is `storage::StopKind`. The error becomes `CoreError::Storage`, whose `Display` is transparent, so `shutdown.unrecorded`'s `%error` text is unchanged.

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

- `AppState` becomes `{ core: Arc<AppCore>, allowed_origins: Vec<String>, shutdown: watch::Receiver<bool> }`, still `#[derive(Clone)]`. `mcp_url` moves into `CoreParts`, because `Grants::issue` builds the command from it (Task 3). This amends §14.4's `AppState` sketch, and Step 7 amends the spec text to match.
- **Startup order changes in one place, and §14.3/§14.4 are amended for it (Step 7).** Today `serve` opens storage, recovers, revokes thread grants and reads the harness versions, and only then binds. The sessions need `mcp_url`, which names the address actually bound (port 0 in tests), so `serve` now binds first and then calls `AppCore::start(&config, mcp_url)`. Nothing is served until `start` returns, so no client sees the difference; a bind failure now happens before recovery rather than after it.
- `McpState` is replaced by `Arc<AppCore>`. `mcp::service(core: Arc<AppCore>) -> Router`.

- [ ] **Step 1: Write the failing test `tests/core.rs`.**

```rust
//! Spec §14.3, §14.4: the application is one `AppCore`, and both adapters
//! and shutdown reach it through that.

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{call, default_settings, names, start_on, test_app, wait_terminal};
use shadows::storage::StopKind;

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
    let kind = app
        .core
        .shut_down(std::time::Duration::from_secs(10), std::future::pending())
        .await
        .unwrap();
    assert_eq!(kind, StopKind::Graceful);
    let done = wait_terminal(&app, &op).await;
    assert_eq!(done.status_kind, "Cancelled");
}
```

Tests include fixtures by `#[path]`, as every existing test does; there is no `tests/fixtures/mod.rs`. `Operation`'s state is the string `status_kind` (as `tests/shutdown.rs` asserts it). If `StopKind` does not derive `PartialEq`, match on it instead of adding a derive. The fixture's `App` gains `pub core: Arc<AppCore>`, built with `AppCore::assemble(CoreParts { … })` from the same `Arc`s it already holds, and its router is built from `AppState { core: core.clone(), … }`. Every other field of `App` stays, so the existing uses of `app.storage` and the rest compile unchanged.

- [ ] **Step 2: Run it.** `cargo test --test core`. Expected: FAIL to compile (`shadows::core` does not exist).
- [ ] **Step 3: Implement `src/core/`.**
  - **The shells are empty in this task** (`pub struct Plans {}` and so on), because a private field nothing reads fails `cargo clippy -D warnings`. Each later task adds, in `assemble`, the fields its methods read, and only those:
    - `Plans`: storage, handles, ui
    - `Grants`: storage, mcp_url
    - `Turns`: storage, runtime, handles, sessions, bus
    - `Harness`: storage, sessions
    - `Projects`: storage
    - `Threads`: storage, sessions
    - `Instructions`: storage
    - `Events`: storage, sessions, bus, ui, and a `Harness` for the `options` frame (Task 7 calls `Harness::choices`; one capability, one method). So `Harness` derives `Clone`, and holds only `Arc`s.
  - `user_command` (above) is not written in this task: it would be dead code, which clippy refuses. Task 2 adds it with its first caller, `Plans::approve`. `protocol::project::ctx` stays until Task 6 removes its last caller, then is deleted.
  - `start` moves `serve`'s pre-bind body out of `cli/mod.rs`, including `harness_version`. Keep it in `src/core/mod.rs`, or in `src/core/start.rs` if `mod.rs` passes 300 lines.
  - `serve` becomes: bind, then `AppCore::start(&config, mcp_url)`, print the address, and serve `protocol::router(AppState { core, allowed_origins, shutdown })`. The graceful-shutdown future calls `core.shut_down(CONFIRMATION_BOUND, second_signal)` and then `stopping.send_replace(true)`, in today's order, keeping every log line (`shutdown.recorded`, `shutdown.unrecorded`, `shutdown.signal_unavailable`).
  - Handlers keep working this task by reading through the core. Add temporary `pub(crate)` getters on `AppCore` with exactly these names:
    - `storage()`, `sessions()`, `handles()`, `runtime()` and `bus()`, each returning its `&Arc<…>` (`bus()` its `&Bus`);
    - `ui_bus()` for the UI sender;
    - `mcp_url()` returning `&str`, for `protocol/grants.rs` until Task 3.

    Change `s.storage` to `s.core.storage()` in each handler and `self.state.storage` to `self.core.storage()` in each tool. **These getters are the allowlist's shadow:**
    - every handler that uses one still names a `FORBIDDEN` identifier, so its file stays on `ALLOWLIST`;
    - each later task deletes the getters its routes no longer need;
    - Task 7 deletes the last one, and `the_core_exposes_no_storage` (Task 7) proves they are gone.
  - `UiSignal` moves now from `src/protocol/ui_signal.rs` to `src/core/events.rs`, with its fields and derives unchanged, because `CoreParts` names it and `core/` must not import `protocol/`. Update its imports in `mcp/tools.rs`, `protocol/sse.rs` and `tests/fixtures/app.rs`. The struct is all `ui_signal.rs` holds, so the file is deleted and its README row with it.
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
- [ ] **Step 5: Update the fixture and the twelve test files' `AppState` literals.** No assertion changes. Tests that import `shadows::protocol::UiSignal` import `shadows::core::UiSignal` (a re-export from `core/mod.rs`).
- [ ] **Step 6: Run the gate.** Expect 308 Rust. `tests/architecture.rs` must pass: `protocol/mod.rs`, `mcp/mod.rs`, `cli/mod.rs`, `protocol/failure.rs` and `mcp/refusal.rs` now name only `core` types, so remove them from `ALLOWLIST` (its stale check fails until you do).
- [ ] **Step 7: Amend the spec.**
  - §14.4: the `AppState` sketch shows `mcp_url` in the core, not in `AppState`.
  - §14.3 and §14.4: `AppCore::start(config, mcp_url)`, and `cli/` binds the listener before calling it, because the sessions need the bound address (see the startup-order note above).
  - §14.3: record `CoreError` in one sentence.
- [ ] **Step 8: Commit.** `feat(core): AppCore holds the application; adapters hold only Arc<AppCore> (§14.3, §14.4)`

---

### Task 2: the grant types move, then `Plans`

**Files:**
- Move (Step 0): `src/mcp/grant.rs` → `src/grant/mod.rs`, contents unchanged. `Grant`, `GrantId`, `GrantKind`, `IssuedGrant`, `Token` and `hash_token` are domain types: `storage/sqlite/grant.rs`, `command/mod.rs` and `planner/setup.rs` use them, and `core/plans.rs` is about to, which must not import `mcp/`. Add `pub mod grant;` to `src/lib.rs` and drop `pub mod grant;` from `src/mcp/mod.rs`. Update every `crate::mcp::grant` import in `src/` (`command/mod.rs`, `planner/setup.rs`, `storage/sqlite/grant.rs`, `mcp/server.rs`, `mcp/tools.rs`, `protocol/grants.rs`), and `shadows::mcp::grant` in `src/bin/fake_acp.rs`, `tests/fixtures/listening.rs`, `tests/fixtures/plan.rs` and `tests/grants.rs`. README: replace the `src/mcp/grant.rs` row with `src/grant/` ("who may do what on `/mcp`", reference `src/grant/mod.rs`).
- Modify: `src/core/plans.rs`, `src/core/mod.rs` (`user_command`, and `Plans`' fields in `assemble`)
- Modify: `src/protocol/workflow.rs` (`list_plans`, `get_plan`, `approve_plan` → one call each)
- Modify: `src/mcp/tools.rs` (every tool → one call). `plan_in_scope`, `planner_draft`, `external_draft`, `edit`, `show`, `own_thread`, `writer_of` and `command` move into `Plans` unchanged in logic and text.
- Create: `docs/codebase/contracts/plans.yaml`
- Test: `tests/core_plans.rs` (new)

**Interfaces:**
- Consumes: `CoreError::Refused` (Task 1), `Grant` and `GrantKind` from `crate::grant` (Step 0).
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

`get_for`'s old `latest_only` flag stays private: `edit` calls the private `in_scope(grant, named, true)`, and `get_for` and `task_for` call it with `false`. Every refusal text moves verbatim. `Refusal::scope(msg)` becomes `CoreError::Refused { code: ErrorCode::GrantScope, message }`, and `Refusal::new(code, msg)` becomes `CoreError::Refused { code, message }`. A `StorageError` a tool returns today (`own_thread`'s `StorageError::GrantScope`, `external_draft`'s `StorageError::PlanInvalid`) stays a `StorageError`, carried as `CoreError::Storage`, so `Refusal`'s existing `From<StorageError>` still writes its text. `edit` returns `EditOutcome` and `start_draft` returns `DraftStarted`, where the tools return `json!(…)` of them today: the serialized answer is the same.

The fingerprints that must not move: `"PlanApprove"` { workflow, expected_revision }; `"DraftStart"` { thread, title, goal } (Planner, anchored to the running turn) and { title, goal, from_workflow_id } (external, anchored to the `draft_ref`); `"PlanEdit"` { workflow, expected_revision, ops }; `"PlanShow"` { workflow, task_number, place }. Copy the `json!` literals, do not retype them.

- [ ] **Step 0: Move the grant types** (Files above), run the gate, and commit alone: `refactor(grant): the grant types are domain types, not the MCP server's`. Nothing else changes, so a failure here is an import.
- [ ] **Step 1: Write the failing tests `tests/core_plans.rs`.** They pin the moved behaviour through the adapters, which is what a client sees. Use the existing helpers in `tests/fixtures/plan.rs` and `tests/fixtures/listening.rs`.

```rust
//! Spec §14.3, §14.7: plan operations answer the same after moving into
//! `Plans` — a replay is the first answer, and a grant's scope still holds.

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

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
    // A new draft is at revision 0 (tests/mcp_tools.rs edits it so).
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

Match the `use` lines to what `listening.rs` needs beside it (it names `super::acp`, `super::app` and `super::plan`); read an existing user such as `tests/mcp_tools.rs` first.

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

  Delete the `AppCore` getters no route or tool uses any more, and any `AppCore` field only they read.
- [ ] **Step 4: Remove `src/protocol/workflow.rs` and `src/mcp/tools.rs` from `ALLOWLIST`, then run the gate.** Expect 310 Rust. Rerun `tests/core_plans.rs` and the existing `tests/mcp_tools.rs`, `tests/plan_grants.rs`, `tests/plan_storage.rs`, `tests/planner_mcp.rs` and `tests/plan_in_conversation.rs`, all unchanged.
- [ ] **Step 5: Write `docs/codebase/contracts/plans.yaml`** from the template. Its obligations include:
  - grant scope, with `tested_by` the scope tests in `tests/plan_grants.rs`;
  - replay;
  - the UI signal sent only when not replayed, with `tested_by` the `plan_show` test in `tests/plan_in_conversation.rs`.

  `not_the_caller's` includes: "check a grant's scope before calling a `*_for` method".
- [ ] **Step 6: Commit.** `refactor(core): Plans owns every plan operation, HTTP and MCP alike (§14.3)`

---

### Task 3: `Grants`

**Files:**
- Modify: `src/core/grants.rs`, `src/protocol/grants.rs`, `src/mcp/auth.rs`, `src/core/mod.rs` (`start` calls `grants.revoke_thread_grants()` where it calls storage today, keeping the `recovery.thread_grants_revoked` log line; the `mcp_url()` getter goes)
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

`protocol/grants.rs`'s `IssuedGrantBody` keeps its name and fields, so OpenAPI is unchanged, and it is built from `IssuedView`. `mcp/auth.rs::require_grant` already takes `State<Arc<AppCore>>` (Task 1); it now calls `core.grants().authorize(token)`, keeping its three answers (401, 401, 503) and its log lines.

- [ ] **Step 1: Failing test**, in `tests/core.rs`: `revoked_grant_is_refused_after_the_move`. Read the exact route paths in `protocol/grants.rs`'s `#[utoipa::path]`s first.
  - issue a grant through the issue route over HTTP, and take the token from its body;
  - connect with `listening::mcp_client(&l.base, &token)` and call `workflow_list`, which succeeds;
  - revoke it through the revoke route (`DELETE`, `command_id` in the query);
  - a raw `POST {base}/mcp` with that bearer answers 401. Do not use `mcp_client` for this step: it panics when the connection fails. Follow `tests/mcp_server.rs::a_revoked_token_is_401`, which already sends such a request.

  `tests/core.rs` then also includes `fixtures/listening.rs` and `fixtures/plan.rs`. Run it; it passes before the move, and this pins it.
- [ ] **Step 2: Move the logic.** Delete the getters no longer needed. Remove `src/protocol/grants.rs` and `src/mcp/auth.rs` from `ALLOWLIST`.
- [ ] **Step 3: Run the gate.** `tests/grants.rs`, `tests/plan_grants.rs` and `tests/mcp_server.rs` pass unchanged.
- [ ] **Step 4: Write `grants.yaml`.** Among its obligations:
  - the token is stored only as its hash;
  - a replayed issue returns no token;
  - thread grants die at startup.
- [ ] **Step 5: Commit.** `refactor(core): Grants owns issue, revoke and the bearer check (§14.3)`

---

### Task 4: `Turns` (the risk task)

**Files:**
- Modify: `src/core/turns.rs`. `start`, the `Turn` struct, `record`, `offer_for_model` and `focus_block`'s call move from `protocol/conversation.rs`, and `stop_turn`'s body moves too. The command is built with `user_command` (Task 2), whose body is `protocol::project::ctx`'s, so the `"turn.start"` fingerprint does not move.
- Modify: `src/core/mod.rs` (`Turns`' fields in `assemble`; delete the getters and fields no adapter reads any more)
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

The route clones the `Arc<AppCore>` into the detached task: `let core = s.core.clone(); detached(async move { core.turns().send(thread_id, turn).await.map_err(Failure::from) })`. `stop_turn` does the same with `core.turns().stop(&op_id)`.

- [ ] **Step 1: Failing tests `tests/core_turns.rs`.** They pin behaviour and pass before the move.
  - `turn_start_replays_after_the_move`: the same `command_id` posted twice gives one operation id, `202` both times.
  - `stop_after_the_move_records_cancelled`: start with prompt `hang`, stop it, and the operation is `Cancelled`.
  - `a_second_start_while_busy_is_thread_busy`: start `hang`, and a second start with a new `command_id` gets 409 `THREAD_BUSY`.

  Use `start_on`, `http_start`, `wait_terminal` and `fresh_command` from `tests/fixtures/app.rs`, included by `#[path]` as in `tests/core.rs`. Stop through the route `POST /api/operations/{id}/stop`, not `PlannerTurn::stop`, so the test crosses the adapter.
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

`Harness` gains its fields (storage, sessions) in `assemble` and derives `Clone`, because `Events` holds one (Task 7). `open_failure` and `choices_for` have callers in `protocol/conversation.rs` (Task 4 moved them into `Turns`) and `protocol/sse.rs` (`options_event`, until Task 7): this task switches `sse.rs`'s `options_event` to `s.core.harness().choices(…)`, so `choices_for` is deleted, not left behind (`sse.rs` stays on the allowlist for its other reads).

- [ ] **Step 1: Failing test.** `change_model_is_refused_while_a_turn_runs`: start `hang`, then `PUT /api/threads/{id}/session/model` (the path in `harness.rs`'s `#[utoipa::path]`) with a model the fake offers gives 409 `THREAD_BUSY`, and the running operation still finishes as it would have (stop it and see `Cancelled`), so the session was not touched. It passes before the move.
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

`fs.rs`'s `blocking` helper moves with `list_dirs` and `create_dir` into `Projects`, because it is how the browse calls run and not a transport detail. `known_harness` becomes one `pub(super)` function in `src/core/threads.rs`, which `Projects` also calls (one rule, one place), and returns `CoreError::SettingNotOffered { what: "harness" … }`.

- [ ] **Step 1: Pin the tests.** The existing tests `tests/protocol.rs`, `thread_contract.rs`, `thread_routes.rs`, `fork.rs`, `fs_routes.rs`, `project_directory.rs`, `project_contract.rs`, `planner_instructions.rs` and `storage_contract.rs` cover these routes. Every service command is built with `user_command`, so no fingerprint moves, and `protocol::project::ctx` is deleted once its last caller is gone. Add `project_create_replays_after_the_move` to `tests/core.rs`: the same `command_id` posted twice gives one project.
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
/// One frame's worth of what today's `stream` sends, one per `sse.rs` frame kind.
pub enum Delivery {
    Durable(StoredEvent),                 // one journal event, `seq` order, replay and live alike
    CaughtUp { seq: i64 },
    Delta { op: OperationId, text: String },
    TurnEnd { op: OperationId, subtype: &'static str, stop_reason: Option<String> },
    Usage { thread: ThreadId, used: u64, size: u64, limits: Option<AccountLimits> },
    Options { thread: ThreadId, choices: SessionChoices },
    PlanShow(UiSignal),
    Lagged,
    Fatal(String),                        // the journal could not be read; the stream ends after it
}
impl Events {
    /// Takes the committed-sequence watch, the bus, the options watch and the
    /// UI receiver *before* anything is read, as `sse::subscribe` does today,
    /// then answers the replay, `CaughtUp`, then live (§2.4, §6.18).
    pub fn subscribe(&self, thread: ThreadId, after: i64) -> Subscription;
}
pub struct Subscription { /* private: the four receivers, last_seq, a buffer of journal events */ }
impl Subscription {
    /// The next frame, or `Err(why)` once the stream is over, `why` being the
    /// text `sse.closed` logs today ("client gone" stays in `sse.rs`).
    pub async fn next(&mut self) -> Result<Delivery, &'static str>;
}
```

The journal type is `storage::StoredEvent`, which `Storage::read_events_after` returns, not `events::DurableEvent`. `Usage` carries the limits `usage_event` reads today (`turn_context`, then `latest_limits`), because that read is storage and so belongs in the core. `sse.rs` keeps the framing: event names, JSON bodies, `lagged`, `fatal`, the bounded channel and `sse.subscribe`/`sse.caught_up`/`sse.closed`. **What must not change:**
- the order of the `select!` arms. Today `shutdown` is the first `biased` arm of the same `select!` as the others. It stays in `sse.rs`, because it is the transport's, so `sse.rs` runs `select! { biased; _ = shutdown.wait_for(..) => …, d = sub.next() => … }`, and `next` keeps the rest in today's order: committed, bus, options, signals;
- the re-read on the committed-sequence signal;
- the receivers taken before the replay is read.

- [ ] **Step 1: Pin the tests.** `tests/resync.rs`, `tests/disconnect.rs` and every `subscribe` test in `tests/protocol.rs` are the pins. Read their bodies before moving anything.
- [ ] **Step 2: Move the loop.** Read the old and new paths side by side, as §14.8's risk note requires. Delete the last `AppCore` getters. Remove `src/protocol/sse.rs` from `ALLOWLIST`: **it is now empty.** Change the test so that a non-empty `ALLOWLIST` itself fails, by adding `assert!(ALLOWLIST.is_empty())` at the end. Then add §14.5's second rule to `tests/architecture.rs`:

```rust
/// Spec §14.5 rule 2: `AppCore` and its services expose nothing an adapter
/// could use to bypass them — no public or `pub(crate)` field, and no
/// public or `pub(crate)` method whose return type names a FORBIDDEN type.
/// Only these nine structs: the plain-data types beside them (`SendTurn`,
/// `IssuedView`, `HarnessInfo`, `UiSignal`, `CoreParts` …) have public
/// fields by design, and `CoreParts` exists so tests can assemble the core.
const GUARDED: &[&str] = &[
    "AppCore", "Projects", "Threads", "Turns", "Harness",
    "Plans", "Grants", "Instructions", "Events",
];

#[test]
fn the_core_exposes_no_storage() {
    use quote::ToTokens;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src/core"), &mut files);
    let types = ["Storage", "Sessions", "LiveHandles", "Runtime"];
    let mut problems = Vec::new();
    for file in files {
        let parsed = syn::parse_file(&std::fs::read_to_string(&file).unwrap()).unwrap();
        for item in &parsed.items {
            match item {
                syn::Item::Struct(s) if GUARDED.contains(&s.ident.to_string().as_str()) => {
                    for f in &s.fields {
                        if !matches!(f.vis, syn::Visibility::Inherited) {
                            problems.push(format!("{}: {} has a visible field", file.display(), s.ident));
                        }
                    }
                }
                syn::Item::Impl(i) if i.trait_.is_none()
                    && GUARDED.iter().any(|g| i.self_ty.to_token_stream().to_string() == *g) =>
                {
                    for it in &i.items {
                        if let syn::ImplItem::Fn(m) = it
                            && !matches!(m.vis, syn::Visibility::Inherited)
                            && let syn::ReturnType::Type(_, ty) = &m.sig.output
                        {
                            // Compared by identifier, so `StorageError` is not `Storage`.
                            let mut named = Vec::new();
                            idents(ty.to_token_stream(), &mut named);
                            if named.iter().any(|n| types.contains(&n.as_str())) {
                                problems.push(format!("{}: {} returns a type it must not", file.display(), m.sig.ident));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    assert!(problems.is_empty(), "the core exposes what adapters must not reach (§14.5):\n{problems:#?}");
}
```

This adds `syn` (already a dev-dependency) and `quote = "1"` to `[dev-dependencies]`, which is already in the lock file through `syn`; state it in the report. Break the test twice to prove it bites: make a service field `pub`, then give `AppCore` a `pub(crate) fn storage(&self) -> &Arc<Storage>`. Both must fail.
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
  - The "Built:" module list under Architecture gains `core/` and `grant/`.
  - Two rules join "Rules (project-specific)":
    - a new operation is a new method on one service, with its line in that service's contract;
    - a route or tool translates and holds no rule.
- Modify: `docs/codebase/README.md`.
  - `src/core/` and each service get their rows.
  - `src/protocol/` becomes "the HTTP surface over the core", and `src/mcp/` becomes "the MCP surface over the core". The codemap test refuses a job stated with " and ", so do not write "HTTP and SSE".
  - `src/core/` gets one row per service file, each with its job from §14.3's table.
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
