# Project structure and layers

Status: revised and approved 2026-11-06. Slices 1 and 2 are built
(PRs #59 and #60). Slices 3 to 5 are built on `feature/structure-elements`
(B-71 to B-81, F-92 to F-94). Mohammed owns their pending manual
acceptance on an existing project; real-provider acceptance is not
recorded.

**Mohammed's decisions.**
- **2026-11-05**
  - **No backward compatibility.** Every project made so far is test data.
    The daemon must still start on an existing database, and its migration
    may convert old rows crudely.
  - **Shadows never writes `contract.yaml`.** Files already on disk are left
    alone.
  - **The task view and Show phases (§6.2)** are approved as drawn.
- **2026-11-06: the standards hold principles, not a layout.**
  - Each part kind has a presence rule (required, waivable with a reason,
    or optional) and a list of layers. Each layer has a role, and the
    layers it may use. That is the whole catalog.
  - Every project follows one runtime rule: each runnable part has one
    owner object (its AppCore). It builds every service once and is the one
    way in from outside. One rule lives in one place, with no duplicate way
    in.
  - Where the folders go is decided by the Planner and the person, for the
    stack they chose. Laravel, FastAPI and Node each have their own layout,
    and their own tools create it. Shadows does not dictate it.
  - Shadows' job is to hold the Planner to the principles and to warn when
    a map breaks them.
  - Elements, planning detail per element and the trace view are dropped.
    They planned "every file", which this decision rejects.
  - **No command writes the structure.** The approved map records the
    layout. An executor writes it as a task, or the Planner writes it in
    Agent Mode, and the layout may still change after that. Create
    structure, its planning step and `SHADOWS.md` are removed (§5.4).

**Controller's calls (2026-11-06), approved by Mohammed.** Each one is marked
"(call)" where it is used: C1 in §5.3, C3 in §5.2, C4 in §7.

**Words used with one meaning each:**

| Word | Meaning |
|---|---|
| **Planning step** | Shadows' guided steps: Idea, Vision, Map, Roadmap, Plans. |
| **Project stage** | A roadmap outcome of the person's project, such as "Stage 1: working support loop". |
| **Delivery slice** | A numbered piece of this work in the plan. |

Plan: [../plans/2026-11-05-structure-and-depth.md](../plans/2026-11-05-structure-and-depth.md).
The mockup in [2026-11-05-structure-and-depth-mockup/](2026-11-05-structure-and-depth-mockup/README.md)
predates the 2026-11-06 decision. Its element, depth and trace screens are
not part of this design. Its task view and Show phases screen still are.

## 1. Why

**2026-11-05: no structure at all.** Two test projects showed that Shadows
did not hold the structure of the projects it planned.
- **testing55.** The standards said "one core; every operation is a method
  of one service". The Planner read that literally, and every folder landed
  in the project root as one block.
- **testing56.** One folder per part came from the model, not from
  Shadows.
- **No inside.** In both projects the backend had no layers and no modules.
  The database part was a `contract.yaml` and nothing else.
- **`contract.yaml` duplicated Shadows.**
- **Bug.** Map approval accepted a part with no folder, and Create
  structure then refused it.

Slices 1 and 2 fixed these. Each part now has one folder, each part kind
has layers, and the map is checked on every write path.

**2026-11-06: now too strict.** testing1 (a Laravel support system) showed
the other side.
- The standards and the Planner prompt drove one layout for every stack:
  `backend/src/modules/{module}/domain`, a `database/` part in the project
  root, and an `integration/` part in the root.
- Laravel already has its own layout. `composer create-project` makes
  `app/`, `config/`, `database/migrations/` and `routes/`, and
  `php artisan migrate` reads migrations from inside the application. An
  outside review of the map said the same thing: put migrations and adapters
  inside the backend, and keep the module boundaries.
- Create structure made layer folders before the framework ran. A scaffold
  such as `composer create-project` needs an empty or missing folder. And a
  layout written once by a command is stale as soon as the project grows.
- What the review praised is the principle, not the folders: domain
  modules, one API contract, and a composition root with no business logic.

## 2. Goals

1. **The standards name each part's layers as roles, with the direction of
   dependency,** and one runtime rule: one owner object per runnable part,
   which builds the services and is the one way in from outside.
2. **Every part has one folder, and every required layer is accounted
   for:** the map says where it lives in this stack, or why it is not used.
3. **The layout follows the stack.** The Planner proposes it with the
   person, starting from the framework's own layout. Shadows checks the
   principles, never a folder pattern.
4. **The building agent asks Shadows where to work** (`task_context`),
   instead of reading per-part files.
5. **A task has phases,** and a phase can wait for a phase of another task.

## 3. Non-goals

- No folder layout per framework in Shadows.
- No element model: no tables, use cases, endpoints or symbols recorded in
  Shadows.
- No code generation, no scaffolding, and no command that writes folders.
  The framework's own tools and the agents create the project.
- No compatibility layer for existing projects.

## 4. Anti-goals

These would count as failure even if everything works:

- **A plan longer than the code it describes.**
- **Clean Architecture forced word for word on a stack that does not fit
  it,** such as a separate entity class beside every Laravel Eloquent model.
- **Shadows refusing a sound map because of where a folder sits.**
- **Shadows writing folders on disk.** A framework's scaffold refuses to
  run in them, and they go stale as the layout changes.

## 5. The model

```text
Project
 └─ Part          a section with one folder; it may contain child parts
     └─ Layer     a role from the standards' catalog, with where it lives
Plan version
 └─ Task          its paths, acceptance and the parts and layers it touches
     └─ Phase     an ordered step of the task; may wait for a phase elsewhere
```

### 5.1 Parts

- **They stay as they are:** title, responsibility, design, kind, serves,
  one folder and layers.
- **One folder per part.** A child part's folder lies inside its parent's
  folder, and two parts never claim the same folder. One `MapRules` policy
  checks proposal, approval and person edits (slice 1).
- **Nesting is how a stack's layout is expressed.** In Laravel, the
  database part can be a child of the backend, with the folder
  `backend/database`. In a project where the database code stands apart, it
  can be a root part. Both are valid maps.
- **A grouping part** has a child of its own kind, such as a `frontend`
  holding `staff-app` and `supervisor-dashboard`. It needs no layers, and
  nothing new is stored.
- **Integration is optional.** Adapters to outside services can live as a
  layer place inside the backend (for Laravel,
  `backend/app/Infrastructure/Telegram`), or as their own part when they run
  separately.

### 5.2 The standards catalog (base v3)

Each part kind has:
- `name`, and `owns`: one sentence;
- `presence`: `required`, `waivable` (with a reason the person approves) or
  `optional`;
- `layers`, each with `id`, `role` (one sentence), `required`, and
  `may_use` (the layers it may depend on).

**The layer catalog's `elements` field is removed** (call C3). It named
element kinds for the element model, which is dropped, and nothing else
reads it.

The kinds and layers are those of base v2, minus `elements`:

| Kind | Presence | Layers (required unless marked) |
|---|---|---|
| backend | required | domain, application, interface, infrastructure, composition |
| database | required | schema, migrations, connection |
| api | waivable | contract |
| frontend | waivable | screens, features, data_access, design_system (optional) |
| integration | optional | adapters |

The roles and `may_use` lists are as in base v2:
- the domain depends on nothing;
- infrastructure implements the application's ports;
- composition builds every module once;
- the frontend's `data_access` is the only code that calls the api.

**The rules.** S1 to S5, S7 and S8 read as in base v2. S6, S9 and S10
read:

- **S6. The database code is its own part or layer.** Business logic
  reaches the database only through the layer that owns it. A change of
  database stays in that owner. It needs no folder of its own.
- **S9. One folder per part; layers follow the stack.** Each part owns one
  folder. Each layer records where it lives, in the layout the framework
  and the person chose.
- **S10. One owner at runtime.** Each runnable part has one owner object
  (its AppCore). It builds every service once and is the one way in from
  outside: an adapter such as an HTTP route or a CLI reaches a service only
  through it. Services reach each other through the dependencies they
  declare, never through the owner, so it does not become a go-between for
  every internal call. A rule lives in one service, and there is no second
  way in.

The base standards are compiled in. Base v3 replaces v2 for every project,
and needs no migration, because no stored row holds `elements`.

### 5.3 Layers in a project

**At the map stage**, the map card proposes each part with its layers:
- every required layer of its kind, and any optional layer the person
  wants;
- each with its **place**: where the layer lives in this stack, as a
  project-relative path. The `{module}` marker may stand for a module name
  anywhere in the path, for example `backend/app/Modules/{module}/Domain`.
- or, for a required layer the project does not use, **not used** with a
  reason the person approves.

**Shadows refuses the map only on principle** (call C1):

| Refusal | When |
|---|---|
| `missing_layer` | A required layer has neither a place nor a not-used reason |
| `unknown_layer` | A layer is not in its kind's catalog |
| `layer_not_used_without_reason` | `not_used` is blank, or set while a place is given |
| `layer_place_shared` | Two layers of one part have the same place (case-insensitive), so their code would mix |
| `invalid_path` | A place is absolute, holds `..` or `\`, or is empty while used |

**Where a place lies is the Planner's and the person's choice.** Shadows
does not refuse a place for where it lies:
- A place may lie outside its part's folder. A Laravel database part at
  `backend/database` has its connection in `backend/config/database.php`,
  which is outside the part's folder and still right.
- A place may lie inside a child part's folder.
- Places are not checked for pattern overlap or containment, and a place
  may hold `{module}` more than once. `{module}` is text that the Planner
  and the agent read. It is not a path Shadows creates.

### 5.4 Who writes the structure

**No command writes it** (Mohammed, 2026-11-06). The approved map is the
record of the layout. The folders are written by whoever builds:
- an executor, as a task of a plan, for example "T1: run
  `composer create-project` in `backend/` and set up the layers";
- or the Planner, in Agent Mode, when the person asks.

The layout may change after that. The person and the Planner change the
map, and the next task follows it. Agents learn the layout from
`task_context` (§5.5), not from a file on disk.

**Removed:**
- the Create structure button, its route and `Design::create_structure`;
- the Structure planning step. Roadmap opens once the map is approved;
- `SHADOWS.md` and every structure intent. The `design_structure_write`
  table is dropped. Nothing on disk is deleted;
- the map card's **starter files**. The scaffold writes `composer.json`,
  `package.json` and `pubspec.yaml` itself, with the right content for the
  framework version. The part content's `files` field is removed.

### 5.5 Plans, tasks and phases

**`task_context`: the building agent's question.** It takes a task of an
approved plan version, and answers from records Shadows already holds:
- the task's title, purpose, acceptance and paths;
- the parts that own its paths: the deepest part whose folder holds a
  path, and every part with a layer whose place names it. A Laravel
  database part owns `backend/config/database.php` through its connection
  layer, though the file is in the backend's folder. Each part comes with
  its folder, its layers with places and not-used reasons, and how it owns
  each path;
- the layers' `may_use` and the rules (S1 to S10) that apply to those
  parts;
- the plan and standards versions it read.

It grants no execution authority. Execution, attempts and evidence stay with
§17. For a task with no path in any part's folder, it says so, and never
guesses an owner.

**A task gains `phases`:** an ordered list. Each phase has:
- a stable id, a display `number`, a `title`, a `goal` and acceptance item
  references;
- and `waits_for`, a list of `{task, phase}`. The target may be in this
  plan, or in another plan of this project at an exact approved version.

A task's phases are written as one ordered list. Adding, removing and
reordering are one edit, and each phase keeps its id at its new position.

**Phase rules:**
- A task's acceptance items still decide when the task is done. Phases give
  the order inside it.
- Ordered phases imply completion-to-start edges, and `waits_for` means the
  same. The existing task-level `needs` and `completes_after` remain. One
  event graph checks all of them, using the start/complete model in
  `core-plans`.
- Phase links never form a cycle across the combined graph. A refusal names
  the whole path. A missing target is a visible blocker.
- Phase numbers are labels, not identities. A rename or reorder never
  redirects a wait.
- Phase status belongs to execution records (§17). A planning checkbox or
  an agent's statement cannot satisfy a wait.
- `task_context` takes an optional phase id once phases exist.

## 6. What the person sees

### 6.1 The map (built in slice 2)

- The Project map has a canvas and a list. Each node shows its kind, folder
  and layers with their places.
- Edges say "Dependencies not recorded". An optional Allowed dependencies
  overlay is dashed and labelled as permission.
- The part form asks, for each layer, **Used** (with its place) or **Not
  used** (with a reason). A grouping part shows "Groups: …" with links.

**Changes in slice 3:**
- the form's place input says the path follows the stack's own layout;
- the Standards page drops the element kinds and shows S10;
- the Create structure panel and the Structure step go. The stage line
  reads Idea, Vision, Map, Roadmap, Plans.

### 6.2 Plan view and task view (approved 2026-11-05)

- The plan stays the graph it is today.
- Clicking a task opens the **task view**: the same kind of graph, one
  level down. Its nodes are phases, drawn as task cards are. The task is
  the start node, and the edges carry labels. The view keeps the legend,
  zoom controls and minimap.
- A phase of another task that this one waits for, or that waits for it,
  appears as a dashed node with its task number. Clicking it opens that
  task's view.
- **Show phases** is a button in the plan header, beside Versions. It opens
  every task at once, in the same graph:
  - each task becomes a frame titled with its number and title;
  - its phases sit inside the frame as cards, each with a badge such as
    `T3 · P2`;
  - edges run phase to phase, for example `T1 · P3` → `T3 · P3`;
  - **Hide phases** returns the graph to task cards;
  - the minimap and zoom stay.
- Task-level edges stay drawn when phases are expanded. Tasks with no
  phases stay visible. Arrows point from prerequisite to dependent.
- Selection, filters and viewport survive expand and collapse. Foreign
  phases show their plan version. A list offers the same operations.

### 6.3 Interaction acceptance

- URLs identify the selected part, plan version and task.
- Browser Back restores the prior selection. Escape closes a panel before
  navigating up.
- Items are keyboard-reachable, colour states have text labels, and the
  views use the existing light/dark tokens.
- Large graphs and narrow screens have a list fallback.
- Loading, empty and failed reads have distinct messages.

## 7. The Planner's instructions

The prompt's map section says (call C4):
- The standards give roles, dependency direction and the one-owner rule.
  They are not a folder layout.
- Agree the layout with the person for the stack you chose. Start from the
  framework's own layout and its scaffold command, and map each layer to
  where it lives there.
- Nest a part inside another when the framework keeps it there, such as a
  Laravel database part at `backend/database`.
- Do not record layer folders the framework does not use. Mark a required
  layer not used, with a reason, when the stack has no place for it.
- Shadows writes no folders. The layout is built by a plan's tasks, usually
  starting with the framework's scaffold in each part's folder, or by you in
  Agent Mode when the person asks.
- When the layout changes during the work, propose the map change, so the
  record stays true.

The standards text the Planner reads drops the element kinds and adds S10.

**External agents (project grant)** get `task_context`.

## 8. Owners in Shadows' code

| Concern | Owner |
|---|---|
| Layer catalog and rules | `core-standards` |
| Parts and their layers | `core-design` |
| Folder and layer rules | `MapRules` |
| `task_context`, phases and the combined graph | `core-plans`, reading `Design` through its API |

No new service is needed.

## 9. Delivery in slices

1. **One folder per part, and no contract files.** Built (PR #59).
2. **Standards v2 and layers on the map.** Built (PR #60).
3. **Principles, not layout.** Base v3 (no `elements`, S6, S9, S10), the
   loosened place rules, the removal of Create structure, the Structure
   step and starter files, and the Planner's map instructions. The web
   changes follow.
4. **`task_context`.** The building agent's read, over MCP for both grants.
5. **Phases** inside tasks, with links across tasks and plans, the task
   view and Show phases.

**Dropped (2026-11-06):** elements, planning detail per element, the part
summary built on elements, the element approval gate, and the trace view.

**Deferred, not planned:**
- **Outcome coverage** ("is Stage 1 fully planned?"), built on tasks and
  their acceptance rather than elements. It waits for slice 5.
- **Code observation:** checking that the code respects the layers'
  `may_use` and the one-owner rule. It waits for the code index's
  languages.

## 10. Open questions

1. **Where an external agent's proposal cards appear.** A thread is needed
   today. This closes when the external-agent design is taken up with the
   global connection (deferred).
2. **OPEN: execution integration.** Before phase completion can be shown,
   §17 must supply attempt and result identity and their owner APIs. Until
   then, phases show no completion.

## 11. Acceptance examples

- **The daemon on the daily database** after the slice 2 migration (done).
- **A Laravel project:**
  - the database part is a child of the backend;
  - the connection layer's place is `backend/config/database.php`;
  - the map is approved, and Roadmap opens with no Structure step;
  - a plan's first task, or the Planner in Agent Mode, runs
    `composer create-project` in `backend/`.
- **A FastAPI project** whose domain and infrastructure places follow its
  own package layout, approved without a folder refusal.
- **Refusals:** a map with a required layer neither placed nor marked not
  used is refused. So is a map with two layers at one place.
- **A fresh agent** reads `task_context` for a task and names the part, its
  layers and the rules, without the old chat.
- **A mixed task and phase cycle** is refused, naming the whole path.
