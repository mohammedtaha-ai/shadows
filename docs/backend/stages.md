# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

The architecture restructuring ([plan](plans/2026-10-21-architecture.md)) shipped in
PR #41 (`c7e7bac4`): A-0 to A-21, A-23, B-10 and B-11 are in this file at `c7e7bac4`.
The extraction tasks A-8c, A-8d, A-19b and A-19c finished in `d7a722cd`.
The rules are CLAUDE.md "Code structure"; its strict rule binds every task: start at
`AppCore`, work only in the owning service. Live keys: only `OPENROUTER` and
`API_KEY_MINIMAX`, read with a loop, never `. ./.env`, never printed or committed.

Before dispatch the controller expands each task's named scope into exact caller,
test, contract and map paths and records the affected test commands. A blocked
task is not a builder assignment. Per-task checks are focused; the full gate and
whole-branch review run once before each delivery PR.

A-6 is settled with no code change (Mohammed confirmed the approval-authority spec's §24.2
on 2026-10-27); its entry is in this file at `4d5c022d`.

A-7b (the breakdown route goes) and B-12 (test fixtures close their database before their
folder goes) shipped in PR #45 (`5a0af4ff`); their entries are in this file at `29ddfd88`.

A-22 (product workspaces) is deferred on 2026-10-28: its trigger is in
[docs/core/future.md](../core/future.md) "Housekeeping"; its entry is in this file at `3dc2437f`.

B-13 (the person record takes a partial update, `PATCH /api/me`) shipped in PR #46
(`e95b79c3`); its entry is in this file at `e95b79c3`.

A-8b1 and A-8b2 (the Hermes product daemon goes; the AYLUL project check and the master key
move into Core) and B-14 to B-18 (W-11's Core side) shipped in PR #47 (`f6551819`); their
entries are in this file at `4f23c717`.

Tool activity ([spec](specs/2026-10-31-tool-activity-design.md)) and B-19 to B-23 (ACP tool
facts, the `ToolCall` card, the task list, the `tool` and `todos` frames, the `TD` ids) shipped
in PR #48 (`ce31036d`); their entries are in this file at `ce31036d`.

AYLUL agents and panel, Core side: B-24 to B-34 shipped in PR #49 (`b40e077b`); their entries
are in this file at `b40e077b`. The design is `docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md`.
The plan was `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` at `b40e077b`.

Agent identity and background tasks ([tool activity §6](specs/2026-10-31-tool-activity-design.md#6-background-tasks)):
B-35 to B-37 shipped in PR #51 (`2790473b`); their entries are in this file at `16da6b8c`.

The real-app acceptance of the design's §10 is done (Mohammed, 2026-11-03, in the desktop app): an
HTTP MCP tool with a header token (Context7) answered in a conversation, and its token was found in
no database, WAL, log, agent home or session file.

Per-tool MCP permissions ([AYLUL agents and panel §7.2.1](../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md)):
B-38 and B-39 shipped in PR #52 (`6b2c66ed`), with F-80; their entries are in this file at
`9c7c7e5a`. The plan was `docs/backend/plans/2026-11-03-aylul-tool-permissions.md` and the request
`docs/shared/requests/2026-11-03-mcp-tool-permissions.md`, both at `9c7c7e5a`.
Its review findings (whole-token redaction, the listing's environment and folder, deny over
allow on a name clash, edits queued during a save) were fixed in PR #53 (`349606bf`).

Agent groups, messaging slice 1 ([spec](specs/2026-11-03-agent-messaging-design.md)): B-40 to B-46 shipped in PR #54
(`9e5b9ab2`), with F-81 to F-85, U-9 and U-10; their entries, and the commits outside their
`Touch:` lists, are in this file at `9e5b9ab2`. The plan was
`docs/backend/plans/2026-11-03-agent-groups.md` at `9e5b9ab2`.
Known and left: a persistent storage error while starting a delivery retries forever and
stalls dispatch for every group (`crates/core-messages/src/deliveries.rs`).

Planner context, slice 1 ([spec](specs/2026-11-04-planner-context-design.md)): B-47 to B-54 shipped in PR #55
(`fc78cb7`), with the three Planner defects in `docs/evidence/2026-11-04-planner-session-defects.md`. Their entries,
and the commits outside their `Touch:` lists, are in this file at `fc78cb7`. The plan was
`docs/backend/plans/2026-11-04-planner-context-slice-1.md` at `fc78cb7`. Its shared request (`docs/shared/requests/2026-11-04-planning-context-routes.md`, at `1b413fc4`) was built by the inspection view in PR #57. Local acceptance with real provider and browser is in the
[dated results](../evidence/2026-11-04-planner-context-probe.md#4-slice-1-implementation-acceptance-2026-11-04).
Planner context, slice 2 (the re-send after a compaction, spec §4.3): B-55 to B-58 shipped in PR #56
(`024432fc`); their entries, and the commits outside their `Touch:` lists, are in this file at
`024432fc`. The plan was `docs/backend/plans/2026-11-04-planner-context-slice-2.md` at `024432fc`. Its live
acceptance passed on 2026-11-04 ([evidence §5](../evidence/2026-11-04-planner-context-probe.md#5-slice-2-live-acceptance-2026-11-04)).
Its shared request (`docs/shared/requests/2026-11-04-planning-context-resend.md`, at `1b413fc4`) was built in PR #57.
Planner context, slice 3 (the inspection view): B-59 and F-86 to F-89 shipped in PR #57
(`1b413fc4`); their entries are in this file at `1b413fc4`. The plan was `docs/frontend/plans/2026-11-05-planner-context-view.md` at `1b413fc4`.
Its shared request (`docs/shared/requests/2026-11-05-turn-context-state.md`, at `1b413fc4`) was built by that PR.
Not built: `Plans::owner_thread` (spec §6, slice 4), which waits for messaging slice 3.

Structure and depth, slice 1 (one folder per part, no contract files): the design and its plan
were merged in PR #58 (`96f9ff3d`). Slice 1 shipped in PR #59 (`26bf739e`), with B-60 to B-62;
their entries, and the commits outside their `Touch:` lists (`d148c111`, `ec20c911`, `6874e9b2`,
`80c1b934`), are in this file at `26bf739e`. The plan is `docs/backend/plans/2026-11-05-structure-and-depth.md`;
its live acceptance is not recorded.

Structure and depth, slice 2 (standards v2 and layers on the map): B-63 to B-70 shipped in PR #60
(`b64242a7`), with F-90 and F-91. Their entries, and the commits outside their `Touch:` lists
(`ed4005a7`, `d0fdc70a`, `0774ac2a`), are in this file at `b64242a7`. B-68 to B-70 are test tooling:
a contract-to-test name check (`cargo test -p shadows --test contract_tests`), a weekly coverage
report and an on-demand mutation run. The plan is `docs/backend/plans/2026-11-05-structure-and-depth.md`;
its live acceptance is not recorded.

## Tasks

Structure and planning depth: Slice 3 implementation and automated checks are complete
locally (B-71 to B-75, F-92, through `3deb1333`). Manual acceptance belongs to Mohammed
on the existing project; it is pending. Slices 4 and 5 are implemented as
B-76 to B-81 and F-93/F-94 on `feature/structure-elements`, committed and pushed
per task. Focused backend/web checks and isolated daemon/browser phase checks
passed; real-provider and daily-project acceptance are not recorded. No merge
is recorded. Elements and trace are dropped.
The [design](specs/2026-11-05-structure-and-depth-design.md) and
[delivery plan](plans/2026-11-05-structure-and-depth.md#dispatch-and-progress) define the
readiness checklist. Existing projects are test data: no compatibility layer. Coverage and code
observation must not be recorded as completed execution.

B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at `f1459dcc`.

### B-71 · Base v3 holds principles, not a layout
- Status: done
- Base: `31b8ebdc`
- Plan: `docs/backend/plans/2026-11-05-structure-and-depth.md#task-31-base-v3-and-parts-without-starter-files`
- Owner: `core-standards`, `crates/core-standards/contract.yaml`
- Read: AppCore's owner table; the structure spec §5.2; core-standards contract.
- Touch: `crates/core-standards/src/standards.yaml`, `src/model.rs`, `src/base.rs`, `tests/base.rs`, `contract.yaml`; `api/openapi.json`; `web/src/api/schema.d.ts`; this stages file; the plan; shared request `2026-11-06-principles-not-layout.md`.
- Needs: accepted shared request `2026-11-06-principles-not-layout.md`.
- Input/output: Base v2 becomes v3; LayerSpec loses elements; S6/S9 change and S10 is added. Starter-file removal moves to B-73 with the writer that consumes it.
- Failure cases: serialized element kinds remain; runtime rule wrongly routes internal dependencies through AppCore.
- Done when: `cargo test -p core-standards`; generated OpenAPI and web schema; `cargo fmt --all --check`.

### B-72 · Map rules check principles, not places
- Status: done
- Base: `288670b2`
- Plan: `docs/backend/plans/2026-11-05-structure-and-depth.md#task-32-map-rules-on-principle-only`
- Owner: `core-design`, `crates/core-design/contract.yaml`
- Read: AppCore owner table; structure spec §5.3; core-design contract.
- Touch: `crates/core-design/src/map_rules.rs`, `tests/map_rules.rs`, `contract.yaml`; `crates/shadows-core/tests/design_parts/map.rs`; this stages file.
- Needs: B-71 base v3; accepted principles-not-layout request.
- Input/output: folder ownership stays; layer places no longer require containment or disjoint pattern expansions. Missing, unknown, duplicate or malformed layers refuse.
- Done when: `cargo test -p core-design`; `cargo test -p shadows-core --test design_parts`; `cargo test -p shadows --test design_routes`; fmt check.

### B-73 · Remove the structure writer and starter files
- Status: done
- Base: `bb0f541a`
- Plan: `docs/backend/plans/2026-11-05-structure-and-depth.md#task-33-remove-create-structure-and-the-structure-step`
- Owner: Design; core-db owns migrations; HTTP/MCP translate only.
- Read: AppCore owner table; Design, foundation and adapter contracts; structure spec §5.4; guided-planning §23.
- Needs: B-72 and accepted principles-not-layout request.
- Input/output: no PartContent.files, InitialFile, Structure stage, count, command or route. Migrations 0048/0049 remove obsolete data; map approval opens Roadmap. Existing disk files remain.
- Failure cases: unreadable old parts; reordered map snapshots; missing mandatory nested kind; roadmap write before map approval; stale writer route or UI type.
- Done when: core-design; shadows-core design_parts/design_migration/design_vision; shadows openapi/design_routes/workspace_mcp/codemap/contract_tests/architecture; affected all-target clippy; generated API; fmt check.
- Touch: `api/openapi.json`, `crates/core-db/contract.yaml`, `crates/core-design/contract.yaml`, `crates/core-design/src/lib.rs`, `crates/core-design/src/map_rules.rs`, `crates/core-design/src/model.rs`, `crates/core-design/src/stage.rs`, `crates/core-design/src/store/mod.rs`, `crates/core-design/src/store/roadmap_approval.rs`, `crates/core-design/src/store/roadmap_proposals.rs`, `crates/core-design/src/store/roadmap_shape.rs`, `crates/core-design/src/store/stage.rs`, `crates/core-design/src/store/structure.rs`, `crates/core-design/src/structure.rs`, `crates/core-design/src/structure_create.rs`, `crates/core-design/src/structure_write.rs`, `crates/core-design/tests/map_rules.rs`, `crates/core-design/tests/stage.rs`, `crates/core-design/tests/structure.rs`, `crates/shadows-core/src/lib.rs`, `crates/shadows-core/tests/agreement_bindings.rs`, `crates/shadows-core/tests/agreement_review.rs`, `crates/shadows-core/tests/design_migration.rs`, `crates/shadows-core/tests/design_outcomes.rs`, `crates/shadows-core/tests/design_parts.rs`, `crates/shadows-core/tests/design_parts/map.rs`, `crates/shadows-core/tests/design_parts/map_proposals.rs`, `crates/shadows-core/tests/design_parts/roadmap.rs`, `crates/shadows-core/tests/design_parts/stage.rs`, `crates/shadows-core/tests/design_parts/structure.rs`, `crates/shadows-core/tests/plan_migration.rs`, `crates/shadows-http/src/contract.yaml`, `crates/shadows-http/src/design.rs`, `crates/shadows-http/src/lib.rs`, `crates/shadows-mcp/src/contract.yaml`, `crates/shadows-mcp/src/workspace_tools.rs`, `crates/shadows/tests/agreement_journey.rs`, `crates/shadows/tests/design_routes.rs`, `crates/shadows/tests/openapi.rs`, `crates/shadows/tests/workspace_mcp.rs`, `docs/codebase/README.md`, `docs/core/specs/2026-10-08-guided-planning-design.md`, `web/src/api/schema.d.ts`, `crates/core-db/migrations/0048_part_files_removed.sql`, `crates/core-db/migrations/0049_structure_write_dropped.sql`

- Additional fixture scope: `crates/shadows/tests/workspace_mcp/roadmap.rs`; remove writer calls and seed complete vision for the roadmap-stage fixture.

### B-74 · The Planner follows the stack
- Status: done
- Base: `2e2b8e63`
- Plan: `docs/backend/plans/2026-11-05-structure-and-depth.md#task-34-the-planner-follows-the-stack`
- Owner: Harness, `crates/core-harness/contract.yaml`.
- Read: AppCore owner table; structure spec §7; Harness contract and its standards renderer.
- Touch: `crates/core-harness/src/prompt.txt`, `contract.yaml`; `crates/shadows/tests/planner_mcp.rs`, `planner_mcp/standards.rs`; this stages file.
- Needs: B-73's final stage vocabulary and base v3.
- Input/output: Planner instructions describe stack-native layouts, nested kinds, no generated files and Agent Mode when explicitly requested. Standards rendering already prints roles and S10 without element kinds.
- Failure cases: opening prompt still imposes a layout or asks for Structure; recorded standards version diverges from v3.
- Done when: `cargo test -p shadows --test planner_mcp`; `cargo test -p core-harness`; fmt check.

### B-75 · Review fixes for layer-place policy
- Status: done
- Base: `016a22f9`
- Owner: `core-design`, `crates/core-design/contract.yaml`.
- Read: AppCore owner table; structure spec §5.3; independent Slice 3 review.
- Touch: `crates/core-design/src/map_rules.rs`, `tests/map_rules.rs`, `contract.yaml`; this stages file.
- Input/output: compare canonical case-folded places, including direct policy calls; missing-layer guidance requests the stack layout rather than part containment.
- Done when: both new regressions observed RED to GREEN; core-design, design_parts, design_routes and contract_tests; affected clippy; fmt check.

### B-76 · Read accepted layout through Design
- Status: done
- Base: `ae70fa04`, `feature/structure-elements`; controller serializes commits.
- Plan: `docs/backend/plans/2026-11-05-structure-and-depth.md#task-41-the-layout-read-core-design`
- Owner: Design; `crates/core-design/contract.yaml`.
- Read: AppCore owner table; structure spec §5.3/§5.5/§8; Design and Standards contracts; code-map rows for map_versions, parts and map_rules. No LSP is available; direct reads follow those modules.
- Touch: `crates/core-design/src/map_rules.rs`, `crates/core-design/src/layout.rs`, `crates/core-design/src/store/layout.rs`, `crates/core-design/src/store/mod.rs`, `crates/core-design/src/lib.rs`, `crates/core-design/contract.yaml`, `crates/shadows-core/src/lib.rs`, `crates/shadows-core/tests/design_parts.rs`, `crates/shadows-core/tests/design_parts/layout.rs`, `docs/codebase/README.md`, this stages file, plan Task 4.1/progress.
- Needs: Slice 3 implementation; no public HTTP shape change or shared request.
- Input/output: `Design::layout`/`layout_in` return accepted map order, revision, effective standards versions, joined layer roles/may_use and applicable rules. Pure `ProjectLayout::owners` owns folder/place matching; store remains private. `i64` standards numbers; no private additions id exposed.
- Failure cases: proposal leakage, mixed snapshots, removed project, sibling path prefix, repeated/embedded marker mismatch, duplicated/omitted ownership claims, grouping rule copied outside MapRules.
- Done when: tests named in Task 4.1 (including snapshot, invalid paths and liveness) pass via `cargo test -p shadows-core --test design_parts` and `cargo test -p core-design`; `cargo test -p shadows --test architecture --test codemap --test contract_tests`; fmt. Parent runs required affected clippy/checks and owns commit/push. No live acceptance claimed.


- Verified: design_parts 41 passed; core-design suite, architecture, codemap, contract_tests, affected all-target clippy and fmt passed.
### B-77 · Read approved task context
- Status: done
- Base: previous task commit on `feature/structure-elements`.
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 4.2.
- Owner: Plans; its current contract and AppCore owner row.
- Read: owner spec §5.5/§6/§8; exact Task 4.2 interfaces, files and tests; owner contracts and code map.
- Touch: the exact Files list in Task 4.2, this stages file and that plan task. Compiler-reported constructor fixtures are recorded before edits.
- Needs: B-76.
- Done when: cargo test -p shadows-core --test task_context; cargo test -p core-plans; affected clippy; fmt; architecture, codemap and contracts. No live acceptance claimed by unit tests.

- Create: `crates/core-plans/src/task_context.rs`
- Modify: `crates/core-plans/src/lib.rs`, `crates/core-plans/src/store/mod.rs`,
  `crates/core-plans/src/store/design_parts.rs`, `crates/core-plans/contract.yaml`,
  `crates/shadows-core/src/lib.rs`, `docs/codebase/README.md`.
- Create: `crates/core-plans/src/store/task_context.rs`,
  `crates/shadows-core/tests/task_context.rs` (standalone integration target).


- Verified: task_context integration cases and core-plans passed; architecture, codemap, contract_tests and affected all-target clippy passed. Scope helper moved with this task.
### B-78 · Expose task_context over MCP
- Status: done
- Base: previous task commit on `feature/structure-elements`.
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 4.3.
- Owner: Plans; Grants owns tool policy; Harness owns instructions; its current contract and AppCore owner row.
- Read: owner spec §5.5/§6/§8; exact Task 4.3 interfaces, files and tests; owner contracts and code map.
- Touch: the exact Files list in Task 4.3, this stages file and that plan task. Compiler-reported constructor fixtures are recorded before edits.
- Needs: B-77.
- Done when: cargo test -p shadows --test planner_mcp --test mcp_tools --test mcp_server --test contract_tests --test codemap; affected clippy; fmt; architecture, codemap and contracts. No live acceptance claimed by unit tests.

- Modify: `crates/shadows-mcp/src/plan_tools.rs`,
  `crates/shadows-mcp/src/contract.yaml`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt` (one line: the building
  agent reads `task_context` first)
- Modify: `crates/core-harness/contract.yaml`.
- Test: `crates/shadows/tests/planner_mcp.rs`,
  `crates/shadows/tests/mcp_tools.rs`, `crates/shadows/tests/mcp_server.rs`.


- Verified: planner_mcp 24, mcp_tools 16, mcp_server 6, codemap and contract_tests passed; affected all-target clippy and fmt passed.
### B-79 · Persist phase values in task contracts
- Status: done
- Base: previous task commit on `feature/structure-elements`.
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 5.1.
- Owner: Plans; its current contract and AppCore owner row.
- Read: owner spec §5.5/§6/§8; exact Task 5.1 interfaces, files and tests; owner contracts and code map.
- Touch: the exact Files list in Task 5.1, this stages file and that plan task. Compiler-reported constructor fixtures are recorded before edits.
- Needs: B-78; accepted task-phases request.
- Done when: cargo test -p core-plans; cargo test -p shadows --test openapi; affected clippy; fmt; architecture, codemap and contracts. No live acceptance claimed by unit tests.

- Modify: `crates/core-plans/src/model.rs`, `crates/core-plans/src/lib.rs`,
  `crates/core-plans/src/rules.rs`, `crates/core-plans/contract.yaml`,
  `crates/shadows-core/src/lib.rs`, `api/openapi.json`,
  `web/src/api/schema.d.ts`, `docs/codebase/README.md`.
- Create: `crates/core-plans/src/phases.rs` for pure public phase values.
- Test: `crates/core-plans/tests/workflow_rules.rs`.
- Constructor fixture inventory must be completed by the controller before
  B-79 dispatch: adding a Rust struct field breaks every `TaskContent` literal.
  The exact compiler-identified fixture files are appended to B-79 Touch; no
  blanket permission to edit arbitrary tests or unrelated product modules.


- Constructor fixture Touch: `crates/core-plans/tests/workflow_rules.rs`, `crates/shadows-core/tests/design_outcomes.rs`, `crates/shadows-core/tests/design_parts.rs`, `crates/shadows-core/tests/design_parts/roadmap_gate.rs`, `crates/shadows-core/tests/fixtures/cross_plan.rs`, `crates/shadows/tests/fixtures/plan.rs`.
- Verified: workspace all-target check; 21 core-plans rules tests; persisted phase reopen; OpenAPI 8, schema generation and web typecheck; architecture/codemap/contracts; affected clippy.
### B-80 · Edit phases with one version-aware dependency graph
- Status: done
- Base: previous task commit on `feature/structure-elements`.
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 5.2.
- Owner: Plans; its current contract and AppCore owner row.
- Read: owner spec §5.5/§6/§8; exact Task 5.2 interfaces, files and tests; owner contracts and code map.
- Touch: the exact Files list in Task 5.2, this stages file and that plan task. Compiler-reported constructor fixtures are recorded before edits.
- Needs: B-79.
- Verified: 24 pure workflow cases; 3 phase integration cases, 9 existing cross-plan cases; replay/fingerprint checks, OpenAPI, generated schema/typecheck, architecture, codemap, contracts and affected clippy passed. Added `src/phases.rs` and private `store/phase_refs.rs` to the exact owner scope. Disk-space failures were retried successfully after pruning obsolete local incremental variants; dependency artifacts were retained.
- Done when: cargo test -p core-plans; cargo test -p shadows-core --test phase_dependencies --test cross_plan_resolution --test cross_plan_links; affected clippy; fmt; architecture, codemap and contracts. No live acceptance claimed by unit tests.

- Modify: `crates/core-plans/src/dependency_graph.rs`, `src/ops.rs`,
  `crates/core-plans/src/rules.rs`, `crates/core-plans/src/store/graph.rs`,
  `crates/core-plans/src/store/edit.rs`, `crates/core-plans/src/store/read.rs`,
  `crates/core-plans/src/model.rs`, `crates/core-plans/src/lib.rs`,
  `crates/core-plans/src/store/mod.rs`, `crates/core-plans/contract.yaml`,
  `crates/shadows-core/src/lib.rs`, `api/openapi.json`,
  `web/src/api/schema.d.ts`, `docs/codebase/README.md`.
- Create: `crates/core-plans/src/store/phase_projection.rs` and
  `crates/shadows-core/tests/phase_dependencies.rs`.
- Test: `crates/core-plans/tests/workflow_rules.rs`,
  `crates/shadows-core/tests/cross_plan_resolution.rs`,
  `crates/shadows-core/tests/cross_plan_links.rs`.


### B-81 · Read selected phase context through MCP
- Status: done
- Base: previous task commit on `feature/structure-elements`.
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 5.3.
- Owner: Plans; adapters translate; its current contract and AppCore owner row.
- Read: owner spec §5.5/§6/§8; exact Task 5.3 interfaces, files and tests; owner contracts and code map.
- Touch: the exact Files list in Task 5.3, this stages file and that plan task. Compiler-reported constructor fixtures are recorded before edits.
- Needs: B-80.
- Verified: context integration 3, MCP tools 17, Planner 24, pure rules 24, OpenAPI 8, codemap/contracts and affected clippy passed. Selected phase dependencies reuse the exact resolver in one snapshot; whole-task acceptance stays present.
- Done when: cargo test -p shadows-core --test task_context; cargo test -p shadows --test planner_mcp --test mcp_tools --test openapi; affected clippy; fmt; architecture, codemap and contracts. No live acceptance claimed by unit tests.

- Modify: `crates/shadows-mcp/src/plan_tools.rs` (the `plan_edit`
  description lists `phases_set`), `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-harness/contract.yaml`, `crates/core-plans/contract.yaml`,
  `crates/core-plans/src/store/task_context.rs`,
  `crates/core-plans/src/lib.rs`, `crates/shadows-core/src/lib.rs`
- Modify: `crates/core-harness/src/prompt.txt` (phases: when to use them,
  and that the Planner never marks a phase complete)
- Modify: `crates/core-plans/src/task_context.rs`: an optional
  `phase: Option<&PhaseId>`
- Test: `crates/shadows/tests/planner_mcp.rs`,
  `crates/shadows/tests/mcp_tools.rs`, `crates/shadows-core/tests/task_context.rs`.
- B-79/B-80 own HTTP schema regeneration as their public shapes change.
  B-81 verifies those files remain current; MCP-only phase_id is not an HTTP
  route and does not alone warrant changing OpenAPI.
