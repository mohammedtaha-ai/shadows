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
`docs/backend/plans/2026-11-04-planner-context-slice-1.md` at `fc78cb7`. The shared request
`docs/shared/requests/2026-11-04-planning-context-routes.md` stays until the frontend slice that shows the
inspection view is built. Local acceptance with real provider and browser is in the
[dated results](../evidence/2026-11-04-planner-context-probe.md#4-slice-1-implementation-acceptance-2026-11-04).
Not built: the re-send after a compaction (spec §6, slice 2, planned below), the inspection view
(frontend), and `Plans::owner_thread` (spec §6, slice 4).

## Tasks

B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at `f1459dcc`.

Planner context, slice 2: re-send after a compaction, or on request
([spec](specs/2026-11-04-planner-context-design.md) §4.3, [plan](plans/2026-11-04-planner-context-slice-2.md),
[shared request](../shared/requests/2026-11-04-planning-context-resend.md)). In order: B-55, B-56,
B-57, then B-58. The live acceptance follows them (the plan's last section).

### B-55 · The adapter's compaction is one event (shadows-agent, fake-acp)
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-2.md#task-b-55-the-adapters-compaction-is-one-event-shadows-agent-fake-acp
- Read: the plan's "What the adapter sends"; `docs/evidence/2026-11-04-planner-context-probe.md` §1;
  the `shadows-agent` rows of docs/codebase/README.md
- Touch: crates/shadows-agent/src/events.rs, crates/shadows-agent/src/acp/mapping.rs,
  crates/shadows-agent/src/acp_tests.rs, crates/core-turns/src/entries.rs,
  crates/fake-acp/src/prompts.rs
- Done when: `cargo test -p shadows-agent` (the five tests the plan names) and
  `cargo test -p core-turns` pass

### B-56 · A thread forgets what its session heard (Harness)
- Status: done
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-2.md#task-b-56-a-thread-forgets-what-its-session-heard-harness
- Read: crates/core-harness/contract.yaml; spec §4.3 "What a compaction or a request forgets";
  crates/core-db/contract.yaml (the journal, commands)
- Touch: crates/core-db/migrations/0046_context_forget.sql,
  crates/core-db/tests/fixtures/base_migrations.sha384, crates/core-db/tests/migration_checksums.rs,
  crates/core-db/tests/storage_contract/schema.rs, crates/core-db/contract.yaml,
  crates/core-harness/src/context_record.rs, crates/core-harness/src/store/context_record.rs,
  crates/core-harness/src/setup.rs, crates/core-harness/src/model.rs,
  crates/core-harness/src/error.rs, crates/core-harness/contract.yaml,
  crates/shadows-core/src/error.rs, crates/shadows-core/src/lib.rs (re-export `Forget`),
  crates/shadows/tests/architecture/table-owners.txt, crates/shadows/tests/context_record.rs
- Done when: `cargo test -p core-harness`, `cargo test -p shadows --test context_record`,
  `cargo test -p core-db --test migration_checksums --test storage_contract` pass

### B-57 · A completed compaction forgets (Turns)
- Status: todo
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-2.md#task-b-57-a-completed-compaction-forgets-turns
- Read: crates/core-turns/contract.yaml; B-55's event and B-56's method
- Touch: crates/core-turns/src/turn.rs, crates/core-turns/contract.yaml,
  crates/shadows/tests/planning_context.rs
- Done when: `cargo test -p shadows --test planning_context` (the six tests the plan names) and
  `cargo test -p core-turns` pass

### B-58 · The "send the context again" route (shadows-http)
- Status: todo
- Plan: docs/backend/plans/2026-11-04-planner-context-slice-2.md#task-b-58-the-send-the-context-again-route-shadows-http
- Read: crates/shadows-http/src/contract.yaml; the shared request
  docs/shared/requests/2026-11-04-planning-context-resend.md
- Touch: crates/shadows-http/src/planning_context.rs, crates/shadows-http/src/lib.rs,
  crates/shadows-http/src/contract.yaml, api/openapi.json, web/src/api/schema.d.ts,
  crates/shadows/tests/planning_routes.rs
- Done when: `cargo test -p shadows --test planning_routes --test openapi` pass
