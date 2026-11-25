# The project's state, its ER diagram and its screen flow

- Status: accepted
- Asked by: drafted by Hamoud for Masada at the controller's request, 2026-11-19 (Masada reviews the
  Need and Fields, and accepts or replies)
- Screen: the code record design §5.4
  (`docs/backend/specs/2026-11-19-project-code-record-design.md`); backend plan
  `docs/backend/plans/2026-11-19-code-record-2a.md` at `4d69977d` (B-177, B-180)

## Need

**What the person does.** On a project, the person opens three read-only views:
- **The project's state:** its stage, each map part with its plans and their tasks (done, started,
  not started), its tables and screens counted by state, and the gaps.
- **The ER diagram:** the project's tables, their columns and keys, and the relations their
  foreign keys draw.
- **The screen flow:** the project's screens and the arrows "leads to … when …".

**On each element.** It shows its state: planned ⏳, built ✅, gap ⚠, unchecked ?, or removed,
which is hidden by default. Opened, it shows:
- its revisions and its part;
- the roadmap outcomes it serves;
- the tasks that built it or will, each with plan, version, task, file and symbol;
- why it is a gap.

A part's page shows that part's elements.

**Plan review and the task brief.** A task's contract shows the elements it pins (E3 r2, create,
file, symbol), resolved to their names. The executor's brief lists the pinned revisions in full.

**Refreshing.** The views refresh from the project event stream. Nothing is edited here: tables and
screens are written by the Planner, and a hand editor is later work on the same record.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/projects/{id}/state` | query `part_id?: string` | `ProjectStateView` (below) |
| `GET /api/projects/{id}/elements` | — | `ElementView[]`, by element number, at most 500 |
| `GET /api/projects/{id}/elements/{element}/revisions` | — | `DesignElement[]`, oldest first |
| `GET /api/workflows/{id}` (exists) | — | `tasks[i].contract.elements?: ElementPin[]` (new; absent when empty) |
| `GET /api/attempts/{id}/brief` and the task context (exist) | — | `elements?: DesignElement[]` (new; absent when empty): each pinned revision |
| project event stream (exists) | — | new kinds `DesignElementChanged { project_id, element_id, revision }`, `CodeRecordChanged { project_id, workflow_id, version, task }` and `TaskProgressChanged { project_id, workflow_id, version, task, attempt_id: string\|null }` |

Shapes (exact names; `?` is absent when empty or none):

```text
ProjectStateView {
  project_id, stage: "idea"|"vision"|"map"|"roadmap"|"plans", missing: string[], map_revision,
  parts: PartState[] (map order, ≤ 200), parts_more: number,
  unassigned: PlanState[] (≤ 20), unassigned_more,
  gaps: ElementGap[] (≤ 100), gaps_more,
  part?: PartDetail (only with part_id)
}
PartState  { part_id, parent: string|null, title, kind: string|null,
             plans: PlanState[] (≤ 20), plans_more, elements: StateCounts }
PlanState  { plan_id, title, workflow_id: string|null, version: number|null,
             draft: string|null, tasks: { total, done, started, not_started } }
StateCounts { planned, built, gap, unchecked }
ElementGap { element_id, kind: "table"|"screen", name, part_id, state: "gap"|"unchecked", why }
PartDetail { part_id, plans: PlanTasks[] (≤ 50), plans_more, elements: ElementView[] }
PlanTasks  { plan_id, title, workflow_id, version,
             tasks: { number, title, done, attempt: AttemptState|null }[] }
ElementView { element: DesignElement, state: ElementState }
DesignElement { id: "E3", revision, project_id, part_id, outcomes: string[], deleted: boolean,
  written_by, created_at,
  content: { kind: "table", name, purpose,
             columns: { name, type_name, meaning, primary_key, references?: { table, column } }[] }
         | { kind: "screen", name, purpose, route?, leads_to: { screen, when }[] } }
ElementState { element_id, state: "planned"|"built"|"gap"|"unchecked"|"removed",
  built_revision: number|null, behind: boolean, confirmed?: "file_and_symbol"|"file_only",
  done: ElementTask[], planned: ElementTask[], why? }
ElementTask { workflow_id, version, task, revision, action: "create"|"modify"|"delete", file,
  symbol_id?, symbol_state?: "found"|"missing"|"unchecked" }
ElementPin { id, revision, action: "create"|"modify"|"delete", file, symbol_id? }
```

Reading the shapes:
- **Columns and links** are the element's latest revision, shown "as planned". Only the file and
  the symbol are confirmed (`confirmed`).
- **`behind: true`** means a newer revision is still to be built: show the built revision's shape,
  with the newer one as a planned change (from the revisions route).
- **Foreign keys** name the target table by element id; links name the target screen by id.
- **Refresh:** each of these events invalidates the state and the elements reads (the state with
  and without `part_id`):
  - `DesignElementChanged`, which also invalidates that element's revisions;
  - `CodeRecordChanged`;
  - `TaskProgressChanged`;
  - the design, stage and plan notifications the project stream carries today:
    `ProjectDesignChanged`, `MapProposed`, `RoadmapProposed`, `RoadmapApproved`,
    `WorkflowDraftStarted`, `WorkflowEdited`, `WorkflowFrozen`, `PlanArchived`, `PlanUnarchived`.

  At caught-up they are reconciled once. The work goes through `web/src/stream/` and the API-owned
  query keys, with no extra HTTP surface. No view subscribes to an executor's thread for progress.

## Errors

| When | The screen shows |
|---|---|
| unknown or removed project (404) | the project's not-found state, as the decisions route does |
| `part_id` is not a part of the project (422 `INVALID_COMMAND`) | the part view's error line with the message |
| an element id with no revisions (200 with `[]`) | the element panel with no revision listed |
| a project with no elements | the diagrams' empty state: "No tables yet" / "No screens yet", and that the Planner writes them when a plan needs them |

## Backend answer

As written above. The backend drafted it from the ruled design, so these are the shapes the backend
builds:
- `ProjectState` (new crate `project-state`, B-179) composes Design, Plans, Runs and CodeRecord; its
  owner was approved by Mohammed on 2026-11-19. Its methods `state` and `elements` serve the first two
  routes and the Planner's `project_state` tool.
- `Design::element_revisions` serves the third route.
- `TaskContract.elements` and `TaskContext.elements` are Plans' (B-177).
- The two event kinds are journaled by Design and CodeRecord in their own writes and carried by
  Events' project tail (B-180).
- Contracts touched: `crates/core-design`, `crates/core-plans`, `crates/code-record`,
  `crates/project-state` (new), `crates/core-events`, `crates/shadows-http/src/contract.yaml`.

Nothing changes in `api/openapi.json` before this request is accepted. B-177 and B-180 wait for it.

**Response to the frontend reply (Hamoud, 2026-11-20).** The blocker is accepted (Mohammed and the
controller) and resolved as follows.
- **The event.** No existing attempt event can serve:
  - Runs journals none today, only `TaskExecutorChosen`.
  - An executor thread's operation events do not carry the attempt's state, which Runs decides
    after the turn.

  So Runs journals a new project-scoped kind, `TaskProgressChanged { project_id, workflow_id,
  version, task, attempt_id }`, in every write that changes what `Runs::progress` answers:
  - an attempt inserted (Start, an addition, a session that did not open);
  - its report;
  - its end;
  - a Stop;
  - a startup's interruption;
  - a done or a not-done mark, with `attempt_id` null.

  A replay journals nothing.
- **Where it is decided and built:**
  - owner rule: executors design, "The plan page";
  - the journaling: plan 2a, B-187, new;
  - the project tail: B-180, which now adds all three kinds.
- **The test you asked for:**
  `runs_routes::an_attempt_starting_and_ending_reaches_the_project_stream_and_the_state` (B-180).
  It reads two `TaskProgressChanged` frames, then the state with `started: 1, not_started: 0` and
  the task line `attempt: "failed"`. The running state between them is held by B-179's
  `a_running_attempt_shows_started_with_its_state`.
- **Your invalidation:** recorded under "Refresh" above.
- **Code record design §5.4:** lists the events.

## Frontend reply

Reviewed as Masada, 2026-11-20, against the owner design §5.1–§5.4 and §8, plan 2a
(B-177–B-180), and the existing API and project-stream ownership. The read-only routes, pinned
revisions, server-owned element states, bounded lists with remainder counts, and error shapes
meet the frontend need. The frontend will resolve names and built revisions through the reads,
show columns and links as planned, and generate its schema only after the backend is built.

**Blocker: task progress has no project-stream refresh on an attempt starting.** The overview
contains `started` and `not_started`, and a part's task rows contain the latest `AttemptState`.
`EventsStore::read_project_events_after` currently excludes attempt lifecycle events. B-180 only
adds `DesignElementChanged` and `CodeRecordChanged`; B-178 emits the latter on done/not-done
writes. Starting a first attempt therefore changes the state response without any of the
promised refresh events. An open project view would keep saying "not started" until another
event or a new read happens. Attempt completion or interruption can likewise leave the part's
attempt state stale before a done mark.

Before acceptance, specify a project-scoped notification for every change to the progress this
response exposes: its kind and identifier payload, the owner that journals it, and its inclusion
in the project tail. Reusing an existing attempt event is sufficient if it reaches that tail;
the frontend must not subscribe separately to every task's executor thread. Add a project-stream
test that observes an attempt starting and then ending without a done mark, with state reads
showing the updated counts and attempt state. Amend the owning plan/contract accordingly.

The frontend refresh task must also invalidate these reads for the existing design/stage and
plan notifications, and reconcile them at caught-up, through `web/src/stream/` and API-owned
query keys. That follows the current client architecture; it needs no additional HTTP surface.
This is an agreement review, not confirmation that the backend or frontend is built. The
request remains disputed for the controller's ruling under the shared-request cycle.

**Re-review (Masada, 2026-11-20, after `3f50f96`): accepted.** The blocker is resolved:
- **Every progress change has a project-scoped notice.** `TaskProgressChanged { project_id,
  workflow_id, version, task, attempt_id }` is journaled by Runs in the same write as each change
  `Runs::progress` answers (attempt insert, report, end, Stop, startup interruption, done and
  not-done marks), never on a replay (plan 2a Ruling 11, B-187; executors design, "The plan
  page"). That covers both the overview's `started`/`not_started` and the part detail's
  `attempt`.
- **It reaches the tail.** B-180 adds all three kinds to `read_project_events_after`, with the
  stream test asked for (`runs_routes::an_attempt_starting_and_ending_reaches_the_project_stream_and_the_state`)
  and B-179's `a_running_attempt_shows_started_with_its_state` for the running state.
- **The payload is identifiers only,** so the client invalidates and reads again; no view follows
  an executor's thread.

The frontend tasks are F-130 to F-134 in `docs/frontend/stages.md` (plan
`docs/frontend/plans/2026-11-20-project-state-views.md` at `4d69977d`), blocked until B-177 to B-180 and B-187
are built and `api/openapi.json` shows them.

## Ruling

<The teacher, only when disputed.>
