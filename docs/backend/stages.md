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

## Tasks

### B-38 · Extensions lists an MCP item's tools and keeps per-agent rules
- Status: todo
- Spec: [AYLUL agents and panel §7.2.1](../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md) (per-tool permissions)
- Read: `crates/shadows-core/src/app.rs` (`Extensions`); `crates/core-extensions/contract.yaml`; `crates/core-extensions/src/{lib,mcp,model}.rs` and `store/`; how `crates/shadows/Cargo.toml` already uses `rmcp` as a client in tests.
- Touch: `crates/core-extensions/` (src, store, a migration in the place the crate's migrations live, `Cargo.toml` for `rmcp` client features, contract, tests); the workspace `Cargo.toml` only if `rmcp` features are set there; `docs/codebase/README.md` rows for new files.
- Needs: none.
- Done when: `Extensions::item_tools(slug, item)` returns the server's tools (name, description) over `http` and `stdio` with the stored secret values, with a 10-second limit; `Extensions::tool_rules(slug, item)` and `set_tool_rules(slug, item, rules)` read and replace an agent's rules (`allow`, `ask`, `deny`; `ask` stores nothing); switching the item off or deleting it removes its rules in the same write; the resolved snapshot the harness reads carries each switched-on item's rules. `cargo test -p core-extensions` passes, with `tools_are_listed_from_an_http_server`, `tools_are_listed_from_a_stdio_server`, `a_silent_server_times_out`, `rules_are_per_agent`, `ask_stores_no_row`, `switching_off_removes_the_rules`.

### B-39 · Rules reach the agent's settings.json; HTTP routes
- Status: todo
- Spec: [AYLUL agents and panel §7.2.1](../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md)
- Read: `crates/core-harness/contract.yaml`; `crates/core-harness/src/extension_home.rs` (`project`) and `profile_home.rs`; the name each MCP item is delivered under in ACP `mcpServers`; `crates/shadows-http/src/contract.yaml`; the agent extension routes in `crates/shadows-http/src/`.
- Touch: `crates/core-harness/src/` (projection), its contract and tests; `crates/shadows-core/tests/aylul_session/` (home file tests); `crates/shadows-http/src/` (routes, openapi) and its contract; `crates/shadows/tests/` (a new `tool_rules_routes.rs`; `openapi.rs` route list); `api/openapi.json`; `web/src/api/` (regenerated only).
- Needs: B-38.
- Done when: at opening the harness writes `allow` and `deny` rules as `mcp__<server>__<tool>` into `permissions.allow` / `permissions.deny`, keeps rules it did not write, and removes the ones it wrote for an item now off; `GET /api/aylul/agents/{slug}/extensions/{item}/tools` (tools plus each one's rule, and rules for missing tools) and `PUT /api/aylul/agents/{slug}/extensions/{item}/tool-rules` work. First probe: the exact `<server>` spelling Claude Code uses for an ACP-delivered server, and whether a deny rule holds in `bypassPermissions`; report both. Tests: `opening_writes_allow_and_deny_rules`, `rules_of_an_item_switched_off_are_removed`, `foreign_rules_are_kept` and the route tests pass; `npm run gen:api` and `cargo test -p shadows --test openapi` pass.

None else open. B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at
`f1459dcc`.
