# The Planner's project switches: the web section

- **Date:** 2026-11-25. Masada. Tasks F-145 to F-148 are in `docs/frontend/stages.md`.
- **Owner spec:** §24.3, "The person may widen the Planner's reach, never its hands"
  (`docs/core/specs/2026-10-08-approval-authority-design.md`).
- **Backend:** `docs/backend/plans/2026-11-25-planner-project-switches.md`. Only **B-234**
  (the routes, and `planner_reads_claude_md` on `Project`) changes what the web sees.
- **Shared request:** `docs/shared/requests/2026-11-25-planner-project-switches.md`.
  **Status `accepted`** (Hamoud's answer at `77ed3cd`, "as asked"; Mohammed's rulings of
  2026-11-25 in it). F-146 to F-148 stay `blocked` until B-234 is `done` and its routes show
  in `api/openapi.json`.
- **Checked against the code at `3e9fb7f`**: every path, name and line below was read there.

## What the person gets

In a project's settings, below "Allowed modes", a section **"What the Planner may use"**:

1. One fixed line saying the switches apply to the Planner on Claude Code only, and when a
   change arrives.
2. A switch **"Read the project's CLAUDE.md"**, always drawn once the project is known,
   whatever the library answers.
3. AYLUL's library: its skills, then its MCP servers, one switch each. No plugin and no
   plugins heading. No tool rules under an MCP server.
4. The "Add to library" link, which opens Settings › Plugins & Skills over the page.

Every switch saves at once, shows Core's answer, and goes back on a failure with the
failure's line under it. This is the agent form's switch, the same code (below).

## Reuse first

| Job | AYLUL web today | This plan |
|---|---|---|
| A switch that saves at once: name, description, `role="switch"` checkbox, optimistic state, one-at-a-time lock, kept command id, `ErrorLine` | `ExtensionSwitch`, private in `web/src/app/agents/agent-extensions.tsx` (lines 58–97), bound to an agent slug, `setAgentExtension`, `saveAgentExtensionAnswer` and `ToolRules` | moved to `web/src/app/saving-switch.tsx` as `SavingSwitch`, bound to nothing; the agent form draws it with its own calls and keeps `ToolRules` as the switch's children (F-145) |
| The section frame: loading skeleton, `ErrorLine` with a retry button, the empty line, "Add to library" | `AgentExtensions` (lines 11–56) | the same pattern in `PlannerSwitches`, with its own strings; not shared (two callers, different lists and words) |
| The library view's type, the toggle body | `AgentExtensions`, `AgentItem`, `ExtensionKind` in `web/src/api/extensions.ts`; `Toggle` in the schema | reused as they are |
| Writing a saved item into the view | `saveAgentExtensionAnswer` (`extensions.ts` 63–67) | its merge becomes one private updater shared with `saveProjectExtensionAnswer` (F-146) |
| Writing a saved `Project` into the list | `saveProjectAnswer` (`projects.ts` 43–46) | reused for the CLAUDE.md switch |
| The command id kept across retries | `attemptFor` (`web/src/api/command-id.ts`) | reused inside `SavingSwitch` |
| Library edits refresh every view of the library | three call sites in `web/src/app/settings/` invalidate `extensionKeys.library()` and `extensionKeys.agents()` | one function `invalidateLibraryViews` in `extensions.ts` that also covers projects; the three sites call it (F-148) |

Nothing here is a second copy: no second switch, no second merge, no second key scheme.

## Decisions

1. **The shared switch lives in `shared`, not in `agents`.** `project-settings` may import
   only `api, ui, shared` (architecture table in `docs/frontend/codebase/README.md`). A
   switch in `app/agents/` would make project settings depend on the AYLUL agents folder.
   `web/src/app/saving-switch.tsx` joins the `shared` row, as `error-line.tsx` is.
2. **The agent form's behaviour does not change.** Same DOM, same strings, same fingerprint
   (`{ slug, kind, id, enabled }`), same order of cache writes. Its ten tests in
   `agent-extensions.test.tsx` and the tests in `tool-rules.test.tsx` pass **unchanged**;
   F-145 edits no test.
3. **The project's kind type excludes plugins.** `PlannerExtensionKind =
   Exclude<ExtensionKind, 'plugin'>`, so a plugin cannot be sent by construction. The
   section reads only `skills` and `mcp` from the view, and never `plugins`.
4. **No Core rule in the screen.** The screen does not decide what is offered, whether AYLUL
   is configured, or whether CLAUDE.md exists. It draws Core's view and Core's answers.
5. **A 404 on a switch refetches the view** (an item removed meanwhile leaves the list; a
   removed project turns the section into its error view). No other failure refetches: a
   failed refetch would replace the list with the error view.
6. **The project view follows library edits** through `invalidateLibraryViews` (F-148), so
   an item added or removed in Settings › Plugins & Skills shows or leaves here without a
   reload, as it does for agents today.
7. **The section shows for every project the page shows**, and the screen copies no rule
   about which project may have it. The assistant (AYLUL) project needs nothing (Mohammed,
   2026-11-25): the web already hides it from the sidebar and home, so its settings page is
   not opened; Core still accepts the switches for any live project.
8. **The screen keys on the status, never on `code` or the text** (the backend answer). The
   codes are recorded below for the tests; the screen reads none of them.

## Settled with the backend (Mohammed, 2026-11-25; not to be reopened)

1. **AYLUL not configured:** no signal. The view is all three lists empty, the same as an
   empty library, and the section shows the empty line.
2. **A skipped CLAUDE.md** (missing, too large, a symlink, not UTF-8): no notice on the page.
   Core only logs `planner.claude_md_skipped`.
3. **The assistant project:** nothing on either side (decision 7).
4. **Codex projects:** the fixed line "applies to the Planner on Claude Code" is enough; the
   section shows whatever harness the project's conversations use.

## Core's answers, exactly

From the backend answer. The item `PUT` answers `AgentItem` for both kinds (`transport` set
for MCP); `GET` always sends `plugins: []`, `skills` and `mcp` ordered by name, then id;
`planner_reads_claude_md` comes after `assistant` and starts `false`.

| When | Route | Status | `code` | `message` | The screen |
|---|---|---|---|---|---|
| `kind` is `plugin` | item `PUT` | 400 | `PROFILE_INVALID` | `invalid profile: plugins are not offered to the Planner` | never sent (`PlannerExtensionKind`); would go back with the message |
| unknown or removed project | `GET` | 404 | `EXTENSION_NOT_FOUND` | `extension not found` | error view with Retry; CLAUDE.md switch stays |
| unknown or removed project, item removed meanwhile, or AYLUL not configured | item `PUT` | 404 | `EXTENSION_NOT_FOUND` | `extension not found` | goes back with the message, and refetches the view |
| unknown or removed project | CLAUDE.md `PUT` | 404 | `INVALID_COMMAND` | `not found: project` | goes back with the message; no refetch (the page already handles a missing project) |
| same `command_id`, different body | every `PUT` | 409 | `COMMAND_CONFLICT` | `command conflict: the same command id was reused with a different request` | goes back with the message; never retried |

## F-145 · The agent form's switch becomes a shared switch

**Goal:** move the switch row out of the agent form into `shared`, unchanged in behaviour.

**Needs:** none on the backend. The request is `accepted`, so it can start now.

**New file `web/src/app/saving-switch.tsx`** (one job: *One switch saving each change at
once*). It exports one generic component and its props type, nothing else:

```tsx
export interface SavingSwitchProps<T> {
  /** The switch's label, also its accessible name. */
  name: string
  /** The line under the switch. */
  description: string
  /** Core's saved value; shown whenever no change is in flight. */
  enabled: boolean
  /** What identifies this switch's change, without `enabled`. With `enabled` it is the
   *  fingerprint `attemptFor` keeps a command id for. */
  request: Record<string, unknown>
  /** Sends one change; answers Core's saved value. */
  send: (enabled: boolean, commandId: string) => Promise<T>
  /** Writes the answer into the cache. Awaited before the optimistic state clears. */
  onSaved: (saved: T) => Promise<void>
  /** Called with the failure, after the switch has gone back. Optional. */
  onFailed?: (error: Error) => void
  /** Drawn under the error line, inside the row (the agent form's tool rules). */
  children?: ReactNode
}
export function SavingSwitch<T>(props: SavingSwitchProps<T>): JSX.Element
```

Its body is lines 59–96 of today's `ExtensionSwitch`, with these substitutions and nothing
else:

| Today | In `SavingSwitch` |
|---|---|
| `attemptFor(attempt.current, { slug, kind, id: item.id, enabled })` | `attemptFor(attempt.current, { ...request, enabled })` |
| `setAgentExtension(slug, kind, item.id, enabled, attempt.current.commandId)` | `send(enabled, attempt.current.commandId)` |
| `onSuccess`: `attempt.current = null; await saveAgentExtensionAnswer(..); await client.invalidateQueries(..)` | `onSuccess`: `attempt.current = null; await onSaved(saved)` |
| no `onError` | `onError: error => onFailed?.(error)` |
| `item.name`, `item.enabled`, `item.description` | `name`, `enabled`, `description` |
| `{kind === 'mcp' && item.enabled && <ToolRules slug={slug} id={item.id} />}` | `{children}` |

`useMutation({ retry: false, ... })`, the `locked` ref, `optimistic` state, `onSettled`,
the markup and class names (`surface-panel space-y-2 rounded-lg p-3`, the `label`, the
`input type="checkbox" role="switch" aria-label={name}`, `disabled={toggle.isPending}`, the
`<p className="text-xs text-muted-foreground">`, `{toggle.error && <ErrorLine .../>}`)
stay exactly as they are. It imports `attemptFor`/`Attempt` from `@/api/command-id` and
`ErrorLine` from `./error-line`; nothing from `app/agents/`.

**`web/src/app/agents/agent-extensions.tsx`:** delete the private `ExtensionSwitch`.
`AgentExtensions` calls `useQueryClient()` once, and in its item map draws:

```tsx
<SavingSwitch
  key={`${slug}:${kind}:${item.id}`}
  name={item.name}
  description={item.description}
  enabled={item.enabled}
  request={{ slug, kind, id: item.id }}
  send={(enabled, commandId) => setAgentExtension(slug, kind, item.id, enabled, commandId)}
  onSaved={async saved => {
    await saveAgentExtensionAnswer(client, slug, kind, saved)
    await client.invalidateQueries({ queryKey: agentExtensionsQuery(slug).queryKey, exact: true })
  }}
>
  {kind === 'mcp' && item.enabled && <ToolRules slug={slug} id={item.id} />}
</SavingSwitch>
```

Imports of `useMutation`, `useRef`, `useState`, `attemptFor`, `Attempt` and `ErrorLine`
leave the file if nothing else in it uses them (`ErrorLine` stays: the load error uses it).

**Code map** `docs/frontend/codebase/README.md`:
- In the architecture table's `shared` row, add `app/saving-switch.tsx` to **Sources** and to
  **Entry**.
- A row in "What each file owns":
  `` | `web/src/app/saving-switch.tsx` | One switch saving each change at once | `web/src/app/error-line.tsx` | ``.

**Contract** `web/src/app/agents/contract.yaml`: the `AgentExtensions` note adds "Each item is
drawn with the shared `SavingSwitch` (`app/saving-switch.tsx`); `ToolRules` is its child for
an enabled MCP item." The obligation at line 112 keeps its three tests.

- **Touch:** `web/src/app/saving-switch.tsx` (new), `web/src/app/agents/agent-extensions.tsx`,
  `web/src/app/agents/contract.yaml`, `docs/frontend/codebase/README.md`.
- **Done when**, from `web/`: `npx vitest run src/app/agents/agent-extensions.test.tsx
  src/app/agents/tool-rules.test.tsx src/app/agents/agent-form.test.tsx` passes with no test
  file changed, then `npm run typecheck`, `npm run lint`, `npm run build` and
  `npx vitest run src/codemap.test.ts src/architecture`.

## F-146 · The project's switch calls in `web/src/api/`

**Goal:** regenerate the schema from B-234 and add the three calls and their cache writes.

**Needs:** the request `accepted`; B-234 `done` (its three routes and
`Project.planner_reads_claude_md` in `api/openapi.json`). `blocked` until then.

**First:** `npm run gen:api`. `schema.d.ts` changes only through it.

**`web/src/test/contract-fixtures.ts`:** `projectWithModes` adds
`planner_reads_claude_md: false` after `assistant: false`. If `npm run typecheck` then names
another file that builds a whole `Project` literal, stop and report it (every other test
spreads `projectFixture`, at `3e9fb7f`).

**`web/src/api/extensions.ts`:**

```ts
/** The kinds a project may switch for its Planner. Core refuses a plugin. */
export type PlannerExtensionKind = Exclude<ExtensionKind, 'plugin'>

export const extensionKeys = {
  library: () => ['aylul-extensions'],
  agents: () => ['aylul-agent-extensions'],
  agent: (slug: string) => ['aylul-agent-extensions', slug],
  projects: () => ['aylul-project-extensions'],
  project: (projectId: string) => ['aylul-project-extensions', projectId],
}

/** GET /api/projects/{id}/planner-extensions: the library with this project's switches. */
export function projectExtensionsQuery(projectId: string)        // queryOptions, retry: false

/** PUT /api/projects/{id}/planner-extensions/{kind}/{item}, body { command_id, enabled }. */
export function setProjectExtension(projectId: string, kind: PlannerExtensionKind, id: string,
  enabled: boolean, commandId: string): Promise<AgentItem>

/** Writes a saved project switch into that project's view, cancelling a stale read. */
export function saveProjectExtensionAnswer(client: QueryClient, projectId: string,
  kind: PlannerExtensionKind, saved: AgentItem): Promise<void>

/** Refreshes every view that lists the library: the library, each agent's, each project's. */
export function invalidateLibraryViews(client: QueryClient): Promise<void>
```

- `saveAgentExtensionAnswer` keeps its signature and its result. Its updater moves into one
  private function, `withSaved(kind: ExtensionKind, saved: AgentItem)`, returning the
  `(current: AgentExtensions | undefined) => AgentExtensions | undefined` updater, and both
  `save…Answer` functions call `writeSaved` with it on their own key.
- `invalidateLibraryViews` awaits `Promise.all` of `invalidateQueries` on
  `extensionKeys.library()`, `extensionKeys.agents()` and `extensionKeys.projects()`
  (prefixes, not `exact`).
- Every call goes through `client` from `./transport` with the generated paths and
  `params.path`; no URL is built by hand.

**`web/src/api/projects.ts`:**

```ts
/** PUT /api/projects/{id}/planner-claude-md, body { command_id, enabled }: whether the
 *  Planner on Claude Code receives the project directory's CLAUDE.md. Answers the Project. */
export function setPlannerClaudeMd(projectId: string, commandId: string, enabled: boolean): Promise<Project>
```

Its answer is written with the existing `saveProjectAnswer`.

**Tests**:
- `web/src/api/extensions.test.ts`, with the file's `vi.stubGlobal('fetch', …)` pattern:
  - `project switches use the project routes with the caller's command id`: the GET is
    `/api/projects/p1/planner-extensions`; `setProjectExtension('p1', 'mcp', 'm1', true,
    'cmd')` is `PUT /api/projects/p1/planner-extensions/mcp/m1` with body exactly
    `{ command_id: 'cmd', enabled: true }`.
  - `a saved project switch replaces only its item in its project's view`: over a
    `new QueryClient()` holding views for `p1` and `p2`, `saveProjectExtensionAnswer` for
    `p1`'s skill changes that one item; `p1`'s other items and `p2`'s view are equal to
    before.
  - `invalidateLibraryViews refreshes the library, every agent and every project`: three
    queries seeded under the three prefixes are all `isInvalidated` after it.
- `web/src/api/requests.test.ts`, inside `describe('client')`:
  - `setPlannerClaudeMd sends the toggle to the project's route`: one call
    `PUT /api/projects/p1/planner-claude-md`, body `{ command_id: 'cmd-c', enabled: true }`.

**Contract** `web/src/api/contract.yaml`: under `extensions:` the three new functions with
their route and key (`['aylul-project-extensions', projectId]`); under `extension_cache:`
`keys` names the project prefix, and `saveProjectExtensionAnswer` and
`invalidateLibraryViews` get a line each; under the projects functions (near line 183)
`setPlannerClaudeMd: "(projectId: string, commandId: string, enabled: boolean) ->
Promise<Project>"`; the four tests under their rules.

- **Touch:** `web/src/api/schema.d.ts` (through `npm run gen:api` only),
  `web/src/api/extensions.ts`, `web/src/api/projects.ts`, `web/src/api/extensions.test.ts`,
  `web/src/api/requests.test.ts`, `web/src/api/contract.yaml`,
  `web/src/test/contract-fixtures.ts`.
- **Done when**, from `web/`: `npx vitest run src/api/extensions.test.ts
  src/api/requests.test.ts src/app/agents/agent-extensions.test.tsx` passes, then the four
  checks (`npm run typecheck`, `npm run lint`, `npm run build`,
  `npx vitest run src/codemap.test.ts src/architecture`).

## F-147 · The section "What the Planner may use"

**Goal:** draw the section in project settings over F-145's switch and F-146's calls.

**Needs:** F-145, F-146 (so the request `accepted` and B-234 `done`).

**New file `web/src/app/project-settings/planner-switches.tsx`** (one job: *Switches for
what the Planner may use*). It exports one component:

```tsx
export function PlannerSwitches({ project }: { project: Project }): JSX.Element
```

Its body, top to bottom:

```tsx
<section role="group" aria-label="What the Planner may use" className="space-y-3">
  <h2 className="text-sm font-medium">What the Planner may use</h2>
  <p className="text-xs text-faint-foreground">{FIXED_LINE}</p>
  <SavingSwitch …CLAUDE.md… />
  {library part}
  <Button type="button" variant="link" onClick={() => open('plugins')}>Add to library</Button>
</section>
```

- `open` is `useOpenSettings()` from `@/app/navigation`.
- **The CLAUDE.md switch:** `name` "Read the project's CLAUDE.md", `description` the line
  below, `enabled={project.planner_reads_claude_md}`,
  `request={{ project: project.id, setting: 'planner_reads_claude_md' }}`,
  `send={(enabled, commandId) => setPlannerClaudeMd(project.id, commandId, enabled)}`,
  `onSaved={saved => saveProjectAnswer(client, saved)}`. It does not wait for the library.
- **The library part**, from `useQuery(projectExtensionsQuery(project.id))`:
  - pending: `<SkeletonContent label="Loading the library…" />`;
  - error: `<ErrorLine error={state.error} />` and
    `<Button type="button" variant="outline" disabled={state.isFetching}
    onClick={() => { void state.refetch() }}>Retry</Button>`;
  - `skills` and `mcp` both empty: `<p>No skills or MCP servers in the library.</p>`
    (also the answer when AYLUL is not configured; the screen does not tell them apart);
  - otherwise a block per non-empty list, skills first: `<h3 className="text-sm
    text-muted-foreground">` "Skills" or "MCP servers", then one `SavingSwitch` per item.
  - `plugins` is never read.
- **An item's switch:** `key={`${project.id}:${kind}:${item.id}`}`, `name={item.name}`,
  `description={item.description}`, `enabled={item.enabled}`,
  `request={{ project: project.id, kind, id: item.id }}`,
  `send={(enabled, commandId) => setProjectExtension(project.id, kind, item.id, enabled, commandId)}`,
  `onSaved` = `saveProjectExtensionAnswer(client, project.id, kind, saved)` then
  `client.invalidateQueries({ queryKey: projectExtensionsQuery(project.id).queryKey, exact: true })`
  (as the agent form), and
  `onFailed={error => { if (error instanceof ApiError && error.problem.kind === 'daemon' &&
  error.problem.status === 404) void client.invalidateQueries({ queryKey:
  projectExtensionsQuery(project.id).queryKey, exact: true }) }}`. No children: no
  `ToolRules`, no tools query.

**Strings shown**, exactly:

| Where | Text |
|---|---|
| heading, group name | What the Planner may use |
| fixed line | These switches apply to the Planner on Claude Code only. A skill or MCP server reaches the next conversation session that opens. |
| CLAUDE.md switch | Read the project's CLAUDE.md |
| its description | Adds the CLAUDE.md in the project's folder to the Planner's instructions. Only a new conversation receives it. |
| list headings | Skills · MCP servers |
| empty library | No skills or MCP servers in the library. |
| loading label | Loading the library… |
| retry | Retry |
| link | Add to library |

**Errors**, as "Core's answers, exactly" above, with no code beyond the above: every refused
or failed `PUT` (400, 404, 409, a network failure) sends the switch back to its saved value
and shows `ErrorLine` with Core's message under it (`SavingSwitch` already does both). A 404
on an item `PUT` also refetches the view; the CLAUDE.md switch has no `onFailed`. A retry of
the same change resends the same command id (`attemptFor`); a 409 is never retried by the
screen. A 404 on the `GET` is the error view with Retry; the CLAUDE.md switch stays usable.
The screen reads the status only, never `code` or the text.

**`web/src/app/project-settings/project-settings.tsx`:** inside the existing
`project !== undefined` block for `ProjectModes`, after it:
`<PlannerSwitches key={`planner-${projectId}`} project={project} />`.

**`web/src/test/fake-daemon.ts`:** `answers()` adds
`'GET /api/projects/p1/planner-extensions': { skills: [], mcp: [], plugins: [] }`, so every
existing page test sees the empty section instead of a 404.

**Tests**, a new file `web/src/app/project-settings/planner-switches.test.tsx`, over
`startApp('/projects/p1/settings', …)` and `answers()`, with the agent test's `switchOf`
helper. A skill `review` and an MCP server `research` (stdio transport), and a plugin `git`
in `plugins` that must never show:
- `lists the library's skills and MCP servers, never a plugin, with no tool rules`: switches
  `review`, `research` and "Read the project's CLAUDE.md"; no `git` switch; no text "Plugins";
  the fixed line; with `research` enabled, no call ending in `/tools`.
- `a switch saves at once with a command id`: clicking `review` sends one
  `PUT /api/projects/p1/planner-extensions/skill/<id>` with body
  `{ enabled: true, command_id: <string> }`; the switch is checked and disabled while it
  waits, then checked and enabled; the cached view has `skills[0].enabled === true`.
- `a failed switch goes back, and its retry resends the same command id`: the first `PUT`
  answers 500 `STORAGE_UNAVAILABLE`; the message shows and the switch is unchecked; the
  second click's body equals the first; a third click after a success has a new id (as
  `a failed switch goes back` in the agent tests).
- `a switch whose item is gone refetches the list`: the `PUT` answers 404
  `EXTENSION_NOT_FOUND` "extension not found"; the switch goes
  back, a second `GET /api/projects/p1/planner-extensions` is made, and with the item gone
  from that answer its switch is no longer drawn.
- `a conflict is shown and not retried`: the `PUT` answers 409 `COMMAND_CONFLICT` "command
  conflict: the same command id was reused with a different request"; the message shows, the switch is unchecked, and exactly one `PUT` was made.
- `the CLAUDE.md switch saves the project`: one `PUT /api/projects/p1/planner-claude-md`
  with `{ enabled: true, command_id: <string> }`; answering the project with the field
  `true`, the projects cache holds it and the switch is checked. A second case answers 404
  `INVALID_COMMAND` "not found: project": the message shows, the switch is unchecked, and no
  second `GET …/planner-extensions` is made.
- `an empty library says so, links to the library, and keeps the CLAUDE.md switch`: the
  `GET` answers all three lists empty (also what AYLUL not configured answers); the
  empty line; the CLAUDE.md switch is drawn and saves; "Add to library" opens the dialog
  titled "Plugins & Skills" and the path stays `/projects/p1/settings`.
- `a failed load shows Core's message with Retry, and the CLAUDE.md switch still works`: the
  `GET` answers 404 `EXTENSION_NOT_FOUND` "extension not found"; it shows with "Retry", not the empty line; Retry makes a
  second `GET`; the CLAUDE.md switch is drawn.
- `a late answer stays with its project`: `p2` added to `GET /api/projects` with its own
  view; a `review` `PUT` on p1 is held, the page navigates to p2 (as the existing
  `another project's settings do not show this one's draft`), the held answer is released;
  p2's cached view is equal to before and p2's `review` switch is unchecked.

**Contract** `web/src/app/project-settings/contract.yaml`: the header comment names the
Planner's switches; `external.Project` adds `planner_reads_claude_md`; `components` gets
`PlannerSwitches: "({ project: Project }) => JSX.Element"` with a note (skills and MCP
servers only, no tool rules, the CLAUDE.md switch independent of the library, drawn with
`SavingSwitch`); `obligations` get the rules above, each with its test names; `tests` get
the nine tests; `project_cache_owner` adds the CLAUDE.md answer; a line
`extension_cache_owner: "api/extensions.ts owns the project view's key and saved-switch merge"`.

**Code map** `docs/frontend/codebase/README.md`:
- the `project-settings` row's **Allowed dependencies** become `api, ui, shared, navigation`
  (for `useOpenSettings`);
- a row
  `` | `web/src/app/project-settings/planner-switches.tsx` | Switches for what the Planner may use | `web/src/app/project-settings/contract.yaml` | ``.

- **Touch:** `web/src/app/project-settings/planner-switches.tsx` (new),
  `web/src/app/project-settings/planner-switches.test.tsx` (new),
  `web/src/app/project-settings/project-settings.tsx`,
  `web/src/app/project-settings/contract.yaml`, `web/src/test/fake-daemon.ts`,
  `docs/frontend/codebase/README.md`.
- **Done when**, from `web/`: `npx vitest run src/app/project-settings` passes (the new file
  and every existing page test), then the four checks.

## F-148 · A library change reaches the project's switches

**Goal:** the three library edits in Settings › Plugins & Skills refresh the project views
too, through one call.

**Needs:** F-146, F-147.

In each of `web/src/app/settings/mcp-form.tsx` (line 33), `extension-import.tsx` (line 22)
and `extension-list.tsx` (line 22), the
`await Promise.all([client.invalidateQueries(… library …), client.invalidateQueries({ queryKey: extensionKeys.agents() })])`
becomes `await invalidateLibraryViews(client)`. The surrounding `onSuccess` order (clear the
attempt, clear drafts, then invalidate, then `onDone()` / `setConfirming(null)`) stays.
Imports that fall out of use leave.

**Test**, added to `web/src/app/project-settings/planner-switches.test.tsx`:
- `a library removal refreshes the project's switches`: on `/projects/p1/settings` with
  `review` listed, open "Add to library", remove `review` in the dialog (as
  `library removal clears every agent view` does), and the project's `GET` is made again;
  with `review` gone from it, its switch is no longer drawn.

**Contract** `web/src/app/settings/contract.yaml`: the notes at lines 35 and 40, the rule at
line 108 and `extension_refresh_owner` (line 199) say library writes call
`invalidateLibraryViews` (library, every agent view, every project view); the rule's
`tested_by` adds `a library removal refreshes the project's switches`.

- **Touch:** `web/src/app/settings/mcp-form.tsx`, `web/src/app/settings/extension-import.tsx`,
  `web/src/app/settings/extension-list.tsx`, `web/src/app/settings/contract.yaml`,
  `web/src/app/project-settings/planner-switches.test.tsx`.
- **Done when**, from `web/`: `npx vitest run src/app/settings src/app/agents/agent-extensions.test.tsx
  src/app/project-settings/planner-switches.test.tsx` passes, then the four checks.

## Order and what waits

| Task | Waits on |
|---|---|
| F-145 | nothing: the request is `accepted`, and it needs no backend |
| F-146 | B-234 `done` |
| F-147 | F-145, F-146 |
| F-148 | F-146, F-147, so B-234 too: its call `invalidateLibraryViews` comes with F-146, and its test runs on F-147's page |

The browser check of the page (a person turns each switch in `shadows serve` and reads the
next Planner opening in the logs) waits for Mohammed's machine, with B-235.

## Left out, on purpose

- Telling "AYLUL is not configured" apart from an empty library (settled 1).
- Tool rules for a project's MCP server (backend decision 3), plugins (decision 2), Codex
  (decision 6).
- Saying in the screen whether `CLAUDE.md` exists or was skipped (settled 2). Showing it later
  is a row in `docs/core/future.md`.
- A test file for `saving-switch.tsx` alone: its behaviour is held by the agent form's
  tests and F-147's, through the two screens that draw it.
