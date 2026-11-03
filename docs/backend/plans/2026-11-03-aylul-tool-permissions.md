# AYLUL per-tool MCP permissions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each tool of a switched-on MCP item is allowed, asking or blocked per AYLUL agent,
and Claude Code enforces it through the agent's `settings.json`.

**Architecture:** `Extensions` (owner of MCP items and per-agent switches) gains a rules
table, an MCP client that lists an item's tools, and puts each switched-on item's rules in
the opening snapshot. `Harness` writes the rules into the home's `settings.json` beside the
existing extension projection. HTTP translates two methods; the web form shows the tools
under each switched-on MCP item.

**Tech Stack:** Rust, SQLx/SQLite, `rmcp` 3.4.1 (client), axum/utoipa, React + TanStack Query.

**Spec:** `docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md` §7.2.1. Tasks:
B-38, B-39 in `docs/backend/stages.md`; F-80 in `docs/frontend/stages.md`.

## Global Constraints

- CLAUDE.md "Code structure" and "Rules": start at `AppCore`; the rule lives in
  `Extensions`; HTTP and the harness only translate and deliver.
- A mutating command carries `command_id`; replay with an equal fingerprint returns the
  first answer, a different one is `CommandConflict` (as `set_agent_extension`).
- Every `crates/**/*.rs` line at or under 100 characters. No free `pub fn`.
- Processes start only through `shadows-process` (it owns `tokio::process` and
  `process-wrap`): do not enable rmcp's `transport-child-process`.
- Secret values never appear in an answer, a log line, an error string or `Debug`.
- Rule values: `allow`, `ask`, `deny`. `ask` is never stored or written.
- Tool names: 1..=128 characters of `A-Z a-z 0-9 _ -` (MCP's tool-name set); others refused.
- Listing limit: 10 seconds for connect + `initialize` + `tools/list` together.
- Web: tokens only (no hex, `rgba`, raw palette classes, `text-[1`); `schema.d.ts` is
  regenerated, never edited.
- Focused tests only per task; the full gate runs in CI after the branch review.

## Review Focus

1. **A stdio server that hangs or never exits.** Listing must end at 10 s and the child
   process tree must be terminated, not left running. Test in Task 1.
2. **A secret value inside an error.** A server answering 401 or a command failing must not
   echo the header or env value into `ExtensionsError` text. Test in Task 1.
3. **Hand-written rules in `settings.json`.** Mohammed or Claude Code itself may add
   `permissions.allow` entries; projection removes only what it wrote. Test in Task 3.
4. **An item renamed after rules were written.** The old `mcp__<old>__…` strings must leave
   `settings.json` at the next opening (the manifest holds what was written, not the name).
   Test in Task 3.
5. **Rules for tools the server dropped.** They stay stored and visible as missing; they
   are still written (harmless) until cleared. Test in Task 2 (route) and Task 4 (form).

---

### Task 1 (B-38, part 1): rules table and the two rule methods

**Files:**
- Create: `crates/core-db/migrations/0040_aylul_mcp_tool_rules.sql`
- Create: `crates/core-extensions/src/tool_rules.rs` (rule commands and validation)
- Modify: `crates/core-extensions/src/model.rs` (types below), `src/lib.rs` (module, exports),
  `src/store/mod.rs` (SQL; `remove_agent_in` also clears rules),
  `crates/core-extensions/contract.yaml`, `docs/codebase/README.md` (row for `tool_rules.rs`)
- Test: `crates/core-extensions/tests/` (the crate's existing integration test file for MCP)

**Interfaces:**
- Produces (in `model.rs`, `Serialize`/`Deserialize`/`ToSchema`):
  - `enum ToolRule { Allow, Ask, Deny }` (`snake_case`)
  - `struct ToolRuleEntry { tool: String, rule: ToolRule }`
- Produces (on `Extensions`):
  - `async fn tool_rules(&self, slug: &str, id: &str) -> Result<Vec<ToolRuleEntry>, ExtensionsError>`
    (stored rules ordered by tool name; never contains `Ask`)
  - `async fn set_tool_rules(&self, command_id: String, slug: &str, id: &str,
    rules: Vec<ToolRuleEntry>) -> Result<Vec<ToolRuleEntry>, ExtensionsError>`
    (replaces the agent's whole rule set for the item; `Ask` entries are dropped; command
    kind `extension.tool_rules`, fingerprint `{slug,id,rules}` with rules sorted by tool)

- [ ] **Step 1: Migration.** Table `aylul_agent_mcp_tool_rule(agent_slug TEXT NOT NULL
  REFERENCES aylul_agent(slug), mcp_id TEXT NOT NULL REFERENCES aylul_mcp_server(id) ON
  DELETE CASCADE, tool TEXT NOT NULL, rule TEXT NOT NULL CHECK (rule IN ('allow','deny')),
  PRIMARY KEY (agent_slug, mcp_id, tool))`.
- [ ] **Step 2: Failing tests**, through the public API:
  - `rules_are_per_agent`: set `[{tool:"read", rule:Allow}]` for agent A on item X; agent B's
    `tool_rules` on X is `[]`; A's is that one entry.
  - `ask_stores_no_row`: set `[{read,Allow},{write,Ask}]` → `tool_rules` is `[{read,Allow}]`.
  - `rules_survive_switching_off`: set a rule, `set_agent_extension(.., false)`, then `true`;
    the rule is still there.
  - `deleting_the_item_or_agent_removes_rules`: after `delete_extension(Mcp, X)` the table
    has no row for X; after the harness's agent delete path (`remove_agent_in`) none for A.
  - `tool_rule_input_is_checked`: a tool name with a space, an empty one, one of 129
    characters, and a duplicate tool are `Invalid`; an unknown item is `NotFound`; a deleted
    agent refuses (as `toggle`, via `Agents::require_live_in`).
  - `tool_rules_replay_and_conflict`: same `command_id` and rules → same answer; same id,
    other rules → `CommandConflict`.
- [ ] **Step 3: Run** `cargo test -p core-extensions` → the new tests fail.
- [ ] **Step 4: Implement** in `tool_rules.rs` (validation, command) and `store/mod.rs`
  (one `write_txn_as`: classify, `require_live_in`, item exists, delete the agent's rows
  for the item, insert the non-`ask` ones, record the answer). Hold `extension_guard`.
- [ ] **Step 5: Run** `cargo test -p core-extensions` → all pass.
- [ ] **Step 6: Contract.** Add the two functions, the table to the store note, and the
  rules `Per-agent tool rules; ask stores nothing; rules survive the switch, go with the
  item or agent` with the test names. Commit `feat(core-extensions): per-agent MCP tool rules (B-38)`.

### Task 2 (B-38, part 2): list an item's tools; rules in the opening snapshot

**Files:**
- Create: `crates/core-extensions/src/tool_list.rs` (the MCP client)
- Modify: `crates/core-extensions/Cargo.toml` (`rmcp` with features `client`,
  `transport-streamable-http-client-reqwest`, `transport-async-rw`; `shadows-process`
  moves from dev to normal dependencies), `src/mcp.rs` (`McpServerConfig` carries rules),
  `src/model.rs`, `src/lib.rs`, contract, `docs/codebase/README.md`
- Test: same test file; a small stdio MCP server fixture (reuse `fake-acp`'s rmcp server
  pattern or a test binary) and an in-process streamable-HTTP server built with `rmcp`'s
  server features in the test

**Interfaces:**
- Consumes: Task 1's `ToolRuleEntry`, `tool_rules`.
- Produces:
  - `struct McpTool { name: String, description: String }` (model, `ToSchema`)
  - `struct AgentTools { tools: Vec<McpTool>, list_error: Option<String>,
    rules: Vec<ToolRuleEntry> }` (model, `ToSchema`). `list_error` is a fixed, value-free
    sentence: `"the server did not answer in 10 seconds"`, `"the server refused the
    connection"`, `"the server's answer was not MCP"`.
  - `async fn agent_tools(&self, slug: &str, id: &str) -> Result<AgentTools, ExtensionsError>`
    (unknown item `NotFound`; a listing failure is `list_error`, not an `Err`)
  - `McpServerConfig::{Http, Stdio}` gain `rules: Vec<ToolRuleEntry>`; `opening_snapshot`
    fills them for each switched-on item.

- [ ] **Step 1: Failing tests:**
  - `tools_are_listed_from_an_http_server` (server requires a header; the item's stored
    header value reaches it; answer lists two tools with descriptions),
  - `tools_are_listed_from_a_stdio_server` (the stored env value reaches the child),
  - `a_silent_server_times_out` (stdio child that never answers: `list_error` is the
    timeout sentence within 12 s, and the child is no longer running),
  - `a_refused_server_leaks_no_value` (HTTP 401: `list_error` set, and neither the error nor
    `format!("{:?}", tools)` contains the header value),
  - `opening_snapshot_carries_rules_of_switched_on_items` (rules of a switched-off item are
    absent; of a switched-on one present).
- [ ] **Step 2: Run** `cargo test -p core-extensions` → fail.
- [ ] **Step 3: Implement** `tool_list.rs`: resolve values as `resolve_mcp_servers` does;
  HTTP via rmcp's reqwest streamable client with the headers; stdio via
  `shadows_process::spawn` + `take_stdio`, handing `(stdout, stdin)` to rmcp, and
  `terminate_tree` on every path; wrap the whole exchange in `tokio::time::timeout(10 s)`;
  page through `tools/list` cursors.
- [ ] **Step 4: Run** → pass. **Step 5:** contract (functions, rule `Listing is bounded,
  value-free and leaves no child process`), commit `feat(core-extensions): list an MCP
  item's tools (B-38)`.

### Task 3 (B-39, part 1): write the rules into `settings.json` at opening

**Files:**
- Create: `crates/core-harness/src/tool_rules_home.rs`
- Modify: `crates/core-harness/src/extension_home.rs` (`write_extensions` calls it after
  `project`, under the same home lock), `src/lib.rs`, `crates/core-harness/contract.yaml`,
  `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/aylul_session/homes.rs`

**Interfaces:**
- Consumes: `McpServerConfig` with `rules` (Task 2).
- Produces: `fn write_tool_rules(home: &Path, servers: &[McpServerConfig]) -> Result<(), HarnessError>`
  (private to the crate, an associated fn on `Homes`). Manifest `.aylul-tool-rules.json`
  `{ "allow": [..], "deny": [..] }` holds exactly the strings it wrote last time.

- [ ] **Step 1: Probe first** (report in the commit message and in spec §7.2.1's Modes
  bullet): with a real ACP session through `harness/claude`, the tool name Claude Code
  shows for an ACP-delivered server named `context7` (expected `mcp__context7__<tool>`),
  and whether a `permissions.deny` entry refuses the tool under `bypassPermissions`.
- [ ] **Step 2: Failing tests** in `homes.rs`:
  - `opening_writes_allow_and_deny_rules`: item `docs` switched on with `{read:Allow,
    exec:Deny}` → `settings.json` has `"mcp__docs__read"` in `permissions.allow` and
    `"mcp__docs__exec"` in `permissions.deny`.
  - `rules_of_an_item_switched_off_are_removed`: open, switch off, open → neither string.
  - `foreign_rules_are_kept`: a hand-written `"Bash(ls)"` in `permissions.allow` survives.
  - `a_renamed_item_leaves_no_old_rule`: rename `docs` → `docs2` via `update_mcp`, open →
    only `mcp__docs2__…` strings.
- [ ] **Step 3: Run** `cargo test -p shadows-core --test aylul_session homes` → fail.
- [ ] **Step 4: Implement**: read `settings.json`, remove the manifest's previous strings
  from both lists, add the current ones (sorted, no duplicates), drop an empty
  `permissions` object only if it was created here, write with the file's `atomic` helper,
  then the manifest. Name format from the probe.
- [ ] **Step 5: Run** → pass. **Step 6:** harness contract, commit
  `feat(core-harness): write MCP tool rules into the agent home (B-39)`.

### Task 4 (B-39, part 2): HTTP routes

**Files:**
- Modify: `crates/shadows-http/src/aylul_extensions.rs` (two handlers), `src/lib.rs`
  (routes), `src/openapi.rs`, `crates/shadows-http/src/contract.yaml`,
  `crates/shadows/tests/openapi.rs` (route list), `api/openapi.json`, `web/src/api/schema.d.ts`
- Test: `crates/shadows/tests/tool_rules_routes.rs` (new)

**Interfaces:**
- `GET /api/aylul/agents/{slug}/extensions/mcp/{id}/tools` → 200 `AgentTools`; 404 unknown
  agent or item. Calls `agent_tools`.
- `PUT /api/aylul/agents/{slug}/extensions/mcp/{id}/tool-rules` body
  `{ command_id: String, rules: Vec<ToolRuleEntry> }` → 200 `Vec<ToolRuleEntry>`; 400
  invalid; 404; 409 conflict. Calls `set_tool_rules`.

- [ ] **Step 1: Failing tests:** `tools_and_rules_are_listed`, `rules_are_replaced`,
  `an_unknown_item_is_404`, `a_bad_tool_name_is_400`, `a_replayed_put_conflicts_on_change`,
  `missing_tools_keep_their_rule` (a rule for a tool the server does not list comes back
  in `rules`).
- [ ] **Step 2: Run** `cargo test -p shadows --test tool_rules_routes` → fail.
- [ ] **Step 3: Implement** the two handlers (translate only).
- [ ] **Step 4:** `npm run gen:api` (from repo root as the README says), then
  `cargo test -p shadows --test tool_rules_routes` and `--test openapi` → pass.
- [ ] **Step 5:** contract, commit `feat(shadows-http): MCP tool rules routes (B-39)`.

### Task 5 (F-80): the tools under a switched-on MCP item

**Files:**
- Create: `web/src/app/agents/tool-rules.tsx`, `web/src/app/agents/tool-rules.test.tsx`
- Modify: `web/src/app/agents/agent-extensions.tsx` (`ExtensionSwitch` renders
  `<ToolRules slug id />` under an enabled `mcp` item), `web/src/api/extensions.ts`
  (`agentToolsQuery(slug, id)`, `setToolRules(slug, id, rules, commandId)`),
  `web/src/app/agents/contract.yaml`, `docs/frontend/codebase/README.md` if a row changes

**Interfaces:**
- Consumes: Task 4's routes through `web/src/api`; `attemptFor` for the command id, as
  `ExtensionSwitch` does.
- Produces: `export function ToolRules({ slug, id }: { slug: string; id: string })`.

- [ ] **Step 1: Failing tests** (`tool-rules.test.tsx`):
  - lists each tool with its description and three buttons named
    `Allow <tool>`, `Ask before <tool>`, `Block <tool>`, the current one `aria-pressed`;
  - clicking `Block exec` sends `PUT …/tool-rules` with `[{tool:"exec",rule:"deny"}]` plus
    the other stored rules;
  - `Set all` to `Allow` sends every listed tool as `allow`;
  - a rule whose tool is not listed shows under "No longer offered by the server" with a
    `Clear` button that sends the rules without it;
  - `list_error` shows the sentence, and stored rules are still shown and clearable.
- [ ] **Step 2: Run** `npm test -- tool-rules` → fail.
- [ ] **Step 3: Implement** with existing `Button` and tokens; the three buttons use
  lucide icons (`Check`, `Hand`, `Ban`) with visually hidden labels; pending state disables
  the row and shows `Saving…`; errors use `ErrorLine`.
- [ ] **Step 4: Run** `npm test -- tool-rules`, `npm run typecheck`, `npm run lint`,
  `npm run build` → pass.
- [ ] **Step 5:** agents contract, commit `feat(web): per-tool permissions in the agent form (F-80)`.
