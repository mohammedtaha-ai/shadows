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

Still open from that plan:
- The real-app acceptance of the design's §10 (items 1–9): a person runs `shadows serve` with
  the desktop, and reads the logs for any MCP value, header or env value.
- A ruling on whether an AYLUL agent may answer its own permission request: the OPEN in
  [AYLUL agents and panel §7.8](../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md).

## Tasks

### B-35 · An agent knows its name and role
- Status: done
- Spec: [AYLUL agents and panel §7.1](../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md) ("Name and description are who the agent is")
- Read: `crates/shadows-core/src/app.rs` (`Agents`); `crates/core-agents/contract.yaml`; `crates/core-agents/src/home.rs`; the bundled `CLAUDE.md` the contract names; the Harness caller of `home_instructions` (`crates/core-harness/contract.yaml`).
- Touch: `crates/core-agents/src/home.rs`; the bundled `CLAUDE.md`; `crates/core-agents/contract.yaml`; `crates/core-agents/tests/`; a Harness test that asserts the written home `CLAUDE.md`, if one does.
- Needs: none.
- Done when: `home_instructions` returns `Your name is <name>.`, then `Your role: <description>.` when the description is not empty, a blank line, then the stored text or the template. The template names no agent and keeps the general rules. The stored-empty and import rules are unchanged. `cargo test -p core-agents` passes, with `home_instructions_open_with_the_name_and_role` and `an_empty_description_writes_no_role_line`.

### B-36 · The adapter's background tasks reach Core
- Status: todo
- Spec: [tool activity §6](specs/2026-10-31-tool-activity-design.md#6-background-tasks)
- Read: `crates/shadows-agent/src/acp/connection.rs` (`capabilities`, the notification handler); `acp/mapping.rs`; `src/events.rs`; the adapter's `harness/claude/node_modules/@agentclientprotocol/claude-agent-acp/dist/async-tasks.js` (`announce`, `publishState`) and `acp-agent.js` (`ASYNC_TASK_STOP_METHOD`, `stopAsyncTask`).
- Touch: `crates/shadows-agent/src/acp/*.rs`; `crates/shadows-agent/src/events.rs` (and `events/task.rs` if it earns a file); `crates/shadows-agent/src/acp_tests.rs`; `crates/fake-acp/src/` (a scripted prompt that sends the three kinds and answers the stop request); `crates/shadows-agent/src/acp.rs` (the crate has no `contract.yaml`; the code map names `acp.rs` as its contract) and `docs/codebase/README.md` rows for any new file; every exhaustive `match` on `HarnessEvent` that the new variant breaks (`crates/core-turns/src/entries.rs`, `turn.rs`, `crates/core-events/src/subscription.rs`, `crates/core-harness/src/*.rs`), with an arm that ignores `Task` until B-37, and their contracts if a rule changes; any existing test whose assertion §6 changes (`crates/shadows/tests/harness_effort.rs` asserts the advertised capability list).
- Needs: none.
- Done when: `initialize` advertises `["recommendedValue", "asyncTasks"]`; the three `async_task_*` kinds become `HarnessEvent::Task` events, read untyped because ACP's `SessionUpdate` has no variant for them, and an unknown kind never drops another notification; a connection method sends `_session/async_task/stop` and returns `stopped`. `cargo test -p shadows-agent` passes, with `async_tasks_are_advertised`, `async_task_updates_become_task_events` and `stopping_a_task_asks_the_adapter`. First probe: what the SDK does today with an unknown `sessionUpdate`; report it.

### B-37 · Harness keeps the session's tasks; frame and stop route
- Status: todo
- Spec: [tool activity §6](specs/2026-10-31-tool-activity-design.md#6-background-tasks)
- Read: `crates/shadows-core/src/app.rs` (`Harness`); `crates/core-harness/contract.yaml`; how `HarnessEvent::Commands` and `SessionTitle` reach a thread's stream between turns; `crates/shadows-http/contract.yaml`; `crates/shadows-http/src/sse.rs`, `harness.rs`, `openapi.rs`.
- Touch: `crates/core-harness/src/`, its `contract.yaml` and `tests/`; the `core-events` subscription and contract only if the send after `caught-up` needs it; `crates/shadows-http/src/{sse,harness,lib,openapi}.rs` and its contract; `crates/shadows/tests/task_routes.rs` (new) and `openapi.rs` (route list); `api/openapi.json`; `web/src/api/` (regenerated only).
- Needs: B-36.
- Done when: per open session Harness keeps §6's task list and publishes a `tasks` frame on every change, between turns too, and once after `caught-up` when the list is not empty; `Harness::stop_task` and `POST /api/threads/{id}/session/tasks/{task_id}/stop` behave as §6 says. `cargo test -p core-harness` and `cargo test -p shadows --test task_routes` pass, with `tasks_outlive_the_turn_that_started_them`, `a_subscriber_gets_the_tasks_after_caught_up`, `stop_reaches_the_adapter`, `unknown_task_is_404` and `an_ended_task_is_not_stopped`; `npm run gen:api` and `cargo test -p shadows --test openapi` pass.

None else open. B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at
`f1459dcc`.
