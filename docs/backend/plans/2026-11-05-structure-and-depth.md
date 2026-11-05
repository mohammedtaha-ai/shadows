# Structure, layers and planning depth: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shadows holds each project's structure, as parts with one folder
each, layers from the standards and elements inside the layers. It
measures how deep each part is planned, gates plan approval on that depth,
and answers the agent building a task instead of writing `contract.yaml`.

**Architecture:** No new service.
- `core-standards` holds the layer catalog.
- `core-design` holds layers, elements, links and depth. It reads
  `core-code` for built symbols.
- `core-plans` holds task elements and phases, and the approval gate. It
  already depends on `core-design`.
- Adapters translate only: HTTP in `shadows-http`, MCP tools in
  `shadows-mcp`.
- The web views follow the mockup.

**Tech Stack:** Rust 1.94+, SQLx 0.9, SQLite, rmcp, axum; React and React
Flow in `web/`.

**Spec:** [docs/backend/specs/2026-11-05-structure-and-depth-design.md](../specs/2026-11-05-structure-and-depth-design.md).
Mockup sources are in [2026-11-05-structure-and-depth-mockup/](../specs/2026-11-05-structure-and-depth-mockup/README.md).

**How this plan grows.** Mohammed asked for the whole plan now, with detail
added as each slice starts.
- **Slice 1** is written to step level.
- **Slices 2 to 8** are written to task level: files, interfaces, test
  names and acceptance. Each ends with "Decide when the slice starts", the
  questions its first task settles.
- When a slice starts, its tasks get `B-`/`F-` numbers in
  `docs/backend/stages.md` and `docs/frontend/stages.md`, and its steps are
  written here.

## Global Constraints

- **Every Core task starts at `crates/shadows-core/src/app.rs`.** Work only
  in the owning service. No second way in, no free `pub fn`.
- **A change to a module updates its `contract.yaml` in the same commit.**
- **An HTTP response change needs an accepted shared request** in
  `docs/shared/requests/` before the web side builds on it.
  - `api/openapi.json` changes only with a route.
- **Rust lines are at most 100 characters.**
- **Comments say why, never history.**
- **Each commit runs only the tests its change touches.** The full gate
  runs in CI once per branch, never locally.
- **`model.rs` has no persistence imports.** SQL stays in the owning store.
- **Ordering is explicit:** an ordinal or a stable key, never `rowid` or
  insertion order.
- **A mutating command carries a `CommandId` and a fingerprint.** A replay
  needs an equal fingerprint.
- **A refusal names the rule and says how to fix it**, like the existing
  `MapRefusal` messages.
- **Projects on standards base v1 keep working unchanged** until the person
  adopts v2.

## Review Focus

1. **An old map that breaks a new rule.** testing55's backend part has four
   folders. Editing a different part must not be refused because of it. The
   new rules apply to the parts a write changes, like `unknown_vision_item`
   today. (Slice 1: `an_old_part_with_four_folders_does_not_block_another_part`.)
2. **A link that breaks dependency direction across parts.** Example: a
   domain element linking to a table. It must be refused at proposal time,
   not found later. (Slice 3:
   `a_domain_element_cannot_link_to_a_table`.)
3. **A task names an element that is later removed.** Plan approval must
   report the missing element, not crash and not pass. (Slice 5:
   `a_removed_element_blocks_approval_by_name`.)
4. **Phase waits that form a cycle through another plan.** They are refused
   with both phases named. (Slice 6: `a_cycle_through_another_plan_is_refused`.)
5. **Depth of a part with an optional layer that has no elements.** The
   empty optional layer does not pull the bar down. (Slice 4:
   `an_empty_optional_layer_is_left_out_of_depth`.)

---

## Slice 1: one folder per part, no contract files

Fixes the bug where map approval accepted a part with no folder and
Create structure then refused it. Shadows stops writing `contract.yaml`.
Backend, plus one web test fixture.

Existing code (read first):
- `crates/core-design/src/map_rules.rs`: `MapRules::check_shape`. It
  runs at proposal (`store/map_proposal_shape.rs:153`) and on person edits
  (`store/vision.rs:258`).
- `crates/core-design/src/structure.rs`: `Structure::render` writes
  `<first folder>/contract.yaml`. It refuses "part has no folder".
- `MapRules::check_reserved` reserves `SHADOWS.md` and
  `<folder>/contract.yaml`.

### Task 1.1: a written part has exactly one folder, inside its parent's

**Files:**
- Modify: `crates/core-design/src/map_rules.rs` (`check_shape`)
- Modify: `crates/core-design/contract.yaml` (MapRules section)
- Test: `crates/core-design/tests/map_rules.rs`

**Interfaces:**
- Consumes: `MapPart { key, parent, content: &PartContent }`, as today.
- Produces: two new refusal rules, `one_folder` and
  `folder_outside_parent`. The `PartContent.folders: Vec<String>` shape is
  unchanged in this slice. Slice 2 turns it into `folder`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_written_part_needs_exactly_one_folder() {
    // A part with folders [] and one with ["app", "config"], each in `written`.
    // Expected: Err(MapRefusal) whose rule is "one_folder", and whose message
    // names the part title and says "Give the part exactly one folder".
}

#[test]
fn a_child_folder_sits_inside_its_parents() {
    // Parent "frontend" with folder "frontend";
    // child "dashboard" with folder "dashboard".
    // Expected: rule "folder_outside_parent"; the message names both folders.
    // Same child with folder "frontend/dashboard": Ok(()).
}

#[test]
fn an_old_part_with_four_folders_does_not_block_another_part() {
    // Part A, folders ["app","config","routes","tests"], not in `written`.
    // Part B, folder ["database"], in `written`. Expected: Ok(()).
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p core-design --test map_rules a_written_part a_child_folder an_old_part`
Expected: FAIL (the rules do not exist).

- [ ] **Step 3: Implement both rules inside `check_shape`**
  - Check only parts in `written`.
  - The parent's folder is the parent part's first folder, matched with
    the existing private `inside` helper.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test -p core-design --test map_rules`
Expected: PASS. Also run the existing tests in that file.

- [ ] **Step 5: Update `crates/core-design/contract.yaml`, then commit**

```bash
git add crates/core-design/src/map_rules.rs crates/core-design/contract.yaml crates/core-design/tests/map_rules.rs
git commit -m "fix(core-design): a map part has one folder, inside its parent's"
```

### Task 1.2: Create structure writes no contract files

**Files:**
- Modify: `crates/core-design/src/structure.rs` (`render`, and remove the
  private `contract` function)
- Modify: `crates/core-design/src/map_rules.rs` (`check_reserved`
  reserves only `SHADOWS.md`)
- Modify: `crates/core-design/contract.yaml`
- Test: `crates/core-design/tests/structure.rs`
- Modify: `crates/shadows/tests/design_routes.rs:81`
- Modify: `web/src/app/design/create-structure.test.tsx:45,58` (fixture
  paths only)

**Interfaces:**
- Produces: the `Planned` files are `SHADOWS.md` plus the starter files.
  `Planned::file_count`, and so the stage read's count, drops by one per
  part, through the same `plan`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn structure_writes_no_contract_files() {
    // render() over two parts with folders "backend" and "database".
    // Expected: no planned path ends with "contract.yaml".
    // The folders contain "backend" and "database".
    // SHADOWS.md contains "Ask Shadows" and does not contain "contract".
}

#[test]
fn a_starter_file_may_now_be_named_contract_yaml() {
    // A part lists the starter file "backend/contract.yaml".
    // Expected: check_reserved returns Ok.
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p core-design --test structure`
Expected: FAIL.

- [ ] **Step 3: Change `render` and `check_reserved`**
  - Drop the contract files from `render`.
  - `SHADOWS.md` text:
    `# <name>\n\nShadows wrote this structure from map version <n>.\n`
    `Standards: base v<b>, additions <a>.\n`
    `Shadows is the source of truth for the vision, the map and the plans.\n`
    `Ask Shadows over MCP where a part, layer or task's code lives.\n`
  - Existing `contract.yaml` files on disk are left alone.

- [ ] **Step 4: Update the two fixtures that expected `backend/contract.yaml`**
  - `design_routes.rs:81` asserts the file is **absent**.
  - `create-structure.test.tsx` fixture paths become
    `['SHADOWS.md','backend/main.rs']`, and the expectation becomes
    `backend/main.rs`.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p core-design --test structure`, then
`cargo test -p shadows --test design_routes`, then (in `web/`)
`npx vitest run src/app/design/create-structure.test.tsx`.
Expected: PASS.

- [ ] **Step 6: Contract, then commit**

```bash
git commit -m "feat(core-design): create structure writes no contract files"
```

### Task 1.3: the Planner is told the new map rules

**Files:**
- Modify: `crates/core-harness/src/prompt.txt` ("The project map" section)
- Test: `crates/shadows/tests/planner_mcp.rs` (prompt assertion)

- [ ] **Step 1: Write the failing assertion**
  The appended prompt contains `"Each part has exactly one folder"` and
  does not contain `"contract.yaml"`.
- [ ] **Step 2: Run and see it fail**
  Run: `cargo test -p shadows --test planner_mcp`.
- [ ] **Step 3: Replace the map section's first bullet:**
  > "Shadows writes a root SHADOWS.md itself; never propose a file at that
  > path. Each part has exactly one folder, and a child's folder lies inside
  > its parent's folder."
- [ ] **Step 4: Run and see it pass, then commit**

```bash
git commit -m "feat(core-harness): the Planner proposes one folder per part"
```

**Slice 1 acceptance (in the running app, testing56 or a new project):**
1. A map card with a part without a folder is refused at proposal, naming
   the part.
2. Create structure writes `SHADOWS.md` and the starter files, and no
   `contract.yaml`.
3. The "View contract" button still shows the contract head from Shadows.

---

## Slice 2: standards v2 and layers on the map

**Outcome.**
- Each part kind has a catalog of layers.
- A map card carries each part's layers and their folders.
- Shadows refuses a map that misses a required layer.
- Create structure makes the layer folders.
- The Project map page is a canvas.

### Task 2.1: the catalog in `core-standards`
- **Files:**
  - `crates/core-standards/src/standards.yaml`: becomes `standards-v1.yaml`
    (unchanged), plus a new `standards-v2.yaml`.
  - The loader in `core-standards/src`.
  - Its `contract.yaml`.
- **Interfaces:**
  - `LayerSpec { id: String, role: String, required: bool, elements: Vec<String>, may_use: Vec<LayerRef> }`
  - `LayerRef { part_kind: Option<String>, layer: String }`, where `None`
    means the same part.
  - `PartSpec.layers: Vec<LayerSpec>`.
  - `Standards::base(version: i64) -> &BaseStandards`.
- **v2 content:**
  - the layers in design §5.2, with infrastructure holding the
    repositories;
  - rules S1 to S9; S3 reads "Every part has a contract head in Shadows".
- **Tests:**
  - `v1_still_loads_unchanged`
  - `v2_names_required_backend_layers`
  - `every_may_use_points_at_a_known_layer`

### Task 2.2: `PartContent.folder` and `layers`
- **Files:**
  - `core-design/src/model.rs`
  - the parts store, with a storage migration
  - the OpenAPI types
  - `docs/shared/requests/<date>-part-folder-and-layers.md`
- **Interfaces:**
  - `PartContent.folder: String` replaces `folders`. The migration takes
    the first folder; an empty list becomes the part's slug.
  - `PartContent.layers: Vec<PartLayer { id: String, folder: String }>`.
- **Tests:**
  - `a_stored_part_with_folders_reads_back_with_its_first_folder`
  - `a_part_round_trips_its_layers`

### Task 2.3: map rules for layers
- **File:** `core-design/src/map_rules.rs`.
- **New refusals:**
  - `missing_layer`: a required layer of the kind is absent; under v2 only.
  - `unknown_layer`
  - `layer_outside_part`
  - `layer_folder_shared`
- **Tests:** one per refusal, plus
  `a_v1_project_proposes_a_map_without_layers`.

### Task 2.4: layer folders on disk
- **Change:** `Structure::render` adds each layer folder.
- **Test:** `structure_makes_each_layer_folder`.

### Task 2.5: map card, tools and prompt carry layers
- **Files:**
  - `shadows-mcp/src/workspace_tools.rs` (the `map_propose` description)
  - `MapPropose`
  - `prompt.txt` (layers at map)
- **Test:** `planner_mcp` asserts the description names layers.

### Task 2.6: adopting v2
- **Interfaces:**
  - `Standards::adopt_base(project, version: 2, CommandId)`
  - its route, `POST /api/projects/{id}/standards/adopt`
- **Test:** `adopting_v2_keeps_the_map_and_starts_layers_empty`.

### Task 2.7 (web, F-): the Project map canvas
- **Files:**
  - `web/src/app/design/` (a new `map-canvas.tsx`, using the same React
    Flow setup as `PlanGraph`)
  - the trimmed served list in `map-part-details.tsx`
- **Behaviour:**
  - Part nodes show the kind, the folder and the depth bar (empty until
    slice 4).
  - Edges are the cross-part `may_use` pairs between parts that exist.
  - The database sits on the bottom rank.
- **Tests:**
  - `the_canvas_draws_one_node_per_part`
  - `served_items_show_only_the_parts_own`

**Decide when the slice starts:**
1. Is the v2 frontend catalog one list for web and mobile, or two?
   (Design open question 3.)
2. Is the map card shape a new `layers` field, or nested parts? This draft
   says a field.

---

## Slice 3: elements, schema first

**Outcome.**
- The Planner proposes elements in cards, and the person approves them.
- The layer view draws the schema as an ER diagram.
- An element panel shows the element's fields, file, symbols and links.

### Task 3.1: the element model and store (`core-design`)
- **Interfaces:**
  - `Element { id, part, layer, kind, name, summary, module: Option<String>, fields: ElementFields, file: Option<String>, symbols: Vec<Symbol>, links: Vec<ElementLink> }`
  - `ElementFields` is one enum variant per kind. The schema slice has
    `Table { columns: Vec<Column>, keys: Vec<Key> }`.
  - `Symbol { name, signature, job }`
  - `ElementLink { kind: LinkKind (Uses|Implements|Reads|Writes|Calls), to: ElementId }`
- **Store:** new tables `element` and `element_link`, ordered by an
  explicit ordinal.
- **Tests:**
  - `an_element_round_trips`
  - `links_keep_their_order`

### Task 3.2: element proposals and approval
- **Interfaces:**
  - `Design::propose_elements(&Grant, ElementPropose) -> ProposalCard`
  - `approve_element_proposal` and `approve_element_proposal_as_planner`
- **Rules:**
  - the kind is allowed by its layer;
  - a link direction obeys S7 (`may_use`);
  - a table's foreign keys name tables that exist.
- **Tests:**
  - `a_domain_element_cannot_link_to_a_table`
  - `a_foreign_key_to_a_missing_table_is_refused`
  - `one_card_carries_many_elements`

### Task 3.3: the MCP tools
- **Tools:**
  - `element_propose` and `element_approve`, thread grant;
  - `layer_get`, thread and project grants.
- **Files:** `core-grants/src/tools.rs` (both lists) and
  `shadows-mcp/src/workspace_tools.rs`.
- **Test:** `planner_mcp` lists them per grant.

### Task 3.4: HTTP
- **Routes:**
  - `GET /api/projects/{id}/layers/{part}/{layer}`
  - the card approve route
- **Also:** a shared request, and the OpenAPI document.

### Task 3.5 (web, F-): the layer view and element panel
- **Files:** `web/src/app/design/layer-view.tsx`, `element-panel.tsx`,
  and the element card in the conversation.
- **Tests:**
  - `the_schema_view_draws_a_card_per_table_and_an_edge_per_foreign_key`
  - `the_element_panel_shows_file_and_symbols`

### Task 3.6: further element kinds
`use_case`, `port`, `endpoint`, `repository`, `screen` and `client_call`,
each with its fields and its view: a list for use cases, operations by
path for the API, and a route tree for screens.

**Decide when the slice starts:** whether proposing elements needs its own
stage gate, or only an approved map. This draft says an approved map.

---

## Slice 4: depth

### Task 4.1: computing depth (`core-design`)
- **Interfaces:**
  - `Design::depth(&ProjectId) -> ProjectDepth { parts: Vec<PartDepth> }`
  - `PartDepth { part, layers: Vec<LayerDepth>, next: Option<String> }`
  - `LayerDepth { layer, required, state: DepthState, elements, with_files }`
  - `DepthState` is one of `NotDiscussed`, `Designed`, `PlannedToFiles`
    or `Built`.
- **States:** computed as in design §5.5. `Built` waits for slice 8, so
  this slice stops at state 2.
- **Tests:**
  - `a_layer_with_an_unresolved_link_is_not_designed`
  - `a_layer_whose_elements_all_have_files_is_planned`
  - `an_empty_optional_layer_is_left_out_of_depth`
  - `next_names_the_lowest_blocking_layer`

### Task 4.2: depth over MCP and HTTP
- `depth_get`, for both grants;
- `GET /api/projects/{id}/depth`, with a shared request.

### Task 4.3 (web, F-): depth on the map
- depth bars and the panel;
- the "Next" hint, as on mockup screen 1.

---

## Slice 5: tasks name elements, approval gate, `task_context`

### Task 5.1: task elements (`core-plans`)
- **Interfaces:**
  - `TaskContent.elements: Vec<ElementId>`.
  - `plan_edit`'s task operations carry `elements`.
  - `reads` and `writes` are derived from the elements' files when a
    version is saved. A task with no elements keeps the paths it was given,
    so old plans keep working.
- **Tests:**
  - `a_task_derives_its_paths_from_its_elements`
  - `an_old_task_keeps_its_typed_paths`

### Task 5.2: the approval gate
- **What:** a new blocker, `element_below_planned`, read through
  `core-design`. `core-plans` already depends on `core-design`.
- **Tests:**
  - `approval_is_blocked_by_a_designed_element`
  - `a_removed_element_blocks_approval_by_name`

### Task 5.3: `task_context`, the builder's question
- **Interface:**
  `Plans::task_context(&Grant, workflow_id, task: u32) -> TaskContext { elements: Vec<ElementContext>, neighbours: Vec<ElementContext> }`.
  - `ElementContext` carries the layer folder, file and symbols.
  - For built symbols, it adds `where_is` and `who_uses` from `core-code`.
- **MCP:** `task_context`, for both grants.
- **Tests:**
  - `task_context_names_files_and_the_port_it_calls`
  - `an_external_agent_reads_task_context`

### Task 5.4 (web, F-): element chips and the blocker text
On task cards and on the plan header.

---

## Slice 6: phases inside tasks

### Task 6.1: the phase model (`core-plans`)
- **Interfaces:**
  - `TaskContent.phases: Vec<Phase>`
  - `Phase { number: u32, title, goal, elements: Vec<ElementId>, waits_for: Vec<PhaseRef> }`
  - `PhaseRef { plan: Option<PlanId>, task: u32, phase: u32 }`
- **Rules:**
  - a phase's elements are a subset of its task's;
  - a link into another plan resolves against that plan's latest approved
    version;
  - no cycle, across tasks or plans.
- **Tests:**
  - `phase_elements_must_belong_to_the_task`
  - `a_cycle_through_another_plan_is_refused`
  - `a_wait_on_an_unapproved_plan_is_refused`

### Task 6.2: plan editing and tools
- the `plan_edit` operations `phase_put` and `phase_remove`;
- the MCP descriptions and the prompt.

### Task 6.3 (web, F-): the task view
Mockup screen 4:
- the same React Flow look as `PlanGraph`;
- the task is the start node;
- other tasks' phases are dashed nodes;
- the Back button returns to the plan graph.

---

## Slice 7: the trace view

### Task 7.1: `Design::trace(&ElementId) -> Vec<TraceStep>`
- It follows links across parts, both ways, up to the first client and
  the deepest table.
- **Test:** `the_tickets_table_traces_to_the_queue_screen`.

### Task 7.2 (web, F-): the trace in the element panel
Mockup screen 3.

---

## Slice 8: built state (state 3)

**Waits for:** the code-intelligence work.

### Task 8.1: index grammars
`tree-sitter-php` and `tree-sitter-dart` in `shadows-index`, with tag
queries.
- **Tests:** a PHP class and method are tagged; a Dart class and function
  are tagged.

### Task 8.2: depth reaches `Built`
- `core-design` depends on `core-code`; there is no cycle, because
  `core-code` depends only on `core-db` and `core-projects`.
- A layer is built when every symbol is found in its file.
- **Test:** `a_layer_is_built_when_the_index_finds_every_symbol`.
