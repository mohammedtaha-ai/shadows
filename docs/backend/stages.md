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

## Tasks

Planner context, slice 1, with the fixes for three Planner defects ([spec](specs/2026-11-04-planner-context-design.md), [plan](plans/2026-11-04-planner-context-slice-1.md)).
B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at `f1459dcc`.

### B-47 · A thread's planning anchor (Design)
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-47-a-threads-planning-anchor-design
- Read: docs/backend/specs/2026-11-04-planner-context-design.md §4.1; crates/core-design/contract.yaml
- Touch: crates/core-db/migrations/0043_thread_anchor.sql; crates/core-design/src/anchors.rs,
  src/store/, src/lib.rs, contract.yaml, tests/anchors.rs; crates/core-db/tests/fixtures/base_migrations.sha384; crates/core-db/tests/migration_checksums.rs
- Done when: `cargo test -p core-design --test anchors` and `-p core-db --test migration_checksums` pass

### B-48 · What a turn carried, per session (Harness)
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-48-what-a-turn-carried-per-session-harness
- Read: docs/backend/specs/2026-11-04-planner-context-design.md §4.4; crates/core-harness/contract.yaml
- Touch: crates/core-db/migrations/0044_invocation_context.sql; crates/core-harness/src/context_record.rs,
  src/store/, src/model.rs, src/lib.rs, contract.yaml; crates/shadows/tests/context_record.rs; crates/core-db/tests/fixtures/base_migrations.sha384; crates/core-db/tests/migration_checksums.rs
- Done when: `cargo test -p core-harness` and `cargo test -p shadows --test context_record` pass

### B-49 · Compile the planning context (Turns)
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-49-compile-the-planning-context-turns
- Read: docs/backend/specs/2026-11-04-planner-context-design.md §4.2; crates/core-turns/contract.yaml; crates/core-plans/src/design_views.rs
- Touch: crates/core-turns/src/planning/ (new), src/lib.rs, contract.yaml,
  tests/planning_context.rs; crates/shadows-core/src/app.rs
- Done when: `cargo test -p core-turns` passes, with compiler tests in core-turns and the three AppCore cases in shadows/tests/planning_context.rs

### B-50 · Send it when the session has not heard it
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-50-send-it-when-the-session-has-not-heard-it
- Read: docs/backend/specs/2026-11-04-planner-context-design.md §4.3; crates/core-turns/src/spawn.rs, prompt.rs
- Touch: crates/core-turns/src/lib.rs, spawn.rs, prompt.rs, store/operation.rs, contract.yaml;
  crates/core-harness/src/context_record.rs, contract.yaml; crates/shadows-core/src/testing/turn.rs;
  crates/shadows/tests/planning_context.rs, turn_command.rs, plan_boundaries.rs, planner_mcp/standards.rs
- Done when: `cargo test -p shadows --test planning_context` passes, with the eight tests the task names

### B-51 · Routes for the anchor and the record
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-51-routes-for-the-anchor-and-the-record
- Read: crates/shadows-http/src/contract.yaml
- Needs: docs/shared/requests/2026-11-04-planning-context-routes.md, accepted
- Touch: crates/shadows-http/src/ (thread routes, openapi.rs, contract.yaml); api/openapi.json;
  web/src/api/schema.d.ts; crates/shadows/tests/planning_routes.rs (new)
- Done when: `cargo test -p shadows --test planning_routes --test openapi` passes

### B-52 · Map refusals say which rule, which parts and what to do
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-52-map-refusals-say-which-rule-which-parts-and-what-to-do-defect-1
- Read: docs/evidence/2026-11-04-planner-session-defects.md §1; crates/core-design/src/map_rules.rs; crates/shadows-mcp/src/refusal.rs
- Touch: crates/core-design/src/map_rules.rs, error.rs, map_proposals.rs, contract.yaml, tests/;
  crates/shadows-core (ErrorCode); crates/shadows-mcp/src/refusal.rs, contract.yaml;
  crates/shadows-http/src/ failure mapping
- Done when: `cargo test -p core-design` and `cargo test -p shadows-mcp` pass, with the four tests named

### B-53 · Vision IDs are given when proposed
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-53-vision-ids-are-given-when-proposed-defect-2
- Read: docs/evidence/2026-11-04-planner-session-defects.md §2; crates/core-design/src/store/proposal_approval.rs, store/vision.rs
- Touch: crates/core-db/migrations/0045_vision_reserved_id.sql; crates/core-design/src/store/,
  proposals.rs, contract.yaml, tests/; crates/core-db/tests/fixtures/base_migrations.sha384; crates/core-db/tests/migration_checksums.rs
- Done when: `cargo test -p core-design` passes, with the two tests in shadows/tests/vision_proposal_ids.rs

### B-54 · A map proposal lists vision items no part serves
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-1.md#task-b-54-a-map-proposal-lists-vision-items-no-part-serves-defect-3
- Read: docs/evidence/2026-11-04-planner-session-defects.md §3; crates/core-design/src/map_proposals.rs
- Touch: crates/core-design/src/map_proposals.rs, map_rules.rs, store/map_proposals.rs, contract.yaml;
  crates/shadows/tests/map_proposal_coverage.rs; api/openapi.json; web/src/api/schema.d.ts
- Done when: `cargo test -p core-design` passes, with `a_map_answer_lists_the_unserved_vision_items` in shadows/tests/map_proposal_coverage.rs

## Slice 1 integration checks

The branch also updates the canonical contract entries, code map, table-owner inventory,
foundation schema fixture and existing prompt/vision expectations. Harness context-record
tests run through AppCore in `crates/shadows/tests/context_record.rs`, preserving its
no-Runtime dependency boundary. Source footer and cut markers count toward the index estimate;
depth-two index entries retain their parent ids.

Focused checks passed: Core architecture (49), code map (1), migration checksums (1),
foundation storage (9), Core contracts (6), design/vision suites, Turns (36), HTTP (32),
MCP refusal mapping, OpenAPI (8), the twelve selected application regression targets,
generated web types and focused all-target Clippy. After the final review fixes, Turns has 39 passing tests and the application context suite
has 13. Production Clippy, the executable build and real provider/browser acceptance passed:
[dated results](../evidence/2026-11-04-planner-context-probe.md#4-slice-1-implementation-acceptance-2026-11-04).
The full workspace gate remains for CI under Mohammed's instruction.
