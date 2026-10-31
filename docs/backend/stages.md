# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

The architecture restructuring ([plan](plans/2026-10-21-architecture.md)) shipped in
PR #41 (`c7e7bac4`): A-0 to A-21, A-23, B-10 and B-11 are in this file at `c7e7bac4`.
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
[docs/future.md](../future.md) "Housekeeping"; its entry is in this file at `3dc2437f`.

B-13 (the person record takes a partial update, `PATCH /api/me`) shipped in PR #46
(`e95b79c3`); its entry is in this file at `e95b79c3`.

A-8b1 and A-8b2 (the Hermes product daemon goes; the AYLUL project check and the master key
move into Core) and B-14 to B-18 (W-11's Core side) shipped in PR #47 (`f6551819`); their
entries are in this file at `4f23c717`.

## Tasks

Tool activity ([spec](specs/2026-10-31-tool-activity-design.md),
[plan](plans/2026-10-31-tool-activity.md)): B-19 to B-22 build the request
`docs/shared/requests/2026-10-31-tool-activity.md` (accepted at `7d20c439`), in order.

### B-19 · ACP tool facts and plan updates reach `HarnessEvent`
- Status: done
- Plan: docs/backend/plans/2026-10-31-tool-activity.md#b-19--acp-facts-reach-harnessevent
- Read: spec §1; code-map rows for `crates/shadows-agent/src/acp/mapping.rs` and `events/`; `crates/core-turns/contract.yaml` `live.cut` / `changed_cards` (the collector you only keep compiling)
- Touch: `crates/shadows-agent/src/events.rs`, `crates/shadows-agent/src/events/tool.rs` (new), `crates/shadows-agent/src/acp/mapping.rs`, `crates/shadows-agent/src/acp_tests.rs`, `crates/core-turns/src/entries.rs` (compile only), `crates/core-turns/tests/entries.rs` (compile only), `docs/codebase/README.md`
- Done when: `cargo test -p shadows-agent -- a_tool_call_carries_its_kind_locations_input_and_diffs an_update_carries_only_what_it_names a_plan_update_becomes_the_task_list a_failed_update_carries_its_text_and_raw_output` pass, `cargo test -p core-turns --test entries` and `cargo test -p shadows-core --test acp_connection` still pass

### B-20 · A `ToolCall` entry carries its `ToolCallCard`
- Status: done
- Plan: docs/backend/plans/2026-10-31-tool-activity.md#b-20--a-toolcall-entry-carries-its-card
- Read: spec §2 and §5; `crates/core-turns/contract.yaml` (`live`, the tool/subagent obligation); `crates/core-turns/src/subagents.rs` as the pattern; `crates/core-threads/contract.yaml` (the tool-line obligation); `docs/backend/AGENTS.md` rule 4
- Touch: `Cargo.toml`, `Cargo.lock`, `crates/core-turns/Cargo.toml`, `crates/core-turns/src/hunks.rs` (new), `crates/core-turns/src/tools.rs` (new), `crates/core-turns/src/entries.rs`, `crates/core-turns/src/turn.rs`, `crates/core-turns/src/spawn.rs`, `crates/core-turns/src/lib.rs`, `crates/core-turns/tests/entries.rs`, `crates/core-turns/contract.yaml`, `crates/core-threads/src/model.rs` (doc comments only), `crates/core-threads/contract.yaml`, `crates/shadows-core/src/lib.rs`, `crates/shadows-core/src/testing/turn.rs`, `crates/shadows-core/src/testing/mod.rs`, `crates/shadows/tests/turn_command.rs`, `crates/shadows/tests/planner_mcp/standards.rs`, `crates/shadows/tests/plan_boundaries.rs`, `crates/shadows-http/src/openapi.rs`, `api/openapi.json`, `crates/fake-acp/src/prompts.rs`, `crates/shadows/tests/planner_turn.rs`, `docs/codebase/README.md`
- Done when: `cargo test -p core-turns --test entries` (the eight new tests and the kept ones) and `cargo test -p shadows --test planner_turn tool_entries_carry_their_cards` pass; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` then `cargo test -p shadows --test openapi` pass

### B-21 · The task list becomes a `TodoList` entry; a running turn's activity is readable
- Status: done
- Plan: docs/backend/plans/2026-10-31-tool-activity.md#b-21--the-task-list-is-kept-and-a-running-turns-activity-is-readable
- Read: spec §3 and §4; `crates/core-turns/contract.yaml` (`registry`, `called_by_other_services`); `crates/core-threads/contract.yaml` (`ThreadEntryKind`)
- Touch: `crates/core-threads/src/model.rs`, `crates/core-threads/contract.yaml`, `crates/core-turns/src/entries.rs`, `crates/core-turns/src/tools.rs`, `crates/core-turns/src/turn.rs`, `crates/core-turns/src/handles.rs`, `crates/core-turns/src/spawn.rs`, `crates/core-turns/src/model.rs`, `crates/core-turns/src/lib.rs`, `crates/core-turns/contract.yaml`, `crates/core-turns/tests/entries.rs`, `crates/shadows-core/src/lib.rs`, `crates/shadows-http/src/openapi.rs`, `api/openapi.json`, `crates/fake-acp/src/prompts.rs`, `crates/shadows/tests/planner_turn.rs`, `crates/shadows/tests/thread_contract.rs`
- Done when: `cargo test -p core-turns --test entries`, `cargo test -p shadows --test planner_turn a_turns_task_list_is_kept_as_a_todo_list_entry`, `cargo test -p shadows --test thread_contract every_entry_kind_round_trips_as_its_stored_name` pass; openapi regenerated and `cargo test -p shadows --test openapi` passes

### B-22 · `tool` and `todos` frames, re-sent after `caught-up` for a running turn
- Status: done
- Plan: docs/backend/plans/2026-10-31-tool-activity.md#b-22--a-subscriber-gets-tool-and-todos-and-a-reload-gets-them-back
- Read: spec §4 and §5; `crates/core-events/contract.yaml` (`Delivery`, `next`, `transient`, the no-gap obligation); `crates/core-turns/contract.yaml` `live_activity`
- Touch: `crates/core-events/src/subscription.rs`, `crates/core-events/src/lib.rs`, `crates/core-events/contract.yaml`, `crates/shadows-http/src/sse.rs`, `api/openapi.json`, `crates/fake-acp/src/prompts.rs`, `crates/shadows/tests/stream_frames.rs`
- Done when: `cargo test -p shadows --test stream_frames -- tool_and_todos_frames_reach_a_running_turns_stream a_stream_opened_mid_turn_gets_its_open_calls_and_list_after_caught_up` pass, and the existing `stream_frames` and `protocol` tests still pass; openapi regenerated and `cargo test -p shadows --test openapi` passes

### B-23 · Technical direction ids read `TD`, never a task's `T`
- Status: done
- Plan: the user's ruling, 2026-10-31: a vision item `T1` (technical direction) reads the same as plan task T1, in the app and in what the Planner writes. The prefix becomes `TD`, as `PU` keeps purpose apart from a rule's `P1`. Ids already stored are renamed, so every project reads `TD<n>` and no `T<n>` vision id is left.
- Read: `docs/superpowers/specs/2026-10-08-guided-planning-design.md` "Ids" (line ~112); `crates/core-design/contract.yaml` (vision items); `crates/core-design/src/vision_items.rs` (`prefix`, `of_id`); `crates/core-db/migrations/0022_vision_items.sql` (where ids are stored: `design_workspace.vision_content`, `design_vision_version.content_json`, `design_vision_proposal.item_id` and the proposal's ops JSON)
- Touch: `crates/core-design/src/vision_items.rs`, `crates/core-design/contract.yaml`, a new `crates/core-db/migrations/0038_vision_technical_ids.sql` (renames `T<n>` to `TD<n>` in every stored place the read names, technical direction only; the counters stay as they are), the tests that name `T1` (`crates/shadows/tests/design_routes.rs`, `crates/shadows-core/tests/design_vision/*.rs`) and a migration test beside `crates/shadows-core/tests/plan_migration.rs`, the spec's "Ids" line (amended in place: technical direction `TD`, with the reason), and `web/src/test/contract-fixtures.ts` / `web/src/test/fake-daemon.ts` only if they hold a vision `T1`
- Needs: none
- Done when: `cargo test -p shadows-core --test design_vision`, `cargo test -p shadows --test design_routes`, `cargo test -p shadows --test codemap` and the new migration test `stored_technical_direction_ids_become_td` pass (a vision version, the current vision and a pending proposal holding `T1`/`T2` read `TD1`/`TD2` after migrating; `of_id("T1")` is `None`, `of_id("TD1")` is technical direction); `cargo clippy` on the touched crates is clean

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
