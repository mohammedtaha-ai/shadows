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

Tool activity ([spec](specs/2026-10-31-tool-activity-design.md)) and B-19 to B-23 (ACP tool
facts, the `ToolCall` card, the task list, the `tool` and `todos` frames, the `TD` ids) shipped
in PR #48 (`ce31036d`); their entries are in this file at `ce31036d`.

AYLUL agents and panel, Core side ([plan](plans/2026-11-01-aylul-agents-panel-backend.md),
design `docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md`, accepted 2026-11-01):
B-24 to B-34 on `feature/aylul-agents-panel`. Order and parallel runs: plan §4. Tasks marked
`blocked` change what the frontend sees and wait for the shared request that accepts plan §3
(`docs/shared/requests/2026-11-01-aylul-agents-panel.md`, not yet filed).

## Tasks

### B-24 · Migration 0039: agent profile columns, MCP tables, bundled flag, declined names
- Status: todo
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#2-how-each-part-is-built (§2.1, §2.5, §2.6 storage lines)
- Read: `crates/core-db/contract.yaml` (migration catalog rule); `crates/core-db/migrations/0035_aylul_extensions.sql`; `crates/shadows/tests/architecture/table-owners.txt`.
- Touch: `crates/core-db/migrations/0039_aylul_agent_profile.sql` (new); `crates/core-db/tests/fixtures/base_migrations.sha384`; `crates/core-db/tests/migration_checksums.rs` (count 39); `crates/shadows/tests/architecture/table-owners.txt` (`aylul_mcp_server`, `aylul_agent_mcp`, `aylul_extension_declined` :: extensions).
- Needs: nothing.
- Schema: `aylul_agent` + `description TEXT NOT NULL DEFAULT ''`, `instructions TEXT` (NULL = home file not imported), `outfit TEXT NOT NULL DEFAULT 'default'`; `aylul_extension` + `bundled INTEGER NOT NULL DEFAULT 0 CHECK(bundled IN (0,1))`; new `aylul_mcp_server(id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT NOT NULL, config_json TEXT NOT NULL, created_at TEXT NOT NULL)`; new `aylul_agent_mcp(agent_slug TEXT NOT NULL REFERENCES aylul_agent(slug), mcp_id TEXT NOT NULL REFERENCES aylul_mcp_server(id) ON DELETE CASCADE, enabled INTEGER NOT NULL CHECK(enabled IN (0,1)), PRIMARY KEY(agent_slug, mcp_id))`; new `aylul_extension_declined(kind TEXT NOT NULL, name TEXT NOT NULL, declined_at TEXT NOT NULL, PRIMARY KEY(kind, name))`. No table rebuild.
- Done when: `cargo test -p core-db --test migration_checksums`, `cargo test -p core-agents --test agents`, `cargo test -p core-extensions --test library`, `cargo test -p shadows --test architecture` pass.

### B-25 · Agents: description, instructions, outfit, the template, the one-time home import
- Status: blocked (shared request for plan §3.1)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#21-agents-71-73--b-25
- Read: `crates/core-agents/contract.yaml`; design §7.1, §7.3; plan §2.1 and §3.1; `crates/shadows-core/src/app.rs` `AppCore::start`.
- Touch: `crates/core-agents/src/{model.rs,lib.rs,home.rs,store/mod.rs}`; `crates/core-agents/contract.yaml`; `crates/core-agents/tests/agents.rs`; `crates/shadows-core/src/app.rs` (call `agents.import_home_instructions()` in `start`, after `build_aylul_owners`); `crates/shadows-http/src/aylul_agents.rs` (`GET /api/aylul/agent-template`); `crates/shadows-http/src/lib.rs` (route); `crates/shadows-http/src/openapi.rs`; `crates/shadows-http/src/aylul_profiles_tests.rs`; `crates/shadows-http/contract.yaml`; `api/openapi.json`.
- Needs: B-24; the shared request.
- Contract lines: shapes `Agent`/`AgentInput` (description, instructions, outfit); functions `instructions_template`, `home_instructions`, `import_home_instructions`; `seed_home` no longer writes `CLAUDE.md`; obligations for each rule below.
- Done when, each by name in `cargo test -p core-agents --test agents`: `existing_agents_migrate_with_an_empty_profile`, `description_is_one_trimmed_line_of_200`, `instructions_are_bounded_and_refuse_nul`, `instructions_equal_to_the_template_are_stored_empty`, `outfit_shape_is_checked_and_unknown_is_kept`, `update_without_profile_fields_keeps_them`, `home_instructions_import_once_and_never_again`, `an_unreadable_home_file_stays_unimported`; and `cargo test -p shadows-http agent_template_is_served`, `cargo test -p shadows-http agent_profile_fields_round_trip`; then `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` and `cargo test -p shadows --test openapi`.

### B-26 · Secrets: the `mcp/<id>` scope and its name rule
- Status: todo
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#22-secrets-72--b-26
- Read: `crates/core-secrets/contract.yaml`; design §7.2 (secret bullets).
- Touch: `crates/core-secrets/src/lib.rs`; `crates/core-secrets/src/model.rs` (only if a type changes); `crates/core-secrets/contract.yaml`; `crates/core-secrets/tests/secret_env.rs`.
- Needs: nothing.
- Contract lines: functions `mcp_scope` (the one place naming `mcp/{id}`), `mcp_value_write`, the MCP name rule beside `validate_env_name`; obligation "MCP names are 1..=128 of A-Z a-z 0-9 - _; reserved names are refused in every scope".
- Done when: `cargo test -p core-secrets --test secret_env mcp_` runs and passes `mcp_names_follow_their_own_rule`, `mcp_scope_refuses_reserved_names`, `mcp_values_round_trip_under_their_scope`, `mcp_value_absent_from_db_and_command_record`; the whole `cargo test -p core-secrets` still passes.

### B-27 · ACP session setup carries a list of MCP servers (http or stdio)
- Status: todo
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#23-the-acp-sessions-mcp-list-72-delivery--b-27
- Read: `crates/shadows-agent/src/acp/session.rs`; `crates/core-harness/contract.yaml` (`opening`, Planner MCP lines); code-map rows for `shadows-agent` and `fake-acp` in `docs/codebase/README.md`.
- Touch: `crates/shadows-agent/src/acp/session.rs`; `crates/shadows-agent/src/acp_tests.rs`; `crates/core-harness/src/setup.rs`; `crates/core-harness/src/assistant.rs` (`aylul_setup`: empty list); `crates/core-harness/contract.yaml`; `crates/fake-acp/src/session.rs`; `crates/fake-acp/src/prompts.rs` (a `report-mcp` prompt answering names, transports and value hashes).
- Needs: nothing.
- Done when: `cargo test -p shadows-agent mcp_` passes `mcp_servers_carry_http_headers_and_stdio_env` and `mcp_spec_debug_never_prints_values`; `cargo test -p shadows --test planner_mcp` and `cargo test -p shadows-core --test planner_isolation` pass unchanged; `cargo test -p shadows-core --test aylul_session opening_uses_agent_and_provider` passes (AYLUL: no MCP).

### B-28 · ACP connection: hold, answer and refuse permission requests
- Status: todo
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#24-holding-a-permission-request-78--b-28-b-32-b-33
- Read: `crates/shadows-agent/src/acp/connection.rs` (`open`); `crates/shadows-agent/src/acp/prompt.rs` (`cancel`); `crates/shadows-agent/src/events.rs`; design §7.8.
- Touch: `crates/shadows-agent/src/acp.rs`; `crates/shadows-agent/src/acp/connection.rs`; `crates/shadows-agent/src/acp/permissions.rs` (new: the held map, `PermissionPolicy`); `crates/shadows-agent/src/acp/prompt.rs`; `crates/shadows-agent/src/events.rs` (`PermissionAsked`, `PermissionAnswered`, `PermissionRefused { title, request, reason }`); `crates/shadows-agent/Cargo.toml` (only if `uuid` is not yet a dependency: workspace library, infrastructure); `crates/core-harness/src/live.rs` (pass `PermissionPolicy::Refuse` everywhere for now); `crates/core-turns/src/entries.rs` (new variants ignored, refusal keeps today's entry); `crates/shadows-core/tests/acp_connection.rs`; `crates/shadows-core/tests/aylul_extensions/live.rs`; `docs/codebase/README.md` (row for `acp/permissions.rs`).
- Needs: nothing.
- Done when: `cargo test -p shadows-core --test acp_connection` passes `a_refusing_connection_rejects_at_once`, `a_held_request_waits_for_its_answer`, `an_option_not_offered_is_refused_and_the_request_stays`, `a_second_answer_finds_the_request_settled`, `cancel_refuses_held_requests_before_cancelling`, `refuse_held_ends_every_waiting_request`; `cargo test -p shadows --test planner_turn` passes unchanged; `cargo test -p core-turns --test entries` passes.

### B-29 · Extensions: MCP items, write-only values, the Secrets edge
- Status: blocked (shared request for plan §3.2)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#25-extensions-mcp-items-72--b-29
- Read: `crates/core-extensions/contract.yaml`; `crates/core-secrets/contract.yaml` (after B-26); `crates/core-providers/contract.yaml` (`validate_url`); plan §2.5, §3.2.
- Touch: `crates/core-extensions/Cargo.toml` (`core-secrets`, `core-providers` as dependencies); `crates/core-extensions/src/{lib.rs,model.rs,error.rs,store/mod.rs}`; `crates/core-extensions/src/mcp.rs` (new: MCP item commands and checks); `crates/core-extensions/contract.yaml`; `crates/core-extensions/tests/library.rs`; `crates/core-providers/src/validation.rs` (`validate_url` public); `crates/core-providers/contract.yaml`; `crates/shadows-core/src/app.rs` (`Extensions::new(storage, secrets, home_root)`); `crates/shadows-core/src/lib.rs` (re-exports); `crates/shadows-core/src/error.rs` (ExtensionsError's Secrets arm); `crates/shadows-http/src/failure/other.rs` (only if the new arm needs it); `crates/core-harness/tests/fixture/mod.rs`; `crates/shadows-http/src/aylul_extensions.rs` (POST and PUT `/api/aylul/extensions/mcp…`; the folder import refuses `mcp`); `crates/shadows-http/src/aylul_extensions_tests.rs`; `crates/shadows-http/src/lib.rs`; `crates/shadows-http/src/openapi.rs`; `crates/shadows-http/contract.yaml`; `api/openapi.json`.
- Needs: B-24, B-25 (API document order), B-26; the shared request.
- Contract lines: `dependencies` + secrets, providers; `ExtensionKind` values Plugin, Skill, Mcp; functions `add_mcp`, `update_mcp`, `agent_mcp_servers`; `delete_extension` and `remove_agent_in` cover MCP rows and the `mcp/<id>` scope; obligations below.
- Done when: `cargo test -p core-extensions --test library mcp_` passes `mcp_item_is_configuration_with_write_only_values`, `mcp_input_is_checked`, `mcp_edit_keeps_its_id_switches_and_unsent_values`, `mcp_delete_removes_switches_and_scope_in_one_write`, `mcp_commands_replay_and_conflict`, `mcp_values_never_leave_secrets`, `agent_mcp_servers_resolve_only_enabled_items`; the rest of `--test library` passes; `cargo test -p shadows-http aylul_extensions` passes incl. `mcp_routes_answer_names_never_values` and `folder_import_refuses_mcp`; openapi regenerated and `cargo test -p shadows --test openapi` passes; `cargo test -p shadows --test architecture` passes.

### B-30 · Extensions: bundled library items, imported at start, declined when deleted
- Status: blocked (shared request for plan §3.2, `Item.bundled`)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#26-extensions-bundled-items-74--b-30
- Read: `crates/core-extensions/contract.yaml`; design §7.4; plan §2.6.
- Touch: `crates/core-extensions/assets/library/skills/aylul-hello/SKILL.md` (new); `crates/core-extensions/build.rs` (new); `crates/core-extensions/Cargo.toml` (`build`); `crates/core-extensions/src/bundled.rs` (new); `crates/core-extensions/src/{lib.rs,model.rs,store/mod.rs}`; `crates/core-extensions/contract.yaml` (modules incl. the assets folder); `crates/core-extensions/tests/library.rs`; `crates/shadows-core/src/app.rs` (call `extensions.import_bundled()` in `start`, after owners); `api/openapi.json`.
- Needs: B-24, B-29.
- Done when: `cargo test -p core-extensions --test library bundled_` passes `bundled_items_import_once_at_start`, `a_restart_replays_the_bundled_import`, `a_deleted_bundled_item_is_declined`, `a_bundled_item_starts_switched_off`, `no_aylul_configuration_imports_nothing`; openapi regenerated and `cargo test -p shadows --test openapi`; `cargo test -p shadows --test codemap` passes.

### B-31 · Harness opening: write CLAUDE.md, send the agent's MCP servers
- Status: todo (after B-25, B-27, B-29)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#27-the-opening-71-72--b-31
- Read: `crates/core-harness/contract.yaml` (`opening`, extension projection obligations, `write_extensions` agreement); `crates/core-agents/contract.yaml` (`home_instructions`); `crates/core-extensions/contract.yaml` (`agent_mcp_servers`).
- Touch: `crates/core-harness/src/assistant.rs`; `crates/core-harness/src/profile_home.rs` (atomic `CLAUDE.md` write under the home lock); `crates/core-harness/src/extension_home.rs` (MCP snapshot under the same guard); `crates/core-harness/contract.yaml`; `crates/shadows-core/tests/aylul_session/homes.rs`; `crates/shadows-core/tests/aylul_session/opening.rs`.
- Needs: B-25, B-27, B-29.
- Done when: `cargo test -p shadows-core --test aylul_session` passes incl. `opening_writes_the_stored_instructions`, `opening_writes_the_template_for_empty_instructions`, `opening_leaves_an_unimported_home_file`, `opening_sends_only_enabled_mcp_items`, `an_aylul_opening_never_receives_shadows_mcp`, `mcp_values_reach_the_session_as_hashes_only`; `cargo test -p shadows --test planner_mcp` and `cargo test -p shadows --test acp_log_safety` pass.

### B-32 · Turns: permission entries and the `PermissionsChanged` event
- Status: blocked (shared request for plan §3.3 entries)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#24-holding-a-permission-request-78--b-28-b-32-b-33
- Read: `crates/core-turns/contract.yaml` (entries, `persist`); `crates/core-threads/contract.yaml` (`ThreadEntryKind`); `crates/core-events/contract.yaml` (`subscribe_project`); plan §3.3 entry table.
- Touch: `crates/core-threads/src/model.rs` (`PermissionAsked`, `PermissionAnswered`); `crates/core-threads/contract.yaml`; `crates/core-turns/src/entries.rs`; `crates/core-turns/src/turn.rs` (`persist`, the project id on `TurnWatch`); `crates/core-turns/src/spawn.rs` (only to fill that project id); `crates/core-turns/contract.yaml`; `crates/core-turns/tests/entries.rs`; `crates/core-events/src/store.rs` (allowlist `PermissionsChanged`); `crates/core-events/contract.yaml`; `crates/core-events/tests/cursor.rs` (or a new `crates/core-events/tests/project_permissions.rs`); `api/openapi.json`.
- Needs: B-28; B-30 (API document order); the shared request.
- Done when: `cargo test -p core-turns --test entries permission_` passes `permission_asked_and_answered_become_entries`, `an_open_ask_is_refused_at_finish`, `the_planner_refusal_entry_is_unchanged`; `cargo test -p core-events` passes incl. `project_stream_carries_permissions_changed`; `cargo test -p shadows --test planner_turn` passes; openapi regenerated and `cargo test -p shadows --test openapi`.

### B-33 · Harness: AYLUL asks, `answer_permission`, `waiting_permissions`, refusal on cancel/close/stop
- Status: blocked (shared request for plan §3.3 error codes)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#24-holding-a-permission-request-78--b-28-b-32-b-33
- Read: `crates/core-harness/contract.yaml` (whole: sessions, terminate, close_all, `change_mode`'s command pattern); design §7.8; approval authority §24.1 (`docs/superpowers/specs/2026-10-08-approval-authority-design.md`); harness controls §12.2 (`docs/superpowers/specs/2026-09-24-harness-controls-design.md`).
- Touch: `crates/core-harness/src/permissions.rs` (new: `answer_permission`, `waiting_permissions`); `crates/core-harness/src/store/permission.rs` (new: the command record) and `crates/core-harness/src/store/mod.rs`; `crates/core-harness/src/{lib.rs,model.rs,error.rs,live.rs,sessions.rs,access.rs}`; `crates/core-harness/contract.yaml`; `crates/shadows-core/src/error.rs` (`ErrorCode` `PermissionNotFound`, `PermissionSettled`, `PermissionOptionNotOffered`); `crates/shadows-core/src/lib.rs` (re-exports); `crates/shadows-http/src/failure/{other.rs,cases.rs}` (404, 409, 422); `crates/shadows-core/tests/aylul_session.rs` (mod) and `crates/shadows-core/tests/aylul_session/permissions.rs` (new); `crates/fake-acp/src/prompts.rs` (only for a second-request prompt); `docs/superpowers/specs/2026-09-24-harness-controls-design.md` (§12.2 amended in place: AYLUL asks, the Planner refuses); `api/openapi.json`.
- Needs: B-28, B-31 (same contract), B-32; the shared request.
- Done when: `cargo test -p shadows-core --test aylul_session permissions::` passes `an_aylul_request_waits_and_is_listed`, `answering_runs_the_tool_and_records_by_you`, `a_rejected_request_ends_rejected_and_the_turn_goes_on`, `an_option_not_offered_is_refused`, `a_second_answer_is_settled`, `an_equal_replay_answers_the_first_result`, `cancelling_while_asked_ends_refused_and_publishes`, `closing_the_session_while_asked_ends_refused`, `waiting_is_listed_oldest_first_across_agents`, `permissions_changed_reaches_the_aylul_project_stream`; `cargo test -p shadows --test planner_turn` (the Planner still refuses at once); `cargo test -p shadows-http failure` passes; openapi regenerated and `cargo test -p shadows --test openapi`.

### B-34 · HTTP: list and answer permission requests
- Status: blocked (shared request for plan §3.3)
- Plan: docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md#33-permission-requests
- Read: `crates/shadows-http/contract.yaml`; `crates/core-harness/contract.yaml` (`answer_permission`, `waiting_permissions`); plan §3.3.
- Touch: `crates/shadows-http/src/harness.rs` (`GET /api/aylul/permissions`, `POST /api/threads/{id}/session/permissions/{request_id}`); `crates/shadows-http/src/lib.rs`; `crates/shadows-http/src/openapi.rs`; `crates/shadows-http/src/sse.rs` (doc text of the project stream only); `crates/shadows-http/contract.yaml`; `crates/shadows/tests/permission_routes.rs` (new); `api/openapi.json`.
- Needs: B-33; the shared request.
- Done when: `cargo test -p shadows --test permission_routes` passes `waiting_requests_are_listed_oldest_first`, `answering_over_http_answers_the_adapter`, `unknown_settled_and_not_offered_answer_their_codes`, `a_replayed_answer_answers_the_first_result`, `no_aylul_lists_nothing`; openapi regenerated and `cargo test -p shadows --test openapi`.

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
