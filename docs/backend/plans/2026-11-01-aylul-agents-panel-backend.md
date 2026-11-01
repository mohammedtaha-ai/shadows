# AYLUL agents and panel: the Core side

Date: 2026-11-01. Branch `feature/aylul-agents-panel`.
Design (accepted by Mohammed 2026-11-01):
[AYLUL agents and panel](../../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md),
§7.1, §7.2, §7.3, §7.4 and §7.8. That file owns the decisions; this plan owns only
how Core builds them, in which order, and the HTTP shapes the web and the desktop
build against. Tasks: B-24 to B-34 in [stages.md](../stages.md).

## 1. Owners (from `AppCore`)

| Change | Owner (`app.rs`) | Contract |
|---|---|---|
| `description`, `instructions`, `outfit`; the instructions template; the one-time home import | `Agents` | `crates/core-agents/contract.yaml` |
| The `mcp/<id>` scope and its name rule | `Secrets` | `crates/core-secrets/contract.yaml` |
| MCP items (third kind), bundled items, declined names | `Extensions` | `crates/core-extensions/contract.yaml` |
| The URL rule an MCP item reuses (`https://`, `http://` on localhost) | `Providers` (rule stays there, made public) | `crates/core-providers/contract.yaml` |
| Writing `CLAUDE.md` at opening; MCP servers in the ACP setup; holding, listing and answering permission requests | `Harness` (with `shadows-agent`, its ACP connection) | `crates/core-harness/contract.yaml` |
| `PermissionAsked` / `PermissionAnswered` entries and the `PermissionsChanged` journal event | `Turns` (entries through `Threads::append_entry_in`) | `crates/core-turns/contract.yaml` |
| The two new entry kinds | `Threads` | `crates/core-threads/contract.yaml` |
| `PermissionsChanged` on the project stream | `Events` | `crates/core-events/contract.yaml` |
| Routes | `shadows-http` (translate only) | `crates/shadows-http/contract.yaml` |

No new service. New dependency edges: `Extensions → Secrets` (design §7.2) and
`Extensions → Providers` (for the URL rule only; see §6, point 2). The
architecture ratchet already allows both for `core-extensions`.

## 2. How each part is built

### 2.1 Agents (§7.1, §7.3) — B-25

- Columns (migration 0039, B-24): `description TEXT NOT NULL DEFAULT ''`,
  `instructions TEXT` (NULL means "home file not imported yet"),
  `outfit TEXT NOT NULL DEFAULT 'default'`.
- `AgentInput` gains `description`, `instructions`, `outfit`, each optional.
  Create: absent is `''`, `''`, `default`. Update: absent keeps the stored value,
  filled in before fingerprinting (the colour rule).
- Rules, all in `Agents`: description trimmed, one line, at most 200 characters;
  instructions at most 32 KiB, no NUL; **instructions equal to the bundled
  template are stored as `''`** (so a form that sends the prefilled template back
  keeps following the template); outfit uses the colour's id rule (one private
  function for both, not a copy).
- `Agents::instructions_template()` answers the bundled
  `assets/aylul-profile/CLAUDE.md`. `Agents::home_instructions(slug)` is the one
  rule for what the home's `CLAUDE.md` says: the stored text, the template for
  `''`, and `None` for NULL (never imported: the opening leaves the file alone).
- `Agents::import_home_instructions()` runs once in `AppCore::start`, after the
  owners are built and before anything is served: for each live agent with NULL
  instructions it reads `<home>/CLAUDE.md`; absent or equal to the template is
  `''`; valid UTF-8 without NUL is stored as read (any size; the 32 KiB limit is
  an input rule); anything else stays NULL and logs
  `agent.instructions_import_skipped` with the slug. A later start finds no NULL
  row and reads nothing. The design said "the migration reads"; SQL cannot read a
  home file, so this start step does it, once per agent.
- `seed_home` stops writing `CLAUDE.md` (it keeps `settings.json`): the opening
  is the one writer of that file.

### 2.2 Secrets (§7.2) — B-26

- `Secrets::mcp_scope(id)` is the one place that names `mcp/{id}`.
- `Secrets::mcp_value_write(command_id, id, name, value)` seals a value under that
  scope with the MCP name rule: 1..=128 characters of `A-Z a-z 0-9 - _`. The
  reserved environment names stay refused in every scope. The value rules
  (non-empty, no NUL, size) are the existing ones.
- Values are resolved with the existing `resolve_env(scope)`; removed with
  `remove_in` and `remove_scope_in`.

### 2.3 The ACP session's MCP list (§7.2 delivery) — B-27

- `SessionSetup.mcp: Option<McpServerSpec>` becomes `mcp: Vec<McpServerSpec>`,
  and `McpServerSpec` becomes an enum:
  `Http { name, url, headers: Vec<(String, String)> }` and
  `Stdio { name, command, args: Vec<String>, env: Vec<(String, String)> }`.
  `Debug` prints names and URL/command only, never a header or env value.
- The Planner's grant is `Http { name: "shadows", url, headers: [("Authorization",
  "Bearer <token>")] }`, built where it is today (`setup.rs`); its existing tests
  hold it unchanged.
- `fake-acp` records every server it was given (name, transport, URL or command,
  header and env **names**, and a hash of each value), and keeps finding the
  `shadows` server for its `mcp` prompt.

### 2.4 Holding a permission request (§7.8) — B-28, B-32, B-33

- `shadows-agent` (B-28): `Connection::open` takes a `PermissionPolicy`:
  `Refuse` (today's answer, for the Planner) or `Ask`. Under `Ask` the request
  handler does **not** wait inside the dispatch loop: it stores the responder in
  the connection's held map under a new request id (UUID v4), emits
  `HarnessEvent::PermissionAsked { request, title, options, asked_at }`, and
  returns, so later notifications keep flowing.
  - `Connection::answer_permission(request, option)` removes the held entry
    (one winner), refuses an option the adapter did not offer, emits
    `PermissionAnswered { request, option, kind, name }` **before** it responds to
    the adapter, then responds `Selected(option)`.
  - `Connection::cancel(session)` first ends that session's held requests:
    `PermissionRefused { title, request: Some(id), reason: Some("cancelled") }`
    and an ACP `Cancelled` outcome, then sends `session/cancel`.
  - `Connection::refuse_held(reason)` does the same for every held request; the
    Harness calls it with `closed` before a terminate and `stopped` in
    `close_all`.
  - `Connection::held_permissions()` lists the waiting ones.
  - `PermissionRefused` gains `request: Option<String>` and
    `reason: Option<String>`; the Planner's refusal sends `None, None` and keeps
    today's entry exactly.
  - Settled request ids are remembered per connection, so a late answer is
    "settled", not "unknown".
- `Turns` (B-32) turns the events into entries through its `Collector` and
  `persist`, as it does every other durable entry:
  - `PermissionAsked`: author the turn's agent author, body the title, card
    `{request_id, options}`.
  - `PermissionAnswered`: author the person (the author of `UserMessage`
    entries), body the option's name, card
    `{request_id, option_id, kind, by: {agent: "human", principal: "user:local"}}`
    (approval authority §24.1).
  - `PermissionRefused` for a held request: author system, body the title, card
    `{request_id, reason}`. The Planner's refusal keeps `card: null`.
  - An ask still open when the collector finishes is closed as refused with
    reason `ended`, so every asked entry has an end in the same turn.
  - In the same write as each of these three entries (only when a `request_id`
    exists), Turns appends the journal event `PermissionsChanged` with the
    thread's `project_id`, `thread_id`, and payload
    `{request_id, state: "waiting" | "answered" | "refused"}`.
  - `Events` allowlists `PermissionsChanged` on the project stream. The panel
    and the web watch the hidden AYLUL project (`AgentWorkspace.project_id`) and
    refetch the waiting list on each frame. The Harness changes its live list
    before it emits the event, so the refetch already sees the change.
- The report's `PermissionOption` is normalized once in `shadows-agent`, so entry
  cards and the waiting list use `option_id` rather than ACP's `optionId`.
  Turns reads the project through `Threads::get_in` inside the entry transaction;
  the project notification and entry therefore commit together without another
  retained project field in `TurnWatch`. AYLUL entry authors use the opened
  session's agent identity; Planner entries retain their Planner author.
- `Harness` (B-33):
  - Chooses the policy at opening: `Ask` for an `aylul` thread, `Refuse`
    otherwise. Harness controls §12.2 is amended in place to say so.
  - `Harness::answer_permission(command_id, thread, request, option)
    -> Result<PermissionAnswer, HarnessError>`: replay first (command
    `thread.permission_answer`, schema 1, fingerprint
    `{thread_id, request_id, option_id}`, Thread scope, its outcome is the
    `PermissionAnswer`); then the thread's live session; then the request
    (unknown is `PermissionNotFound`, settled is `PermissionSettled`); then the
    option (`PermissionOptionNotOffered`); then the command is recorded and the
    connection answers. A failed write answers nothing and the request stays
    waiting.
  - `Harness::waiting_permissions() -> Vec<WaitingPermission>` over every live
    `aylul` session, oldest `asked_at` first, request id as tie break.
  - Closing or terminating a session, and `close_all`, call `refuse_held` first.
  - A prepared adapter answer holds a settlement gate while the command commits.
    Cancellation and closure wait on that gate; dropping a failed preparation
    leaves the request waiting. This prevents an answer from racing cancellation
    between the durable record and the adapter response, without blocking dispatch.
  - A held request trips no timeout: the idle close already skips a held slot,
    and nothing else may end it.
  - A prompt's result or disconnect settles any remaining requests of that session
    before returning to Turns: `ended` on a result, `closed` on disconnect. Turns
    then drains their reports before finishing entries; requests from other sessions
    on the same ACP connection are left to their own prompts.

### 2.5 Extensions: MCP items (§7.2) — B-29

- Storage (B-24): MCP items get their own tables, `aylul_mcp_server` (`id`,
  `name UNIQUE`, `description`, `config_json`, `created_at`) and
  `aylul_agent_mcp` (`agent_slug`, `mcp_id ON DELETE CASCADE`, `enabled`).
  The existing `aylul_extension.kind` has a `CHECK (kind IN ('plugin','skill'))`;
  widening it means rebuilding a table that another table references with
  `ON DELETE CASCADE`, where a careless drop deletes every switch. Separate
  tables avoid that; the rules stay the same and live in the same service.
- `ExtensionKind` gains `Mcp` (`"mcp"`). The existing delete and switch routes
  take `mcp` as their `{kind}`; the folder import refuses it.
- `Extensions::add_mcp(command_id, McpInput)` and
  `Extensions::update_mcp(command_id, id, McpInput)`; one write each: replay,
  the item row, and each value through `Secrets::put_in` (command id
  `<command_id>:<name>`). The item's fingerprint holds the configuration and the
  keyed fingerprint of each value sent, never a value.
  `EnvWrite::value_mac()` exposes that prepared write's existing keyed fingerprint
  by reference; sealing and secret material stay private to Secrets.
  - Item names: `^[a-z][a-z0-9_-]{0,63}$`, not `shadows` (the Planner's server),
    unique among MCP items.
  - HTTP URL: `Providers::validate_url`, made public; Extensions answers its own
    words ("invalid MCP URL").
  - A name appears once across an item's headers and env. On create every name
    needs a value. On edit an absent value keeps the stored one, a name dropped
    from the list loses its value, a new name without a value is refused.
  - The id never changes on edit, so every agent keeps its switch.
- `delete_extension(.., Mcp, id)`: the item, every switch and the `mcp/<id>`
  scope in one write. `remove_agent_in` also removes the agent's MCP switches.
- `Extensions::agent_mcp_servers(slug) -> Vec<McpServerConfig>`: the agent's
  switched-on items with their values resolved, for the opening only. `Debug`
  redacts values.

### 2.6 Extensions: bundled items (§7.4) — B-30

- `crates/core-extensions/assets/library/skills/<name>/` and
  `.../plugins/<name>/`, the layout an import accepts. This branch ships one
  skill, `aylul-hello`, whose `SKILL.md` describes what the bundled library is.
- A `build.rs` in `core-extensions` lists the files and embeds them with
  `include_bytes!` (our code; no new dependency). At start
  (`AppCore::start`, after owners) `Extensions::import_bundled()` writes each item
  to a staging folder under the home root, beside the library (normal imports
  refuse sources inside the destination library), and imports it through the same
  path as `add_extension`, under command id `bundled:<kind>:<name>:<digest>`,
  unless the name is already in the library or is declined. Without AYLUL
  configuration it does nothing.
  Staging is a fresh owned temporary directory under the canonical home root;
  files are created exclusively. Existing predictable staging paths are never
  reused. The existing workspace `tempfile` library supplies this infrastructure.
- Column `aylul_extension.bundled` (B-24); `Item` answers it.
- Deleting a bundled item records `(kind, name)` in `aylul_extension_declined`
  in the same write.
- A bundled item starts switched off for every agent (no switch row).

### 2.7 The opening (§7.1, §7.2) — B-31

- In the AYLUL opening, under the home lock and the same library guard as the
  file projection: write `<home>/CLAUDE.md` atomically from
  `Agents::home_instructions` (nothing when it is `None`), then put
  `Extensions::agent_mcp_servers` into `SessionSetup.mcp`. An AYLUL setup never
  holds the `shadows` grant.
  `Extensions::opening_snapshot` retains the library guard with its agent library
  and resolved servers through home projection. `agent_mcp_servers` uses the same
  resolver; the Harness does not reacquire the library lock while holding it.

## 3. Shapes the frontend relies on

All bodies are JSON. `command_id` is a client UUID, as everywhere. Existing error
bodies (`ErrorBody`: `code`, `message`) are unchanged.

### 3.1 Agents

`Agent` (in `GET /api/aylul/agents` → `AgentWorkspace.agents`, and the answers of
`POST /api/aylul/agents` and `PUT /api/aylul/agents/{slug}`) gains:

```json
{ "description": "Finds and checks sources",
  "instructions": "",
  "outfit": "headphones" }
```

- `instructions` is `""` when the agent follows the template (including a text
  equal to the template), and also for an agent whose home file could not be
  imported.
- `outfit` matches `^[a-z][a-z0-9-]{0,31}$`; any well-formed id is kept.

`AgentInput` (the body of create and update, beside `command_id`) gains three
optional fields: `description`, `instructions`, `outfit`. Absent on update keeps
the stored value. Errors: 400 `PROFILE_INVALID` (too long, NUL, two lines, bad
outfit id).

`GET /api/aylul/agent-template` → 200 `{ "instructions": "<template text>" }`.
The new-agent form prefills from it. (Not under `/agents/`, where `template` could
be a slug.)

### 3.2 MCP tools

`ExtensionKind` gains `"mcp"`. `Item` gains `"bundled": bool`.

`McpTransport` (tagged by `type`):

```json
{ "type": "http", "url": "https://example.com/mcp",
  "headers": [ { "name": "Authorization", "set": true } ] }
{ "type": "stdio", "command": "npx", "args": ["-y", "some-server"],
  "env": [ { "name": "API_KEY", "set": true } ] }
```

`McpItem`: `{ "id", "name", "description", "transport": McpTransport }`. Values are
never answered; `set` is always `true` for a listed name.

`GET /api/aylul/extensions` → `ExtensionLibrary` gains `"mcp": [McpItem]`.
`GET /api/aylul/agents/{slug}/extensions` → `AgentExtensions` gains
`"mcp": [McpItem & { "enabled": bool }]`.

`POST /api/aylul/extensions/mcp` → 201 `McpItem`, body:

```json
{ "command_id": "…", "name": "github", "description": "Issues and PRs",
  "transport": { "type": "http", "url": "https://…",
                 "headers": [ { "name": "Authorization", "value": "Bearer x" } ] } }
```

(stdio: `"transport": { "type": "stdio", "command": "…", "args": […],
"env": [ { "name": "API_KEY", "value": "…" } ] }`; `description` optional.)

`PUT /api/aylul/extensions/mcp/{id}` → 200 `McpItem`, same body; a pair's
`value` may be absent to keep the stored one.

Switch and delete use the existing routes with `{kind}` = `mcp`:
`PUT /api/aylul/agents/{slug}/extensions/mcp/{id}` (`{command_id, enabled}` →
`AgentItem`) and `DELETE /api/aylul/extensions/mcp/{id}?command_id=…` → 204.

`POST /api/aylul/extensions` with `"kind": "mcp"` → 400 `PROFILE_INVALID`.

Errors: 400 `PROFILE_INVALID` (name, URL, duplicate name, new name without
value), 400 `SECRET_NAME_INVALID` / `SECRET_VALUE_INVALID`, 409
`SECRET_NAME_RESERVED`, 409 `EXTENSION_NAME_CONFLICT`, 404
`EXTENSION_NOT_FOUND`, 409 `COMMAND_CONFLICT`, 503 `SECRETS_UNAVAILABLE`.

### 3.3 Permission requests

`PermissionOption`: `{ "option_id": "allow_once", "name": "Allow", "kind":
"allow_once" | "allow_always" | "reject_once" | "reject_always" }` (the adapter's
own ids and labels).

`GET /api/aylul/permissions` → 200 `[WaitingPermission]`, oldest first; `[]`
when AYLUL is not configured:

```json
{ "thread_id": "…", "agent": "researcher", "request_id": "…uuid…",
  "title": "Run curl https://…", "options": [PermissionOption],
  "asked_at": "2026-11-01T10:00:00Z" }
```

`POST /api/threads/{id}/session/permissions/{request_id}` with
`{ "command_id": "…", "option_id": "allow_once" }` → 200:

```json
{ "thread_id": "…", "request_id": "…", "option_id": "allow_once",
  "kind": "allow_once", "answered_at": "…" }
```

Errors: 404 `PERMISSION_NOT_FOUND` (no such request on this thread, including
after a daemon restart), 409 `PERMISSION_SETTLED` (already answered or ended),
422 `PERMISSION_OPTION_NOT_OFFERED`, 409 `COMMAND_CONFLICT`. An equal replay
answers the first result.

Entries (`GET /api/threads/{id}/entries`, `ThreadEntryKind`): two new kinds.

| kind | author | body | card |
|---|---|---|---|
| `PermissionAsked` | the agent | the tool's title | `{ "request_id", "options": [PermissionOption] }` |
| `PermissionAnswered` | the person | the option's name | `{ "request_id", "option_id", "kind", "by": { "agent": "human", "principal": "user:local" } }` |
| `PermissionRefused` (held request) | system | the tool's title | `{ "request_id", "reason": "cancelled" \| "closed" \| "stopped" \| "ended" }` |
| `PermissionRefused` (Planner) | system | the tool's title | `null` (unchanged) |

Live updates: `GET /api/projects/{project_id}/events` on the AYLUL project
(`AgentWorkspace.project_id`) sends a `durable` frame with
`"kind": "PermissionsChanged"`, `thread_id` set and
`"payload": { "request_id": "…", "state": "waiting" | "answered" | "refused" }`.
Refetch `GET /api/aylul/permissions` on it. The thread stream keeps sending
`ThreadEntryAppended` for the entries, as for every entry.

## 4. Order

```text
B-24 ──► B-25 ∥ B-26 ∥ B-27 ∥ B-28 ──► B-29 ──► B-30 ∥ B-31 ──► B-32 ──► B-33 ──► B-34
```

- Tasks joined by `∥` touch disjoint files and may run at once. Every task that
  changes `api/openapi.json`, `crates/shadows-core/src/app.rs`,
  `crates/shadows-core/src/error.rs` or `crates/shadows-http/src/lib.rs` runs
  alone among those: B-25, B-29, B-30, B-32, B-33 (new `ErrorCode` values),
  B-34 change the API document in that order.
- B-25, B-29, B-30, B-32, B-33 and B-34 change what the frontend sees. They wait for the
  shared request that accepts §3 (AGENTS.md rule 4).

## 5. Done for the branch

The full gate once, then a person runs `shadows serve` with the desktop and walks
the design's acceptance §10 (1–9), reading the logs: no MCP value, header or env
value in any log line (`acp_log_safety` covers the ACP traffic).

## 6. Design points to rule on

1. **An agent can answer its own request.** The answer route is plain HTTP on
   localhost; the guard lets a request without `Origin` through (curl). An AYLUL
   agent that once got "allow always" for a shell command, or holds an MCP tool
   that can POST, can approve its next request, and the entry says "by you".
   Approval authority §24.3 forbids exactly this for the Planner. The plan builds
   what the design says; closing it needs a ruling (for example a per-daemon
   client token that the web and desktop hold and an agent's home does not).
2. **The URL rule needs `Extensions → Providers`.** §7.2 names only the
   `Secrets` edge, but "as providers" is Providers' `validate_url`; a copy breaks
   "one rule, one owner". The plan adds the edge and makes that function public.
3. **"The migration reads each home file"** cannot be SQL; it is a one-time start
   step keyed on NULL (§2.1). Same outcome.
4. **MCP items in their own tables** rather than a third value of
   `aylul_extension.kind` (§2.5), to avoid rebuilding a table under a cascading
   foreign key. Same rules and owner.
