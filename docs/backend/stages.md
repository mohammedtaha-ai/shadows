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

## Tasks

Structure and planning depth, slices 3 to 9: no slice is an executable `B-` assignment yet.
The [design](specs/2026-11-05-structure-and-depth-design.md) and
[delivery plan](plans/2026-11-05-structure-and-depth.md#dispatch-and-progress) define the
readiness checklist. Existing projects are test data: no compatibility layer. Coverage and code
observation must not be recorded as completed execution.

Structure and depth, slice 2 ([plan](plans/2026-11-05-structure-and-depth.md#slice-2-standards-v2-and-layers-on-the-map)),
on `feature/structure-layers`, base `26bf739e`. The stack profile is decided (spec §5.3, D1 to
D4). Each task's steps, interfaces and test names are in the plan. Run the tasks in order,
because each builds on the one before. Rust lines stay within 100 columns.

### B-63 · Standards v2 names each kind's layers
- Status: done
- Plan: Task 2.1
- Read: crates/core-standards/contract.yaml; crates/core-standards/src/standards.yaml, model.rs, base.rs;
  crates/core-harness/src/standards.rs
- Touch: crates/core-standards/src/standards.yaml, model.rs, base.rs, validation.rs, lib.rs,
  contract.yaml, tests/base.rs; crates/core-harness/src/standards.rs, contract.yaml;
  crates/core-design/src/store/map_proposal_shape.rs, contract.yaml; crates/shadows-core/src/lib.rs,
  tests/design_vision/standards.rs, tests/design_parts/map_proposals.rs
- Done when: `cargo test -p core-standards`, `cargo test -p shadows-core --test design_vision
  --test design_parts`, `cargo test -p core-harness` and `cargo test -p core-design` pass

### B-64 · A part has one folder and its layers; old parts are migrated
- Status: done
- Plan: Task 2.2 (its Files list names every fixture file)
- Read: crates/core-design/contract.yaml; crates/core-design/src/model.rs, map_rules.rs, structure.rs;
  crates/core-db/migrations/0014_design_parts.sql, 0023_project_map.sql
- Touch: crates/core-design/src/model.rs, map_rules.rs, structure.rs, structure_create.rs,
  structure_write.rs, store/structure.rs, store/roadmap_seed.rs, contract.yaml;
  crates/core-db/migrations/0047_part_folder.sql; crates/shadows-mcp/src/workspace_tools.rs;
  api/openapi.json; web/src/api/schema.d.ts; docs/shared/requests/2026-11-05-part-folder-and-layers.md;
  the fixture files the plan lists (fixture data only)
- Done when: the commands in Task 2.2 Step 5 pass

### B-65 · The map refuses a missing, unknown or misplaced layer
- Status: todo
- Plan: Task 2.3
- Read: crates/core-design/contract.yaml; crates/core-design/src/map_rules.rs
- Touch: crates/core-design/src/map_rules.rs, contract.yaml, tests/map_rules.rs;
  crates/shadows/tests/design_routes.rs
- Done when: `cargo test -p core-design --test map_rules` and `cargo test -p shadows --test design_routes` pass

### B-66 · Create structure makes each layer's folder
- Status: todo
- Plan: Task 2.4
- Read: crates/core-design/src/structure.rs, map_rules.rs, store/structure.rs, store/map_versions.rs
- Touch: crates/core-design/src/structure.rs, store/structure.rs, contract.yaml, tests/structure.rs;
  crates/shadows-core/tests/design_parts/structure.rs
- Done when: `cargo test -p core-design --test structure` and `cargo test -p shadows-core --test design_parts` pass

### B-67 · The Planner is told each kind's layers
- Status: todo
- Plan: Task 2.5
- Read: crates/core-harness/contract.yaml; crates/core-harness/src/standards.rs, prompt.txt;
  crates/shadows-mcp/src/workspace_tools.rs
- Touch: crates/core-harness/src/standards.rs, prompt.txt, contract.yaml;
  crates/shadows-mcp/src/workspace_tools.rs; crates/shadows/tests/planner_mcp.rs
- Done when: `cargo test -p shadows --test planner_mcp` and `cargo test -p core-harness` pass

B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at `f1459dcc`.
