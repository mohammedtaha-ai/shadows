# Detailed task contracts and project decisions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** A task names real file work and symbol behavior and references producer contracts without providing finished code.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, serde/utoipa; existing React client consumes the shared API.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** B-96 inventory and shared model; no hashed packet prerequisite.

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

- **B-103 / `legacy_task_serialization_is_unchanged`:** Old command fingerprints and Frozen versions stay stable.
- **B-103 / `named_work_and_glob_scope_coexist`:** A concrete Redis file may live inside docker/** permission.
- **B-104 / `output_reference_cycle_is_refused_atomically`:** Producer/consumer pins cannot introduce a hidden dependency cycle.
- **B-105 / `decision_revision_preserves_approved_history`:** A changed decision cannot rewrite a pinned approval.
- **B-106 / `planner_contract_names_behavior_without_function_body`:** The executor must receive the required brief, not an invented implementation.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: B-103 Persist typed file/symbol work

**Owner:** Plans.

**Files:**
- Create: `crates/core-plans/src/task_contract.rs`
- Create: `crates/core-plans/tests/task_contract.rs`
- Modify: `crates/core-plans/src/lib.rs`
- Modify: `crates/core-plans/src/model.rs`
- Modify: `crates/core-plans/src/store/task.rs`
- Modify: `crates/core-plans/src/ops.rs`
- Modify: `crates/core-plans/src/rules.rs`
- Modify: `crates/core-plans/tests/workflow_rules.rs`
- Modify: `crates/shadows-core/tests/design_outcomes.rs`
- Modify: `crates/shadows-core/tests/design_parts.rs`
- Modify: `crates/shadows-core/tests/design_parts/roadmap_gate.rs`
- Modify: `crates/shadows-core/tests/fixtures/cross_plan.rs`
- Modify: `crates/shadows-core/tests/runs_attempts.rs`
- Modify: `crates/shadows/tests/fixtures/plan.rs`
- Modify: `crates/shadows/tests/plan_routes.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Test: `crates/core-plans/tests/task_contract.rs`

**Interfaces:**
- Consumes: TaskContract model in shared agreement; TaskRows contract_json and existing PlanOp::TaskAdd/TaskUpdate.
- Produces: TaskContent.contract: Option<TaskContract>; Plans::validate_task_contract(task:&TaskContent) -> Vec<Problem> (associated pure method); contract serialized inside existing contract_json, omitted when absent.

- [ ] **Step 1: Write the failing tests**

In the named test module add `legacy_task_serialization_is_unchanged`, `named_work_and_glob_scope_coexist`, `symbol_work_requires_owned_file_and_acceptance`. Roundtrip a legacy task with no contract; then one with docker/** writes and a named docker/redis/redis.conf file action. Reject escaped path, missing symbol file and nonexistent acceptance numbers. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert!(!legacy_json.contains("contract"));
assert_eq!(roundtrip, legacy);
assert!(Plans::validate_task_contract(&redis_task).is_empty());
assert!(!Plans::validate_task_contract(&escaping_task).is_empty());
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p core-plans --test task_contract --test workflow_rules`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Define shared-agreement structs in task_contract.rs and export through Plans. Keep scope fields independent; validate normalized identities/paths/actions here; B-108 checks permission containment using Runs' existing Scope matcher. Update listed TaskContent literal constructors mechanically with contract: None. No JSON Schema comparison/fingerprint subsystem.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p core-plans --test task_contract --test workflow_rules`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-plans/src/task_contract.rs crates/core-plans/tests/task_contract.rs crates/core-plans/src/lib.rs crates/core-plans/src/model.rs crates/core-plans/src/store/task.rs crates/core-plans/src/ops.rs crates/core-plans/src/rules.rs crates/core-plans/tests/workflow_rules.rs crates/shadows-core/tests/design_outcomes.rs crates/shadows-core/tests/design_parts.rs crates/shadows-core/tests/design_parts/roadmap_gate.rs crates/shadows-core/tests/fixtures/cross_plan.rs crates/shadows-core/tests/runs_attempts.rs crates/shadows/tests/fixtures/plan.rs crates/shadows/tests/plan_routes.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml
git commit -m "feat(core): persist typed file/symbol work"
```

### Task 2: B-104 Resolve producers and consumers in the existing graph

**Owner:** Plans.

**Files:**
- Create: `crates/core-plans/src/contract_refs.rs`
- Create: `crates/core-plans/src/store/contract_refs.rs`
- Create: `crates/shadows-core/tests/task_contract_refs.rs`
- Modify: `crates/core-plans/src/lib.rs`
- Modify: `crates/core-plans/src/dependency_graph.rs`
- Modify: `crates/core-plans/src/store/edit.rs`
- Modify: `crates/core-plans/src/task_context.rs`
- Modify: `crates/core-plans/src/store/task_context.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Test: `crates/shadows-core/tests/task_contract_refs.rs`

**Interfaces:**
- Consumes: B-103 contract; existing shared-agreement pins/operation validation; same-project workflow identity.
- Produces: Plans::resolve_contracts(&self,workflow:&WorkflowId,task:u32) -> Result<Vec<ResolvedContract>,PlansError>; required task-output edges feed the existing dependency graph and task_context.dependencies.

- [ ] **Step 1: Write the failing tests**

In the named test module add `output_reference_cycle_is_refused_atomically`, `consumer_reads_one_producer_signature`, `external_contract_pin_never_follows_latest`. T1 produces findCustomer; T2 consumes that output. Remove the output, pin a foreign project/version, and introduce T1->T2 back-reference in separate attempted edits. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(resolved[0].symbol.as_ref().unwrap().name, "findCustomer");
assert!(cycle_edit.is_err());
assert_eq!(revision_after_refusal, revision_before);
assert!(matches!(resolved_pin, ContractTarget::Pinned { version: 1, .. }));
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test task_contract_refs --test phase_dependencies --test task_context`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Resolve in the edit/approval transaction through Plans and Design public _in APIs. No cross-service SQL. Consumers hold reference identity/purpose, not copied signature fields. Use existing graph cycle machinery; do not invent a second scheduler.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test task_contract_refs --test phase_dependencies --test task_context`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-plans/src/contract_refs.rs crates/core-plans/src/store/contract_refs.rs crates/shadows-core/tests/task_contract_refs.rs crates/core-plans/src/lib.rs crates/core-plans/src/dependency_graph.rs crates/core-plans/src/store/edit.rs crates/core-plans/src/task_context.rs crates/core-plans/src/store/task_context.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml
git commit -m "feat(core): resolve producers and consumers in the existing graph"
```

### Task 3: B-105 Minimal accepted decision revisions

**Owner:** Design.

**Files:**
- Create: `crates/core-design/src/decisions.rs`
- Create: `crates/core-design/src/store/decisions.rs`
- Create: `crates/core-db/migrations/0057_design_decisions.sql`
- Create: `crates/shadows-core/tests/design_decisions.rs`
- Modify: `crates/core-design/src/lib.rs`
- Modify: `crates/core-design/src/model.rs`
- Modify: `crates/core-design/src/store/mod.rs`
- Modify: `crates/core-plans/src/contract_refs.rs`
- Modify: `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-design/contract.yaml`
- Test: `crates/shadows-core/tests/design_decisions.rs`

**Interfaces:**
- Consumes: DecisionInput/Record and current ApprovedBy authority from Grants; contract DecisionPin.
- Produces: Design::decisions(&self,project:&ProjectId) -> Result<Vec<DecisionRecord>,DesignError>; propose_decision_for(&self,grant:&Grant,command:String,input:DecisionInput) -> Result<DecisionRecord,DesignError>; accept_decision_for(&self,grant:&Grant,command:String,id:&str,version:i64) -> Result<DecisionRecord,DesignError>; person variants derive human actor. Person operations: Design::propose_decision(&self,command:String,project:&ProjectId,input:DecisionInput) -> Result<DecisionRecord,DesignError>; Design::accept_decision(&self,command:String,project:&ProjectId,id:&str,version:i64) -> Result<DecisionRecord,DesignError>.

- [ ] **Step 1: Write the failing tests**

In the named test module add `decision_revision_preserves_approved_history`, `executor_cannot_accept_decision`, `decision_replay_returns_original_revision`. Accept decision v1, propose v2 with a new rationale, and read a task pinned to v1. Attempt approval with an executor grant and repeat a command after a later edit. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(pinned.version, 1);
assert_eq!(pinned.rationale, "Keep framework layout");
assert!(executor_approval.is_err());
assert_eq!(replayed.id, first.id);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows-core --test design_decisions --test task_contract_refs`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

One Design-owned immutable revision table and current status transitions. Current section 24 attribution applies; no new identity proof or decision graph. Validate accepted task pins through Design API in the same Plans snapshot.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows-core --test design_decisions --test task_contract_refs`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/core-design/src/decisions.rs crates/core-design/src/store/decisions.rs crates/core-db/migrations/0057_design_decisions.sql crates/shadows-core/tests/design_decisions.rs crates/core-design/src/lib.rs crates/core-design/src/model.rs crates/core-design/src/store/mod.rs crates/core-plans/src/contract_refs.rs docs/codebase/README.md docs/backend/stages.md crates/core-plans/contract.yaml crates/core-design/contract.yaml
git commit -m "feat(core): minimal accepted decision revisions"
```

### Task 4: B-106 Expose contract editing, context and decisions

**Owner:** Plans/Design adapters.

**Files:**
- Modify: `crates/shadows-mcp/src/plan_tools.rs`
- Modify: `crates/shadows-mcp/src/workspace_tools.rs`
- Modify: `crates/shadows-mcp/src/agreement_tools.rs`
- Modify: `crates/shadows-mcp/src/server.rs`
- Modify: `crates/shadows-http/src/workflow.rs`
- Modify: `crates/shadows-http/src/design.rs`
- Modify: `crates/shadows-http/src/lib.rs`
- Modify: `crates/shadows-http/src/openapi.rs`
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
- Consumes: B-103..105 types; existing plan_edit task_add/task_update and workflow reads.
- Produces: Generated contract types on workflow/task endpoints; decision routes/tools per shared agreement; Planner instruction requires named work/inputs/returns/errors and references.

- [ ] **Step 1: Write the failing tests**

In the named test module add `planner_contract_names_behavior_without_function_body`, `http_and_mcp_contract_content_matches`. MCP writes a contract with a function obligation and an artifact task without functions; HTTP reads equal content. Legacy omission remains readable. Planner instructions say to inspect files and leave implementation bodies to the executor. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```rust
assert_eq!(http_task["contract"], mcp_task["contract"]);
assert!(prompt.contains("inputs"));
assert!(prompt.contains("returns"));
assert!(!artifact_task_requires_fake_function);
```

- [ ] **Step 2: Run and observe RED**

Run from repository root: `cargo test -p shadows --test planner_mcp --test plan_routes`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Generate schemas from owner types, preserving existing edit/replay/revision semantics. Validate richer fields in Core; show structured errors through current adapters. Add decisions to workspace context as bounded relevant pins, not the full transcript.

- [ ] **Step 4: Run and observe GREEN**

Run: `cargo test -p shadows --test planner_mcp --test plan_routes`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `cargo fmt --all --check`; expected exit 0. Run architecture/codemap on source ownership changes; preserve any pre-existing unrelated failure as a reported blocker, not a hidden pass.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- crates/shadows-mcp/src/plan_tools.rs crates/shadows-mcp/src/workspace_tools.rs crates/shadows-mcp/src/agreement_tools.rs crates/shadows-mcp/src/server.rs crates/shadows-http/src/workflow.rs crates/shadows-http/src/design.rs crates/shadows-http/src/lib.rs crates/shadows-http/src/openapi.rs crates/core-grants/src/tools.rs crates/core-harness/src/setup.rs crates/shadows-core/src/lib.rs api/openapi.json docs/codebase/README.md docs/backend/stages.md crates/core-harness/contract.yaml crates/core-grants/contract.yaml crates/shadows-http/src/contract.yaml crates/shadows-mcp/src/contract.yaml crates/shadows/tests/planner_mcp.rs
git commit -m "feat(core): expose contract editing, context and decisions"
```

## Slice acceptance and handoff

Complete the companion frontend task before declaring the slice user-runnable. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the paired web gate after API regeneration. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

