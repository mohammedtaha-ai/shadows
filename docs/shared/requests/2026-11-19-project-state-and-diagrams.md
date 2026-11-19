# The project's state, its ER diagram and its screen flow

- Status: answered
- Asked by: drafted by Hamoud for Masada at the controller's request, 2026-11-19 (Masada reviews the
  Need and Fields, and accepts or replies)
- Screen: the code record design §5.4
  (`docs/backend/specs/2026-11-19-project-code-record-design.md`); backend plan
  `docs/backend/plans/2026-11-19-code-record-2a.md` (B-177, B-180)

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
| project event stream (exists) | — | new kinds `DesignElementChanged { project_id, element_id, revision }` and `CodeRecordChanged { project_id, workflow_id, version, task }` |

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
- **Refresh:** `DesignElementChanged`, `CodeRecordChanged`, `WorkflowFrozen`, `PlanArchived` and
  `PlanUnarchived` refresh the state and the elements. `DesignElementChanged` also refreshes that
  element's revisions.

## Errors

| When | The screen shows |
|---|---|
| unknown or removed project (404) | the project's not-found state, as the decisions route does |
| `part_id` is not a part of the project (422 `INVALID_COMMAND`) | the part view's error line with the message |
| an element id with no revisions (404 `NOT_FOUND`) | the element panel's error line |
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

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
