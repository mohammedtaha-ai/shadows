# Local delivery web views Implementation Plan

**Slice 4 revision, 2026-11-14:** F-102 below is superseded by
[F-102 in the detailed slice 4 plan](../../backend/plans/2026-11-14-slice-4-task-questions.md#task-f-102-saved-brief-and-conflict-presentation).
Other tasks in this earlier draft retain their existing scope and blocked status.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Preserve the user's execution-method choice at handoff; this plan itself does not begin implementation.

**Goal:** Show real files, precise task work and evidence-backed results through existing Core APIs.

**Architecture:** Extend the existing AppCore owners and their private stores. Adapters translate and web displays Core results; no second composition root, task authority store or framework layout template.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, shadows-process, React, TanStack Query/Router, Vitest.

**Spec:** [approved section 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09). [Shared model/API agreement](../../shared/requests/2026-11-09-local-delivery.md). [Program](../../backend/plans/2026-11-09-local-delivery-program.md).

**Prerequisites:** Each task waits for the backend IDs named in its Interfaces; same shared request for all views.

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
- Use generated schema.d.ts only after the named backend endpoint ships; all calls use api/ transport.
- Run npm run gen:api after an upstream OpenAPI change; never hand-edit schema.d.ts.
- Existing testing1 and user databases are not destructive acceptance fixtures.

## Review Focus

- **F-99 / `files_view_does_not_show_secret_source`:** Inventory metadata must not become a secret-content preview.
- **F-100 / `preparation_retry_keeps_command_identity`:** A lost response must not start another scaffold.
- **F-101 / `contract_view_separates_permissions_from_work`:** docker/** scope must not replace the named Redis file work.
- **F-103 / `committed_attempt_with_failed_check_is_not_verified`:** A commit/report must not look like successful acceptance.
- **F-104 / `busy_owner_and_third_failure_remain_actionable`:** The person must see why automatic repair stopped.

## File Structure

Each task's Create entries define focused files for that task's job; Modify entries name current integration points. New domain types are defined by the shared agreement and exported by their owning crate. Test fixtures stay in the named test module, reusing the existing `code_index`, `runs_attempts` or UI TestApp patterns; no production test facade is added. Modified source symbols are rechecked against the current checkout with the LSP before a task is detailed.

## Tasks

### Task 1: F-99 Files view and server status

**Owner:** web API and design UI.

**Files:**
- Create: `web/src/app/design/files-view.tsx`
- Create: `web/src/app/design/files-view.test.tsx`
- Modify: `web/src/api/code.ts`
- Modify: `web/src/api/code.test.ts`
- Modify: `web/src/app/design/workspace-page.tsx`
- Modify: `web/src/router.tsx`
- Modify: `web/src/api/schema.d.ts`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `docs/frontend/stages.md`
- Modify: `web/src/app/design/contract.yaml`
- Test: `web/src/app/design/files-view.test.tsx`

**Interfaces:**
- Consumes: B-99 inventory/servers routes; generated InventoryPage/LspStatus. Backend owns page ordering, exclusions and readiness.
- Produces: inventoryQuery(project:string,query:InventoryQuery) and serverStatusQuery(project:string) in api/code.ts; FilesView({projectId}:{projectId:string}); Workspace view=files.

- [ ] **Step 1: Write the failing tests**

In the named test module add `files_view_does_not_show_secret_source`, `files_view_pages_and_refreshes_backend_state`. Mock a two-page Laravel tree with composer.json/User.php and metadata-only .env. Show incomplete then refreshed ready state and one configured server. Unknown view still falls back to Vision. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(app.text()).toContain('composer.json');
expect(app.text()).toContain('User.php');
expect(app.text()).not.toContain('SECRET_CANARY');
expect(app.text()).toContain('Configured');
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/design/files-view.test.tsx src/api/code.test.ts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Use only api/code.ts hooks through the existing transport. List exact paths and reasons, with refresh/load-more. Add Files tab to existing workspace; no new filesystem browser service. Server setup shows missing/configured/ready honestly and provides installation guidance without running all installers.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/design/files-view.test.tsx src/api/code.test.ts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/app/design/files-view.tsx web/src/app/design/files-view.test.tsx web/src/api/code.ts web/src/api/code.test.ts web/src/app/design/workspace-page.tsx web/src/router.tsx web/src/api/schema.d.ts web/src/api/contract.yaml docs/frontend/codebase/README.md docs/frontend/stages.md web/src/app/design/contract.yaml
git commit -m "feat(web): files view and server status"
```

### Task 2: F-100 Preparation status from the Planner's project

**Owner:** web API and design UI.

**Files:**
- Create: `web/src/api/preparation.ts`
- Create: `web/src/api/preparation.test.ts`
- Create: `web/src/app/design/preparation-panel.tsx`
- Create: `web/src/app/design/preparation-panel.test.tsx`
- Modify: `web/src/app/design/files-view.tsx`
- Modify: `web/src/api/schema.d.ts`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `docs/frontend/stages.md`
- Modify: `web/src/app/design/contract.yaml`
- Test: `web/src/app/design/preparation-panel.test.tsx`

**Interfaces:**
- Consumes: B-102 preparation read/start/stop and B-99 inventory refresh.
- Produces: preparationsQuery(project:string), preparationQuery(id:string), startPreparation(project:string,input:PrepareRequest), stopPreparation(id:string,command:string); PreparationPanel({projectId}).

- [ ] **Step 1: Write the failing tests**

In the named test module add `preparation_retry_keeps_command_identity`, `interrupted_preparation_shows_partial_work`. Start loses the first response, then retry returns the same preparation. Show partial/interrupted files and Stop. Existing Laravel is shown for adoption rather than a destructive reinstall. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(new Set(sentBodies.map(x => x.command_id)).size).toBe(1);
expect(app.text()).toContain('Interrupted');
expect(app.text()).toContain('backend/composer.json');
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/design/preparation-panel.test.tsx src/api/preparation.test.ts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Preserve mutation command identity across lost responses using existing command-id conventions. Poll only while running, render server refusals. Framework/destination display comes from recorded request; don't invent a second readiness check in React.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/design/preparation-panel.test.tsx src/api/preparation.test.ts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/api/preparation.ts web/src/api/preparation.test.ts web/src/app/design/preparation-panel.tsx web/src/app/design/preparation-panel.test.tsx web/src/app/design/files-view.tsx web/src/api/schema.d.ts web/src/api/contract.yaml docs/frontend/codebase/README.md docs/frontend/stages.md web/src/app/design/contract.yaml
git commit -m "feat(web): preparation status from the planner's project"
```

### Task 3: F-101 Readable task contract and decision pins

**Owner:** web API, task and workflow UI.

**Files:**
- Create: `web/src/app/tasks/contract-section.tsx`
- Create: `web/src/app/tasks/contract-section.test.tsx`
- Create: `web/src/api/decisions.ts`
- Create: `web/src/api/decisions.test.ts`
- Modify: `web/src/api/workflows.ts`
- Modify: `web/src/app/tasks/details-tab.tsx`
- Modify: `web/src/app/workflows/inspect-panel.tsx`
- Modify: `web/src/api/schema.d.ts`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `docs/frontend/stages.md`
- Modify: `web/src/app/tasks/contract.yaml`
- Modify: `web/src/app/workflows/contract.yaml`
- Test: `web/src/app/tasks/contract-section.test.tsx`

**Interfaces:**
- Consumes: B-106 optional TaskContract and decision APIs; no client-side contract validation duplication.
- Produces: ContractSection({contract}:{contract:TaskContract}); decisionsQuery(project:string); symbols/files/uses/produces/steps/checks rendered as separate semantic sections.

- [ ] **Step 1: Write the failing tests**

In the named test module add `contract_view_separates_permissions_from_work`, `contract_view_shows_inputs_returns_and_producer`. Show docker/** permission plus a named create docker/redis/redis.conf action. Show a method's inputs/returns/errors and producer T1 separately. Missing legacy contract displays an explicit legacy label. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(app.text()).toContain('docker/**');
expect(app.text()).toContain('docker/redis/redis.conf');
expect(app.text()).toContain('findCustomer');
expect(app.text()).toContain('Legacy task');
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/tasks/contract-section.test.tsx src/api/decisions.test.ts`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Keep permissions visible but give named work the main detail. No full function body editor. Task reads show reasons, symbol contract, output references and accepted decision revisions; unresolved pins show Core's reason. Render all source/contract text safely with per-block direction.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/tasks/contract-section.test.tsx src/api/decisions.test.ts`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/app/tasks/contract-section.tsx web/src/app/tasks/contract-section.test.tsx web/src/api/decisions.ts web/src/api/decisions.test.ts web/src/api/workflows.ts web/src/app/tasks/details-tab.tsx web/src/app/workflows/inspect-panel.tsx web/src/api/schema.d.ts web/src/api/contract.yaml docs/frontend/codebase/README.md docs/frontend/stages.md web/src/app/tasks/contract.yaml web/src/app/workflows/contract.yaml
git commit -m "feat(web): readable task contract and decision pins"
```

### Task 4: F-102 Attempt brief and scoped question results

**Owner:** web API and task UI.

**Files:**
- Create: `web/src/app/tasks/brief-panel.tsx`
- Create: `web/src/app/tasks/brief-panel.test.tsx`
- Modify: `web/src/api/runs.ts`
- Modify: `web/src/app/tasks/run-tab.tsx`
- Modify: `web/src/api/schema.d.ts`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `docs/frontend/stages.md`
- Modify: `web/src/app/tasks/contract.yaml`
- Test: `web/src/app/tasks/brief-panel.test.tsx`

**Interfaces:**
- Consumes: B-109 brief endpoint and TaskAnswer tool entries; existing run transcript rendering.
- Produces: attemptBriefQuery(attempt:string); BriefPanel({attemptId}:{attemptId:string}).

- [ ] **Step 1: Write the failing tests**

In the named test module add `brief_view_keeps_original_version`, `planned_symbol_is_not_rendered_as_existing_code`. A run retains v1 while the workspace has v2 Draft. Display its exact attempt/task/dependency source; a planned function result has no existing source hit; out-of-scope/stale answers stay visibly refused. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(app.text()).toContain('Version 1');
expect(app.text()).toContain('Planned by T2');
expect(app.text()).toContain('outside_task_reads');
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/tasks/brief-panel.test.tsx`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Display the immutable brief and Core's distinction among source, LSP result and obligation. Do not recompute dependencies from latest plan data or expose raw unrestricted file reads.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/tasks/brief-panel.test.tsx`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/app/tasks/brief-panel.tsx web/src/app/tasks/brief-panel.test.tsx web/src/api/runs.ts web/src/app/tasks/run-tab.tsx web/src/api/schema.d.ts web/src/api/contract.yaml docs/frontend/codebase/README.md docs/frontend/stages.md web/src/app/tasks/contract.yaml
git commit -m "feat(web): attempt brief and scoped question results"
```

### Task 5: F-103 Check evidence, critic model and acceptance

**Owner:** web API and task UI.

**Files:**
- Create: `web/src/app/tasks/assessment-panel.tsx`
- Create: `web/src/app/tasks/assessment-panel.test.tsx`
- Modify: `web/src/api/runs.ts`
- Modify: `web/src/app/tasks/details-tab.tsx`
- Modify: `web/src/app/tasks/report-tab.tsx`
- Modify: `web/src/app/tasks/task-header.tsx`
- Modify: `web/src/app/tasks/use-task-run.ts`
- Modify: `web/src/app/tasks/task-page.test.tsx`
- Modify: `web/src/api/schema.d.ts`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `docs/frontend/stages.md`
- Modify: `web/src/app/tasks/contract.yaml`
- Test: `web/src/app/tasks/assessment-panel.test.tsx`

**Interfaces:**
- Consumes: B-114 assessment APIs, critic_model Start input, acceptance_source; B-113 person accept-notes.
- Produces: attemptAssessmentQuery(attempt:string), runChecks(attempt:string,command:string), acceptNotes(attempt:string,command:string); AssessmentPanel({attemptId}).

- [ ] **Step 1: Write the failing tests**

In the named test module add `committed_attempt_with_failed_check_is_not_verified`, `manual_marks_and_notes_are_labeled`. Committed executor report plus failed check must show failure, not verified. Show testing HEAD, timeout/interruption, truncation and per-item critic findings. PASS_WITH_NOTES waits for person; legacy done stays manual. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(app.text()).toContain('Failed');
expect(app.text()).not.toContain('Verified');
expect(app.text()).toContain('Manual');
expect(app.text()).toContain('Needs your decision');
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/tasks/assessment-panel.test.tsx src/app/tasks/task-page.test.tsx`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Poll assessment independently from executor completion; preserve explicit text statuses and evidence links. Add critic model selection without exposing unavailable harness options as ready. Server acceptance remains authoritative. Plain text for report/check output.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/tasks/assessment-panel.test.tsx src/app/tasks/task-page.test.tsx`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/app/tasks/assessment-panel.tsx web/src/app/tasks/assessment-panel.test.tsx web/src/api/runs.ts web/src/app/tasks/details-tab.tsx web/src/app/tasks/report-tab.tsx web/src/app/tasks/task-header.tsx web/src/app/tasks/use-task-run.ts web/src/app/tasks/task-page.test.tsx web/src/api/schema.d.ts web/src/api/contract.yaml docs/frontend/codebase/README.md docs/frontend/stages.md web/src/app/tasks/contract.yaml
git commit -m "feat(web): check evidence, critic model and acceptance"
```

### Task 6: F-104 Repair state and replan controls

**Owner:** web API and task UI.

**Files:**
- Create: `web/src/app/tasks/repair-panel.tsx`
- Create: `web/src/app/tasks/repair-panel.test.tsx`
- Modify: `web/src/api/runs.ts`
- Modify: `web/src/app/tasks/report-tab.tsx`
- Modify: `web/src/app/tasks/task-header.tsx`
- Modify: `web/src/api/schema.d.ts`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `docs/frontend/stages.md`
- Modify: `web/src/app/tasks/contract.yaml`
- Test: `web/src/app/tasks/repair-panel.test.tsx`

**Interfaces:**
- Consumes: B-117 repair/retry APIs, existing Stop/continue-plan navigation; Core decides caps and readiness.
- Produces: attemptRepairQuery(attempt:string), retryAttempt(attempt:string,input:RepairCommand); RepairPanel({attemptId}).

- [ ] **Step 1: Write the failing tests**

In the named test module add `busy_owner_and_third_failure_remain_actionable`, `repair_duplicate_click_does_not_duplicate_request`. Display waiting for busy owner, directed third attempt, missing owner and stopped_for_person. A new Draft shows stopping/plan_replanning. Retry duplicate click keeps command identity; no automatic fourth attempt. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(app.text()).toContain('Waiting for Planner');
expect(app.text()).toContain('Stopped for you');
expect(app.text()).toContain('plan_replanning');
expect(new Set(commands).size).toBe(1);
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/tasks/repair-panel.test.tsx`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Show numbered attempts and reasons. Expose Stop, explicit retry and existing plan-edit entry point. No automatic reset/rollback UI, client-side loop or duplicated retry count policy.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/tasks/repair-panel.test.tsx`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/app/tasks/repair-panel.tsx web/src/app/tasks/repair-panel.test.tsx web/src/api/runs.ts web/src/app/tasks/report-tab.tsx web/src/app/tasks/task-header.tsx web/src/api/schema.d.ts web/src/api/contract.yaml docs/frontend/codebase/README.md docs/frontend/stages.md web/src/app/tasks/contract.yaml
git commit -m "feat(web): repair state and replan controls"
```

### Task 7: F-105 Browser acceptance of the complete local journey

**Owner:** Frontend/controller.

**Files:**
- Create: `web/src/app/tasks/local-delivery.test.tsx`
- Modify: `docs/frontend/stages.md`
- Modify: `docs/backend/stages.md`
- Modify: `web/src/api/contract.yaml`
- Modify: `docs/frontend/codebase/README.md`
- Modify: `web/src/app/tasks/contract.yaml`
- Test: `web/src/app/tasks/local-delivery.test.tsx`

**Interfaces:**
- Consumes: F-99..104 and B-118; real browser/daemon trial is distinct from fixtures.
- Produces: Fixture journey and dated browser/log acceptance tied to implementation commit/harness/OS.

- [ ] **Step 1: Write the failing tests**

In the named test module add `browser_journey_connects_files_contract_checks_and_repair`. Fixture app navigates files -> preparation -> exact task contract -> run -> failed check -> critic -> repair -> verified result. Real disposable Laravel journey repeats it without touching testing1. The fixture creates the scenario above; the assertions below are the required observations, not a supplied implementation body.

```typescript
expect(journey.visited).toEqual(['files','preparation','contract','run','assessment','repair']);
expect(journey.finalStatus).toBe('Verified');
expect(journey.changedExistingTestingProject).toBe(false);
```

- [ ] **Step 2: Run and observe RED**

Run from web/: `npx vitest run src/app/tasks/local-delivery.test.tsx`.
Expected: new named tests fail for the missing operation/type or the stated behavior; an unrelated setup failure does not count as RED.

- [ ] **Step 3: Implement the Interfaces in the listed files**

Use existing TestApp/answers fixtures for automated UI coverage. During real acceptance observe actual daemon logs and source changes; don't equate fixtures with a real provider. Record untested platform/server cases as such.

- [ ] **Step 4: Run and observe GREEN**

Run: `npx vitest run src/app/tasks/local-delivery.test.tsx`.
Expected: every named test and the affected existing tests pass, exit 0.
Also run `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture.test.ts`; expected exit 0.

- [ ] **Step 5: Commit this verified task**

Set its stages status with the observed command/result and update the named contracts/maps. Stage only the paths listed above, including a newly generated API file only when this task changes public routes/types.

```powershell
git add -- web/src/app/tasks/local-delivery.test.tsx docs/frontend/stages.md docs/backend/stages.md web/src/api/contract.yaml docs/frontend/codebase/README.md web/src/app/tasks/contract.yaml
git commit -m "feat(web): browser acceptance of the complete local journey"
```

## Slice acceptance and handoff

Use the paired backend plan and the user-visible outcomes in each task. Start `shadows serve`, use the browser and inspect logs on a disposable project. Record implementation commit, OS, harness/model, commands and remaining untested cases. New tests are real (real folder, real Git, real language server, live adapter); see the program rulings. The final task of the slice records that evidence before delivery.

The full gate runs in CI after the slice branch is pushed; each task runs its own tests locally. Run the affected web gate above as well. API changes are limited to intended route/type diffs. One fresh whole-branch review follows implementation, not a reviewer after every step. Do not claim this planning document is executed evidence.

