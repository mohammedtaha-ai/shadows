# Execution briefs and task-scoped code questions Implementation Plan

**Superseded, 2026-11-14:** use the [detailed slice 4 plan](2026-11-14-slice-4-task-questions.md).
The steps below are earlier draft material and are not a builder assignment.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** The executor receives one clear attempt brief and asks about code only within its task context.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, serde/utoipa; existing React client consumes the shared API.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** B-98, B-102, B-104..106; no general lease or packet engine.

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

- **B-107 / `brief_keeps_approved_contract_after_later_edit`:** A run must retain the obligations it actually received.
- **B-108 / `create_target_is_not_mistaken_for_missing_input`:** A planned new file need not already exist.
- **B-108 / `new_draft_blocks_start_inside_admission`:** A concurrent replan cannot slip past preflight.
- **B-109 / `task_question_refuses_undeclared_file_before_lsp`:** Global semantic lookup must not leak another task's files.
- **B-109 / `planned_function_answer_is_not_existing_source`:** A producer promise is distinct from an implemented symbol.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: B-107 Store a simple brief on each attempt

**Owner:** Runs.

**Files:**
- Create: `crates/core-runs/src/brief.rs`
- Create: `crates/core-runs/src/store/brief.rs`
- Create: `crates/core-db/migrations/0058_attempt_brief.sql`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/start.rs`
- Modify: `crates/core-runs/src/store/mod.rs`
- Modify: `crates/core-runs/Cargo.toml`
- Modify: `crates/shadows-core/src/app.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: ExecutionBrief, Plans::resolve_contracts, Design decision reads, existing start_head/model/attempt identity.
- Produces: Runs::brief(&self,attempt:&AttemptId) -> Result<ExecutionBrief,RunsError>; private capture_brief stores one immutable JSON value keyed by attempt id before its first turn.

- [ ] **Step 1: Write the failing tests**

In the named test module add `brief_keeps_approved_contract_after_later_edit`, `brief_replay_does_not_duplicate_rows`. Start a task with findCustomer obligation, then create a later Draft changing that text. Read the original attempt's brief and attempt a second write to it. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(brief.task.contract.unwrap().symbols[0].name, "findCustomer");
assert_eq!(brief.version, 1);
assert_eq!(brief_row_count, 1);
assert!(second_brief_write.is_err());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Wire Code/Design dependencies through AppCore explicitly. Resolve contracts/decisions through their owner APIs and capture plan/map/standards versions/base SHA. Do not copy source bytes or add a packet digest table.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/brief.rs crates/core-runs/src/store/brief.rs crates/core-db/migrations/0058_attempt_brief.sql crates/core-runs/src/lib.rs crates/core-runs/src/start.rs crates/core-runs/src/store/mod.rs crates/core-runs/Cargo.toml crates/shadows-core/src/app.rs docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): store a simple brief on each attempt"
```

### Task 2: B-108 Readiness from real inputs and current plan state

**Owner:** Runs with Plans.

**Files:**
- Create: `crates/core-plans/src/execution.rs`
- Modify: `crates/core-plans/src/lib.rs`
- Modify: `crates/core-plans/src/store/read.rs`
- Modify: `crates/core-runs/src/ready.rs`
- Modify: `crates/core-runs/src/start.rs`
- Modify: `crates/core-runs/src/store/mod.rs`
- Modify: `crates/core-runs/src/error.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: B-103 named file actions; Code inventory/semantic observations; current transaction type from core-db.
- Produces: Plans::execution_status_in(tx:&mut core_db::Tx,workflow:&WorkflowId) -> Result<ExecutionStatus,PlansError>; Runs admission checks this and required existing inputs within its serialized begin path.

- [ ] **Step 1: Write the failing tests**

In the named test module add `create_target_is_not_mistaken_for_missing_input`, `new_draft_blocks_start_inside_admission`, `disjoint_glob_scopes_still_run_together`. Task creates New.php and reads Existing.php. Missing New.php is valid; missing Existing.php is refused. Race a Draft creation against Start. Keep two disjoint write-scope tasks runnable. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert!(create_only_start.allowed);
assert_eq!(missing_read.code, Some("missing_contract_input".into()));
assert!(!start_after_draft.allowed);
assert_eq!(held_write_conflicts, 0);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts --test task_context`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Retain the current write matcher/overlap rule. Revalidate existing required definitions; no full-project stale flag or broad read locks. Plans owns its state query; Runs stores only its own attempts. Update start_answer and Start through the same rule.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts --test task_context`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-plans/src/execution.rs crates/core-plans/src/lib.rs crates/core-plans/src/store/read.rs crates/core-runs/src/ready.rs crates/core-runs/src/start.rs crates/core-runs/src/store/mod.rs crates/core-runs/src/error.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml crates/core-runs/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): readiness from real inputs and current plan state"
```

### Task 3: B-109 Task-aware questions and executor tools

**Owner:** Runs, Grants and adapters.

**Files:**
- Create: `crates/core-runs/src/question.rs`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/model.rs`
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `crates/core-harness/src/executor.md`
- Modify: `crates/shadows-http/src/runs.rs`
- Modify: `crates/shadows-http/src/openapi.rs`
- Modify: `api/openapi.json`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/core-harness/contract.yaml`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-http/src/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: B-107 brief, Code::semantic, ResolvedContract; explicit named file work plus recorded permission scopes.
- Produces: Runs::question(&self,grant:&Grant,request:TaskQuestion) -> Result<TaskAnswer,RunsError>; task_brief and task_question; GET attempt brief route. Remove direct global where_is/who_uses/outline from rich executor tools, preserving project/Planner tools.

- [ ] **Step 1: Write the failing tests**

In the named test module add `task_question_refuses_undeclared_file_before_lsp`, `planned_function_answer_is_not_existing_source`, `task_question_joins_source_and_obligation_without_overriding`. Executor allowed to read Repository.php asks findCustomer there, then asks Hidden.php and a function only promised by T2. Change the file during the authorized query; no stale snippet is returned. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(forbidden.code(), "outside_task_reads");
assert_eq!(backend_calls_for_hidden, 0);
assert!(planned.code.hits.is_empty());
assert!(planned.obligations[0].contract.producer_title.is_some());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

Run from repository root: `cargo test -p shadows --test planner_mcp`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Authenticate attempt scope before Code access, join planning obligations with Runs-owned ContractAvailability without Code/Plans depending on Runs. Offer task_brief/task_question to executors instead of global code tools; legacy task_context remains a Plans read and legacy questions use recorded reads scopes. First prompt asks task_brief. HTTP brief is read-only and generated from Core.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts`.
Expected: every named test and the affected existing tests pass, exit 0.

Run: `cargo test -p shadows --test planner_mcp`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/question.rs crates/core-runs/src/lib.rs crates/core-runs/src/model.rs crates/shadows-mcp/src/run_tools.rs crates/shadows-mcp/src/server.rs crates/core-grants/src/tools.rs crates/core-harness/src/executor.md crates/shadows-http/src/runs.rs crates/shadows-http/src/openapi.rs api/openapi.json docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/core-harness/contract.yaml crates/core-grants/contract.yaml crates/shadows-http/src/contract.yaml crates/shadows-mcp/src/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): task-aware questions and executor tools"
```

## Slice acceptance and handoff

Complete the companion frontend task before declaring the slice user-runnable. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the paired web gate after API regeneration. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

