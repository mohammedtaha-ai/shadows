# Repository discovery and language servers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** The Planner and person see the actual repository and query a configured PHP language server through Code.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, serde/utoipa; existing React client consumes the shared API.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** None; PHP real-server trial is required before claiming PHP ready.

## Global Constraints

- AppCore remains the only composition root.
- One build cache per workspace, built over in place.
- No worktree build, CARGO_TARGET_DIR, cargo clean, release/custom profile/target or changed RUSTFLAGS.
- Commit each completed verified task separately.
- Full gate and whole-branch review run once per delivery branch before its PR.
- Keep globs as permission boundaries where needed. They never replace the named file work and behavioral brief.
- Do not build a JSON Schema equivalence/fingerprint engine.
- No source-copy store, hashed execution packets, general read leases or Plans/Runs acknowledgement protocol.
- Source contracts describe implemented behavior only. Each task updates its affected contracts/maps in the same commit.
- Use owner public APIs, including *_in methods over core-db::Tx; no cross-service SQL. Migration names below reserve 0054..0061 on the inspected base; if another branch consumes one, renumber only the unshipped migration and record that ruling before code.
- New dependencies are infrastructure and the commit says so. Use async-lsp 0.2.4 for the first LSP client trial, not copied Serena/Claude internals.
- Existing testing1 and user databases are not destructive acceptance fixtures.

## Review Focus

- **B-96 / `inventory_partial_scan_keeps_unseen_files`:** A failed directory read must not imply deletion.
- **B-96 / `inventory_redacts_dotenv_and_rejects_junction_escape`:** A visible filename must not disclose secret content or another root.
- **B-97 / `server_catalog_preserves_uppercase_extensions`:** C/C++ extension casing must survive Windows parsing.
- **B-98 / `semantic_query_rejects_changed_source_and_same_name_guess`:** A stale or ambiguous symbol cannot direct a guessed edit.
- **B-99 / `planner_files_scope_does_not_grant_executor_global_search`:** The Planner's broader view must not leak through executor tools.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: B-96 Inventory independent of parser support

**Owner:** Code.

**Files:**
- Create: `crates/core-code/src/inventory.rs`
- Create: `crates/core-code/src/store/inventory.rs`
- Create: `crates/core-db/migrations/0054_code_inventory.sql`
- Modify: `crates/core-code/src/lib.rs`
- Modify: `crates/core-code/src/model.rs`
- Modify: `crates/core-code/src/scan.rs`
- Modify: `crates/core-code/src/watch.rs`
- Modify: `crates/core-code/src/store/mod.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-code/contract.yaml`
- Test: `crates/shadows-core/tests/code_index.rs`

**Interfaces:**
- Consumes: Code::new/storage/projects and the existing core_with_project() fixture.
- Produces: Code::inventory(&self, asker: Asker<'_>, query: InventoryQuery) -> Result<InventoryPage, CodeError>; Code::refresh_inventory(&self, project: &ProjectId) -> Result<InventoryPage, CodeError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `inventory_lists_php_and_manifests_without_parser_support`, `inventory_partial_scan_keeps_unseen_files`, `inventory_redacts_dotenv_and_rejects_junction_escape`. Create composer.json, app/Models/User.php, .env with a canary, and a directory whose scan is forced incomplete. Query with limit 1, then continue the returned cursor. Repeat after changing the scan generation. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert!(paths.contains(&"composer.json"));
assert!(paths.contains(&"app/Models/User.php"));
assert!(!serialized.contains("SECRET_CANARY"));
assert_eq!(stale_cursor.code(), "inventory_stale");
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test code_index`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Separate inventory traversal/filtering from tag eligibility, using one Code-owned scan path. Store excluded metadata without contents, normalize/order paths, never follow escapes, and preserve prior entries after incomplete scans. Do not change shadows-index language support.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test code_index`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-code/src/inventory.rs crates/core-code/src/store/inventory.rs crates/core-db/migrations/0054_code_inventory.sql crates/core-code/src/lib.rs crates/core-code/src/model.rs crates/core-code/src/scan.rs crates/core-code/src/watch.rs crates/core-code/src/store/mod.rs docs/codebase/README.md docs/backend/stages.md crates/core-code/contract.yaml crates/shadows-core/tests/code_index.rs
git commit -m "feat(core): inventory independent of parser support"
```

### Task 2: B-97 Server catalog and controlled process lifecycle

**Owner:** Code.

**Files:**
- Create: `crates/core-code/src/lsp/catalog.rs`
- Create: `crates/core-code/src/lsp/mod.rs`
- Create: `crates/core-code/src/lsp/worker.rs`
- Create: `crates/core-code/src/lsp/catalog.json`
- Create: `crates/core-db/migrations/0055_code_lsp_config.sql`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/core-code/Cargo.toml`
- Modify: `crates/core-code/src/lib.rs`
- Modify: `crates/core-code/src/model.rs`
- Modify: `crates/core-code/src/store/mod.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-code/contract.yaml`
- Test: `crates/core-code/src/lsp/worker.rs`

**Interfaces:**
- Consumes: Inventory project root and shadows_process::{ProcessSpec,spawn}; ProcessHandle::take_stdio.
- Produces: Code::servers(&self, project: &ProjectId) -> Result<Vec<LspStatus>, CodeError>; Code::configure_server(&self, command: String, project: &ProjectId, config: LspConfig) -> Result<LspStatus, CodeError>; private LspWorker::start(root: PathBuf, config: LspConfig) -> Result<Self, CodeError> and shutdown(&mut self) -> Result<(), CodeError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `server_catalog_preserves_uppercase_extensions`, `server_is_ready_only_after_initialize_and_query`, `server_shutdown_reaps_owned_child`. Load all 13 descriptors from the pinned catalog. Start the real Intelephense server (ignored unless it is installed) for initialize, an unsupported capability and a killed process; the child is spawned through shadows-process. Configure one absolute executable with a space in its path. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(catalog.len(), 13);
assert!(clangd.extensions.iter().any(|x| x.extension == ".C"));
assert_eq!(before_query.state, LspState::Configured);
assert_eq!(after_query.state, LspState::Ready);
assert!(!child_still_alive);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p core-code lsp`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Pin async-lsp 0.2.4, use its new_client/run_buffered driver and tokio-util compat for ProcessHandle streams. Catalog entries retain upstream source pins; private descriptor fields are id/extensions/command/args/timeout/install_hint. Keep one lazy worker per project/backend. Persist user configuration separately from catalog. No installer or new process implementation; the commit explains infrastructure dependency use. Add core-code's shadows-process dependency, async-lsp's compatible lsp-types version and tokio-util compat; do not spawn via tokio::process directly.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p core-code lsp`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-code/src/lsp/catalog.rs crates/core-code/src/lsp/mod.rs crates/core-code/src/lsp/worker.rs crates/core-code/src/lsp/catalog.json crates/core-db/migrations/0055_code_lsp_config.sql Cargo.toml Cargo.lock crates/core-code/Cargo.toml crates/core-code/src/lib.rs crates/core-code/src/model.rs crates/core-code/src/store/mod.rs docs/codebase/README.md docs/backend/stages.md crates/core-code/contract.yaml
git commit -m "feat(core): server catalog and controlled process lifecycle"
```

### Task 3: B-98 Verified semantic queries with bounded results

**Owner:** Code.

**Files:**
- Create: `crates/core-code/src/semantic.rs`
- Create: `crates/core-code/src/source.rs`
- Create: `crates/shadows-core/tests/code_semantic.rs`
- Modify: `crates/core-code/src/lib.rs`
- Modify: `crates/core-code/src/model.rs`
- Modify: `crates/core-code/src/error.rs`
- Modify: `crates/core-code/Cargo.toml`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-code/contract.yaml`
- Test: `crates/shadows-core/tests/code_semantic.rs`

**Interfaces:**
- Consumes: B-96 inventory and B-97 worker/capabilities; existing Code scope validation.
- Produces: Code::semantic(&self, asker: Asker<'_>, query: SemanticQuery, read_scope: Option<&[String]>) -> Result<SemanticAnswer, CodeError>; private source observations use existing sha2 infrastructure, not packet hashes.

- [ ] **Step 1: Write the failing tests**

In the named test module add `semantic_query_rejects_changed_source_and_same_name_guess`, `semantic_result_filters_scope_before_excerpt`, `semantic_result_caps_hits_and_utf8_bytes`. A fixture workspace has two classes with method findCustomer. Select the file/name; return two candidates when that file itself is ambiguous. Change the file's bytes between the query and the real server's answer; return stale. Force a result outside read_scope and one outside the root. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(ambiguous.state, SemanticState::Ambiguous);
assert_eq!(changed.state, SemanticState::Stale);
assert!(answer.hits.iter().all(|h| allowed.contains(&h.path)));
assert!(answer.hits.len() <= 50);
assert!(excerpt_bytes <= 32768);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test code_semantic`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Synchronize document bytes before querying, map one-based public lines to zero-based LSP positions, validate returned locations against source and generation, and truncate on UTF-8 boundaries. Return unsupported/unavailable distinctly. Name fallback stays labeled; diagnostics never imply passing tests.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test code_semantic`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-code/src/semantic.rs crates/core-code/src/source.rs crates/shadows-core/tests/code_semantic.rs crates/core-code/src/lib.rs crates/core-code/src/model.rs crates/core-code/src/error.rs crates/core-code/Cargo.toml docs/codebase/README.md docs/backend/stages.md crates/core-code/contract.yaml
git commit -m "feat(core): verified semantic queries with bounded results"
```

### Task 4: B-99 Repository and LSP APIs reach the Planner

**Owner:** Code, Grants and adapters.

**Files:**
- Modify: `crates/shadows-http/src/code.rs`
- Modify: `crates/shadows-http/src/lib.rs`
- Modify: `crates/shadows-http/src/openapi.rs`
- Modify: `crates/shadows-http/src/code_tests.rs`
- Modify: `crates/shadows-mcp/src/code_tools.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `crates/core-harness/src/setup.rs`
- Modify: `crates/shadows-core/src/lib.rs`
- Modify: `api/openapi.json`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-harness/contract.yaml`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-http/src/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Test: `crates/shadows/tests/planner_mcp.rs`

**Interfaces:**
- Consumes: B-96..B-98 service signatures and shared agreement Code routes.
- Produces: project_files/project_refresh/project_servers/project_question for Planner grants; HTTP inventory/server/question routes exactly as shared agreement.

- [ ] **Step 1: Write the failing tests**

In the named test module add `planner_files_scope_does_not_grant_executor_global_search`, `http_and_mcp_repository_answers_match`. Call HTTP and Planner MCP against the same registered temporary project. An executor asks for project_files; it must be refused. A configured-but-unstarted server is shown configured, not ready. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(http_paths, planner_paths);
assert!(!executor_tools.contains(&"project_files"));
assert_eq!(server_state, "configured");
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows --test planner_mcp --test code_scope`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Handlers translate one Code call; grants own allowlists. Planner opening instructions require inspecting real files/manifests before detailed paths. Register serialized models through shadows-core reexports and regenerate OpenAPI. Preserve existing project-linked query rules.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows --test planner_mcp --test code_scope`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/shadows-http/src/code.rs crates/shadows-http/src/lib.rs crates/shadows-http/src/openapi.rs crates/shadows-http/src/code_tests.rs crates/shadows-mcp/src/code_tools.rs crates/shadows-mcp/src/server.rs crates/core-grants/src/tools.rs crates/core-harness/src/setup.rs crates/shadows-core/src/lib.rs api/openapi.json docs/codebase/README.md docs/backend/stages.md crates/core-harness/contract.yaml crates/core-grants/contract.yaml crates/shadows-http/src/contract.yaml crates/shadows-mcp/src/contract.yaml crates/shadows/tests/planner_mcp.rs
git commit -m "feat(core): repository and lsp apis reach the planner"
```

## Slice acceptance and handoff

Complete the companion frontend task before declaring the slice user-runnable. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the paired web gate after API regeneration. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

