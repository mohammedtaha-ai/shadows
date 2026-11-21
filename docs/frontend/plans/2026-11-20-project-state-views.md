# The project's state, its diagrams and `code_where` in the web

Tasks F-130 to F-135 in [`../stages.md`](../stages.md). Shared requests, both accepted on
2026-11-20:
- `docs/shared/requests/2026-11-19-project-state-and-diagrams.md` (F-130 to F-134);
- `docs/shared/requests/2026-11-19-code-where.md` (F-135).

Owner design: `docs/backend/specs/2026-11-19-project-code-record-design.md` §5.3 and §5.4.
Backend plans:
- `docs/backend/plans/2026-11-19-code-record-2a.md` (B-175 to B-181, B-187);
- `docs/backend/plans/2026-11-19-code-record-2b.md` (B-182 to B-186).

## Rulings for every task

1. **Read only.** Nothing on these screens writes. Tables and screens are the Planner's.
2. **Core states, the web shows.** Element states, task counts, attempt states, ranks, the
   order of `code_where` entries, `live`, `uses_live` and every remainder count are shown as
   Core gives them.
   - No component derives a state, sorts an answer again, or checks a topic's length.
   - No component counts what Core already counted.
   - Presentation is allowed: a label or icon per enum value, grouping uses by their `owner`,
     hiding `removed` elements behind a toggle, drawing an edge only between two shown nodes.
3. **One door.**
   - Every read is a query in `web/src/api/` (new domains `project-state.ts` and
     `code-record.ts`).
   - Its key comes from that domain's key factory.
   - The refresh is classified in `web/src/stream/refresh.ts` and nowhere else.
   - No `fetch`, no URL built by hand, no `EventSource`.
4. **Types are generated.**
   - Every shape is an alias of `components['schemas'][...]` in the API domain, after
     `npm run gen:api`. Nothing is redeclared.
   - If a generated name differs from the names below (`ProjectStateView`, `PartState`,
     `PlanState`, `TaskCounts`, `StateCounts`, `ElementGap`, `PartDetail`, `PlanTasks`,
     `TaskLine`, `ElementView`, `DesignElement`, `ElementState`, `ElementTask`, `ElementPin`,
     `BuildState`, `WhereAnswer`, `WhereEntry`, `SymbolEntry`, `UsesLive`, `Live`, `UseLine`,
     `UnrecordedFile`, `Owner`, `FileTask`, `Rank`), use the generated one and say so in the
     report.
   - When a name the task needs is missing from the generated file, stop and report.
5. **Refusals are shown at once.**
   - Every new query uses `retry: retryLost`, which retries only a lost call, never Core's
     answer.
   - F-130 moves `retryLost` from `web/src/api/runs.ts` to `web/src/api/error.ts`, exported, and
     `runs.ts` imports it.
   - A 404 or 422 shows `ErrorLine` with Core's code and message.
6. **Text direction.** Every name, purpose, `why`, `when`, path and title Core wrote is
   rendered with `dir="auto"`.
7. **Every new file gets its row** in `docs/frontend/codebase/README.md` ("What each file
   owns", tests included), stated without "and". A new folder also gets:
   - a row in "Architecture modules";
   - a line under "Contracts".

## Where the screens live

- **A new route,** `/projects/$projectId/state`, is lazy loaded in `web/src/router.tsx` like
  `AgreementsPage`.
  - Its search is
    `{ view?: 'state' | 'tables' | 'screens' | 'where'; part?: string; element?: string; removed?: boolean; topic?: string }`.
  - An unknown `view` reads as `'state'`. `part`, `element` and `topic` are kept when they are
    strings.
  - `removed` is `true` only for `true` or `'true'`.
  - The whole search is defined in F-131, so later tasks do not touch the router.
- **The design workspace's section nav** gains one `Link`, "State", to that route. It is the
  same pattern as its existing "Contracts" link to `/projects/$projectId/agreements`.
- **A new feature folder,** `web/src/app/project-state/`, owns the page.
  - Architecture row: owner `project-state`, sources `app/project-state/**`, entry
    `app/project-state/index.ts, app/project-state/state-page.tsx`, contract
    `app/project-state/contract.yaml`, allowed dependencies `api, ui, shared, workflows`.
  - `composition`'s allowed dependencies gain `project-state`.
  - It is a folder of its own because these views read `ProjectState` and `CodeRecord`. They do
    not read Design, which `app/design/` owns, and they are not part of the Design workspace's
    editor lifecycle.
- **The shell already holds the project stream** for every project route
  (`useProjectEvents(projectId)` in `web/src/app/shell.tsx`). The page opens no stream of its own.

## What is reused

| Existing | How it is used |
|---|---|
| `useProjectEvents` / `holdProjectRefresh` (`web/src/stream/`) | Unchanged. The shell already holds it; F-130 only adds targets to `classifyRefresh`, `dispatchRefresh`, the replay rule and the project baseline in `refresh.ts`. |
| `retryLost` (`web/src/api/runs.ts`) | Moved to `web/src/api/error.ts` and exported. Every new query uses it. |
| `THEME` (`web/src/app/workflows/flow-theme.ts`, already a public `workflows` entry) | The React Flow colours of the ER diagram and the screen flow. |
| `MapCanvas` (`web/src/app/design/map-canvas.tsx`) | The pattern for `ElementCanvas`, not the component: `MapCanvas` takes `Part[]` and draws layers and permissions. `ElementCanvas` copies its React Flow props (`nodesDraggable={false}`, `nodesConnectable={false}`, `edgesFocusable={false}`, `colorMode="dark"`, `fitView`, `Controls showInteractive={false}`, `MiniMap pannable zoomable`), its `hidden … sm:block` canvas box with a list below that is always shown, and its `data-*` node markers for tests. |
| `placeGraph` / `handles` (`web/src/app/workflows/layout.ts`) | The pattern for `diagram-layout.ts`, not the function. `placeGraph` takes `PlanEdge[]` and writes `LinkEdge`'s offsets and channel lanes, and it staggers lone columns for plan order. It is also not a public `workflows` entry. `diagram-layout.ts` uses `@dagrejs/dagre` (already a dependency) the same way: fixed `width`, `height` and `measured` on each node, its `handles` set, positions from `graph.node(id)`. Edges therefore draw in tests without the browser measuring. |
| `SkeletonContent` (`web/src/components/ui/skeleton.tsx`) | Every loading state, with a label: `layout="rows"` for lists, `"graph"` for a diagram, `"detail"` for the element panel. |
| `ErrorLine` (`web/src/app/error-line.tsx`) | Every refused read (404, 422) with Core's code and message. |
| `Panel` (`web/src/components/ui/panel.tsx`) and `Button` (`web/src/components/ui/button.tsx`) | The page header box, as `WorkspacePage` draws it; the toggles and "Ask again". |
| The workspace section nav (`web/src/app/design/workspace-page.tsx`) | Its pill style (`tab(active)`), copied for the state page's own tabs, plus one `Link` to the new route. |
| `BriefPanel` (`web/src/app/tasks/brief-panel.tsx`) and `InspectPanel` (`web/src/app/workflows/inspect-panel.tsx`) | Gain one section each for pinned elements (F-134). No new panel. |
| `startApp`, `until`, `answers`, `FakeSource` (`web/src/test/`) | Every screen test runs the app over the fake daemon: `app.sources` filtered by `/projects/p1/events`, `.durable(seq, kind, null, payload)` and `.caughtUp(seq)`, as `web/src/stream/use-project-events.test.tsx` does. |
| `lucide-react` icons | The state labels: `Hourglass` planned, `CircleCheck` built, `TriangleAlert` gap, `CircleHelp` unchecked, `CircleSlash` removed. |

New, and why nothing existing fits:
- **`web/src/api/project-state.ts`, `web/src/api/code-record.ts`:** these are new owners
  (`ProjectState`, `CodeRecord`). `design.ts` is Design's and `code.ts` is Codebase's.
- **`web/src/app/element-content.tsx` (shared):** one revision's columns or links as text. Three
  modules show it: the element panel, the attempt brief (`tasks`) and the `code_where` view.
  `tasks` may not import `project-state`, so it sits with the shared root pieces.
- **`state-badge.tsx`:** no component shows a `BuildState`.
- **`diagram-layout.ts`, `element-canvas.tsx`:** see `placeGraph` and `MapCanvas` above.
- **`element-panel.tsx`:** nothing shows an element's revisions and tasks.

## F-130 · The API domain and the stream refresh

**Needs:** B-177 to B-180 and B-187 built, so `api/openapi.json` holds the three routes, the
pinned `elements` fields and the shapes.

**First:** `npm run gen:api` (from `web/`). `web/src/api/schema.d.ts` changes only through it.

**`web/src/api/error.ts`:**

```ts
/** Retries a lost call up to three times; Core's own answer is final. */
export function retryLost(failures: number, error: Error): boolean
```

The body moves verbatim from `runs.ts`, and `runs.ts` imports it from `./error`. It is an
`export function` next to the existing `export { ApiError } from './transport'`.

**`web/src/api/project-state.ts`** (new). Its header comment: "The project's state and its
tables and screens, as ProjectState and Design answer them."

```ts
export type ProjectStateView = components['schemas']['ProjectStateView']
// …and one alias each for PartState, PlanState, TaskCounts, StateCounts, ElementGap, PartDetail,
// PlanTasks, TaskLine, ElementView, DesignElement, ElementState, ElementTask, ElementPin, BuildState.

export const projectStateKeys = {
  all: (projectId: string) => ['projects', projectId, 'project-state'],
  reads: (projectId: string) => ['projects', projectId, 'project-state', 'reads'],
  state: (projectId: string, partId?: string) => ['projects', projectId, 'project-state', 'reads', 'state', partId ?? null],
  elements: (projectId: string) => ['projects', projectId, 'project-state', 'reads', 'elements'],
  revisions: (projectId: string, elementId: string) => ['projects', projectId, 'project-state', 'revisions', elementId],
}

export function projectStateQuery(projectId: string, partId?: string)
  // GET /api/projects/{id}/state, query { part_id: partId } only when partId is given; retry: retryLost
export function elementsQuery(projectId: string)
  // GET /api/projects/{id}/elements; retry: retryLost
export function elementRevisionsQuery(projectId: string, elementId: string)
  // GET /api/projects/{id}/elements/{element}/revisions; retry: retryLost
export function invalidateProjectState(client: QueryClient, projectId: string): void
  // invalidates projectStateKeys.reads(projectId): the state with and without part, and the elements
export function invalidateElementRevisions(client: QueryClient, projectId: string, elementId: string): void
```

Revisions sit outside `reads` because they change only with `DesignElementChanged`.

**`web/src/stream/refresh.ts`:**

- `RefreshTarget` gains `{ kind: 'project-state'; projectId: string }` and
  `{ kind: 'element-revisions'; projectId: string; elementId: string }`.
- It gains a constant:

  ```ts
  const STATE_EVENTS = new Set([...PLAN_EVENTS, 'DesignElementChanged', 'CodeRecordChanged',
    'TaskProgressChanged', 'ProjectDesignChanged', 'MapProposed', 'RoadmapProposed', 'RoadmapApproved'])
  ```

  This is exactly the request's "Refresh" list.
- **`classifyRefresh`,** inside `if (projectId !== undefined)`:
  - `STATE_EVENTS.has(kind)` pushes `{ kind: 'project-state', projectId }`;
  - `kind === 'DesignElementChanged'` with a string `payload.element_id` also pushes
    `{ kind: 'element-revisions', projectId, elementId }`.

  `TaskProgressChanged` and `CodeRecordChanged` give no other target.
- **`dispatchRefresh`** maps `project-state` to `invalidateProjectState` and
  `element-revisions` to `invalidateElementRevisions`.
- **Replay** (`refreshSubscriber.onDurable`, not live): both new kinds join the targets parked
  in `pending` until caught-up. Add them to the exclusion list beside `workflows`. They are reads
  with no dirty editor, so they reconcile once.
- **`baseline`,** project scope: add `{ kind: 'project-state', projectId }`, so every project
  caught-up reconciles the state and the elements. The revisions are not in the baseline.

**Tests** (`web/src/stream/refresh.test.tsx`):
- **`it.each(<the 12 STATE_EVENTS kinds>)('project_state_events_refetch_the_state_and_the_elements: %s')`:**
  - builds `projectStateQuery('p1')`, `projectStateQuery('p1', 'web')` and `elementsQuery('p1')`;
  - dispatches the classified event with `projectId: 'p1'`;
  - asserts all three are invalidated.
- **`a_design_element_change_refetches_only_that_elements_revisions`:** revisions keys for `E3`
  and `E4`; `DesignElementChanged` with `{ project_id: 'p1', element_id: 'E3', revision: 2 }`
  invalidates `E3`'s only.
- **`task_progress_and_record_changes_name_only_the_project_state`:** `classifyRefresh` of
  `TaskProgressChanged` (payload `{ project_id, workflow_id, version, task, attempt_id: null }`)
  and of `CodeRecordChanged` each equals `[{ kind: 'project-state', projectId: 'p1' }]`.
- **`replayed_state_events_reconcile_the_state_once_at_caught_up`:**
  - holds `useProjectEvents('p1')` over a `FakeSource`, as `preserves project and writer
    delivery with overlapping sequences, batching replayed archive and unarchive` does;
  - a state query is in the cache;
  - three replayed events (`TaskProgressChanged`, `CodeRecordChanged`, `DesignElementChanged`)
    invalidate nothing before `caughtUp`;
  - `caughtUp` invalidates the state exactly once;
  - one live `TaskProgressChanged` then invalidates it once more.

**Tests** (`web/src/api/project-state.test.ts`, new, modelled on `web/src/api/groups.test.ts`):
- `the_state_read_sends_part_id_only_when_asked`: the request URLs are
  `/api/projects/p1/state` and `/api/projects/p1/state?part_id=web`.
- `the_reads_retry_only_a_lost_call`:
  - the `retry` of each of the three queries is `retryLost`;
  - `retryLost` is false for a daemon `ApiError` (422 `INVALID_COMMAND`) and true for an
    unreachable one under three failures.

**Contracts:**
- `web/src/api/contract.yaml` gains a `project_state:` section: each query's route and key, the
  two invalidations, and that `retryLost` lives in `error.ts`.
- `web/src/stream/contract.yaml`:
  - `refresh_policy` gains `project_state:`: the 12 kinds, the revisions rule, parked until
    caught-up, the baseline;
  - `tested_by` gains the four test names;
  - `not_the_caller's` gains `STATE_EVENTS` in its list of sets to update.

**Code map:** rows for `web/src/api/project-state.ts` ("ProjectState server state") and
`web/src/api/project-state.test.ts` ("ProjectState request checks").

## F-131 · The state page and the state view

**Needs:** F-130.

**`web/src/router.tsx`:**
- `const ProjectStatePage = lazyRouteComponent(() => import('./app/project-state/state-page'), 'ProjectStatePage')`;
- `stateRoute` at `/projects/$projectId/state` with the search defined above, added to the
  route tree after `workspaceRoute`.

**`web/src/app/design/workspace-page.tsx`:** one `Link`, "State", to `/projects/$projectId/state`,
placed after "Contracts", with `className={tab(false)}`. Nothing else changes in that file.

**`web/src/app/project-state/index.ts`:** the comment "Public lazy route modules:
state-page.tsx." and no exports until a later task needs one.

**`web/src/app/project-state/state-page.tsx`:** `export function ProjectStatePage()`.
- **Reads** `projectId` from `useParams({ from: '/projects/$projectId/state' })`, and `view`,
  `part`, `element`, `removed` and `topic` from `useSearch`.
- **Lays out** like `WorkspacePage`: a `Panel` header with the title "Project state" and a nav
  `aria-label="Project state sections"`.
- **The nav:**
  - in F-131 it holds "State" and a `Link` "Workspace" to `/projects/$projectId/workspace`;
  - later tasks add "Tables" (F-132), "Screens" (F-133) and "Where" (F-135).
- **The body:** `view` selects the body, and in F-131 every view renders `StateView`.

**`web/src/app/project-state/state-badge.tsx`:**
`export function StateBadge({ state }: { state: BuildState })` draws a `<span
data-element-state={state}>` with the icon and the label:

| `state` | Icon | Label |
|---|---|---|
| `planned` | `Hourglass` | Planned |
| `built` | `CircleCheck` | Built |
| `gap` | `TriangleAlert` | Gap |
| `unchecked` | `CircleHelp` | Unchecked |
| `removed` | `CircleSlash` | Removed |

The icon is `aria-hidden`.

**`web/src/app/project-state/state-view.tsx`:**
`export function StateView({ projectId, part, removed }: { projectId: string; part?: string;
removed: boolean })`. It reads `useQuery(projectStateQuery(projectId, part))`.

- **Loading:** `SkeletonContent label="Loading the project's state…" layout="rows"`.
- **Error:**
  - without `part`: `ErrorLine`;
  - with `part`: `ErrorLine` in the part section, then a `Link` "All parts" (`search={{ view:
    'state' }}`).
- **Summary:**
  - `Stage: {stage}`;
  - `Map revision {map_revision}`;
  - when `missing` is not empty, `Missing: {missing.join(', ')}` as Core wrote them.
- **Parts** (`<section aria-label="Parts">`), one `<article data-state-part={part_id}>` per
  `PartState`, in the order given:
  - its title as a `Link` to `search={{ view: 'state', part: part_id }}`;
  - its kind, or "No kind";
  - when `parent` is set, `in {parent's title}`, the title found in the same `parts`, else the id;
  - the counts as `{planned} planned · {built} built · {gap} gap · {unchecked} unchecked`;
  - its plans (below), then, when `plans_more > 0`, `{plans_more} more plans`.

  After the list, when `parts_more > 0`, `{parts_more} more parts`.
- **A plan line (`PlanState`):**
  - its title: when `workflow_id` is set, a `Link` to `/projects/$projectId/workflows/$workflowId`;
    else, when `draft` is set, a `Link` to that workflow with " (draft)"; else plain text;
  - `{done} of {total} done · {started} started · {not_started} not started`.
- **"Plans with no part"** (`aria-label="Plans with no part"`): only when `unassigned` is not
  empty. The same plan lines, then `{unassigned_more} more plans` when it is above 0.
- **Gaps** (`aria-label="Gaps"`): each `ElementGap` shows:
  - `StateBadge`;
  - `{element_id} · {kind} · {name}`, linking to `search={{ view: kind === 'table' ? 'tables' :
    'screens', element: element_id }}`;
  - the part's title (found in `parts`, else `part_id`);
  - `why`.

  Then `{gaps_more} more gaps` when it is above 0. With none, "No gaps."
- **Empty:** with `parts` and `unassigned` both empty, "No parts yet. The map comes from the
  Planner."
- **Part detail,** when `part` is set and `data.part` is present
  (`<section aria-label="Part detail">`):
  - a heading with the part's title, and the `Link` "All parts";
  - each `PlanTasks`: its title linking to the plan, then one line per task:
    `T{number} {title}`, linking to
    `/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task`, then:
    - `Done` when `done`;
    - else Core's `attempt` as a label (Running, Committed, Failed, Stopped, Interrupted);
    - else "Not started".
  - `{plans_more} more plans` when it is above 0;
  - "Tables and screens": one row per `ElementView` whose `state.state` is not `removed`, or
    every row when `removed`. A row is `StateBadge`, then `{id} · {kind} · {name}`, linking to
    the tables or the screens view with `element`. When `state.behind`, it adds "newer revision
    planned".
  - a `Button` "Show removed" / "Hide removed" that navigates with `removed` toggled. It is shown
    only when some row is removed.
  - "No tables or screens in this part." when no row is shown.

**`web/src/app/project-state/contract.yaml`** (new, the template's shape). It records:
- **responsibility:** show the project's state, its tables and screens, and `code_where`, read
  only, over Core's answers;
- **the rulings above;**
- **its obligations, each with its test.**

**Tests** (`web/src/app/project-state/state-view.test.tsx`, happy-dom, `startApp` at
`/projects/p1/state`). The fixtures are typed with the aliases from `@/api/project-state`.
- `the_state_view_shows_stage_parts_plans_and_gaps`
- `remainders_are_shown_as_cores_counts` (`parts_more: 3`, `plans_more: 2`, `gaps_more: 1` read
  "3 more parts", "2 more plans", "1 more gaps")
- `a_part_shows_its_plans_tasks_and_elements` (`?part=web`: the request carries `part_id=web`;
  task lines read Done, Failed and Not started; element rows show their badges)
- `removed_elements_are_hidden_until_asked`
- `an_unknown_part_shows_cores_refusal` (422 `INVALID_COMMAND`: its code and message, and the
  "All parts" link)
- `an_unknown_project_shows_cores_not_found` (404: its code and message)
- `task_progress_on_the_project_stream_moves_a_task_to_started_without_reload`:
  - the state route answers `not_started: 1` until a flag flips, then `started: 1`;
  - after `caughtUp(0)`, a live `durable(1, 'TaskProgressChanged', null, { project_id: 'p1',
    workflow_id: 'w1', version: 1, task: 1, attempt_id: 'a1' })` flips the flag;
  - the text then reads `1 started`;
  - the number of `GET /api/projects/p1/state` calls rose by exactly one for the event.
- `replayed_events_reconcile_the_state_once`: three replayed durable events before `caughtUp(3)`
  add no state read until `caughtUp`, then exactly one.
- `the_workspace_links_to_the_state`: at `/projects/p1/workspace`, a link "State" whose `href`
  ends with `/projects/p1/state`.

**Code map and architecture:**
- rows for each new file;
- the `project-state` module row;
- `composition`'s new dependency;
- `web/src/app/project-state` under "Contracts".

## F-132 · The element panel and the ER diagram

**Needs:** F-131.

**`web/src/app/element-content.tsx`** (shared; added to the `shared` row's sources and entries):

```ts
export function ElementContent({ element, names }: {
  element: DesignElement
  /** An element id's name, when the caller knows it; the id is shown otherwise. */
  names?: (id: string) => string | undefined
}): JSX.Element
```

- **A table:**
  - its `purpose`;
  - a `<table aria-label="{name} columns">` with the headers Name, Type, Meaning, Key and
    References;
  - Key reads "primary key" when `primary_key`;
  - References reads `→ {names(table) ?? table}.{column}`.
- **A screen:**
  - its `purpose`;
  - `Route: {route}` when set;
  - a list "Leads to" of `{names(screen) ?? screen} when {when}`.
- **`deleted: true`:** the line "Deleted in this revision".

**`web/src/app/project-state/diagram-layout.ts`:**

```ts
export interface ElementNodeData extends Record<string, unknown> {
  view: ElementView
  /** The text lines the node shows under its name, decided here so its height is known. */
  rows: string[]
}
export type ElementNode = Node<ElementNodeData, 'element'>
export type DiagramEdge = Edge<{ label: string }>

/** Tables: one node per table, one edge per foreign key between two shown tables. */
export function tableDiagram(views: ElementView[]): { nodes: ElementNode[]; edges: DiagramEdge[] }
/** Screens: one node per screen, one edge per `leads_to` between two shown screens. */
export function screenDiagram(views: ElementView[]): { nodes: ElementNode[]; edges: DiagramEdge[] }
```

- **Input:** the shown views, already filtered by the caller. Node ids are element ids.
- **Table rows:** one per column,
  `{name} {type_name}{primary_key ? ' · key' : ''}{references ? ` → ${name of references.table, or the id}.${references.column}` : ''}`.
  - A table's edge is `{ id: `fk:${source}:${column}`, source, target: references.table, label:
    `${column} → ${targetName}.${references.column}` }`.
  - It is drawn only when the target is among the views and is not the source itself. The column
    row still names it.
- **Screen rows:** `[route ?? 'No route', purpose]`.
  - A screen's edge is `{ id: `link:${source}:${index}`, source, target: link.screen, label:
    link.when }`, only when the target is shown and is not the source.
- **Sizes:** tables are 260 wide, screens 240. The height is the sum of `NODE_LINES` in `diagram-layout.ts` (frame, name, id, badge, the behind line when shown, gap, and 18 per row), the same numbers the node draws with (fixed after the live run, `01f98d9`).
- **Placement:** one private `place(nodes, edges)` runs dagre with `rankdir: 'LR'`,
  `nodesep: 48`, `ranksep: 120`. It sets:
  - `width`, `height` and `measured`;
  - `position` as the top-left from dagre's centre;
  - `handles`: a target in the middle of the left side, a source in the middle of the right
    side, as `handles()` in `workflows/layout.ts` does.

  Deterministic: the same input gives the same output.

**`web/src/app/project-state/element-canvas.tsx`:**

```ts
export function ElementCanvas({ label, nodes, edges, selected, onOpen }: {
  label: string              // the section's aria-label: "ER diagram" or "Screen flow"
  nodes: ElementNode[]; edges: DiagramEdge[]
  selected?: string          // the open element's id, drawn selected
  onOpen: (elementId: string) => void   // called from onNodeClick
})
```

- **React Flow:** the props listed under "What is reused", with `style={THEME}` from
  `@/app/workflows/flow-theme`, `nodeTypes={{ element: ElementNodeView }}`, and edges with
  `label` from `data.label`, `markerEnd` an `ArrowClosed` as in `MapCanvas`.
- **`ElementNodeView`** draws `<article data-element={id} data-element-kind={kind}>` with:
  - the name;
  - `{id} r{revision} · as planned`;
  - `StateBadge`;
  - when `state.behind`, `built r{built_revision}, newer planned`;
  - the `rows`.
- **The canvas box** is `hidden h-[32rem] … sm:block`, as `MapCanvas`'s is.

**`web/src/app/project-state/element-panel.tsx`:**
`export function ElementPanel({ projectId, view, names, onClose }: { projectId: string; view:
ElementView; names: (id: string) => string | undefined; onClose: () => void })`, as `<aside
aria-label="Element {id}">`.

- **Header:** `{id} · {kind} · {name}`, `StateBadge`, and a `Button` "Close".
- **`why`,** when present.
- **Part:** its title from `useQuery(projectStateQuery(projectId))` (cached, shared), else the
  id, as a `Link` to `search={{ view: 'state', part }}`.
- **Outcomes:** each id as a `Link` to `/projects/$projectId/workspace` with
  `search={{ view: 'roadmap', outcome: id }}`. No outcome read.
- **Confirmed:**
  - `file_and_symbol` reads "File and symbol confirmed";
  - `file_only` reads "File confirmed".
- **"Built by" (`done`) and "Planned in" (`planned`):** one line per `ElementTask`,
  `T{task} · v{version} · r{revision} · {action} · {file}{symbol_id ? ` · ${symbol_id}` : ''}{symbol_state ? ` (${symbol_state})` : ''}`,
  linking to the task page. With none: "No task yet."
- **Revisions,** from `useQuery(elementRevisionsQuery(projectId, id))`:
  - loading: `SkeletonContent label="Loading revisions…" layout="detail"`;
  - error: `ErrorLine` (404 `NOT_FOUND`);
  - otherwise each revision oldest first, as a `<details>` titled
    `r{revision} · {created_at}{deleted ? ' · deleted' : ''}` holding `ElementContent`.
  - **When `state.behind`,** two blocks come above the list:
    - "Built (r{built_revision})" with `ElementContent` of that revision;
    - "Planned change (r{latest})" with the latest.

    Both come from the revisions read, with no extra read.

**`web/src/app/project-state/tables-view.tsx`:**
`export function TablesView({ projectId, element, removed }: { projectId: string; element?:
string; removed: boolean })`.

- **Reads:** `useQuery(elementsQuery(projectId))`.
- **Shown:** `element.content.kind === 'table'`, and `state.state !== 'removed'` unless
  `removed`.
- **`names`** resolves an id to `content.name` over every element of the read, removed ones
  included.
- **Loading:** `SkeletonContent label="Loading tables…" layout="graph"`.
- **Error:** `ErrorLine`.
- **No table** (before the removed filter): "No tables yet. The Planner writes them when a plan
  needs them."
- **Layout:** a grid like `PartsView`. On the left:
  - `ElementCanvas label="ER diagram"` over `tableDiagram(shown)`;
  - a `nav aria-label="Tables list"` with one row per shown table (`StateBadge`, id, name), each
    a `Link` with `element`;
  - the "Show removed" / "Hide removed" `Button`, only when some table is removed.

  On the right, `ElementPanel` for `element` when it names a table of the read, else the line
  "No table {element} in this project."
- **Opening:** `onOpen` and `onClose` navigate with `element` set or cleared (`useNavigate`).

**`state-page.tsx`:** adds the "Tables" tab and renders `TablesView` for `view === 'tables'`.

**Tests:**
- `web/src/app/project-state/diagram-layout.test.ts`:
  - `the_layout_is_deterministic_and_every_node_has_its_size`;
  - `a_reference_to_a_table_not_shown_draws_no_edge_but_keeps_its_row`;
  - `a_self_reference_draws_no_edge`;
  - `screen_links_become_edges_labelled_with_when`.
- `web/src/app/project-state/tables-view.test.tsx` (happy-dom, `startApp` at
  `/projects/p1/state?view=tables`):
  - `the_er_diagram_draws_one_node_per_table_and_one_edge_per_foreign_key`: E1 `users`, E2
    `sessions` (`user_id → E1.id`) and E3, a screen, give two `[data-element]` and one
    `.react-flow__edge`, and the list holds `users` and `sessions` only;
  - `a_removed_table_is_hidden_until_shown`;
  - `an_opened_table_shows_its_part_outcomes_tasks_and_revisions` (`&element=E2`: one
    `GET /api/projects/p1/elements/E2/revisions`, the part title, an outcome link, the done task
    line with file and symbol, `why`);
  - `a_behind_table_shows_its_built_shape_and_the_planned_change`;
  - `an_element_without_revisions_shows_cores_error` (404);
  - `no_tables_says_the_planner_writes_them`;
  - `a_design_element_change_reads_the_elements_and_its_revisions_again`: a live
    `DesignElementChanged` for E2 adds one elements read and one E2 revisions read.

**Code map and architecture:** a row for each new file. `app/element-content.tsx` joins the
`shared` row's sources and entries.

## F-133 · The screen flow

**Needs:** F-132.

**`web/src/app/project-state/screens-view.tsx`:**
`export function ScreensView({ projectId, element, removed }: { projectId: string; element?:
string; removed: boolean })`. It is `TablesView` with these differences:
- the filter is `kind === 'screen'`;
- `ElementCanvas label="Screen flow"` over `screenDiagram(shown)`;
- the list is `aria-label="Screens list"`;
- the empty line is "No screens yet. The Planner writes them when a plan needs them.";
- the unknown line is "No screen {element} in this project.";
- a link to a screen that is not shown is listed in the node's rows, not drawn.

Do not share a component between the two views by a flag. If the builder finds the two files
identical except for these values, they may share one private
`ElementsView({ kind, … })` in a file of its own, `elements-view.tsx`, and say so in the report.
That file is then in this task's Touch.

**`state-page.tsx`:** adds the "Screens" tab and renders `ScreensView` for `view === 'screens'`.

**Tests** (`web/src/app/project-state/screens-view.test.tsx`):
- `the_screen_flow_draws_screens_and_arrows_labelled_when`;
- `a_link_to_a_screen_not_shown_is_not_drawn`;
- `no_screens_says_the_planner_writes_them`;
- `an_opened_screen_shows_its_route_and_links_in_the_panel`.

## F-134 · Pinned elements in plan review and in the brief

**Needs:** F-132 and B-177 (the generated `TaskContract.elements` and `TaskContext.elements`).

**`web/src/app/workflows/inspect-panel.tsx`:**
- **When** `task.contract?.elements` is not empty, a `Section name="Tables and screens"
  badge={n}`.
- **The read:** `useQuery({ ...elementsQuery(plan.project_id), enabled: n > 0 })`.
- **One line per pin:**
  `{id} r{revision}{name ? ` · ${name}` : ''} · {action} · {file}{symbol_id ? ` · ${symbol_id}` : ''}`.
  - `name` is the `content.name` of the element whose `element.id === pin.id` in that read.
  - While the read loads or misses the id, the line shows the id alone.
- **Nothing else** in the panel changes.

**`web/src/app/tasks/brief-panel.tsx`:**
- **When** `context.elements` is present and not empty, a
  `<div aria-label="Pinned elements">` after "Resolved contracts".
- **Content:** the title "Tables and screens", then per element:
  - `{id} r{revision} · {kind} {name}`;
  - `ElementContent` with `names` over the brief's own elements.

  No extra read: the brief is the attempt's saved context.

**Tests:**
- `web/src/app/workflows/plan-page.test.tsx`:
  - `the_inspect_panel_lists_pinned_elements_by_name`;
  - `a_pin_whose_element_is_not_read_shows_its_id`.
- `web/src/app/tasks/task-page.test.tsx`:
  - `the_brief_lists_its_pinned_revisions_in_full`;
  - `a_brief_without_elements_has_no_pinned_section`.

**Contracts:**
- `web/src/app/workflows/contract.yaml` and `web/src/app/tasks/contract.yaml` gain one
  obligation each, with these test names.
- The `workflows` module may already depend on `api` and `shared`; no architecture change.

## F-135 · `code_where` in the web

**Needs:** F-132, and B-182, B-183 and B-185 built.

**First:** `npm run gen:api`.

**`web/src/api/code-record.ts`** (new). Its header comment: "Where a topic is in the project, as
CodeRecord answers it."

```ts
export type WhereAnswer = components['schemas']['WhereAnswer']
// …and WhereEntry, SymbolEntry, UsesLive, Live, UseLine, UnrecordedFile, Owner, FileTask, Rank.
export const codeRecordKeys = {
  where: (projectId: string, topic: string) => ['projects', projectId, 'code-record', 'where', topic],
}
export function codeWhereQuery(projectId: string, topic: string)
  // GET /api/projects/{id}/code-where, query { topic } exactly as typed; retry: retryLost;
  // staleTime: Infinity (asked again only on the person's "Ask again", never by the stream)
```

**`web/src/app/project-state/where-view.tsx`:**
`export function WhereView({ projectId, topic }: { projectId: string; topic?: string })`.

- **The form** (`<form aria-label="Where is it">`):
  - an `<input aria-label="Topic" dir="auto">`, whose initial value is `topic`;
  - a submit `Button` "Ask".
  - Submitting navigates with `search={{ view: 'where', topic: <the input's value> }}`, with no
    trimming and no length check.
- **The read:** `useQuery({ ...codeWhereQuery(projectId, topic ?? ''), enabled: topic !==
  undefined })`.
  - A `Button` "Ask again" calls `refetch()` and is shown once an answer or an error is there.
- **Loading:** `SkeletonContent label="Asking the record, then the code…" layout="rows"`.
- **Error:** `ErrorLine` under the form. A 422 shows Core's message; a 404 the same.
- **The answer:**
  - `{asked} language-server questions asked`;
  - `budget`, when present, as a `role="status"` line.
- **Entries,** in the order given (rendered as `answer.entries.map`, never sorted).
- **A file entry (`<article data-where-file={path}>`):**
  - `path` in mono;
  - the rank label: Exact match, Path, Part or Words;
  - `owner` as `{part_title}{layer ? ` · ${layer}` : ''}`;
  - its live line (below), and `confirmed` as in the element panel;
  - its tasks, newest first as given:
    `T{task} v{version} · {action} · {purpose}{commit ? ` · ${commit.slice(0, 8)}` : ''}`,
    linking to the task page;
  - its symbols (below).
- **The live line (`Live`):**
  - `confirmed` reads "Confirmed in the code";
  - `gap` reads `Gap: {why}`;
  - `unchecked` reads `Unchecked: {why}`;
  - `not_asked` shows nothing.
- **A symbol (`data-where-symbol={symbol_id}`):**
  - `{name} ({kind})`;
  - `At done: {at_done}`;
  - its live line;
  - `Line {line}` when `line` is set;
  - `behavior` as a list;
  - its uses by `uses_live.state`:

    | `uses_live.state` | Shows |
    |---|---|
    | `answered`, `uses` empty | "No uses." |
    | `answered` | `{uses_total} uses`, the `uses` grouped (below), then `{uses_more} more` when it is above 0 |
    | `truncated` | the `uses` grouped, then `at least {uses_more} more` when it is above 0, then `why` |
    | `failed` | `Uses not checked: {why}` |
    | `skipped` | `Uses not checked: {why}` |
    | `not_asked` | nothing |

    "No uses." appears for no other state.
  - **Grouped:** consecutive by `owner` as `{part_title}{layer ? ` · ${layer}` : ''}`, else "No
    part", keeping Core's order. Each use reads `{path}:{line}`.
- **An element entry:**
  - `StateBadge`;
  - `{id} · {kind} · {name}`, linking to the tables or the screens view with `element`;
  - `ElementContent`.
- **More entries:** `{entries_more} more entries` when it is above 0.
- **No entries:** "Shadows has no record of this."
- **"Not in the record"** (`aria-label="Not in the record"`): only when `not_in_record` is not
  empty, also with no entries. Each file: `path`, `owner`, symbols `{name}:{line}`. Then
  `{not_in_record_more} more files` when it is above 0.

**`state-page.tsx`:** adds the "Where" tab and renders `WhereView` for `view === 'where'` with
`topic`.

**Tests:**
- `web/src/api/code-record.test.ts`:
  - `the_topic_is_sent_as_typed` (`'  login '` reaches the URL encoded, untrimmed);
  - `the_question_retries_only_a_lost_call` (its `retry` is `retryLost`).
- `web/src/app/project-state/where-view.test.tsx` (happy-dom, `startApp` at
  `/projects/p1/state?view=where`):
  - `asking_reads_once_and_keeps_cores_order` (entries served `words` before `exact` stay in that
    order; one `GET` per submit);
  - `an_answered_empty_lookup_says_no_uses`;
  - `it.each(['failed', 'skipped'])('a_%s_lookup_says_why_and_never_no_uses')`;
  - `a_truncated_lookup_keeps_its_uses_and_says_at_least`;
  - `answered_uses_show_the_total_and_the_rest_grouped_by_part`;
  - `a_not_asked_symbol_shows_no_uses_line`;
  - `no_entry_says_shadows_has_no_record_with_files_not_in_the_record`;
  - `a_blank_topic_shows_cores_refusal` (422 `INVALID_COMMAND`);
  - `the_budget_line_is_shown_when_core_gives_one`;
  - `ask_again_reads_the_answer_again`;
  - `project_events_do_not_ask_again` (a live `TaskProgressChanged` adds no `code-where` call).

**Contracts:**
- `web/src/api/contract.yaml` gains `code_record:`;
- `web/src/app/project-state/contract.yaml` gains the `uses_live` obligation with these tests.

**Code map:** rows for the four new files.

## Not in these tasks

- **The plan page's progress** (`planProgressQuery`) and the task page are not refreshed by
  `TaskProgressChanged` here. The request asks only for the state and the elements reads. Using
  the event there is a later choice.
- **The live acceptance in a browser is B-181 and B-186.** The controller runs them on these
  screens with `shadows serve`.
