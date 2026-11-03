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

## Tasks

Agent groups, messaging slice 1 ([spec](specs/2026-11-03-agent-messaging-design.md), [plan](plans/2026-11-03-agent-groups.md)).
B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at `f1459dcc`.

### B-40 · Hidden member threads
- Status: done
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-40-hidden-member-threads-threads
- Read: crates/core-threads/contract.yaml; crates/core-harness/src/sessions.rs (`close_agent`)
- Touch: crates/core-db/migrations/0041_thread_hidden.sql; crates/core-threads/src/lib.rs;
  crates/core-threads/src/store/thread.rs; crates/core-threads/src/store/records.rs;
  crates/core-threads/contract.yaml; crates/core-threads/tests/hidden.rs;
  crates/core-projects/src/harness_policy.rs; crates/core-projects/contract.yaml
- Done when: `cargo test -p core-threads` passes, with `a_hidden_thread_is_left_out_of_the_agent_list`
  and `a_hidden_thread_is_still_closed_with_its_agent`

### B-41 · Ended turns from the journal
- Status: done
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-41-ended-turns-from-the-journal-events
- Read: crates/core-events/contract.yaml; crates/core-operations/src/store/lifecycle.rs
- Touch: crates/core-events/src/lib.rs, src/store.rs, src/model.rs, src/store_tests.rs, contract.yaml;
  docs/codebase/README.md
- Done when: `cargo test -p core-events` passes, with
  `ended_turns_come_back_in_seq_order_after_the_cursor` and `an_interrupted_turn_is_an_ended_turn`

### B-42 · The Messages service, groups and their log
- Status: done
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-42-the-messages-service-groups-and-their-log
- Read: docs/backend/specs/2026-11-03-agent-messaging-design.md §6.2, §7; crates/shadows-core/src/app.rs; docs/codebase/README.md
- Touch: crates/core-messages/ (new); crates/core-db/migrations/0042_messages.sql;
  crates/shadows-core/src/app.rs; crates/shadows-core/Cargo.toml; docs/codebase/README.md
- Done when: `cargo test -p core-messages` and `cargo test -p shadows --test codemap` pass

### B-43 · Mohammed writes; deliveries are scheduled and dispatched
- Status: done
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-43-mohammed-writes-deliveries-are-scheduled-and-dispatched
- Read: the plan's "Routing rules" and "Delivery rules"; crates/core-turns/contract.yaml (`send`)
- Touch: crates/core-messages/src/send.rs, mentions.rs, bounds.rs, deliveries.rs, prompt.rs,
  lib.rs, store/mod.rs, store/write.rs, store/deliveries.rs, Cargo.toml, contract.yaml,
  tests/send.rs, tests/support/mod.rs; crates/core-turns/src/lib.rs; crates/core-turns/contract.yaml;
  crates/shadows/tests/group_routes.rs (the tests that need a running turn: AppCore and fake-acp)
- Done when: `cargo test -p core-messages` verifies routing, ordering and bounded prompts, and
  `cargo test -p shadows --test group_routes` verifies prompts, one turn per delivery and replay.

### B-44 · Members answer, from the journal, inside bounds
- Status: done
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-44-members-answer-from-the-journal-inside-bounds
- Read: the plan's "Delivery rules"; B-41's `Events::ended_turns_after`
- Touch: crates/core-messages/src/replies.rs, deliveries.rs, error.rs, lib.rs, contract.yaml;
  crates/shadows-core/src/app.rs; crates/core-harness/src/settings.rs;
  crates/core-harness/contract.yaml; crates/fake-acp/src/prompts.rs;
  crates/fake-acp/src/handlers/session.rs; crates/shadows/tests/group_routes.rs
- Done when: `group_routes` covers durable recovery, failure, bounds, busy retry and shutdown.

### B-45 · Routes; messages on the project stream
- Status: done
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-45-routes-messages-on-the-project-stream
- Read: crates/shadows-http/src/contract.yaml; crates/core-events/contract.yaml
- Touch: crates/shadows-http/src/groups.rs, lib.rs, sse.rs, failure/other.rs, contract.yaml;
  crates/core-events/src/store.rs, contract.yaml; api/openapi.json;
  crates/shadows-core/src/lib.rs; crates/shadows/tests/group_routes.rs;
  crates/shadows/tests/openapi.rs; crates/shadows/tests/openapi/read_only.rs; web/src/api/schema.d.ts
- Done when: `cargo test -p shadows --test group_routes --test openapi` passes and the generated
  API describes the group routes.

### B-46 · When a group started needing Mohammed
- Status: todo
- Plan: docs/backend/plans/2026-11-03-agent-groups.md#task-b-46-when-a-group-started-needing-mohammed
- Read: crates/core-messages/contract.yaml (`needs_you`); the UI plan's U-10
  (docs/frontend/plans/2026-11-03-agent-groups-ui.md)
- Touch: crates/core-messages/src/model.rs, src/store/mod.rs, contract.yaml, tests/groups.rs;
  api/openapi.json; web/src/api/schema.d.ts
- Done when: `cargo test -p core-messages` and `cargo test -p shadows --test openapi` pass, with
  `needs_you_since_is_the_first_unanswered_call`

Core's local acceptance used `shadows serve` with synthetic ACP and two agents. Alpha alone
answered the addressed message. Restart preserved the group and reopened Alpha's member
session. The local log recorded `OperationCompleted` and graceful shutdown. CI and the AYLUL
desktop acceptance remain separate gates.
