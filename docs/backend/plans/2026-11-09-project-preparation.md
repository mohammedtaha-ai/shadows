# Planning-time framework preparation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** The Planner prepares an agreed empty destination and receives refreshed real files before detailed planning.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, serde/utoipa; existing React client consumes the shared API.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** B-96 and B-99; no frozen product task prerequisite.

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

- **B-100 / `preparation_replay_keeps_original_destination`:** A repeated command must never launch another installer.
- **B-100 / `preparation_refuses_nonempty_or_escaping_destination`:** Existing work cannot be overwritten.
- **B-101 / `preparation_interruption_preserves_partial_files`:** A restart must expose the partial setup.
- **B-101 / `preparation_does_not_enable_planner_shell`:** Preparation must not silently change normal Planner tools.
- **B-102 / `preparation_report_refreshes_repository`:** Completion must be visible to planning rather than stale inventory.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: B-100 Preparation request, scope and persistence

**Owner:** Runs and Grants.

**Files:**
- Create: `crates/core-runs/src/preparation.rs`
- Create: `crates/core-runs/src/store/preparation.rs`
- Create: `crates/core-db/migrations/0056_preparation.sql`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/model.rs`
- Modify: `crates/core-runs/src/store/mod.rs`
- Modify: `crates/core-grants/src/model.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `crates/core-grants/src/lib.rs`
- Modify: `crates/core-grants/src/store/mod.rs`
- Modify: `crates/core-threads/src/model.rs`
- Modify: `crates/core-threads/src/lib.rs`
- Modify: `crates/core-threads/src/store/thread.rs`
- Modify: `crates/core-grants/src/writer.rs`
- Modify: `crates/core-plans/src/lib.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/core-runs/src/report.rs`
- Modify: `crates/core-turns/src/record.rs`
- Modify: `crates/core-turns/src/planning/mod.rs`
- Modify: `crates/core-turns/src/stage.rs`
- Modify: `crates/core-harness/src/live.rs`
- Modify: `crates/core-harness/src/setup.rs`
- Modify: `crates/core-threads/src/store/records.rs`
- Modify: `crates/shadows-core/tests/executor_session.rs`
- Modify: `crates/core-grants/tests/tokens.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/core-harness/contract.yaml`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-threads/contract.yaml`
- Test: `crates/shadows-core/tests/preparation.rs`

**Interfaces:**
- Consumes: PrepareRequest/Preparation in shared agreement; Projects directory and Threads public source-message lookup.
- Produces: Runs::prepare_for(&self, grant: &Grant, request: PrepareRequest) -> Result<Preparation, RunsError>; Runs::preparation(&self,id:&str) -> Result<Preparation,RunsError>; Runs::preparations(&self,project:&ProjectId) -> Result<Vec<Preparation>,RunsError>. New Preparation grant kind scopes one request/project/destination. Runs::prepare(&self,project:&ProjectId,request:PrepareRequest) -> Result<Preparation,RunsError> is the person variant. Threads::create_preparation_in(conn:&mut core_db::Tx,project:&ProjectId,title:String,harness:&str) -> Result<ThreadId,core_db::StorageError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `preparation_replay_keeps_original_destination`, `preparation_refuses_nonempty_or_escaping_destination`, `preparation_source_message_must_belong_to_planner`. One owned Planner message requests Laravel in backend/. Issue the same command twice, then change destination under that command; also use nonempty and ../outside destinations. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(first.id, replay.id);
assert_eq!(changed.code(), "command_conflict");
assert!(nonempty.is_err());
assert!(foreign_message.is_err());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test preparation`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Validate attributed message through Threads, project canonical path and existing held write overlap through Runs. Persist the attempt before session startup. Add preparation grant scope through Grants public methods; no direct read of another service's tables. Extend startup revoke/recovery kind handling.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test preparation`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/preparation.rs crates/core-runs/src/store/preparation.rs crates/core-db/migrations/0056_preparation.sql crates/core-runs/src/lib.rs crates/core-runs/src/model.rs crates/core-runs/src/store/mod.rs crates/core-grants/src/model.rs crates/core-grants/src/tools.rs crates/core-grants/src/lib.rs crates/core-grants/src/store/mod.rs crates/core-threads/src/model.rs crates/core-threads/src/lib.rs crates/core-threads/src/store/thread.rs crates/core-grants/src/writer.rs crates/core-plans/src/lib.rs crates/shadows-mcp/src/server.rs crates/core-runs/src/report.rs crates/core-turns/src/record.rs crates/core-turns/src/planning/mod.rs crates/core-turns/src/stage.rs crates/core-harness/src/live.rs crates/core-harness/src/setup.rs crates/core-threads/src/store/records.rs crates/shadows-core/tests/executor_session.rs crates/core-grants/tests/tokens.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml crates/core-runs/contract.yaml crates/core-harness/contract.yaml crates/core-grants/contract.yaml crates/shadows-mcp/src/contract.yaml crates/core-threads/contract.yaml crates/shadows-core/tests/preparation.rs
git commit -m "feat(core): preparation request, scope and persistence"
```

### Task 2: B-101 Bounded preparation worker and refreshed result

**Owner:** Runs and Harness.

**Files:**
- Create: `crates/core-harness/src/preparation.rs`
- Create: `crates/core-harness/src/preparation.txt`
- Modify: `crates/core-harness/src/lib.rs`
- Modify: `crates/core-harness/src/setup.rs`
- Modify: `crates/core-harness/src/sessions.rs`
- Modify: `crates/core-runs/src/preparation.rs`
- Modify: `crates/core-runs/src/recover.rs`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/Cargo.toml`
- Modify: `crates/shadows-core/src/app.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/preparation.rs`

**Interfaces:**
- Consumes: B-100 records/grant and Code::refresh_inventory; Runs receives Code via AppCore.
- Produces: Harness::open_preparation(&self,thread:&ThreadId,destination:&str,model:String) -> Result<OpenSession,OpenError>; Runs::finish_preparation(&self,grant:&Grant,report:String,commit:Option<String>) -> Result<Preparation,RunsError>; Runs::stop_preparation(&self,id:&str) -> Result<Preparation,RunsError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `preparation_interruption_preserves_partial_files`, `preparation_does_not_enable_planner_shell`, `preparation_report_refreshes_repository`. A live worker (the real adapter, ignored unless its key is present) creates backend/composer.json in a real temporary project then reports; a second run is stopped after its first file is written. Restart recovery marks an unfinished record interrupted. Assert normal Planner tool set unchanged. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert!(partial_file.exists());
assert_eq!(recovered.state, PreparationState::Interrupted);
assert!(!planner_tools.contains(&"Bash"));
assert!(refreshed_paths.contains(&"backend/composer.json"));
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test preparation`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Run the framework's own setup through a worker with destination/** permissions, not a generated framework layout. Grant offers preparation_report only for its request. Preserve other work, exclude installers/binaries from commit, refresh inventory on confirmed completion and stop. Recovery never reruns setup automatically.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test preparation`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-harness/src/preparation.rs crates/core-harness/src/preparation.txt crates/core-harness/src/lib.rs crates/core-harness/src/setup.rs crates/core-harness/src/sessions.rs crates/core-runs/src/preparation.rs crates/core-runs/src/recover.rs crates/core-runs/src/lib.rs crates/core-runs/Cargo.toml crates/shadows-core/src/app.rs docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/core-harness/contract.yaml crates/shadows-core/tests/preparation.rs
git commit -m "feat(core): bounded preparation worker and refreshed result"
```

### Task 3: B-102 Preparation HTTP/MCP and observable status

**Owner:** Runs adapters.

**Files:**
- Create: `crates/shadows-http/src/preparation.rs`
- Create: `crates/shadows-http/src/preparation_tests.rs`
- Modify: `crates/shadows-http/src/lib.rs`
- Modify: `crates/shadows-http/src/openapi.rs`
- Modify: `crates/shadows-http/src/failure/runs.rs`
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `crates/core-harness/src/setup.rs`
- Modify: `api/openapi.json`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-harness/contract.yaml`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-http/src/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Test: `crates/shadows-http/src/preparation_tests.rs`

**Interfaces:**
- Consumes: B-100/B-101 and shared preparation routes; actor derived from request/grant.
- Produces: project_prepare, preparation_get, preparation_report and HTTP preparation read/start/stop routes; completion retains original Planner/source-message attribution.

- [ ] **Step 1: Write the failing tests**

In the named test module add `preparation_routes_replay_and_show_failed_start`, `preparation_report_refreshes_repository`. HTTP starts once and polls completed; duplicate MCP start returns the same record. Nonempty Laravel is reported for adoption, not deleted. Missing session startup is a visible failed preparation. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(first["id"], replay["id"]);
assert_eq!(failed["state"], "failed");
assert!(existing_scaffold.exists());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-http preparation`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Translate only Runs calls, return 201 for a new start and its record on replay, preserve stable refusal codes. Advertise preparation instructions only after file inspection; regenerate OpenAPI. No general HTTP filesystem/command endpoint.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-http preparation`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/shadows-http/src/preparation.rs crates/shadows-http/src/preparation_tests.rs crates/shadows-http/src/lib.rs crates/shadows-http/src/openapi.rs crates/shadows-http/src/failure/runs.rs crates/shadows-mcp/src/run_tools.rs crates/shadows-mcp/src/server.rs crates/core-grants/src/tools.rs crates/core-harness/src/setup.rs api/openapi.json docs/codebase/README.md docs/backend/stages.md crates/core-harness/contract.yaml crates/core-grants/contract.yaml crates/shadows-http/src/contract.yaml crates/shadows-mcp/src/contract.yaml
git commit -m "feat(core): preparation http/mcp and observable status"
```

## Slice acceptance and handoff

Complete the companion frontend task before declaring the slice user-runnable. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the paired web gate after API regeneration. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

