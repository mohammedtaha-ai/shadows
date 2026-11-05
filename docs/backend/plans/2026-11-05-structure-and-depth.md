# Structure, layers and planning depth: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shadows holds each project's structure: parts with one folder
each, layers from the standards, and elements inside the layers. It shows
how much of each part is planned, reports whether a project stage is fully
planned, gates plan approval on recorded detail, and gives the agent
building a task bounded context instead of `contract.yaml` files.

**Architecture:** No new service.
- `core-standards` holds the layer catalog.
- `core-design` holds layers, versioned elements, links and outcome
  criteria.
- `core-plans` holds task bindings, coverage, phases and the approval gate.
  It reads `core-design` and `core-code` through their service APIs.
- Adapters translate only: HTTP in `shadows-http`, MCP tools in
  `shadows-mcp`.
- The web views follow spec §6.

**Tech Stack:** Rust 1.94+, SQLx 0.9, SQLite, rmcp, axum; React and React
Flow in `web/`.

**Spec:** [docs/backend/specs/2026-11-05-structure-and-depth-design.md](../specs/2026-11-05-structure-and-depth-design.md).
Mockup sources are in [2026-11-05-structure-and-depth-mockup/](../specs/2026-11-05-structure-and-depth-mockup/README.md).

**Words.** These terms are defined at the top of the spec and used with one
meaning each:
- a **planning step** is one of Idea, Vision, Map, Structure, Roadmap or
  Plans;
- a **project stage** is a roadmap outcome, such as "Stage 1";
- a **delivery slice** is slice 1 to 9 below.

**Mohammed's decisions (2026-11-05):**
- **No backward compatibility:** every existing project is test data.
- **Shadows never writes `contract.yaml`** after slice 1.
- **Repositories live in backend infrastructure.**
- **The task view and Show phases are approved.**

**How this plan grows.** Mohammed asked for the whole plan now, with detail
added as each slice starts.
- **Slice 1** is written to step level.
- **Slices 2 to 9** are written to task level: files, interfaces, test
  names and acceptance. They are preparation packets, not builder
  assignments.
- When a slice starts, its tasks get `B-`/`F-` numbers in
  `docs/backend/stages.md` and `docs/frontend/stages.md`, and its steps are
  written here.

## Dispatch and progress

No slice is marked done, and this document dispatches no builder. The
controller prepares one runnable vertical slice at a time, including its
UI and API dependencies.

| Slice | Prerequisite | Planning state | Exit evidence |
|---|---|---|---|
| 1 | Caller inventory of the four write paths | Implemented in PR #59 (`26bf739e`); live acceptance not recorded | Proposal, approval, person edit and structure agree; no contract written; old unfinished write cancelled |
| 2 | Slice 1; spec §10 profile decision; accepted API request | Needs design | Layers on the map card, migration on the daily database, canvas in the app |
| 3 | Slice 2; agreed revision and proposal API | Needs preparation | Batch element approval, schema view, stale-card recovery |
| 4 | Slice 3 | Needs preparation | Layer counts and part summary visible |
| 5 | Slice 4; accepted binding and context contract | Needs preparation | Gate refuses an unspecified element; an agent reads bounded context |
| 6 | Slice 5; stable criterion model | Needs preparation | One project stage reported as Needs planning or Planning covered |
| 7 | Slice 5; accepted combined graph semantics | Needs preparation | Mixed dependency cycles refused; both graph modes work |
| 8 | Slice 5 | Needs preparation | Bounded trace, with incomplete relationships visible |
| 9 | Slice 5; a supported index language | Needs preparation | Source observation and its uncertainty visible |

**Before a task is `todo`,** its `B-` or `F-` entry in its side's
`stages.md` names:
- its base commit;
- its owner and contract;
- exact `Read` and `Touch` paths, including callers, fixtures, generated
  artifacts and maps;
- the predecessor's result;
- the accepted shared request;
- its input and output contract;
- its failure cases;
- its exact checks.

The controller resolves the OPEN items that affect that slice. A builder
reports a missing path or decision instead of widening its own assignment.

**Status and closing.**
- Task statuses are `todo | doing | blocked | done`.
- Implementation commit, focused checks, review and live acceptance are
  tracked separately. An implementation commit alone does not close a
  slice.
- A slice's exit records the tested revision, the real UI journey and the
  restart or retry results where they apply.
- No screenshot or mockup is evidence that backend behaviour exists.

## Global Constraints

- **Every Core task starts at `crates/shadows-core/src/app.rs`.** Work only
  in the owning service. No second way in, no free `pub fn`.
- **A change to a module updates its `contract.yaml` in the same commit.**
  This means Shadows' own crate contracts, which stay.
- **An HTTP response change needs an accepted shared request** in
  `docs/shared/requests/` before the web side builds on it.
  - A serialized response change also requires regenerating the schema,
    even when the route path stays the same.
  - Frontend types are generated from that schema.
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
- **No compatibility layer for existing projects; they are test data.**
  - Migrations may convert old rows crudely.
  - The daemon must still start on the daily database.
  - Nothing on the person's disk is deleted.
- **Planning coverage, code observation and verification are separate.**
  Use spec §§5.5–5.7 and §17's execution owner. There is no `Built`
  shortcut.
- **This work covers planning and observation only.** Task execution,
  verified completion of a stage and parallel scheduling wait for their
  roadmap owners.

## Review Focus

1. **A parent move or folder rename puts a child outside it.** Check the
   affected descendants on every write path: proposal, approval and person
   edit. (Task 1.1: `moving_a_parent_checks_descendant_folders`.)
2. **An unfinished structure write from before the update.** It must be
   cancelled with a clear message, never stuck and never resumed with
   contracts. (Task 1.2:
   `an_unfinished_old_write_is_cancelled_with_a_clear_message`.)
3. **The daemon on the daily database after the `folders` → `folder`
   migration.** It starts, and old parts read back. (Task 2.2:
   `the_daemon_starts_on_a_database_with_old_parts`.)
4. **A link that breaks dependency direction across parts.** Example: a
   domain element linking to a table. Refuse it at proposal time. (Task 3.2:
   `a_domain_element_cannot_link_to_a_table`.)
5. **A part summary over an unknown inventory.** It never says complete,
   and an empty optional layer does not count against it. (Task 4.1:
   `an_unknown_inventory_never_reports_complete`.)
6. **A task names an element that is later removed.** Plan approval reports
   the missing element by name. (Task 5.2:
   `a_removed_element_blocks_approval_by_name`.)
7. **A linked plan alone does not cover a project stage.** (Task 6.2:
   `a_linked_plan_alone_does_not_cover_an_outcome`.)
8. **Phase waits that form a cycle through another plan.** Refuse them,
   naming the whole path. (Task 7.1:
   `a_cycle_through_another_plan_is_refused`.)
9. **Show phases hides a whole-task constraint.** Both graph modes keep
   `needs` and `completes_after`, including tasks with no phases. (Task 7.4.)
10. **A found symbol is a stub or belongs to another declaration.** This is
    an observation only, not completion. (Task 9.2.)

---

## Slice 1: one folder per part, no contract files

**Fixes the bug** where map approval accepted a part with no folder and
Create structure then refused it. **Shadows stops writing
`contract.yaml`.** Backend, plus one web test fixture.

Existing code (read first):
- **Map rules:** `crates/core-design/src/map_rules.rs`,
  `MapRules::check_shape`.
  - It runs at proposal (`store/map_proposal_shape.rs:153`) and on person
    edits (`store/vision.rs:258`, through `Design::edit_workspace_in`).
  - The approval path is `store/map_approval.rs`. Confirm it rechecks the
    candidate.
- **Rendering:** `crates/core-design/src/structure.rs`.
  - `Structure::render` writes `<first folder>/contract.yaml` and refuses
    "part has no folder".
  - `MapRules::check_reserved` reserves `SHADOWS.md` and
    `<folder>/contract.yaml`.
- **Writing:** `crates/core-design/src/structure_create.rs`.
  - A resumed intent whose recorded hashes differ from the current plan is
    refused with "recorded structure differs; cannot resume". The intent
    then stays stuck.

### Task 1.1: one folder per part, on every write path

**Files:**
- Modify: `crates/core-design/src/map_rules.rs` (`check_shape`)
- Modify: `crates/core-design/contract.yaml` (the MapRules section)
- Test: `crates/core-design/tests/map_rules.rs`
- Inspect before dispatch: `store/vision.rs` (`edit_workspace_in`),
  `store/map_proposal_shape.rs` and `store/map_approval.rs`. List their
  integration tests in the task's `Touch`.

**Interfaces:**
- Consumes: `MapPart { key, parent, content: &PartContent }`, as today.
- Produces: two refusal rules, `one_folder` and `folder_outside_parent`.
  - The checks run over the **whole candidate map**, not only `written`
    parts, so moves and renames are covered.
  - An old test map that breaks a rule is refused on its next write, with
    the part named.
  - `PartContent.folders: Vec<String>` keeps its shape in this slice.
    Slice 2 turns it into `folder`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_part_needs_exactly_one_folder() {
    // Parts with folders [] and ["app", "config"].
    // Expected: Err(MapRefusal) with rule "one_folder"; the message names the
    // part title and says "Give the part exactly one folder".
}

#[test]
fn a_child_folder_sits_inside_its_parents() {
    // Parent "frontend" with folder "frontend";
    // child "dashboard" with folder "dashboard".
    // Expected: rule "folder_outside_parent"; the message names both folders.
    // Same child with folder "frontend/dashboard": Ok(()).
}

#[test]
fn moving_a_parent_checks_descendant_folders() {
    // A child under parent A ("a/child") is moved under parent B ("b").
    // Only the move is written. Expected: "folder_outside_parent".
}

#[test]
fn renaming_a_parent_folder_checks_unchanged_children() {
    // Parent "frontend" becomes "web"; child stays "frontend/dashboard".
    // Expected: "folder_outside_parent", naming the child.
}

#[test]
fn windows_case_and_segment_boundaries_use_the_same_policy() {
    // Parent "Frontend", child "frontend/dashboard": Ok (case rules).
    // Parent "front", child "frontend/x": refused (segment, not prefix).
}
```

- [ ] **Step 2: Run them and see them fail**

Run each filter separately, for example
`cargo test -p core-design --test map_rules a_part_needs_exactly_one_folder`.
Expected: FAIL, because the rules do not exist.

- [ ] **Step 3: Implement both rules inside `check_shape`**
  - Iterate over every part in the candidate.
  - Compare normalized folders with the existing private `inside` helper,
    by segment, using the existing case rules.
- [ ] **Step 4: Add the write-path regressions**
  - `a_new_part_without_a_folder_is_refused_on_every_write_path`: one each
    for proposal, approval and person edit, in the owning integration
    tests.
  - `a_stale_map_approval_writes_nothing`.
- [ ] **Step 5: Run the tests and see them pass**
  Run: `cargo test -p core-design --test map_rules`, then the integration
  tests listed in the task. Expected: PASS, including the existing tests.
- [ ] **Step 6: Update `crates/core-design/contract.yaml`, then commit**

```bash
git add crates/core-design/src/map_rules.rs crates/core-design/contract.yaml crates/core-design/tests/map_rules.rs
git commit -m "fix(core-design): a map part has one folder, inside its parent's"
```

### Task 1.2: no contract files; cancel an old unfinished write

**Files:**
- Modify: `crates/core-design/src/structure.rs` (`render`; remove the
  private `contract` function)
- Modify: `crates/core-design/src/map_rules.rs` (`check_reserved` reserves
  only `SHADOWS.md`)
- Modify: `crates/core-design/src/structure_create.rs`: a resumed intent
  that differs is cancelled.
- Modify: `crates/core-design/src/store/structure.rs`: a new crate-private
  `cancel_structure(&mut Tx, project)` that clears the recorded intent.
- Modify: `crates/core-design/contract.yaml`
- Test: `crates/core-design/tests/structure.rs`
- Modify: `crates/shadows/tests/design_routes.rs:81`
- Modify: `web/src/app/design/create-structure.test.tsx:45,58` (fixture
  paths only)

**Interfaces:**
- **Files:** the `Planned` files are `SHADOWS.md` plus the starter files.
  `Planned::file_count`, and so the stage's count, follows the same `plan`.
- **Cancelled write:** when a resumed intent's hashes or folders differ,
  `create_structure` cancels the intent and returns
  `DesignError::StageBlocked`. Its message is: "A structure write from an
  earlier version did not finish. Press Create structure again."
- **Next press:** the following Create structure starts a new intent.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn structure_writes_no_contract_files() {
    // render() over two parts with folders "backend" and "database".
    // Expected: no planned path ends with "contract.yaml".
    // The folders contain "backend" and "database".
    // SHADOWS.md names Shadows as the source and does not mention "contract".
}

#[test]
fn a_starter_file_may_now_be_named_contract_yaml() {
    // A part lists the starter file "backend/contract.yaml".
    // Expected: check_reserved returns Ok.
}

#[tokio::test]
async fn an_unfinished_old_write_is_cancelled_with_a_clear_message() {
    // Record an intent whose files include "backend/contract.yaml", as the old
    // renderer did, and stop after one file (create_structure_stopping_after).
    // Call create_structure again with the new renderer.
    // Expected: StageBlocked with the message above; the intent is cleared.
    // A further create_structure succeeds and writes no contract file.
}

#[tokio::test]
async fn an_existing_contract_file_is_not_deleted() {
    // backend/contract.yaml already exists on disk.
    // create_structure succeeds, and the file is still there, unchanged.
}
```

- [ ] **Step 2: Run them and see them fail**
  Run: `cargo test -p core-design --test structure`. Expected: FAIL.
- [ ] **Step 3: Change `render`, `check_reserved` and the resume branch**
  - The `SHADOWS.md` text:
    `# <name>\n\nShadows wrote this structure from map version <n>.\n`
    `Standards: base v<b>, additions <a>.\n`
    `Shadows is the source of truth for the vision, the map and the plans.\n`
    `Ask Shadows over MCP where a part's code lives.\n`
  - Name only tools that exist: `layer_get` and `task_context` are added
    to this text in the slices that ship them. Never include a grant
    secret.
  - The cancel runs in one transaction with the intent read. A replayed
    `CommandId` of a cancelled command returns the same refusal.
- [ ] **Step 4: Update the two fixtures**
  - `design_routes.rs:81` asserts that `backend/contract.yaml` is
    **absent**.
  - The `create-structure.test.tsx` fixture paths become
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
- Modify: `crates/core-harness/src/prompt.txt` (the "The project map"
  section)
- Modify: `crates/core-harness/contract.yaml`, if it quotes the prompt
- Test: `crates/shadows/tests/planner_mcp.rs` (prompt assertion)

- [ ] **Step 1: Write the failing assertion**
  The appended prompt contains `"Each part has exactly one folder"` and
  does not contain `"contract.yaml"`.
- [ ] **Step 2: Run it and see it fail**
  Run: `cargo test -p shadows --test planner_mcp`.
- [ ] **Step 3: Replace the map section's first bullet:**
  > "Shadows writes a root SHADOWS.md itself; never propose a file at that
  > path. Each part has exactly one folder, and a child's folder lies inside
  > its parent's folder."
  Keep the existing instruction that the Planner writes only what the
  person asks.
- [ ] **Step 4: Run it and see it pass, then commit**

```bash
git commit -m "feat(core-harness): the Planner proposes one folder per part"
```

**Slice 1 acceptance (in the running app, on the daily database):**
1. A map card with a part without a folder is refused at proposal, naming
   the part.
2. Moving a part under a parent whose folder does not contain it is refused
   in the person's edit form.
3. Create structure in testing56 writes `SHADOWS.md` and the starter files,
   and no `contract.yaml`. The old `contract.yaml` files are still on disk.
4. The "View contract" button still shows the contract head from Shadows.

---

## Slice 2: standards v2 and layers on the map

**Outcome.**
- Each part kind has a catalog of layers. v2 replaces v1 for every
  project.
- A map card carries each part's layers and their folders.
- Shadows refuses a map that misses a required layer.
- Create structure makes the layer folders.
- The Project map page is a canvas.

### Task 2.1: the catalog in `core-standards`
- **Files:**
  - `crates/core-standards/src/standards.yaml`, rewritten as base v2;
  - the loader;
  - its `contract.yaml`.
- **Interfaces:**
  - `LayerSpec { id: String, role: String, required: bool, elements: Vec<String>, may_use: Vec<LayerRef> }`
  - `LayerRef { part_kind: Option<String>, layer: String }`, where `None`
    means the same part.
  - `PartSpec.layers: Vec<LayerSpec>`.
- **v2 content:**
  - the layers in spec §5.2, with infrastructure holding the
    repositories;
  - rules S1 to S9; S3 reads "Every part has a contract head in Shadows";
  - the stack and profile mapping from spec §10: grouping parts, module
    identity and role applicability;
  - same-layer permissions and `implements` targets follow spec §5.2.
    Allowed edges are not actual uses.
- **Tests:**
  - `v2_names_required_backend_layers`
  - `every_may_use_points_at_a_known_layer`
  - `api_contract_has_no_implementation_dependency`
  - `a_profile_keeps_required_rules_and_explicit_applicability`

### Task 2.2: `folder`, `layers` and the migration
- **Files:**
  - `core-design/src/model.rs`;
  - the parts store, with a storage migration;
  - the OpenAPI types;
  - `docs/shared/requests/<date>-part-folder-and-layers.md`.
- **Interfaces:**
  - `PartContent.folder: String` replaces `folders`.
  - `PartContent.layers: Vec<PartLayer>`.
    - `PartLayer` has a stable id, a role, and a project-relative folder
      inside the part.
    - The final serialized shape belongs to the shared request.
- **Migration:**
  - the stored projects' standards base moves to v2;
  - each part keeps its first folder, and an empty list becomes the part's
    slug;
  - layers start empty;
  - frozen map snapshots are rewritten the same way, because they are test
    data.
- **Tests:**
  - `a_stored_part_with_folders_reads_back_with_its_first_folder`
  - `the_daemon_starts_on_a_database_with_old_parts`
  - `a_part_round_trips_its_layers`

### Task 2.3: map rules for layers
- **File:** `core-design/src/map_rules.rs`.
- **New refusals:**
  - `missing_layer`
  - `unknown_layer`
  - `layer_outside_part`
  - `layer_folder_shared`
- **Tests:** one per refusal. Also cover:
  - nested layer overlaps;
  - Windows case aliases;
  - ancestor moves;
  - a grouping part that does not duplicate its clients' layers.
- Person edits and approval use the same owner policy.

### Task 2.4: layer folders on disk
- **Change:** `Structure::render` adds each layer folder.
- **Tests:**
  - `structure_makes_each_layer_folder`
  - `a_layer_folder_never_holds_a_generated_contract`

### Task 2.5: map card, tools and prompt carry layers
- **Files:**
  - `shadows-mcp/src/workspace_tools.rs` (the `map_propose` description);
  - `MapPropose`;
  - `prompt.txt` (layers at the map step).
- **Test:** `planner_mcp` asserts that the description names layers.

### Task 2.6 (web, F-): the Project map canvas
- **Files:**
  - `web/src/app/design/`: a new `map-canvas.tsx`, using the same React
    Flow setup as `PlanGraph`;
  - the trimmed served list in `map-part-details.tsx`.
- **Behaviour:**
  - Part nodes show their kind and folder. The part summary waits for
    slice 4.
  - Actual dependency edges wait for recorded elements.
  - An optional dashed catalog overlay is labelled "Allowed
    dependencies", with its own legend.
  - A layout may place the database on the bottom rank.
- **Tests:**
  - `the_canvas_draws_one_node_per_part`
  - `served_items_show_only_the_parts_own`
  - `permissions_do_not_render_as_actual_dependencies`
  - `multiple_frontends_keep_distinct_part_ids`
  - `map_and_list_preserve_selection_and_browser_back`
- **Spec §6.3 also applies:** shared theme, keyboard navigation, narrow
  layouts and query errors.
  - Expand the exact API hook, route, stream, contract and map paths into
    the frontend task.
  - Components do no fetching and no domain validation.

**Decide when the slice starts:**
1. Is the v2 frontend catalog one list for web and mobile, or two? (Spec
   open question 3.)
2. The stack and profile mapping in spec §10, before tasks are numbered.

---

## Slice 3: elements, schema first

**Outcome.**
- The Planner proposes elements in cards, and the person approves them.
- The layer view draws the schema as an ER diagram.
- An element panel shows the element's fields, location, symbols and
  links.

### Task 3.1: the element model and store (`core-design`)
- **Interfaces:**
  - `Element { id, revision, part, layer, kind, name, summary, module, fields, location, symbols, links }`
  - **Revisions:** approved revisions are immutable. Current heads and
    tombstones keep history.
  - **Location:** typed as source, artifact or logical, as in spec §5.4.
    A table's implementation points to its migration elements.
  - **Module:** a stable identity. A backend with no split uses one default
    module.
  - `ElementFields` is one enum variant per kind. The schema slice has
    `Table { columns: Vec<Column>, keys: Vec<Key> }`.
  - `Symbol { name, signature, job }`
  - `ElementLink { kind: LinkKind (Uses|Implements|Reads|Writes|Calls), to: ElementId }`
- **Store:** new tables `element` and `element_link`, ordered by an
  explicit ordinal.
- **Tests:**
  - `an_element_round_trips`
  - `links_keep_their_order`
  - `a_table_uses_migrations_without_claiming_their_folder`
  - `renaming_an_element_preserves_id_and_old_revision`
  - `removing_an_element_retains_its_historical_content`

### Task 3.2: element proposals and approval
- **Interfaces:**
  - `Design::propose_elements(&Grant, ElementPropose) -> ProposalCard`
  - `approve_element_proposal` and `approve_element_proposal_as_planner`
- **Rules:**
  - the kind is allowed by its layer;
  - a link's direction obeys S7 (`may_use`);
  - a table's foreign keys name tables that exist;
  - validation runs over the complete candidate batch, including ids within
    the same card. Same-layer schema links can form a cycle without becoming
    execution edges;
  - CommandId replay and revision checks run in the same transaction as the
    approved revision write, with §24 authority and provenance.
- **Tests:**
  - `a_domain_element_cannot_link_to_a_table`
  - `a_foreign_key_to_a_missing_table_is_refused`
  - `one_card_carries_many_elements`
  - `same_card_mutual_foreign_keys_resolve_atomically`
  - `a_stale_element_card_cannot_overwrite_a_new_revision`
  - `lost_approval_response_replays_one_version`
  - `a_foreign_project_element_is_refused`
  - `a_repository_cannot_claim_another_modules_table`

### Task 3.3: the MCP tools
- **Tools:**
  - `element_propose` and `element_approve`, for the thread grant;
  - `layer_get`, for the thread and project grants.
- **Files:** `core-grants/src/tools.rs` (both lists) and
  `shadows-mcp/src/workspace_tools.rs`.
- **Test:** `planner_mcp` lists them per grant.

### Task 3.4: HTTP
- **Routes:**
  - `GET /api/projects/{id}/layers/{part}/{layer}`;
  - the card approve route.
- **Also:** a shared request and the OpenAPI document. They define:
  - bounded pagination;
  - revisions;
  - stale, unknown and forbidden results;
  - invalidation events.
- No route is invented inside a component.

### Task 3.5 (web, F-): the layer view and element panel
- **Files:** `web/src/app/design/layer-view.tsx`, `element-panel.tsx`, and
  the element card in the conversation.
- **Tests:**
  - `the_schema_view_draws_a_card_per_table_and_an_edge_per_foreign_key`
  - `the_element_panel_shows_location_and_symbols`
  - `logical_elements_show_implementation_links`
  - `a_stale_card_preserves_the_draft_and_offers_review`

### Task 3.6: further element kinds
`use_case`, `port`, `endpoint`, `repository`, `screen` and `client_call`,
each with its fields and its view:
- a list for use cases;
- operations grouped by path for the API;
- a route tree for screens.

**Gate:** persisting elements needs an approved map with applicable layers.
Proposals can be discussed earlier. Browsing never moves the session's
anchor.

---

## Slice 4: planning detail and the part summary

### Task 4.1: computing detail (`core-design`)
- **Interfaces:**
  - `Design::depth(&ProjectId) -> ProjectDepth { parts: Vec<PartDepth> }`
  - `PartDepth { part, summary: PartSummary, layers: Vec<LayerDepth> }`
  - `PartSummary { described, specified, need_decisions, inventory_confirmed: bool }`
  - `LayerDepth` gives counts per state, the source revisions and the
    unresolved requirements.
  - The states are `NotRecorded`, `Designed` and `ImplementationSpecified`.
    There is no `Built`.
- **Scope:** this read is project-wide planning detail. It makes no claim
  about whether a project stage is ready.
- **Tests:**
  - `a_layer_with_an_unresolved_link_is_not_designed`
  - `kind_specific_requirements_decide_implementation_detail`
  - `an_empty_optional_layer_is_left_out_of_the_summary`
  - `one_table_does_not_certify_a_whole_layer`
  - `an_unknown_inventory_never_reports_complete`
  - `the_part_summary_counts_described_specified_and_open`

### Task 4.2: detail over MCP and HTTP
- `depth_get`, for both grants;
- `GET /api/projects/{id}/depth`, with a shared request.

### Task 4.3 (web, F-): detail on the map
- Each part node shows its summary sentence, for example "8 described, 5
  specified, 3 need decisions". It is marked "list not confirmed" until
  confirmed.
- The panel lists layer counts and the missing requirements.
- State is shown as text plus colour.
- Empty, partial and failed-query states are distinct.

---

## Slice 5: tasks name elements, the approval gate, `task_context`

Build order: 5.1 → 5.2 → 5.3 → 5.4 → 5.5.

### Task 5.1: task elements (`core-plans`)
- **Interfaces:**
  - A task binding carries the element id, the approved revision and an
    access mode (`read`, `create`, `change` or `remove`).
  - `plan_edit`'s task operations carry `elements`.
  - Element locations suggest paths. The author reviews and freezes the
    full scope, including tests, manifests and generated files.
  - A new non-element task needs an explicit kind, scope and reason.
- **Tests:**
  - `a_task_derives_its_paths_from_its_elements`
  - `reading_a_port_does_not_grant_a_write`
  - `moving_an_element_does_not_change_frozen_task_scope`
  - `an_empty_element_list_cannot_bypass_new_task_validation`
  - `a_test_or_manifest_can_be_in_reviewed_explicit_scope`

### Task 5.2: the element approval gate
- **What:** a new blocker, `element_not_specified`, read through
  `core-design` (`core-plans` already depends on it).
- **It also reports:**
  - stale revisions;
  - missing agreements;
  - unresolved decisions;
  - overlapping resources that have no ordering constraint.
- All approval facts are read in one transaction snapshot.
- **Tests:**
  - `approval_is_blocked_by_a_designed_element`
  - `a_removed_element_blocks_approval_by_name`
  - `two_elements_in_one_file_have_a_shared_write_conflict`
  - `a_read_write_dependency_requires_explicit_order`
  - `shared_generated_files_and_migration_order_are_not_independent`

### Task 5.3: `task_context`, the builder's question
- **Interface:**
  `Plans::task_context(&Grant, workflow_id, task: u32) -> TaskContext`.
  - It returns the fields, provenance and budget diagnostics in spec §7,
    with exact plan, element and agreement revisions.
  - This read grants no execution authority.
  - `ElementContext` carries kind-appropriate source, artifact or logical
    locations.
  - For observed symbols, it adds `where_is` and `who_uses` from
    `core-code`.
- **MCP:** `task_context`, for both grants.
- **Also:** add `task_context` to the `SHADOWS.md` text.
- **Tests:**
  - `task_context_names_files_and_the_port_it_calls`
  - `an_external_agent_reads_task_context`
  - `required_context_over_budget_is_not_silently_dropped`
  - `a_foreign_project_grant_cannot_read_task_context`
  - `task_context_preserves_frozen_locations_and_labels_live_observations`
  - `resuming_without_conversation_history_retains_task_obligations`

### Task 5.4 (web, F-): element chips and the blocker text
- The chips appear on task cards; the blockers appear in the plan header.
- Blockers link to their target, with exact versions.
- Back navigation restores the selected task.
- A failed blocker read shows an error, never zero blockers.

### Task 5.5: design changes reach the anchored Planner context
- Follow `AppCore` to the existing context compilation and recording
  owners. Expand their exact contracts and paths before dispatch.
- Select the scoped layers and element revisions within the existing
  budget and provenance policy. Invalidate on those revisions.
- **Tests:**
  - `an_approved_element_refreshes_affected_anchored_context`
  - `browsing_a_layer_does_not_change_the_planning_anchor`

**Slice 5 acceptance:**
1. Plan one Stage 1 task through its schema, port, repository and endpoint
   elements.
2. Approval is refused while one of them is only designed, and passes once
   it is specified.
3. A fresh external agent reads the task's context, and it names the files
   and the port.
4. No executor and no completed-stage verdict is claimed.

---

## Slice 6: outcome coverage, "is this project stage fully planned?"

Comes right after slice 5, because it needs task bindings. The phases in
slice 7 do not need it.

### Task 6.1: stable outcome criteria (`core-design`)
- Each acceptance item of an outcome gets a stable identity and a revision.
- Editing a criterion changes its revision. Removing it keeps its history.
  A recreated criterion never reuses an old identity.
- Define the wire change in a shared request.
- **Tests:**
  - `recreated_criterion_does_not_reuse_identity`
  - `editing_a_criterion_changes_its_revision`

### Task 6.2: coverage bindings and the composed read (`core-plans`)
- **Ownership:** Plans owns the criterion bindings. A binding points at
  element revisions, at task acceptance items in exact plan versions, and
  at agreements. Plans reads the criterion revision through Design.
- **The read** takes project, outcome id and expected revision. It returns:
  - each criterion's coverage;
  - missing decisions and stale references;
  - `Needs planning` or `Planning covered`;
  - execution evidence marked unsupported;
  - `next`, with a blocker code, a reason and target ids.
- **Writes** carry a CommandId, expected revisions and project authority.
- **Tests:**
  - `a_linked_plan_alone_does_not_cover_an_outcome`
  - `a_required_unplanned_reply_path_blocks_stage_one`
  - `deferred_required_work_still_blocks_the_original_outcome`
  - `an_unplanned_stage_two_does_not_block_stage_one`
  - `changing_one_criterion_marks_only_its_coverage_stale`
  - `an_outcome_revision_change_refuses_a_stale_coverage_read`

### Task 6.3: the gate checks the candidate's coverage
- **What:**
  - Plan approval validates the candidate together with its assigned
    coverage, then pins the resulting version atomically.
  - Bindings to other drafts stay provisional.
- **Tests:**
  - `a_candidate_plan_and_its_coverage_freeze_atomically`
  - `a_stale_criterion_rolls_back_plan_approval_and_coverage`
  - `backend_can_approve_while_frontend_coverage_is_provisional`
  - `outcome_is_covered_only_after_all_required_plan_bindings_are_approved`

### Task 6.4 (web, F-): the Roadmap coverage view
- **What it shows, per spec §6.1:**
  - each outcome's coverage rows;
  - its blockers, each with one next action;
  - its related plans and parts.
- "Continue planning" opens an anchored draft. It never sends a message or
  approves anything by itself.
- The page labels the project stage so it cannot be confused with a
  planning step or a delivery slice.
- **Before dispatch:** this view needs a UI prototype.

### Task 6.5: coverage reaches the anchored Planner context
- **Test:** `compaction_retains_outcome_blockers_and_source_versions`.

**Slice 6 acceptance:**
1. In testing56, Stage 1 reports `Needs planning`, naming "the reply reaches
   the customer in Telegram" as uncovered.
2. Planning it moves Stage 1 to `Planning covered`.
3. Stage 2 stays coarse and blocks nothing.

---

## Slice 7: phases inside tasks

### Task 7.1: the phase model (`core-plans`)
- **Interfaces:**
  - `TaskContent.phases: Vec<Phase>`.
  - A `Phase` has a stable id, a display number, a goal, a subset of the
    task's element bindings, acceptance references and `waits_for`.
  - A `PhaseRef` pins the plan, version, task and phase identity. A draft's
    UI can suggest a newer approved target; frozen content never retargets
    itself.
- **Rules:**
  - a phase's elements are a subset of its task's;
  - implicit sequential phase edges, explicit waits and the existing task
    `needs` and `completes_after` form one event graph, extended in
    `dependency_graph.rs`;
  - there is no cycle across the combined graph;
  - missing or removed targets are blockers;
  - new cross-plan phase links stay inside the current project.
- **Tests:**
  - `phase_elements_must_belong_to_the_task`
  - `a_cycle_through_another_plan_is_refused`
  - `a_wait_on_an_unapproved_plan_is_refused`
  - `implicit_phase_order_and_task_links_can_form_a_cycle`
  - `a_new_target_version_does_not_redirect_a_frozen_phase_wait`
  - `reordering_phases_preserves_their_identity`
  - `a_removed_phase_is_a_named_blocker`
  - `concurrent_approvals_cannot_commit_a_combined_cycle`

### Task 7.2: plan editing and tools
- The `plan_edit` operations `phase_put` and `phase_remove`, with the MCP
  descriptions and the prompt.
- **Editing rules:**
  - every operation is atomic under the expected revision and command
    replay;
  - edits to a frozen version are refused;
  - phase numbers are never reused as ids;
  - the prompt never marks a phase complete.
- `task_context` takes an optional stable phase id.
- **Tests:**
  - `phase_context_keeps_task_acceptance_and_pinned_dependencies`
  - `a_phase_outside_the_named_task_is_refused`

### Task 7.3 (web, F-): the task view
As in mockup screen 4:
- the same React Flow look as `PlanGraph`;
- the task is the start node;
- other tasks' phases are dashed nodes;
- the Back button returns to the plan graph.

### Task 7.4 (web, F-): Show phases on the plan graph
- **The button:** Show phases / Hide phases, in the plan header beside
  Versions.
- **When shown:**
  - every task is a React Flow group node: a frame with its number and
    title;
  - its phases are child nodes, with badges such as `T3 · P2`;
  - explicit waits join phase nodes;
  - whole-task constraints keep their start and completion endpoints and
    their labels;
  - tasks with no phases remain plain nodes.
- **Layout:** left to right by dependency rank, as today. Frames size to
  their phases.
- **Tests:**
  - `show_phases_draws_a_frame_per_task_with_its_phases`
  - `an_edge_points_from_prerequisite_to_waiting_phase`
  - `hide_phases_returns_to_task_cards`
  - `expansion_preserves_whole_task_constraints_and_tasks_without_phases`
  - `foreign_phase_nodes_show_the_pinned_version`
  - `expansion_keeps_selection_and_viewport`
  - `keyboard_list_and_graph_open_the_same_task`

**Slice 7 acceptance:**
1. Open Stage 1, toggle Show phases, follow a foreign phase, and come back
   without losing the selection.
2. Exercise a task without phases, and both task dependency kinds.
3. Check the 1024px and 375px layouts, and a fixture of 100 tasks and 400
   phases.

---

## Slice 8: the trace view

### Task 8.1: a bounded `Design::trace` at approved element revisions
- **The walk:** it follows links across parts, both ways, up to the first
  client and the deepest table.
- **It returns:**
  - each edge's kind and direction;
  - the source revision;
  - the visited identities;
  - the truncation reason, with a continuation when bounded.
- Schema cycles and multiple clients cannot cause an infinite walk or an
  invented linear path.
- **Tests:**
  - `the_tickets_table_traces_to_the_queue_screen`
  - `a_cyclic_schema_trace_terminates`
  - `a_large_trace_marks_truncation`
  - `an_unrecorded_link_stays_unknown`

### Task 8.2 (web, F-): the trace in the element panel
As in mockup screen 3.

---

## Slice 9: code observation

**Waits for:** slice 5's location bindings.
- Rust, TypeScript and JavaScript can exercise the first path.
- PHP and Dart observations need the separate index owner's accepted task.
  Neither blocks planning for those languages.

### Task 9.1: index grammars
- `tree-sitter-php` and `tree-sitter-dart` in `shadows-index`, with tag
  queries.
- **Tests:** a PHP class and method are tagged; a Dart class and function
  are tagged.
- **Owner:** assigned through the code-intelligence owner, with its own
  scope and contracts.

### Task 9.2: observe an implementation without certifying it
- `core-plans` composes the existing `core-code` observations with task
  context. Design state stays independent of the disposable index.
- **It exposes:**
  - `found`, `missing`, `ambiguous`, `stale` or `unsupported`;
  - the source digest;
  - qualified target details.
- **Tests:**
  - `a_found_stub_does_not_complete_a_task`
  - `an_unsupported_language_is_not_missing_code`
  - `same_named_methods_require_disambiguation`
  - `a_changed_source_digest_invalidates_the_old_observation`

## Acceptance inventory

Every slice prepares its exact tests and live journey before `todo`. The
controller records each result against the checked commit. Plan checkboxes
stay open until that evidence exists.

| Concern | Tasks |
|---|---|
| Missing folder, moves and renames | 1.1, 2.3 |
| No contract files; unfinished old write | 1.2, 2.4 |
| Migration on the daily database | 2.2 |
| Standards fit | 2.1 |
| Revisioned elements, same-card links and deletion | 3.1, 3.2 |
| Part summary and unknown inventory | 4.1, 4.3 |
| Frozen scopes, agreements and shared files | 5.1, 5.2 |
| Agent context, missing inputs and compaction | 5.3, 5.5, 6.5 |
| Is a project stage fully planned | 6.1–6.4 |
| Task and phase ordering, both graph modes | 7.1–7.4 |
| Bounded trace and uncertain code matches | 8.1, 9.2 |
| Theme, accessibility, loading and navigation | 2.6, 3.5, 4.3, 5.4, 6.4, 7.4 |

Execution evidence, verification and final acceptance of a project stage
are tracked by the §17 delivery. This plan shows their absence honestly.
