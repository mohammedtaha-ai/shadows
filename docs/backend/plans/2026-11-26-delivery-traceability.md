# Delivery Traceability Implementation Plan

> **Execution:** Use superpowers:executing-plans after the plan handoff. The numbered tasks are
> for inline execution. Each task has one scoped commit.

**Goal:** Follow the same delivery plan and exact tasks from a roadmap outcome, an owning part,
and the project's execution state.

**Architecture:** Start at [AppCore](../../../crates/shadows-core/src/app.rs).
Plans owns the associations and the approval transaction, Design owns outcomes, thread anchors
and layout, Runs owns execution progress, and ProjectState composes the read views. Each adapter
calls its owner through AppCore. The [web companion](../../frontend/plans/2026-11-26-delivery-traceability.md)
and [accepted request](../../shared/requests/2026-11-26-delivery-traceability.md) use those views.

**Tech stack:** Existing Rust workspace, SQLx/SQLite, Axum, Utoipa, rmcp, React and TanStack Query.

**Spec:** [§18.3 Delivery traceability](../../core/specs/2026-10-03-project-planning-workspace-design.md#delivery-traceability-approved-design-2026-11-26).
Approved design at `0da03feb`. Plan prepared on `feature/after-77`.

## Existing libraries and what changes

| AppCore owner | Reuse | Add |
|---|---|---|
| Design, `core-design` | `set_thread_anchor`, the anchor loader, `design_outcome_in`, `edit_workspace_in`, `layout_in`, `ProjectLayout::owners` | Transactional anchor read, and the workspace edit's body callable without recording a command, with the caller's actor. The existing owner rules resolve paths. |
| Plans, `core-plans` | `start_draft`, `approve_plan_with`, `apply_design_links_in`, `edit_design`, `TaskContext`, `live_versions`, `get_plan_scoped` | Save the anchored association, require it at approval, retain the last link, and expose task ownership and delivery references. |
| Runs, `core-runs` | `progress`, `TaskProgressChanged` and saved `AttemptBrief.context` | The existing snapshot carries the additive TaskContext fields. Existing Runs APIs and stored attempts continue to answer. |
| ProjectState, `project-state` | `state`, `facts`, `plan_state`, `detail` and current CodeRecord element facts | Outcome progress and project totals from live versions; part detail filtered by task ownership. |
| HTTP and MCP | Existing plan reads, state route, `workflow_get`, `task_context`, `task_brief`, Planner `project_state` | Extend returned data and the State selector. The person keeps editing associations through the current Design command. |

## Global constraints

- Work on the established feature branch and existing build cache. Keep SQL in the owning
  store and use `Tx` methods when one write spans Plans and Design. Deliver a contract update
  in each commit that changes an owner. Update the code map for any new module.
- Use `design_outcome_plan` keyed to the stable `PlanId`. Task execution identity is
  `(WorkflowId, task number)`. No new table, migration, backfill, grant, approval authority or
  per-task outcome selection. Initial project preparation stays outside delivery approval.
- Effective plan parts are the union of current saved part links and the write owners of the
  exact version being read or approved. Derived parts are read facts; current explicit links
  remain the editable associations. Reads do not add a plan to that part's affected-plan list.
  Part task detail can show an input-only task under the part it reads.
- Given several outcomes, show shared plan membership and count each task once inside each
  result and once in the project total. Use Runs' `done` and latest attempt, with no conversion
  of task completion into outcome acceptance. Evidence stays in task reports and acceptance items.
- Use one targeted regression per distinct behavior and update existing fixtures where the
  new precondition changes their setup. Preserve the unlinked and unauthorised controls.
  Run `cargo fmt --all --check` and the named test targets for each backend task. Run
  the repo's ownership and code-map checks on the finished Core slice. Full workspace gate
  is a separate branch-delivery step.
- On Windows, set `$env:UPDATE_OPENAPI='1'` for the generation command, then remove it
  before the verification run. Stage only the task's Touch files and commit them by
  explicit path. Record the test results with the task's commit in the execution ledger.

## Review focus

- An outcome is removed after the thread is anchored, or the draft is replayed after the
  anchor changes. B-238 covers rollback and immutable replay.
- The final link is moved in either batch order, or removed from a plan with a Frozen version
  and a newer Draft. B-239 covers final-state validation.
- A path is read and written, belongs to a nested part, has a layer owner outside the folder
  owner, or has no owner. B-237 uses the real resolver and preserves both access modes.
- A second Draft must not supersede the live approved version in State. Multiple result and
  part aliases must not inflate totals. B-241 covers these in one fixture.
- A task in another project or plan is outside the caller's grant. An older attempt brief
  must decode with absent delivery fields. B-240 pins both.

## Task 1 · B-237 · Resolve task ownership once

**Why:** TaskContext already maps paths. Approval, the plan read and State must share that
mapping, including nested folders and layer paths.

**Touch:** `crates/core-plans/src/task_context.rs`, `crates/core-plans/src/store/task_context.rs`,
`crates/core-plans/contract.yaml`, `crates/shadows-core/tests/task_context.rs`,
`docs/backend/stages.md`.

**Produces:** A Plans-internal `TaskOwnership::resolve(layout: &ProjectLayout, task: &TaskContent) -> TaskOwnership`.
Its two lists, `reads` and `writes`, each contain `ScopePath { path: String, owners: Vec<ScopeOwner> }`,
where `ScopeOwner { part_id: PartId, via: OwnedVia }`. Use the existing `OwnedVia` wire convention
(`"Folder"` or `{ "Layer": "…" }`). Make these reusable outside the store in the next tasks.

- [ ] In `task_context.rs`, write one unit test `task_ownership_preserves_access_and_nested_owners`.
  Set a parent folder `water`, a nested part `water/internal/store`, and a matching
  infrastructure layer on a different part. Resolve a shared read/write scope
  `water/internal/store/**` and an unowned scope `outside/**`. Assert the shared scope has
  both the folder and the layer owner in each list, and the unmatched scope has no owner.
  Keep the existing task-context integration tests for the legacy grouped context.
- [ ] Run `cargo test -p core-plans --lib task_ownership_preserves_access_and_nested_owners`.
  Expected RED because the shared resolver is absent.
- [ ] Move path resolution out of `PlansStore::task_context_in` into the pure resolver. Iterate in
  input order, deduplicate repeated paths within each access list, call `layout.owners` once per
  distinct path, and preserve the returned folder and layer owners. Build existing
  `TaskContext.parts` and `unowned_paths` from the union in read-then-write order. Retain explicitly
  tied parts in the legacy context even when they have no scope path. Keep scope enforcement
  in its existing owner.
- [ ] Run the focused unit command again and `cargo test -p shadows-core --test task_context`.
  Expected GREEN, including the original task-context assertions. Commit
  `refactor(plans): share task path ownership resolution`.

## Task 2 · B-238 · Link a Planner draft from its outcome

**Why:** Starting at an outcome currently sets a thread anchor. The new plan must appear under
that result even when nobody remembers to use the checkbox, and the Planner names no outcome.

**Touch:** `crates/core-design/src/anchors.rs`, `crates/core-design/src/store/anchors.rs`,
`crates/core-design/src/store/vision.rs`, `crates/core-design/contract.yaml`,
`crates/core-plans/src/store/draft.rs`, `crates/core-plans/contract.yaml`,
`crates/shadows/tests/workspace_mcp.rs`, `crates/shadows/tests/workspace_mcp/delivery.rs` (new),
`docs/backend/stages.md`.

**Consumes:** `Writer::Planner { thread, .. }`, `Writer::actor`, `Design::design_outcome_in` and
`WorkspaceOp::PlanLinkPut`.
**Produces:** `Design::thread_anchor_in(tx: &mut Tx, thread: &ThreadId) -> Result<Option<DesignAnchor>, StorageError>`
using one loader, and `Design::apply_workspace_ops_in(conn, actor: &Actor, project, ops, facts)
-> Result<DesignChange, DesignError>`: the body of `edit_workspace_in` without its revision
check against a caller's expectation and without recording a command. `edit_workspace_in`
keeps its signature, checks `expected_revision`, calls the body, then records its command, so
person edits behave as today.

- [ ] In `workspace_mcp/delivery.rs`, write
  `an_outcome_anchored_draft_links_once_and_a_removed_anchor_rolls_back`. Use the existing
  `workspace_mcp` Planner fixture to set a real thread anchor and invoke `draft_start` during a
  turn. Assert the outcome references the exact returned `PlanId`, the workspace revision and
  that outcome's revision each advance once, and one `ProjectDesignChanged` names the outcome
  with the Planner as actor. An identical replay answers the same IDs and changes no revision.
  An outcome editor save sent with the revision read before the draft is refused as a revision
  conflict. On a second turn the anchored outcome is removed before the draft; assert a
  refusal, no new plan and no extra draft or design event. Include a plain Planner turn with
  no outcome anchor as a control; it creates an unlinked draft.
- [ ] Run `cargo test -p shadows --test workspace_mcp outcome_anchored_draft`.
  Expected RED at the missing association.
- [ ] Extract the existing anchor lookup into `thread_anchor_in`; leave standalone reads using
  that loader. Split `edit_workspace_in` into the unrecorded body and its recorded wrapper;
  the event's actor comes from the caller instead of `Actor::user(&ctx.principal_id)`.
- [ ] In `PlansStore::start_draft`, after the recorded-command replay check, only in the
  **new plan** branch and only for `Writer::Planner`, read the anchor inside the existing
  transaction. For an outcome anchor, validate it with `Design::design_outcome_in` in that
  project, then call `apply_workspace_ops_in` with one `PlanLinkPut` for the new `PlanId` and
  the writer's actor, then `Plans::apply_design_links_in` with the same op. The outer
  `DraftStart` command records once, so its replay covers the link and no second command id
  is derived. A Part anchor or no anchor goes through draft creation as today; a new version
  of an existing plan keeps the PlanId's current links.
- [ ] Run `cargo test -p shadows --test workspace_mcp` and
  `cargo test -p core-turns --test planning_context`. Expected GREEN, including current
  workspace edit attribution and navigation tests. Commit `feat(plans): attach outcome-anchored drafts`.

## Task 3 · B-239 · Enforce delivery links at approval

**Why:** The approval precondition applies whether a person presses Approve or the Planner
approves at the person's request. The last link must survive subsequent workspace edits.

**Touch:** `crates/core-plans/src/design_edit.rs`, `crates/core-plans/src/store/design_links.rs`,
`crates/core-plans/src/store/design_parts.rs`, `crates/core-plans/src/store/edit.rs`,
`crates/core-plans/src/{model,testing}.rs`, `crates/core-plans/contract.yaml`,
`crates/core-design/src/lib.rs`, `crates/core-design/src/store/roadmap_seed.rs`,
`crates/core-design/contract.yaml`, `crates/shadows-core/tests/design_parts/roadmap_gate.rs`,
`crates/shadows/tests/workspace_mcp/delivery.rs`, `crates/shadows/tests/design_routes.rs`,
the explicit fixture families below, and `docs/backend/stages.md`.

**Consumes:** B-237 ownership and B-238's outcome link at draft start.
**Produces:** `Plans::plan_outcome_ids_in(tx: &mut Tx, project: &ProjectId, plan: &PlanId)
-> Result<Vec<OutcomeId>, StorageError>` from the current join table, ordered by outcome ID.
Under the same owner use
`Plans::effective_parts(explicit: &[PartId], tasks: &[TaskOwnership]) -> Vec<PartId>` and
`Plans::missing_delivery_requirements(parts: &[PartId], outcomes: &[OutcomeId]) -> Vec<DeliveryRequirement>`.
Define the `DeliveryRequirement` enum in `model.rs` with lowercase wire keys.
Requirement order is `part`, then `outcome`. Use the existing `STAGE_BLOCKED` mapping with
`plan has no part` for the first existing case and
`plan has no roadmap outcome: link it on the Roadmap page` for the second.

- [ ] Extend `roadmap_gate.rs`: explicit part with no outcome stays Draft, then attaching one
  and retrying freezes that exact version. Include an approved plan with two outcomes; delete
  one, refuse removing the last, then move the last across two outcomes in one batch. Confirm
  a failed transaction changes neither outcome revisions nor links. Add a scoped Planner
  assertion to B-238's test fixture with a real person approval request.
- [ ] Run `cargo test -p shadows-core --test design_parts roadmap_gate` and the selected
  `workspace_mcp` delivery case. Expected RED because approval currently accepts a part-only plan.
- [ ] After authorisation, command replay and the existing Draft/revision checks in
  `approve_plan_with`, read effective parts from explicit links plus the resolved task writes and
  read outcome links. Check both in the same transaction before freezing. The outcome table's
  composite foreign keys already protect project membership and existence. Leave old approved
  versions readable and their recorded approval replays valid.
- [ ] In the common workspace edit, capture each outcome-removal candidate with a nonempty
  initial association and an Active plan with any Frozen version. Apply the full batch of
  links and check that each candidate has at least one outcome afterwards. A business refusal
  rolls back the underlying Design revision and event as well. A Draft-only plan or legacy
  unlinked record remains editable. A separate draft does not deactivate an approved plan.
- [ ] Adapt fixtures through explicit setup. Add test-support
  `Design::seed_delivery_outcome_for_tests(project) -> OutcomeId` in the existing seed module.
  It creates or returns one named fixture result, after roadmap seeding, without changing
  `seed_roadmap_for_tests` semantics. Add
  `Plans::link_plan_outcome_for_test(project, plan, outcome)` with the existing transactional
  validation and link writer. Call these in normal approval fixture setup. Leave
  `approve_plan_for_test`, real approval routes, approval-policy negative controls and live
  scripts using the same gate. Preserve existing event/fingerprint assertions.
- [ ] Run the roadmap-gate, workspace MCP, design-route and affected fixture suites below.
  Expected GREEN with both approval principals and atomic move coverage. Commit
  `feat(plans): require and retain delivery links`.

**Fixture touch inventory.** Make changes at the common helper first. For direct consumers,
add explicit fixture link setup only where that test expects approval or an unrelated approval
refusal. These files are the candidate scopes, and their existing assertions still hold.

- `crates/shadows/tests/fixtures/plan.rs` (`draft_on`, `tie_to_part`, `approved_v1`);
  `crates/shadows/tests/{checks_live,core_plans,mcp_server,mcp_tools,plan_routes,plan_storage,planner_runs_live,planning_context,project_plans,task_questions_live}.rs`;
  `crates/shadows/tests/runs_routes/live.rs`;
  `crates/shadows/tests/workspace_mcp/{approval,roadmap}.rs`.
- `crates/shadows-core/tests/fixtures/cross_plan.rs`;
  `crates/shadows-core/tests/{attempt_briefs,cross_plan_resolution,cross_plan_views,executor_session,phase_dependencies,ready_groups,run_admission,runs_attempts,task_conflicts,task_context,task_contract,task_questions}.rs`;
  `crates/shadows-core/tests/runs_attempts/{checks,contract_check,executor_choice,planner,project_state,rechecks}.rs`.
- For each edited test target run the corresponding `cargo test -p <owning crate> --test <target>`
  with the ordinary ignore defaults. A new live execution is Task 10. To find an indirect caller
  while implementing, use references to `approve_plan_with` and the route fixture rather than
  copying a permissive seed across the test tree.

## Task 4 · B-240 · Expose the same links in plan and task reads

**Why:** The web, Planner and executor need the same outcome and ownership facts without
guessing them from task titles or relying on agent-written notes.

**Touch:** `crates/core-plans/src/{lib,model,task_context}.rs`,
`crates/core-plans/src/store/{read,task_context,design_links,design_parts}.rs`,
`crates/core-plans/contract.yaml`, `crates/core-runs/contract.yaml`,
`crates/shadows-core/tests/{task_context,attempt_briefs}.rs`,
`crates/shadows/tests/{plan_routes,mcp_tools}.rs`, `api/openapi.json`,
`docs/backend/stages.md`.

**Produces:** `Plan.delivery?: PlanDelivery` on public reads, and additive
`TaskContext.outcomes?: DeliveryOutcome[]` and `TaskContext.ownership?: TaskOwnership`.
Exact field types are in the shared request. Owner/title data comes from Design in the same
reader transaction. The core PlanContent and TaskContent stay the canonical frozen input.
Define response DTOs in `model.rs`, retaining the ownership wire types from B-237 in
`task_context.rs`. Re-export the response types from the existing `lib.rs` lists.
Use `Plans::delivery_outcomes_in(tx, project, plan)` in both public reads,
calling B-239's ID read and the existing Design outcome read.

- [ ] In existing plan route and task-context integration tests, assert an exact-version
  task has correct read and write paths plus inherited outcome IDs. Keep an external grant's
  foreign-project denial. Extend the existing attempt-brief test to assert context fields on a
  new attempt and successful decoding of a stored context without the additive fields.
- [ ] Run `cargo test -p shadows-core --test task_context --test attempt_briefs` and
  `cargo test -p shadows --test plan_routes`. Expected RED at the absent response fields.
- [ ] Fill `Plan.delivery` at the boundary of `PlansStore::read_plan`, after existing scoped plan
  authorization. Load the exact plan version's tasks, one Design layout and associated outcomes
  in the same snapshot. Fill the same task ownership and outcome fields in
  `PlansStore::task_context_in`; use serde defaults for stored old briefs. The existing MCP reads
  and attempt briefing reuse those methods. Return approval requirements for active Drafts via
  B-239's method. The public read projection never enters `Plan::content()` or old approval
  journals.
- [ ] Regenerate `api/openapi.json` with `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
  Then run `cargo test -p shadows --test plan_routes --test mcp_tools --test openapi` and the B-240
  Core test command. Expected GREEN and an additive schema diff. Commit
  `feat(plans): expose delivery links on plan and task reads`.

## Task 5 · B-241 · Compose roadmap and map progress in ProjectState

**Why:** Roadmap and Map should consume the same real task state as State. One task can appear
in multiple places while counting once in project totals.

**Touch:** `crates/core-plans/src/model.rs`, `crates/core-plans/src/store/live_versions.rs`,
`crates/core-plans/src/live_versions.rs`,
`crates/core-plans/contract.yaml`, `crates/project-state/src/{state,model,lib}.rs`,
`crates/project-state/src/error.rs`, `crates/project-state/contract.yaml`,
`crates/shadows-core/src/{lib,error}.rs`,
`crates/shadows-http/src/{project_state.rs,contract.yaml}`,
`crates/shadows-mcp/src/state_tools.rs`, `crates/shadows-core/tests/runs_attempts/project_state.rs`,
`crates/shadows/tests/{project_state_routes,element_mcp}.rs`,
`api/openapi.json`, `docs/backend/stages.md`.

**Produces:** Existing `state(project, part)` API delegates to
`state_for(&self, project: &ProjectId, part: Option<&PartId>, outcome: Option<&OutcomeId>)
-> Result<ProjectStateView, ProjectStateError>`.
HTTP and Planner MCP add `outcome_id`; both call ProjectState once. The response fields,
limits and projections are in the shared request. Validate a selected outcome through
`Plans::design_outcome`; a foreign or missing outcome returns the existing Design refusal.

- [ ] Extend the ProjectState fixture with a shared plan linked to two outcomes and writing in
  one part while reading another. One task is marked done, another has a latest attempt. Add a
  newer Draft and a Draft-only plan. Assert outcome counts use the highest Frozen version, the
  project total counts the shared tasks once, part detail includes only the matching tasks,
  Draft-only plans have no completed work, and unknown outcome selection is refused.
- [ ] Run `cargo test -p shadows-core --test runs_attempts project_state`. Expected RED for the
  missing projection. Keep a real progress assertion through `project_state_routes` and the
  existing MCP parity test.
- [ ] Extend `LiveVersion` with `outcomes: Vec<DeliveryOutcome>` and `LiveTask` with
  `ownership: TaskOwnership`. Add `Plans::live_versions_for_layout(project, &ProjectLayout)`
  over the existing live-version loader. The ordinary `live_versions(project)` path loads
  Design's layout inside its existing read transaction and uses the same loader. State supplies
  the layout it fetched for its map view. In the loader, resolve B-237 ownership from the exact
  Frozen version, use B-239's effective parts for affected-plan membership and B-240 for outcome
  references. A Draft-only row has only saved part links. State uses read/write scope owners
  separately for selected-part task detail. Read Runs progress once per live version and project the
  already loaded vector into all views. Retain existing element/gap calculation and limits.
  Outcome summary ordering is stable by outcome ID, while the Roadmap tree retains its order.
- [ ] Wire the additive selector and DTOs via existing HTTP/MCP adapters. Use
  `ProjectStateError::ConflictingSelectors` for a request with both selectors, mapped to
  `INVALID_COMMAND` through the existing CoreError conversion. Export the State DTOs
  alongside the other State models. Keep the existing
  Planner grant inventory and additive MCP semantics. Reject simultaneous `part_id` and
  `outcome_id` in ProjectState for both transports. Existing State callers continue to use
  `state(project, part)`. Count before bounding lists.
- [ ] Generate OpenAPI, then run `cargo test -p shadows --test project_state_routes --test element_mcp --test openapi`
  and the Core State suite above. Run `cargo test -p shadows --test architecture --test codemap`.
  Expected GREEN. Commit
  `feat(project-state): compose delivery progress from existing owners`.

## Tasks 6–8 · Web consumers

F-152 needs no backend task and runs first, on its own. Continue at F-150 and F-151 after
B-241, in the
[frontend plan](../../frontend/plans/2026-11-26-delivery-traceability.md).

## Task 9 · B-243 · Tell the Planner when to version and when to plan anew

**Why:** The Planner is told to continue a plan when one "fits", with no rule for what fits; in
testing8 `--days` became a plan of its own with no outcome. Once approval requires an outcome, a
Planner without a rule can also link a plan to an outcome it does not serve only to pass the check.

**Touch:** `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`,
`crates/shadows/tests/mcp_tools.rs`, `docs/backend/stages.md`.

**Needs:** B-238 and B-241, so the anchored detail's linked plans and `project_state` with
`outcome_id` exist when the rule names them.

- [ ] In `mcp_tools.rs`, write `the_planner_is_told_to_choose_a_version_or_a_new_plan` beside
  `the_planner_is_told_to_read_the_project_state`, asserting one exact prompt line: before
  `draft_start`, read the plans of the outcome the work serves (the anchor's linked plans, or
  `project_state` with its `outcome_id`); a changed way of reaching the same plan goal is that
  plan's next version; a feature with acceptance of its own is a new plan under the outcome it
  serves, or under a new outcome proposed with the roadmap flow; never link a plan to an
  outcome it does not serve to pass approval.
- [ ] Run `cargo test -p shadows --test mcp_tools the_planner_is_told_to_choose`.
  Expected RED at the missing line.
- [ ] Replace "start a new plan only when the person asks for one or none fits" in the plans
  paragraph with that rule, keeping the rest of the paragraph. Add the rule to the harness
  contract beside the `project_state` prompt line and name the test.
- [ ] Run the focused test and `cargo test -p shadows --test mcp_tools`. Expected GREEN.
  Commit `feat(harness): tell the Planner when to version a plan`.

## Task 10 · B-242 · Verify the real delivery path

**Why:** Use one small live case to verify links persist without the Planner or executor
composing them, and a marked-done task updates every linked view.

**Touch:** `docs/evidence/2026-11-26-delivery-traceability-live.md` (new) and
`docs/backend/stages.md`. Use the actual date in the evidence filename if the run is later.

- [ ] After Tasks 1–9, build in place and use the existing Claude Launch configuration and
  primary development database. Use an expressly selected disposable project or a new named
  experiment. Put a short outcome, a write part and a read-only dependency in its map.
- [ ] Start at **Plan from here**, ask for one named task, and verify the saved outcome link
  appears in Roadmap before approval. Start an ordinary draft and verify approval refuses it
  until an outcome is selected. Confirm its affected part comes from task writes. Approve and
  run one task, record the exact plan version, task, commit and acceptance output.
- [ ] In the original app view verify Roadmap, State, Map and task links resolve to that exact
  task and mark it done after the existing project event. Read the Planner tool and executor
  brief for the same IDs. Record an outcome acceptance observation separately from task status.
  Sharing the plan under a second outcome keeps the project count stable. Observe a refused
  last link removal or an atomic move. Record any provider shortage as unverified.
- [ ] Expected evidence has the feature HEAD, project/outcome/plan/version/task IDs, relevant
  durable events, and the visible result. Commit `docs(evidence): record delivery traceability live`.
