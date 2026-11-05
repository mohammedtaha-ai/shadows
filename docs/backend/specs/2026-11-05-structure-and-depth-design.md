# Project structure, layers and planning depth

Status: draft for Mohammed's review, 2026-11-05. Nothing here is built.
Mockup: https://claude.ai/artifact/No9kfXkwK12wctXundandy (four screens).
Its source files are in
[2026-11-05-structure-and-depth-mockup/](2026-11-05-structure-and-depth-mockup/README.md).
Plan: [../plans/2026-11-05-structure-and-depth.md](../plans/2026-11-05-structure-and-depth.md).

## 1. Why

Two test projects showed that Shadows does not hold the structure of the
projects it plans.

- **testing55 (Haiku 5.5).** The standards say the backend is "one core;
  every operation is a method of one service". The Planner read that
  literally. Every folder (`app`, `config`, `routes`, `migrations`, `lib`,
  `dashboard`, `telegram`) landed in the project root, as one application.
- **testing56 (Sonnet 5.5).** The same standards (base v1, no additions)
  gave one folder per part. That came from the model, not from Shadows. The
  only folder rule Shadows holds is that two parts' folders do not overlap
  (`MapRules`, `map_rules.rs`).
- **No section has an inside.** In both projects the backend is one block:
  no layers, no modules, and Telegram, Firebase and Mercure all sit inside
  it. The database part is a `contract.yaml` and nothing else. It has no
  migrations folder, no schema, and no connection code beneath the others.
- **A plan does not say how deep its part was designed.** "T1: Database
  tables for the support loop" lists five acceptance items. The tables,
  columns and relations were never discussed or drawn. What Shadows calls a
  plan is closer to a map of work than a plan of code.
- **`contract.yaml` files duplicate Shadows.** Shadows writes one per part.
  Each file is a copy of the map, and Shadows is the source anyway.
- **Bug.** Map approval accepted a part with no folder (Frontend). Create
  structure then refused it ("part has no folder: Frontend"). These are two
  paths with two answers for one rule.

## 2. Goals

1. **The standards name each section's layers as roles, with the direction
   of dependency.** They do not name folder layouts. A model cannot read
   them as "one block".
2. **Every part and every layer owns one folder, inside its parent's
   folder.** Shadows refuses a map that breaks this, at proposal time.
3. **The database part sits beneath everything.** It holds the schema,
   migrations and connection. Repository implementations live in the
   backend's infrastructure layer (Mohammed's decision, 2026-11-05). A
   change of database touches the database part and that one layer. The
   domain and application layers never change for it.
4. **The person sees the project as a drawing at every depth:**
   project → part → layer → element. Clicking zooms in. A breadcrumb or
   Escape zooms out.
5. **Shadows measures how much of each part is planned,** layer by layer,
   from facts it holds. It does not take the model's word.
6. **A task names the elements it builds, and it has phases.** A phase can
   wait for a phase of another task.
7. **Shadows stops writing `contract.yaml`.** An agent asks Shadows over MCP
   where something is and what it does.

## 3. Non-goals

- Shadows does not dictate folder names per framework. The standards give
  roles; the map stage maps roles to folders with the person.
- No code generation. Elements are names, files and signatures, never
  bodies.
- No new stage in the stage line. Depth work happens after the map, through
  cards, like every other write.
- Not every architectural pattern becomes a rule. The base standards hold
  the few that, broken, cost the most (section 5).

## 4. Anti-goals

These would count as failure even if everything technically works:

- **A plan longer than the code it describes.** Elements stop at the file
  and its symbols.
- **Clean Architecture forced word for word on a stack that does not fit
  it.** For example, a separate entity class beside every Laravel Eloquent
  model. The standards fix roles and dependency direction, and nothing more.
- **A depth number the model can raise by saying so.**

## 5. The model

Four levels. The first exists today; the others are new.

```text
Project
 └─ Part          a section: backend, database, api, frontend, integration
     └─ Layer     a role inside a part, from the standards' catalog
         └─ Element  a named thing in a layer: table, entity, use case,
                     endpoint, screen, adapter ... with its file and symbols
Plan
 └─ Task          names the elements it builds or changes
     └─ Phase     an ordered step of the task; may wait for a phase elsewhere
```

### 5.1 Parts

- **They stay as they are:** title, responsibility, design, kind and serves.
- **`folders` becomes `folder`: exactly one, required.**
  - A child part's folder lies inside its parent's folder.
  - The rule moves into map proposal and map approval. Create structure
    keeps it only as a check against the disk. This fixes the bug in
    section 1 (one rule, one owner).
- **New kind: `integration`.** It is for adapters to outside services such
  as Telegram, Firebase and Mercure. It is optional; a project may also
  keep them as a layer of the backend (5.2).
- **The contract head (header and rule ids) stays in Shadows.** It is shown
  on the part page and given over MCP, but no longer written to disk.

### 5.2 Layers: the standards catalog (base v2)

The standards file gains a `layers` list per kind. Each layer has:

- `id`
- `role`: one sentence
- `required`: true or false
- `elements`: the element kinds it holds
- `may_use`: the layers it may depend on

Dependencies point inward. The catalog is data, so a project's standards
additions can add a layer or make one required.

**Backend**

| Layer | Role | Required | Elements | May use |
|---|---|---|---|---|
| `domain` | Business rules and the state they guard | yes | entity, value, rule, domain_event | (nothing) |
| `application` | Use cases: one operation each, and the ports it needs | yes | use_case, port | domain |
| `interface` | Receives HTTP/queue/console input and calls one use case | yes | endpoint, job, command | application |
| `infrastructure` | Implements the application's ports: repositories over the database, and technical adapters such as mail, files and clock | yes | repository, adapter | application, database.schema, database.connection |
| `composition` | Builds every module once and wires ports to implementations | yes | module | all backend layers |

**Database:** beneath everything.

| Layer | Role | Required | Elements | May use |
|---|---|---|---|---|
| `schema` | Tables, columns, keys and relations | yes | table | (nothing) |
| `migrations` | Append-only changes that produce the schema | yes | migration | schema |
| `connection` | How the application connects: driver, pool, transactions | yes | connection | (nothing) |

The layers are roles from the Clean Architecture explanation Mohammed
brought, not a fixed folder layout. A stack maps them at the map stage
(5.3).

**Api**

| Layer | Role | Required | Elements | May use |
|---|---|---|---|---|
| `contract` | The versioned contract the clients build against | yes | operation, schema_type | backend.interface |

**Frontend:** one part per client.

| Layer | Role | Required | Elements | May use |
|---|---|---|---|---|
| `screens` | What the person sees: routes and pages | yes | screen | features, design_system |
| `features` | A feature's interactions and its state | yes | feature, store | data_access |
| `data_access` | The only code that calls the api | yes | client_call | api.contract |
| `design_system` | Shared components and tokens | no | component | (nothing) |

**Integration**

| Layer | Role | Required | Elements | May use |
|---|---|---|---|---|
| `adapters` | Talks to one outside service and implements a backend port | yes | adapter | backend.application (ports only) |

**Modules.** The backend also holds modules (bounded contexts), such as
`tickets`, `queue` and `agents`.

- A module runs across the domain and application layers.
- Each table in `database.schema` names the one module that owns it.
- The base rule (S1, kept) says a module reads only its own tables and asks
  the owning module for anything else.
- The `composition` layer is the project's AppCore. It is the one place
  that builds every module. It is a layer, not the whole backend: this is
  the "one core" sentence said the way it was meant.

**Rules kept from base v1:** S1 to S6. Their texts change only where they
named folders.

**New rules:**

- **S7. Dependency direction.** A layer depends only on the layers its
  `may_use` lists. S7 is checked between layers, not parts. *(My call.)*
- **S8. Ports.** The domain and application layers never name a database,
  framework or outside service. They declare a port, and an outer layer
  implements it.
- **S9. One folder.** Each part and each layer owns exactly one folder,
  inside its parent's folder.

### 5.3 Layers in a project

**At the map stage**, the map card proposes each part with its layers:

- every required layer of its kind, plus the optional ones the person
  wants;
- each with its folder, relative to the part folder, chosen for the stack.
  Example for Laravel: `domain` → `backend/app/Domain`, `interface` →
  `backend/app/Http`, `composition` → `backend/app/Providers`.

**Shadows refuses the map** when:

- a required layer is missing;
- a layer folder lies outside its part;
- or two layers share a folder.

**Create structure writes:**

- one folder per part and per layer;
- the starter files from the card;
- and one root `SHADOWS.md`. That file says Shadows is the source and how
  to reach it over MCP. Nothing per part.

### 5.4 Elements

An element is the smallest thing Shadows plans. It has:

- `kind`, from its layer's catalog line;
- `name`;
- `summary`: one sentence;
- `module`: backend and database elements only;
- `fields`, depending on the kind:
  - a table has columns (name, type, null, default), keys, and relations
    to other tables;
  - a use case has input, output and the ports it calls;
  - an endpoint has its method, path and the one use case it calls;
  - a screen has its route and the features it shows;
- `file`: the path, inside the layer folder, where it will live;
- `symbols`: names and signatures only, each with one line on its job;
- `links`, to other elements:
  - `uses`;
  - `implements` (a repository implements a port);
  - `reads` and `writes` (a repository reads and writes a table);
  - `calls` (a client call calls an api operation).
  Shadows refuses a link whose direction breaks S7.

**Elements are written through cards,** like the map:

- The Planner proposes with `element_propose`.
- The person approves with the button, or asks the Planner to approve with
  `element_approve`.
- A card may carry many elements. For example, the support-loop tables come
  in one card with their relations.

### 5.5 Depth

Shadows computes one state per layer from the elements it holds. The model
never sets it.

| State | Holds when |
|---|---|
| 0 Not discussed | The layer has no approved element. |
| 1 Designed | It has at least one approved element, and every link from its elements points at an approved element. Example: each port a use case names exists. |
| 2 Planned to files | Every element has a `file` and `symbols`. |
| 3 Built | The code index finds every symbol in its file. |

**What is computed from the states:**

- **A part's depth** is the share of its required layers at each state. It
  is drawn as a bar of four colours, not as one percentage.
- **The "next" hint** names the lowest required layer that blocks the
  layers that use it. Example: "database.schema is planned to files;
  plan backend.infrastructure's repositories next."

State 3 needs the code index to read the project's language. The index
reads Rust, TypeScript and JavaScript today. PHP and Dart are an index
change (section 9). Until then, a layer in another language stops at
state 2, and the page says why.

### 5.6 Plans, tasks and phases

**A task gains `elements`:** the element ids it builds or changes.

- Its `reads` and `writes` follow from those elements' files. The person no
  longer types paths.
- **Approval gate.** A plan version cannot be approved while a task names
  an element below state 2. The blocker names the element and its layer.
  This answers "is this really a plan?": the plan exists only once its
  part is planned to files.

**A task gains `phases`:** an ordered list. Each phase has:

- `number`, `title` and `goal`;
- the subset of the task's elements it touches;
- and `waits_for`, a list of `{task, phase}`. The task may be in this plan
  or, with its plan id, in another plan. A link into another plan resolves
  against that plan's latest approved version. *(My call.)*

**Phase rules:**

- A task's acceptance items still decide when the task is done.
- Phases give the order inside the task.
- Phase links never form a cycle, across tasks either.

## 6. What the person sees

The mockup shows each view with data from the testing56 support system.

1. **Project map is a canvas.**
   - Parts are nodes; edges are dependencies between parts.
   - Each node shows its kind, folder and depth bar.
   - The database lies at the bottom, beneath everything.
   - "Served vision items" shows only the items the part serves, with an
     Add button (fixes point 8).
2. **Part view: the layers as rings,** domain in the centre (the Clean
   Architecture picture).
   - Each ring shows its folder, depth state and element count.
   - Allowed dependencies are arrows pointing inward.
   - Modules are slices across the inner rings.
3. **Layer view: the elements as a diagram that fits their kind.**
   - schema: an entity-relationship diagram;
   - use cases: a list with their ports;
   - api: operations grouped by path;
   - screens: a route tree.
4. **Element panel,** opened by clicking an element:
   - its fields, file and symbols;
   - its links;
   - the tasks that build it;
   - and its **trace**: the chain through every part.
     Example: `tickets` table ← `TicketRepository` (backend
     infrastructure) ← `TicketStore` port ←
     `TakeTicket` ← `POST /tickets/{id}/take` ← agent app queue screen.
5. **Plan view and task view.**
   - The plan stays the graph it is today, and each task card also shows
     its element chips.
   - Clicking a task opens the **task view**: the same kind of graph, one
     level down. Its nodes are phases, drawn as task cards are. The task
     itself is the start node, and the edges carry labels. The view keeps
     the legend, zoom controls and minimap.
   - A phase of another task that this one waits for, or that waits for
     it, appears as a dashed node with its task number. Clicking it opens
     that task's view.
   - **Show phases** is a button in the plan header, beside Versions. It
     opens every task at once, in the same graph:
     - each task becomes a frame titled with its number and title;
     - its phases sit inside the frame as cards, each with a badge such as
       `T3 · P2`, so every phase shows the task it belongs to;
     - the graph grows to fit;
     - edges run phase to phase, for example `T1 · P3` → `T3 · P3`, so the
       person sees exactly which part of a task waits.
     - **Hide phases** returns the graph to task cards.
     - The minimap and zoom stay.
     (Mohammed, 2026-11-05.)
6. **Zoom.** A breadcrumb (Project › Backend › Application › TakeTicket)
   and Escape step out one level. The minimap stays.

## 7. The Planner's tools

**New for the Planner (thread grant):**

- `element_propose` and `element_approve`;
- `layer_get`, which returns a layer's elements and its depth;
- `depth_get`, which returns every part's layers and their states.

**Unchanged in shape, with new fields:**

- `map_propose` carries layers.
- `draft_start` and `plan_edit` carry task elements and phases.

**The prompt's stage text gains one line:** after the map, design the
layers each roadmap outcome needs to state 2 before planning tasks on them.

**The agent that builds a task asks Shadows; it reads no contract file.**
`task_context` (thread and project grants) takes a task and answers:

- each element the task builds, with its layer, the layer's folder, its
  file and its symbols;
- the elements one link away, such as the port it implements or the table
  it writes, with their files;
- and, for symbols already built, what the code index knows: where each one
  is and who uses it.

Example: "T3 · P3: create `TakeTicket` in
`backend/app/Application/Tickets/TakeTicket.php`. It calls port
`TicketStore`, declared in `.../TicketStore.php`."

**What lies inside a layer is not the standards' business.** The standards
name the layers and their dependency direction. How a layer is built
inside, meaning which elements, files and functions, is agreed between the
Planner and the person, and recorded as elements.

**External agents (project grant)** get the reads: `layer_get`,
`depth_get` and `task_context`. Proposals need a conversation to hold their cards, so external
proposals wait for a decision on where their cards appear (section 10).

## 8. Owners in Shadows' code

| Concern | Owner |
|---|---|
| Layer catalog and rules S7 to S9 | `core-standards` |
| Layers, elements and links as part of the design | `core-design` |
| Folder and layer rules | `MapRules` |
| Depth | `core-design`. It reads symbols through `core-code`. |
| Task elements and phases | `core-plans` |
| The approval gate | `core-plans`. It reads depth from `core-design`. |
| Writing the structure, with no per-part `contract.yaml` | `core-design` (`structure_create`) |

No new service is needed. If the element store grows past what the design
service should own, a separate service is a decision for the controller,
not something to improvise. That question is OPEN.

## 9. Delivery in slices

Each slice is usable on its own, in this order.

1. **Folder rule and no contract files.**
   - Each part has one folder, inside its parent's; checked at proposal
     and approval.
   - Create structure writes no `contract.yaml`.
   - Existing files on disk are left alone.
2. **Standards v2 and layers on the map.**
   - The catalog, rules S7 to S9, and map cards with layers and their
     folders.
   - The Project map canvas, and the trimmed "served" list.
3. **Elements, starting with the schema.**
   - `element_propose` and `element_approve`, the ER diagram, and the
     element panel.
   - Then use cases, endpoints and screens.
4. **Depth.** States 0 to 2, the depth bars, and the "next" hint.
5. **Tasks name elements; the approval gate.**
6. **Phases** inside tasks, with links across tasks and plans.
7. **The trace view.**
8. **State 3.** PHP and Dart in the code index. This waits for the
   code-intelligence work.

**Projects made under base v1** keep v1 until the person adopts v2 from the
Standards page. Their layers then start empty, at state 0.

## 10. Open questions

1. **Where an an external agent's proposal cards appear.** A thread is needed
   today. Closes when the external-agent design is taken up with the global
   connection (deferred).
2. **Modules: required for every backend, or only past a size?** This draft
   makes them optional, with S1 binding once a module exists. Closes when
   the first project with modules is planned.
3. **Frontend layers for a mobile app versus a web app:** one catalog or
   two? Closes in slice 2.
