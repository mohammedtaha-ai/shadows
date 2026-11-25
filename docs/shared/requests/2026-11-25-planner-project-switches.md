# What the Planner may use: a project's switches

- Status: accepted
- Asked by: Masada, 2026-11-25
- Screen: a new section of the project settings page
  (`web/src/app/project-settings/project-settings.tsx`), as §24.3 of
  `docs/core/specs/2026-10-08-approval-authority-design.md` ("The person may widen the
  Planner's reach, never its hands") and "What the person gets" in the backend plan
  `docs/backend/plans/2026-11-25-planner-project-switches.md` (B-234) describe it

## Need

**What the person does.** In a project's settings, the person opens the section
**"What the Planner may use"**. It lists AYLUL's extension library, skills and MCP servers
only, with no plugins and no plugins heading. Each item gets one switch for this project,
and one more switch reads **"Read the project's CLAUDE.md"**. Every switch starts off. The
person turns a switch on or off; it saves at once, and the switch shows Core's answer.

**What the section says.** One fixed line: the switches apply to the Planner on Claude
Code only. A skill or MCP server change reaches the next opened conversation session. The
CLAUDE.md switch reaches only a new conversation. An MCP item shows no tool rules: a
project has none (plan decision 3), so the agent form's `ToolRules` is not drawn here.
When the library has no skill and no server, the section shows "No skills or MCP servers
in the library." with the existing "Add to library" link, and the CLAUDE.md switch still
works. AYLUL not configured comes back as the same empty view (decision 8).

**What the screen gets and sends.** It reads the project's library view with each item's
`enabled`, and the CLAUDE.md switch from `Project.planner_reads_claude_md`. The project
comes from the existing `GET /api/projects` list the settings page already reads. It
sends one toggle per switch, with a `command_id` kept across retries of the same change
(`web/src/api/command-id.ts`, `attemptFor`), and writes the answer back into its cache:
the `AgentItem` into the item's list, and the `Project` into the projects list.

**Reuse in the web.** The switch list is the AYLUL agent form's extensions section,
`web/src/app/agents/agent-extensions.tsx` (`AgentExtensions` and its private
`ExtensionSwitch`). Its switch row (name, description, `role="switch"` checkbox,
optimistic state, one-at-a-time lock, `ErrorLine`) is bound today to an agent slug,
`agentExtensionsQuery`, `setAgentExtension` and `ToolRules`. The frontend task makes that row
holder-neutral, so the agent form and the new project section both draw it. The agent form
keeps `ToolRules` on its own side. The project's hooks are new functions in
`web/src/api/extensions.ts` (the library view and the item toggle) and
`web/src/api/projects.ts` (the CLAUDE.md toggle), each over the generated schema.

**What the plan does not give the screen.** One point, not a field request:
- The empty view cannot tell "AYLUL is not configured" from "the library is empty"
  (decision 8). The screen shows the same empty line for both. The "Add to library" link
  then opens a library that may not work without AYLUL. If Hamoud judges that the person
  must see "AYLUL is not configured" here, that needs a signal in the answer. Otherwise
  the screen stays as written, and no field is asked.

Nothing else is missing: the three routes below cover the list, the item switches and the
CLAUDE.md switch.

## Fields

As the plan's B-234 routes give them, tag `projects`. `{id}` is the project id, `{kind}` an
`ExtensionKind` (`skill` or `mcp` from this screen; never `plugin`), `{item}` the library
item's id. `Toggle` is the existing body of the agent switch routes.

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/projects/{id}/planner-extensions` | nothing | 200 `AgentExtensions` `{ skills: AgentItem[], mcp: AgentMcpItem[], plugins: [] }` (`plugins` always empty for a project; all three empty when AYLUL is not configured) |
| `PUT /api/projects/{id}/planner-extensions/{kind}/{item}` | `Toggle` `{ command_id: string, enabled: boolean }` | 200 `AgentItem` `{ id, name, description, bundled?, transport?, enabled }` |
| `PUT /api/projects/{id}/planner-claude-md` | `Toggle` `{ command_id: string, enabled: boolean }` | 200 `Project`, with the new field `planner_reads_claude_md: boolean` (after `assistant`) |
| `GET /api/projects` (existing) | nothing | `Project[]`, each with `planner_reads_claude_md: boolean` |

## Errors

| When | The screen shows |
|---|---|
| 400: a `plugin` item sent to the item route (`plugins are not offered to the Planner`) | The screen never sends it, since it draws no plugin. If it happens, the switch returns to its saved state and `ErrorLine` under it shows Core's message. |
| 404 on `GET …/planner-extensions`: unknown or removed project | The section shows `ErrorLine` with Core's message and a "Retry" button, as the agent form does. The settings page already handles a missing project. |
| 404 on a `PUT`: unknown or removed project, an item removed from the library meanwhile, or AYLUL not configured | The switch returns to its saved state and `ErrorLine` shows Core's message. The section refetches the view, so a removed item leaves the list. |
| 409: the same `command_id` replayed with a different body (`CommandConflict`) | The switch returns to its saved state and `ErrorLine` shows Core's message. `attemptFor` gives a changed request a fresh `command_id`, so the client should not cause this; it is shown, never retried with the same id. |
| Network failure on a `PUT` | The switch returns to its saved state, with `ErrorLine`. A retry of the same change resends the same `command_id`. |

## Backend answer

Hamoud, 2026-11-25. **As asked.** The three routes and `Project.planner_reads_claude_md` are
B-234 exactly as the plan writes them; no route or field changes. The one thing to read
below is the error codes and texts: they are the ones the existing error mapping gives, and
they differ from route to route.

**Owners and contracts.** No new service.
- `GET …/planner-extensions` and `PUT …/planner-extensions/{kind}/{item}`: `Extensions`
  (`crates/core-extensions/contract.yaml`), `project_extensions` and `set_project_extension`
  (B-230, done at `0a9201a`).
- `PUT …/planner-claude-md` and the field on `Project`: `Projects`
  (`crates/core-projects/contract.yaml`), `set_planner_claude_md` (B-231) and
  `shapes.Project` (B-234).
- The routes: `crates/shadows-http/src/planner_switches.rs`, recorded in
  `crates/shadows-http/src/contract.yaml` (B-234). Each calls one method; no rule in the adapter.
- How a switch reaches the Planner is `Harness` (`crates/core-harness/contract.yaml`, B-233);
  the screen sees none of it.

**Fields, confirmed.**
- `PUT …/planner-extensions/{kind}/{item}` answers `AgentItem` for both kinds, as the agent
  toggle does today. For an MCP item `transport` is set; it is never an `AgentMcpItem`. Merge
  it into `mcp` as `saveAgentExtensionAnswer` already does.
- `GET …/planner-extensions` always sends `plugins: []`. `skills` and `mcp` are ordered by
  name, then id.
- `Project.planner_reads_claude_md` comes after `assistant`, starts `false` for every project,
  and is set only by the `PUT`.

**Errors, exactly** (status, `code`, `message`):

| When | Route | Status | `code` | `message` |
|---|---|---|---|---|
| `kind` is `plugin` | item `PUT` | 400 | `PROFILE_INVALID` | `invalid profile: plugins are not offered to the Planner` |
| unknown or removed project | `GET` and item `PUT` | 404 | `EXTENSION_NOT_FOUND` | `extension not found` |
| unknown item, or an item removed meanwhile | item `PUT` | 404 | `EXTENSION_NOT_FOUND` | `extension not found` |
| AYLUL not configured | item `PUT` | 404 | `EXTENSION_NOT_FOUND` | `extension not found` |
| unknown or removed project | CLAUDE.md `PUT` | 404 | `INVALID_COMMAND` | `not found: project` |
| same `command_id`, different body | all `PUT`s | 409 | `COMMAND_CONFLICT` | `command conflict: the same command id was reused with a different request` |

On the extension routes a missing project and a missing item are the same 404 by design (the
service maps both to `ExtensionsError::NotFound`). Key on the status, not the text: the
screen's rule "a 404 refetches the view" is right for all three 404 rows. The 400's text
carries the `invalid profile: ` prefix from the existing mapping; the screen never sends a
plugin, so this is only for the record.

**Masada's open points, settled** (Mohammed, 2026-11-25; not to be reopened):
1. **AYLUL not configured:** no signal. The view is simply empty, like an empty library, and
   the screen stays as written. No field is asked or added.
2. **A skipped CLAUDE.md** (missing, too large, a symlink, not UTF-8): no notice on the page.
   Core only logs `planner.claude_md_skipped`. Showing it later is a row in
   `docs/core/future.md` ("A skipped CLAUDE.md shown in the project's settings").
3. **The assistant (AYLUL) project:** nothing needed. The web already hides it from the
   sidebar and home, so its settings page is never opened. Core still accepts the switches
   for any live project (frontend decision 7 holds), and nothing changes on either side.
4. **Codex projects:** the fixed line "applies to the Planner on Claude Code" is enough. The
   section shows for a project whatever harness its conversations use; Core ignores the
   switches on Codex.

**Order.** B-234 waits for this request to be `accepted`. F-146 to F-148 stay `blocked` until
B-234 is `done` and its routes show in `api/openapi.json`.

## Frontend reply

Masada, 2026-11-25. **Accepted.** The answer gives the screen everything it needs; nothing
must change on the backend side.

- **Fields:** taken as confirmed. The item `PUT`'s `AgentItem` (with `transport` for MCP) is
  merged into `skills` or `mcp` by one private updater shared with the agent form's
  `saveAgentExtensionAnswer`; `plugins` is never read; `planner_reads_claude_md` comes from the
  projects list.
- **Errors:** the screen keys on the status, never on `code` or the text. Every refused or failed
  `PUT` sends its switch back and shows Core's `message` in `ErrorLine`. A 404 on an item `PUT`
  refetches the project's view (all three `EXTENSION_NOT_FOUND` rows). A 404 on the CLAUDE.md
  `PUT` (`INVALID_COMMAND`, `not found: project`) only goes back and shows the message: the
  settings page already handles a missing project. A 409 `COMMAND_CONFLICT` is shown, never
  retried. A 404 on the `GET` is the section's error view with Retry; the CLAUDE.md switch
  stays usable.
- **The four rulings** are in the plan as settled: one empty view when AYLUL is not
  configured, no notice for a skipped CLAUDE.md, nothing for the assistant project, the fixed
  Claude-Code-only line for every harness.

Order, as the answer says: F-145 starts now; F-146 to F-148 stay `blocked` on B-234 until its
routes show in `api/openapi.json`. Plan: `docs/frontend/plans/2026-11-25-planner-project-switches.md`.

## Ruling
