# Bounded repair and full local acceptance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** Failures reach the right Planner, retries stay bounded and the complete local journey survives interruption.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, serde/utoipa; existing React client consumes the shared API.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** B-107..114 and frontend F-99..F-104 for real acceptance.

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

- **B-115 / `retired_report_cannot_commit_new_attempt`:** A late tool call must not attach to a retry.
- **B-116 / `busy_or_missing_owner_does_not_lose_failure`:** Planner availability must not discard the repair request.
- **B-117 / `draft_stops_running_attempt_and_blocks_next_start`:** Replanning cannot continue old-version execution silently.
- **B-117 / `pending_recheck_cannot_self_trigger_forever`:** A dependency event must not create an infinite review loop.
- **B-118 / `restart_never_reports_missing_checks_as_pass`:** The UI must retain interrupted/unknown work honestly.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: B-115 Retry in the same live session with fresh attempt identity

**Owner:** Runs and Harness.

**Files:**
- Create: `crates/core-runs/src/retry.rs`
- Create: `crates/core-runs/src/store/repair.rs`
- Create: `crates/core-db/migrations/0061_run_repair.sql`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/watch.rs`
- Modify: `crates/core-runs/src/store/mod.rs`
- Modify: `crates/core-runs/src/report.rs`
- Modify: `crates/core-harness/src/executor.rs`
- Modify: `crates/core-harness/src/setup.rs`
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/core-harness/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: B-107 immutable briefs, B-112 review, current TaskScope workflow/version/task and thread session.
- Produces: Runs::retry(&self,command:RepairCommand,attempt:&AttemptId) -> Result<Attempt,RunsError>; rich task_report requires attempt_id; idle executor reuse with no active writable attempt.

- [ ] **Step 1: Write the failing tests**

In the named test module add `retired_report_cannot_commit_new_attempt`, `retry_reuses_session_but_not_attempt_record`, `lost_repair_session_is_visible`. Fail attempt 1 and retry into same live native session with new attempt/brief. Submit attempt 1 report after attempt 2 starts; refuse it. Drop the session; show explicit recovery rather than claiming same history. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_ne!(first.id, second.id);
assert_eq!(first.thread_id, second.thread_id);
assert!(late_report.is_err());
assert_eq!(native_session_open_count, 1);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Do not close the executor while checks/review may request a retry. Deactivate mutation authority while idle. Store each retry as a new row, preserving earlier report/evidence; validate explicit attempt_id against the current running attempt. Keep close-on-release behavior for legacy path until migrated.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/retry.rs crates/core-runs/src/store/repair.rs crates/core-db/migrations/0061_run_repair.sql crates/core-runs/src/lib.rs crates/core-runs/src/watch.rs crates/core-runs/src/store/mod.rs crates/core-runs/src/report.rs crates/core-harness/src/executor.rs crates/core-harness/src/setup.rs crates/shadows-mcp/src/run_tools.rs docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/core-harness/contract.yaml crates/shadows-mcp/src/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): retry in the same live session with fresh attempt identity"
```

### Task 2: B-116 Find the actual Planner and enforce the three-attempt ladder

**Owner:** Plans and Runs.

**Files:**
- Create: `crates/core-plans/src/owner_thread.rs`
- Create: `crates/core-runs/src/repair.rs`
- Modify: `crates/core-plans/src/lib.rs`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/store/repair.rs`
- Modify: `crates/core-runs/src/recover.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: Plan WrittenBy::Planner; Threads/Turns existing send/operation APIs; B-115 retry.
- Produces: Plans::owner_thread(&self,workflow:&WorkflowId) -> Result<Option<ThreadId>,PlansError>; Runs::repair_state(&self,attempt:&AttemptId) -> Result<Option<RepairState>,RunsError>; owner-only task_retry/task_stop, with exact failure evidence.

- [ ] **Step 1: Write the failing tests**

In the named test module add `busy_or_missing_owner_does_not_lose_failure`, `third_attempt_requires_direction_and_fourth_is_refused`. Deliver failure to a busy owner, then release its turn and observe one message. Remove the owner and observe stopped_for_person. Fail twice, require investigation plus nonempty third instruction; reject fourth agent retry. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(delivered_failure_count, 1);
assert_eq!(missing_owner.state, RepairPhase::StoppedForPerson);
assert!(empty_third_instruction.is_err());
assert!(fourth_retry.is_err());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Runs owns durable repair counters/state. Queue delivery while the owner is busy and retry delivery idempotently after restart. Use remembered Planner model through Harness; no invented model or autonomous plan approval. Owner investigation is a recorded step, not just an executor prompt.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-plans/src/owner_thread.rs crates/core-runs/src/repair.rs crates/core-plans/src/lib.rs crates/core-runs/src/lib.rs crates/core-runs/src/store/repair.rs crates/core-runs/src/recover.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml crates/core-runs/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): find the actual planner and enforce the three-attempt ladder"
```

### Task 3: B-117 Stop obsolete plans and rerun pending evidence

**Owner:** Runs with Plans.

**Files:**
- Modify: `crates/core-runs/src/watch.rs`
- Modify: `crates/core-runs/src/ready.rs`
- Modify: `crates/core-runs/src/repair.rs`
- Modify: `crates/core-runs/src/acceptance.rs`
- Modify: `crates/core-plans/src/execution.rs`
- Modify: `crates/shadows-http/src/runs.rs`
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `api/openapi.json`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-http/src/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: Plans::execution_status_in, B-116 repair, B-113 acceptance; exact output pins and pending wait target.
- Produces: Watcher-driven confirmed stop on Draft/supersession; durable deduplicated pending recheck; retry/repair HTTP/MCP operations in shared agreement.

- [ ] **Step 1: Write the failing tests**

In the named test module add `draft_stops_running_attempt_and_blocks_next_start`, `pending_recheck_cannot_self_trigger_forever`, `external_consumer_pin_is_not_rewritten`. Create a new Draft during a running turn; block Start immediately through admission rule and stop the old turn before releasing writes. Accept a prerequisite output twice; trigger one pending recheck. A provider change never silently repins a frozen consumer. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(new_start.code, Some("plan_replanning".into()));
assert!(old_turn_ended_before_scope_release);
assert_eq!(pending_recheck_count, 1);
assert_eq!(consumer_pin.version, 1);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts --test task_contract_refs`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Use current Plans state in the existing watcher, not a hold/ack protocol. Draft editing need not wait, but new-version execution waits for old scope release. Required contract/check graph rejects proof cycles. Restart keeps failed/pending delivery visible and does not auto-resume all tasks.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts --test task_contract_refs`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/watch.rs crates/core-runs/src/ready.rs crates/core-runs/src/repair.rs crates/core-runs/src/acceptance.rs crates/core-plans/src/execution.rs crates/shadows-http/src/runs.rs crates/shadows-mcp/src/run_tools.rs crates/core-grants/src/tools.rs api/openapi.json docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml crates/core-runs/contract.yaml crates/core-grants/contract.yaml crates/shadows-http/src/contract.yaml crates/shadows-mcp/src/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): stop obsolete plans and rerun pending evidence"
```

### Task 4: B-118 Disposable full journey and branch delivery evidence

**Owner:** Controller across existing owners.

**Files:**
- Create: `crates/shadows/tests/local_delivery.rs`
- Create: `crates/shadows/tests/local_delivery/live.rs`
- Modify: `docs/backend/stages.md`
- Modify: `docs/frontend/stages.md`
- Modify: `docs/core/future.md`
- Modify: `docs/codebase/README.md`
- Test: `crates/shadows/tests/local_delivery.rs`

**Interfaces:**
- Consumes: All backend tasks and frontend F-99..F-104; approved simplified spec and existing full gate.
- Produces: Repeatable ignored real-harness journey plus deterministic failure/restart integration coverage; dated acceptance record only after actual execution.

- [ ] **Step 1: Write the failing tests**

In the named test module add `restart_never_reports_missing_checks_as_pass`, `local_delivery_producer_consumer_journey`, `local_delivery_scope_and_repair_caps`. A disposable repository first adopts existing Laravel, then prepares another empty destination. Plan producer/consumer functions, run them, force one check/critic failure and repair. Interrupt preparation, executor, check and review separately. Preserve testing1 and its database. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert!(existing_laravel_unchanged);
assert_eq!(consumer_input.name, producer_output.name);
assert!(!interrupted_assessment_is_accepted);
assert!(fourth_agent_attempt_refused);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows --test local_delivery`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

Run from repository root: `cargo test -p shadows --test architecture --test codemap`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Git, folders and checks are real in every test. Agent behaviour is proven by the ignored live test on the real daemon/adapter and a disposable project, which records model/OS/commands; no fake adapter is added. Observe browser and logs before marking acceptance done. Keep current data untouched; no automatic cleanup/rollback of the person's work.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows --test local_delivery`.
Expected: every named test and the affected existing tests pass, exit 0.

Run: `cargo test -p shadows --test architecture --test codemap`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/shadows/tests/local_delivery.rs crates/shadows/tests/local_delivery/live.rs docs/backend/stages.md docs/frontend/stages.md docs/core/future.md docs/codebase/README.md
git commit -m "feat(core): disposable full journey and branch delivery evidence"
```

## Slice acceptance and handoff

Complete the companion frontend task before declaring the slice user-runnable. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the paired web gate after API regeneration. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

