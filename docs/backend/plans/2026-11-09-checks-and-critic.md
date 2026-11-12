# Independent checks and Git-based critic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** Task completion is supported by recorded checks and an independent critic reading the exact Git commit.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, serde/utoipa; existing React client consumes the shared API.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** B-107..109 plus CheckSpec in B-103.

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

- **B-110 / `git_source_never_reads_live_tree_or_revision_injection`:** The reviewed SHA/path is the source, even after a live edit.
- **B-111 / `head_change_during_check_is_inconclusive`:** A green exit cannot certify a different source revision.
- **B-112 / `critic_dispatch_is_deduplicated_and_read_only`:** Duplicate completion must not start multiple reviews or allow edits.
- **B-113 / `failed_required_check_cannot_be_overridden_by_review`:** An asserted PASS cannot erase check failure.
- **B-114 / `manual_and_verified_done_are_distinct`:** Existing manual marks must never be labeled independently verified.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: B-110 Bounded source from the reported Git SHA

**Owner:** Runs.

**Files:**
- Create: `crates/core-runs/src/review_source.rs`
- Modify: `crates/core-runs/src/git.rs`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/error.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/report.rs`

**Interfaces:**
- Consumes: Existing Git::at/run/reaches, accepted candidate commit, ExecutionBrief file/dependency scope.
- Produces: Runs::review_source(&self,grant:&Grant,attempt:&AttemptId,request:SourceRequest) -> Result<SourceAnswer,RunsError>; private Git::source_at(&self,commit:&str,path:&str) -> Result<Vec<u8>,RunsError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `git_source_never_reads_live_tree_or_revision_injection`, `git_source_enforces_scope_and_bounds`. Commit version A, change live file to B, ask the critic for A. Try ../escape, a path beginning --, a binary blob, an undeclared file and a removed Git object. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert!(source.text.contains("version A"));
assert!(!source.text.contains("version B"));
assert!(outside.is_err());
assert!(source.text.len() <= 32768);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts report`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Resolve and validate the stored full SHA, use git show with separately validated object/path arguments, and reject binary/invalid UTF-8. Default 200/max 500 lines. No working-tree checkout or source-copy storage. Missing tracked source is missing evidence.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts report`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/review_source.rs crates/core-runs/src/git.rs crates/core-runs/src/lib.rs crates/core-runs/src/error.rs docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/shadows-core/tests/runs_attempts/report.rs
git commit -m "feat(core): bounded source from the reported git sha"
```

### Task 2: B-111 Run required checks with source provenance

**Owner:** Runs.

**Files:**
- Create: `crates/core-runs/src/checks.rs`
- Create: `crates/core-runs/src/store/checks.rs`
- Create: `crates/core-db/migrations/0059_run_checks.sql`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/ready.rs`
- Modify: `crates/core-runs/src/start.rs`
- Modify: `crates/core-runs/src/recover.rs`
- Modify: `crates/core-runs/src/watch.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: CheckSpec/CheckRun, shadows-process, existing scope release. Project verification slot is transient Runs state reconstructed from active check records.
- Produces: Runs::run_checks(&self,command:String,attempt:&AttemptId) -> Result<AttemptAssessment,RunsError>; Runs::assessment(&self,attempt:&AttemptId) -> Result<AttemptAssessment,RunsError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `head_change_during_check_is_inconclusive`, `check_waits_for_writers_and_blocks_new_start`, `check_timeout_restart_and_output_are_truthful`. Disposable real commands exit 0/nonzero, time out, and produce >65536 bytes. Queue checks until all project executor turns ended; block new starts while checking. Force HEAD change mid-check, and restart while one check is running. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(drifted.state, CheckState::Inconclusive);
assert_eq!(queued.state, CheckState::Queued);
assert_eq!(start_during_check.code, Some("verification_busy".into()));
assert!(captured_output.len() <= 65536);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Execute program/args without shell concatenation in validated cwd. Capture tested HEAD, relevant dirty observations and truncated/redacted streams. Optional baseline stores its own result; do not infer equal failures from equal exit code. Recovery marks unconfirmed checks interrupted, never passed. No general read/write leases.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/checks.rs crates/core-runs/src/store/checks.rs crates/core-db/migrations/0059_run_checks.sql crates/core-runs/src/lib.rs crates/core-runs/src/ready.rs crates/core-runs/src/start.rs crates/core-runs/src/recover.rs crates/core-runs/src/watch.rs docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): run required checks with source provenance"
```

### Task 3: B-112 Read-only critic session and one review per result

**Owner:** Runs, Harness and Grants.

**Files:**
- Create: `crates/core-harness/src/critic.rs`
- Create: `crates/core-harness/src/critic.txt`
- Create: `crates/core-runs/src/review.rs`
- Create: `crates/core-runs/src/store/review.rs`
- Create: `crates/core-db/migrations/0060_run_reviews.sql`
- Modify: `crates/core-harness/src/lib.rs`
- Modify: `crates/core-harness/src/setup.rs`
- Modify: `crates/core-grants/src/model.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `crates/core-grants/src/lib.rs`
- Modify: `crates/core-grants/src/store/mod.rs`
- Modify: `crates/core-threads/src/model.rs`
- Modify: `crates/core-runs/src/watch.rs`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-grants/src/writer.rs`
- Modify: `crates/core-plans/src/lib.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/core-runs/src/report.rs`
- Modify: `crates/core-turns/src/record.rs`
- Modify: `crates/core-turns/src/planning/mod.rs`
- Modify: `crates/core-turns/src/stage.rs`
- Modify: `crates/core-harness/src/live.rs`
- Modify: `crates/core-threads/src/store/records.rs`
- Modify: `crates/shadows-core/tests/executor_session.rs`
- Modify: `crates/core-grants/tests/tokens.rs`
- Modify: `crates/core-threads/src/lib.rs`
- Modify: `crates/core-threads/src/store/thread.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/core-harness/contract.yaml`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-threads/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: B-110 source and B-111 settled checks; chosen critic model from extended Start request, persisted in run_attempts.
- Produces: Harness::open_critic(&self,thread:&ThreadId,scope:TaskScope,model:String) -> Result<OpenSession,OpenError>; Runs::review(&self,grant:&Grant,input:ReviewInput) -> Result<ReviewRecord,RunsError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `critic_dispatch_is_deduplicated_and_read_only`, `critic_review_must_name_exact_result_and_all_items`. Two completion notifications for the same attempt/commit produce one critic thread. Inspect offered tools: task_source/task_review only, no Bash/Edit/Write. Submit a review for another commit and incomplete item list. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(critic_thread_count, 1);
assert!(!tools.contains(&"Bash"));
assert!(!tools.contains(&"Edit"));
assert!(wrong_commit_review.is_err());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Dispatch only after checks settle, with contract/diff/report/evidence. Critic gets no executor transcript. Store one immutable review per attempt/commit, enforce grant role and item coverage, retain failed review evidence. Add critic Thread/Grant kind without exposing Planner approvals.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-harness/src/critic.rs crates/core-harness/src/critic.txt crates/core-runs/src/review.rs crates/core-runs/src/store/review.rs crates/core-db/migrations/0060_run_reviews.sql crates/core-harness/src/lib.rs crates/core-harness/src/setup.rs crates/core-grants/src/model.rs crates/core-grants/src/tools.rs crates/core-grants/src/lib.rs crates/core-grants/src/store/mod.rs crates/core-threads/src/model.rs crates/core-runs/src/watch.rs crates/core-runs/src/lib.rs crates/core-grants/src/writer.rs crates/core-plans/src/lib.rs crates/shadows-mcp/src/server.rs crates/core-runs/src/report.rs crates/core-turns/src/record.rs crates/core-turns/src/planning/mod.rs crates/core-turns/src/stage.rs crates/core-harness/src/live.rs crates/core-threads/src/store/records.rs crates/shadows-core/tests/executor_session.rs crates/core-grants/tests/tokens.rs crates/core-threads/src/lib.rs crates/core-threads/src/store/thread.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml crates/core-runs/contract.yaml crates/core-harness/contract.yaml crates/core-grants/contract.yaml crates/shadows-mcp/src/contract.yaml crates/core-threads/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): read-only critic session and one review per result"
```

### Task 4: B-113 Evidence-backed outputs and honest task acceptance

**Owner:** Runs.

**Files:**
- Create: `crates/core-runs/src/acceptance.rs`
- Modify: `crates/core-runs/src/lib.rs`
- Modify: `crates/core-runs/src/model.rs`
- Modify: `crates/core-runs/src/store/marks.rs`
- Modify: `crates/core-runs/src/ready.rs`
- Modify: `crates/core-runs/src/review.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs`

**Interfaces:**
- Consumes: ProducedOutput acceptance mapping, required checks, ReviewRecord, existing manual mark history.
- Produces: Runs::output_accepted(&self,workflow:&WorkflowId,version:i64,task:u32,output:&str) -> Result<bool,RunsError>; TaskRun extends with acceptance_source manual|verified|none and assessment; Runs::accept_notes(command:String,attempt:&AttemptId) -> Result<AttemptAssessment,RunsError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `failed_required_check_cannot_be_overridden_by_review`, `independent_output_can_pass_while_other_item_pending`, `manual_and_verified_done_are_distinct`. Critic says PASS while one required check failed; refuse accepted state. One output has met items while unrelated item is pending. A legacy manually marked task stays manual; PASS_WITH_NOTES waits for person. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_ne!(failed_check_assessment.state, AssessmentState::Accepted);
assert!(output_ready);
assert_eq!(legacy.acceptance_source, AcceptanceSource::Manual);
assert_eq!(notes.state, AssessmentState::NeedsPerson);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test runs_attempts --test task_contract_refs`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Compute acceptance in one Runs method from current attempt evidence, not stored plan mutations. Require associated items met and checks passed. Preserve legacy manual policy explicitly. Reconcile dependency readiness through this same method; do not treat a pending item as full verification.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test runs_attempts --test task_contract_refs`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-runs/src/acceptance.rs crates/core-runs/src/lib.rs crates/core-runs/src/model.rs crates/core-runs/src/store/marks.rs crates/core-runs/src/ready.rs crates/core-runs/src/review.rs docs/codebase/README.md docs/backend/stages.md crates/core-runs/contract.yaml crates/shadows-core/tests/runs_attempts.rs
git commit -m "feat(core): evidence-backed outputs and honest task acceptance"
```

### Task 5: B-114 Expose checks, critic and result detail

**Owner:** Runs adapters.

**Files:**
- Modify: `crates/shadows-http/src/runs.rs`
- Modify: `crates/shadows-http/src/openapi.rs`
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/shadows-http/src/failure/runs.rs`
- Modify: `crates/core-grants/src/tools.rs`
- Modify: `api/openapi.json`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/shadows-http/src/contract.yaml`
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Test: `crates/shadows/tests/runs_routes.rs`

**Interfaces:**
- Consumes: B-110..113 and shared assessment/checks/accept-notes routes; optional critic_model on Start.
- Produces: task_source/task_review for critic; HTTP assessment/check-start/person accept-notes; Start persists critic model without changing Frozen task.

- [ ] **Step 1: Write the failing tests**

In the named test module add `manual_and_verified_done_are_distinct`, `assessment_routes_preserve_replay_and_role_boundaries`. HTTP reads failed checks separately from a committed attempt/report; a critic cannot invoke person accept-notes or start another attempt. Repeating check command returns recorded result. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(attempt["state"], "committed");
assert_eq!(assessment["checks"][0]["state"], "failed");
assert!(!critic_tools.contains(&"plan_approve"));
assert_eq!(first_check_id, replayed_check_id);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows --test runs_routes --test planner_mcp`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Expose owner results unchanged; update OpenAPI and error mappings. Polling UI can observe assessment independently of executor turn completion. Never render critic text as trusted HTML or claim unconfigured review ran.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows --test runs_routes --test planner_mcp`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/shadows-http/src/runs.rs crates/shadows-http/src/openapi.rs crates/shadows-mcp/src/run_tools.rs crates/shadows-mcp/src/server.rs crates/shadows-http/src/failure/runs.rs crates/core-grants/src/tools.rs api/openapi.json docs/codebase/README.md docs/backend/stages.md crates/core-grants/contract.yaml crates/shadows-http/src/contract.yaml crates/shadows-mcp/src/contract.yaml crates/shadows/tests/runs_routes.rs
git commit -m "feat(core): expose checks, critic and result detail"
```

## Slice acceptance and handoff

Complete the companion frontend task before declaring the slice user-runnable. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the paired web gate after API regeneration. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

