# Planning Workspace Stage 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement task-by-task after Mohammed reviews this plan and selects the execution method. Steps use checkboxes. No implementation is authorized by the existence of this file.

**Goal:** A person writes a project vision, organizes nested parts, associates existing plans and defines roadmap outcomes in a persistent browser workspace.

**Architecture:** Introduce `Design` with its first browser caller, using the existing SQLx transaction, command replay and project journal. Plans retain their identities and immutable versions; Design owns only associations. Deliver three vertical slices, then integrated acceptance.

**Tech Stack:** Existing Rust/Cargo, SQLx 0.9/SQLite, axum/utoipa, React/TypeScript, TanStack Router/Query and Vitest. No new graph, ORM or state-management dependency.

**Spec:** [§18](../specs/2026-10-03-project-planning-workspace-design.md), Stage 1 only: §§18.2–18.3, workspace navigation in §18.9, relevant ownership/persistence in §§18.10–18.12. [Roadmap](./2026-10-03-project-planning-roadmap.md).

**Status:** Proposed implementation plan, 2026-10-03. Written after the user's request to begin the first plan. The working tree also contains preceding vision/spec drafts; preserve those separately from product changes. Baseline inspected: `4bde8e1`.

## Global constraints

- SQLx 0.9 only; Rust 1.94 minimum. Use the existing lockfiles.
- “An adapter translates and holds no rule.” New operations live on Design.
- “References name identities rather than titles or breadcrumb strings.”
- “Parents belong to the same project. A part cannot be its own ancestor.”
- “Existing plans start unassigned and stay accessible in the project plan list.”
- “Whole-subtree deletion is outside the first release.” No delete-part/outcome UI.
- Approval/archive do not complete outcomes. No execution status inferred.
- No API agreements, task bindings, task-to-task links, agent profiles, MCP workspace tools, ACP changes, context compiler or specialized diagrams in this stage. MCP proposals arrive with Stage 2; the first caller here is person HTTP.
- No schema or files for later capabilities. Never migrate the live dev DB during implementation checks.
- Every touched service updates its contract and code-map rows in the same implementation commit. No unrelated cleanup; preserve `.superpowers/` and `.claude/`.
- Use the project's complete commit gate, reproduced below. No commit before its review/authorization requirements are satisfied.

## Review focus

1. Two tabs edit or move siblings: stale state refuses the entire batch without losing either tab's input (Tasks 1–2).
2. A renamed/moved part is open by deep link: identity, breadcrumb and associations remain correct after reload (Task 2).
3. An archived/Frozen plan is associated or its conversation is deleted: the exact old version remains readable and its bytes do not change (Task 3).
4. A project is removed between read and write: new reads/writes refuse it and no journal/association is partially written (Tasks 1–3).
5. Large/deep structures, Arabic text and reconnects: navigation remains bounded, text survives and authoritative data refreshes without dropping an unsaved editor (Tasks 2–4).

## Inspected entry points

Shadows MCP/LSP query tools were not connected to this chat. Read the code map and service contracts first, then the named files directly; no semantic caller analysis is claimed.

- `crates/shadows-core/src/app.rs`: `AppCore` composition/accessors.
- `crates/shadows-core/src/instructions/{mod.rs,store.rs}`: person commands, serialized write, durable replay and project events.
- `crates/shadows-core/src/plans/model.rs`: `PlanId`, `PlanListing`, `PlanVersions`.
- `crates/shadows-core/src/{projects,plans,events}/contract.yaml`: ownership and cross-service declarations.
- `crates/shadows-http/src/instructions.rs`: thin HTTP handlers and OpenAPI annotations.
- `crates/shadows-http/src/failure.rs`: revision conflict currently says “the plan changed”; generalize the entity wording when adding Design conflicts.
- `web/src/router.tsx`: project root currently redirects to a new conversation; keep that route and add explicit workspace URLs.
- `web/src/stream/use-project-events.ts`: one project stream already mounted; extend it, do not add another EventSource.

## Interfaces and data decisions

These are proposed interfaces, not signatures already present in code.

### Service surface

Define public domain types under `design/model.rs`, re-export from `shadows-core`:

```rust
DesignRevision = i64 // nonnegative; use a type alias
PartId, OutcomeId   // existing newtype_id! pattern
VisionContent { purpose: String, users: String, goals: String,
                boundaries: String, technical_direction: String }
PartContent { title: String, responsibility: String, design: String,
              kind: Option<String> }
OutcomeContent { title: String, intended_result: String,
                 acceptance: Vec<String> }
Part { id: PartId, revision: i64, parent: Option<PartId>,
       ordinal: i64, content: PartContent }
Outcome { id: OutcomeId, revision: i64, parent: Option<OutcomeId>,
          ordinal: i64, content: OutcomeContent }
DesignAnchor = Part(PartId) | Outcome(OutcomeId)
DesignChange { revision: DesignRevision }
```

All responses include the project workspace revision used by the next edit.
Use one workspace revision as the batch precondition, deliberately conservative
for this first editor: any intervening Design edit refuses a stale command.
Every affected vision/part/outcome also increments its own revision, including
reparent/order and association changes. Do not use plan revisions for these writes.

```rust
impl Design {
  pub async fn vision(&self, project: &ProjectId) -> Result<VisionView, CoreError>;
  pub async fn part(&self, project: &ProjectId, id: &PartId) -> Result<PartView, CoreError>;
  pub async fn parts(&self, project: &ProjectId, parent: Option<&PartId>,
      after: Option<&PartId>) -> Result<PartPage, CoreError>;
  pub async fn outcome(&self, project: &ProjectId, id: &OutcomeId) -> Result<OutcomeView, CoreError>;
  pub async fn outcomes(&self, project: &ProjectId, parent: Option<&OutcomeId>,
      after: Option<&OutcomeId>) -> Result<OutcomePage, CoreError>;
  pub async fn edit(&self, command_id: String, project: &ProjectId,
      expected_revision: i64, ops: Vec<DesignOp>) -> Result<DesignChange, CoreError>;
}
```

`VisionView { revision, content: VisionContent }` starts empty at revision 0.
`PartView { revision, part, ancestors: Vec<Part>, plans: Vec<PlanId> }` and
`OutcomeView { revision, outcome, ancestors: Vec<Outcome>, parts: Vec<PartId>, plans: Vec<PlanId> }`
return root-first ancestor paths. Pages contain `{ revision, items, next }`,
with `next` the final returned ID when another page exists. Use 50 immediate
children per page, ordered by `(ordinal, id)`; validate cursors in that parent.
On a changed revision, the client discards accumulated pages and reloads.

`DesignOp` is a tagged enum with explicit fields:

- `VisionPut { content: VisionContent }`.
- `PartCreate { id: PartId, parent: Option<PartId>, before: Option<PartId>, content: PartContent }`.
- `PartPut { id: PartId, content: PartContent }`.
- `PartMove { id: PartId, parent: Option<PartId>, before: Option<PartId> }`.
- `OutcomeCreate`, `OutcomePut`, `OutcomeMove`: equivalent fields with Outcome IDs/content.
- `PlanLinkPut { anchor: DesignAnchor, plan: PlanId }`, `PlanLinkRemove { anchor, plan }`.
- `OutcomePartPut { outcome: OutcomeId, part: PartId }`, `OutcomePartRemove { outcome, part }`.

IDs are client-generated UUIDs, validated once by the service. `before: None`
appends; a named sibling must belong to the destination parent and cannot be
the moved item. Compute affected sibling ordinals inside the serialized write.
Trim outer whitespace from titles; reject empty titles. Keep design/vision text
as written, including Arabic. Optional kind is descriptive, never a hierarchy rule.
Association arrays are unique and ordered by stable IDs. No automatic completion field.

### Persistence and replay

- Allocate `0013_design_vision.sql`, `0014_design_parts.sql`, `0015_design_outcomes.sql` to Tasks 1–3. §16's unshipped 1b becomes **0016**; Stage 2 agreements take the next available migration in their own plan. No placeholder 0016 is created here.
- Task 1 adds `design_workspace` (project, workspace revision, vision content/revision) and `design_command_result` (stable result ID, project, immutable result JSON). Lazy initialize the workspace on its first write; reads before it exists return the empty view only for a live project.
- Use `user_command`, kind `DesignEdit`, schema version 1, fingerprint over project, expected revision and normalized ordered ops. Scope is Project. Command replay returns its immutable saved `DesignChange`, even after later edits, without another event.
- Use existing `Storage::write_txn`, `classify`, `record_command`, `append_event`. Replay classification precedes live-state validation; a new command checks live project, expected revision and all references in the same transaction. Any invalid op rolls everything back.
- Task 2 adds `design_part` and `design_part_plan`; Task 3 adds `design_outcome`, `design_outcome_part` and `design_outcome_plan`. Every reference is project-qualified; add a supporting unique `(project_id,id)` key on the existing plan identity table if absent.
- Check both hierarchies for cycles after each relevant move inside the serialized write. Inspect ancestors iteratively, not recursion tied to UI depth. No title-based keys or physical SQL row order.
- Associations belong to Design, including references to archived plans. They do not edit plan content. Declare any cross-service transactional plan lookup in both contracts; no DB access in adapters.
- Emit `ProjectDesignChanged` with `{ project_id, revision, changed_parts, changed_outcomes, vision_changed }`, IDs only. Journal delivery stays owned by Events. Never put full design text in invalidation events.

### HTTP and UI

One handler calls one Design method. Routes under `/api/projects/{id}/design`:
`GET /vision`, `GET /parts?parent=&after=`, `GET /parts/{part}`,
`GET /outcomes?parent=&after=`, `GET /outcomes/{outcome}`, and
`POST /edits` with `{ command_id, expected_revision, ops }`.
GET errors for unknown/removed projects do not look like empty workspaces.
Reuse existing 409 `REVISION_CONFLICT` and `COMMAND_CONFLICT`; invalid references
use the existing invalid-command taxonomy with actionable item details.

Browser entry: `/projects/$projectId/workspace`, with `view=vision|map|roadmap|plans`
and optional `part`/`outcome` IDs in validated search parameters. Defaults to Vision.
Preserve `/new`, thread URLs and version URLs. Sidebar gains a Workspace link.
Stage 1 shows working sections only; Contracts is added with Stage 2.

## Task 1: Vision in a persistent project workspace

**Create:** `crates/shadows-core/src/design/{mod.rs,model.rs,store/mod.rs,store/vision.rs,contract.yaml}`;
`crates/shadows-core/migrations/0013_design_vision.sql`;
`crates/shadows-http/src/design.rs`; `crates/shadows-core/tests/design_vision.rs`;
`crates/shadows/tests/design_routes.rs`;
`web/src/app/design/{workspace-page.tsx,vision-editor.tsx,vision-editor.test.tsx}`;
`web/src/api/design.ts`.

**Modify:** core `app.rs`, `lib.rs`, error wording in `db/mod.rs`/HTTP `failure.rs`;
HTTP `lib.rs`, `openapi.rs`, `sse.rs`; `events/contract.yaml`;
`web/src/{router.tsx,api/queries.ts,stream/frames.ts,stream/use-project-events.ts}`;
`web/src/app/sidebar/sidebar.tsx`; `api/openapi.json`, `web/src/api/schema.d.ts`;
`docs/codebase/README.md`, §14's service composition.

**Consumes:** existing transaction/replay/event primitives and `ProjectId`.
**Produces:** `Design::vision`, `Design::edit` for `VisionPut`, `core.design()`,
workspace route and one project-stream invalidation path. Common model/interface
decisions above apply; add only types used by this task now.

- [ ] Write `vision_edit_is_atomic_replayable_and_revision_checked`: assert revision 0 → 1; replay returns 1 after a later edit reaches 2; changed fingerprint conflicts; two writes expecting 0 produce one success; removed project accepts no new write or event.
- [ ] Write `vision_editor_keeps_unsaved_text_on_conflict`: two simulated tabs, Arabic fields, one stale save, retained local text with explicit Reload; project switch cannot display the prior project's vision.
- [ ] Run `cargo test -p shadows-core --test design_vision` and `npm --prefix web test -- src/app/design/vision-editor.test.tsx`; observe failure for missing feature, not unrelated setup.
- [ ] Implement the first vertical slice. Persist replay result, mutation and event atomically; do not spread SQL into `db/mod.rs`. On SSE/reconnect invalidate authoritative queries while preserving dirty form state and displaying that newer data exists.
- [ ] Add `design_routes` assertions for JSON round-trip, 409 conflict and old conversation URLs. Generate API/types using the commands below; run both focused tests and `cargo test -p shadows --test design_routes`.
- [ ] Amend service contract/code-map/§14 composition for Design as the tenth service, run the commit gate, review the scoped diff, then commit when authorized.

## Task 2: Nested parts and stable plan associations

**Create:** `design/parts.rs`, `design/store/parts.rs` under core;
`migrations/0014_design_parts.sql`; core test `tests/design_parts.rs`;
Web `app/design/{parts-view.tsx,part-editor.tsx,parts-view.test.tsx}`.
**Modify:** Task 1's domain model/contract, HTTP routes/OpenAPI/generated types,
API query functions, workspace page and code map; `plans/contract.yaml` and
`plans/store/plan.rs` only for a needed declared transactional identity lookup.

**Consumes:** Task 1's workspace revision, edit/replay and event mechanism.
**Produces:** `part`, `parts`, `Part*` ops and `PlanLink*` for part anchors.

- [ ] Write `part_moves_keep_identity_and_reject_cycles`: A/B/C chain; moving A under C and moving to a foreign project both refuse without revision/event changes. Rename/reparent preserves ID and plan links; concurrent stale moves refuse.
- [ ] Write `part_order_is_stable_across_restart`: insert/move before sibling, reopen DB and assert explicit order; invalid sibling cursor/reference refuses. An archived/Frozen same-project plan can be associated without changing its stored versions; foreign plan cannot.
- [ ] Write `parts_view_loads_only_open_branches`: 51 siblings paginate without omission; deep breadcrumbs load without rendering descendants; an open deep link survives rename/move/reload. Arabic names remain intact.
- [ ] Run `cargo test -p shadows-core --test design_parts` and `npm --prefix web test -- src/app/design/parts-view.test.tsx` to observe focused failures.
- [ ] Implement SQL/model validation and thin routes, then the tree/detail UI with Create/Edit/Move/Move-before controls. Do not introduce drag-and-drop dependencies. Use iterative ancestry and bounded child pages.
- [ ] Regenerate API/types, run focused tests and `design_routes`; update ownership/contracts, run the gate, review and commit when authorized.

## Task 3: Roadmap outcomes across parts and plans

**Create:** core `design/outcomes.rs`, `design/store/outcomes.rs`,
`migrations/0015_design_outcomes.sql`, `tests/design_outcomes.rs`;
Web `app/design/{roadmap-view.tsx,outcome-editor.tsx,roadmap-view.test.tsx}`.
**Modify:** the shared Design surfaces, routes/generated types and workspace
page; reuse existing plan queries for the Plans section, keeping unassigned
plans visible with the existing archived-list option.

**Consumes:** Design edits/revisions, part identity and existing Plans reads.
**Produces:** `outcome`, `outcomes`, `Outcome*`, `OutcomePart*`, and outcome
`PlanLink*` operations. The same transactional rules apply to both anchor kinds.

- [ ] Write `outcome_spans_parts_without_copying_them`: one login outcome references two parts and two plans; changing a part is visible through its identity; invalid foreign reference rolls back an entire multi-op edit.
- [ ] Write `outcome_hierarchy_is_independent_of_parts`: each tree rejects cycles; moving one hierarchy leaves the other unchanged. Reordering survives reopen and stale writes refuse.
- [ ] Write `plan_lifecycle_never_completes_outcomes`: approve/archive a linked plan and delete its writer conversation using existing APIs; outcome remains planned, links persist and old Frozen content is byte-for-byte unchanged.
- [ ] Write `roadmap_navigation_keeps_old_plans_accessible`: linked and unassigned plans remain reachable, old version URLs still work, no Done badge appears because of archive/approval.
- [ ] Run `cargo test -p shadows-core --test design_outcomes` and `npm --prefix web test -- src/app/design/roadmap-view.test.tsx` to observe failures; implement this vertical slice, regenerate API/types and rerun for passes.
- [ ] Update contracts/code-map, run the gate, review and commit when authorized.

## Task 4: Integrated migration and Windows browser acceptance

**Create:** `crates/shadows-core/tests/design_migration.rs`,
`web/src/stream/use-project-events.test.tsx` if still absent at execution time;
dated `docs/evidence/YYYY-MM-DD-planning-workspace-stage-1.md` after the trial.
**Modify:** `crates/shadows/tests/design_routes.rs` for integrated regressions;
update `docs/status.md` only with observed results.

**Consumes:** the three complete slices. **Produces:** demonstrated Stage 1,
not a new service or abstraction.

- [ ] Start with a disposable pre-0013 DB containing old plans/Frozen versions/events. Migrate through 0015; assert identity/content/event preservation and `PRAGMA foreign_key_check` empty. Reopen and assert persisted order/links/revisions.
- [ ] Repeat on a consistent copy of the dev DB, obtained while the source daemon is stopped or through SQLite's backup mechanism. Never copy only the main file from a live WAL database; never run the new binary against the original DB.
- [ ] Test project SSE replay, duplicate event handling, disconnect/reconnect and cross-tab updates for vision/parts/outcomes. A stale editor keeps local input. Existing plan invalidation remains covered.
- [ ] Run the complete Rust/Web gate below. Review the whole Stage 1 diff once; verify fixes rather than repeating the same full review per dispatch.
- [ ] Start `shadows serve` against a disposable DB and the Web dev server. Create vision, two parts plus a child, and an outcome referencing both parts/plans. Rename/reparent/reorder, associate an archived plan, open an old Frozen version, then restart daemon/reload. Verify persistence and read logs.
- [ ] Record actual commands, platform, result and remaining failures in the evidence file. Do not mark accepted based solely on automated tests. Commit only after review/gate authorization; no automatic push/merge from this plan.

## Generation and commit gate

After a route change, from repository root in PowerShell:

```powershell
$env:UPDATE_OPENAPI = '1'
cargo test -p shadows --test openapi
Remove-Item Env:UPDATE_OPENAPI
npm --prefix web run gen:api
```

Review the intentional API/type diff. After those changes are committed,
regeneration must produce no further diff. Before every implementation commit:

```powershell
Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue
cargo fmt --all --check
cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo tree -e features,no-dev --workspace | Select-String 'test-support'
git diff --exit-code api/
npm --prefix web run typecheck
npm --prefix web run lint
npm --prefix web test
npm --prefix web run build
git diff --check
```

Each command must succeed; feature search prints nothing. Route commits stage
their reviewed generated API changes before the `git diff --exit-code api/`
check; inspect the staged API diff explicitly. No unrelated API drift is allowed.
Also run `cargo test -p shadows --test codemap` and
`cargo test -p shadows-core --test contracts` when diagnosing ownership failures.

## Handoff

Review this plan before product edits. Recommended execution: one implementer
and one independent reviewer per vertical slice, sequentially, because shared
revision/replay interfaces feed every later slice. The user still selects the
execution method. The plan does not spawn agents or authorize implementation.
