# Structure and layers: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shadows holds each project's structure as principles: parts with
one folder each, and layers from the standards with where they live in the
chosen stack. It checks the principles, never a folder layout. It gives the
agent building a task bounded context, and lets a task have phases.

**Architecture:** No new service.
- `core-standards` holds the layer catalog and rules.
- `core-design` holds parts and their layers, and the map rules.
- `core-plans` holds `task_context`, phases and the combined graph. It
  reads `core-design` through its service API.
- Adapters translate only: HTTP in `shadows-http`, MCP tools in
  `shadows-mcp`.
- The web views follow spec §6.

**Tech Stack:** Rust 1.94+, SQLx 0.9, SQLite, rmcp, axum; React and React
Flow in `web/`.

**Spec:** [docs/backend/specs/2026-11-05-structure-and-depth-design.md](../specs/2026-11-05-structure-and-depth-design.md).

**Words.** These terms are defined at the top of the spec and used with one
meaning each:
- a **planning step** is one of Idea, Vision, Map, Roadmap or Plans;
- a **project stage** is a roadmap outcome, such as "Stage 1";
- a **delivery slice** is slice 1 to 5 below.

**Mohammed's decisions:**
- **2026-11-05**
  - **No backward compatibility:** every existing project is test data.
  - **Shadows never writes `contract.yaml`** after slice 1.
  - **The task view and Show phases are approved.**
- **2026-11-06**
  - **The standards hold principles, not a layout.**
  - **Each runnable part has one owner object at runtime.**
  - **Elements, element detail and the trace view are dropped.**
  - **No command writes the structure.** An executor task or the Planner
    in Agent Mode writes it. Create structure, the Structure step and
    `SHADOWS.md` are removed.

**How this plan grows.**
- Slices 1 and 2 are built. Their sections below stay as the record of what
  was built.
- Slice 3 is implemented locally as B-71 to B-75 and F-92, through
  `3deb1333`. Its automated checks passed; Mohammed will perform manual
  acceptance on an existing project. Slices 4 and 5 are prepared below for inline execution on the same branch.

## Dispatch and progress

The controller prepares one runnable vertical slice at a time, including its
UI and API dependencies.

| Slice | Prerequisite | Planning state | Exit evidence |
|---|---|---|---|
| 1 | Caller inventory of the four write paths | Implemented in PR #59 (`26bf739e`); live acceptance not recorded | Proposal, approval, person edit and structure agree; no contract written; old unfinished write cancelled |
| 2 | Slice 1; accepted API request (B-64) before F-90 | Implemented in PR #60 (`b64242a7`), with B-68 to B-70 tooling; live acceptance not recorded | Layers on the map card, migration on the daily database, canvas in the app |
| 3 | Slice 2; accepted API request `2026-11-06-principles-not-layout` | Implemented locally through `3deb1333`; manual acceptance pending with Mohammed | A Laravel map with a nested database part is approved; Roadmap opens with no Structure step |
| 4 | Slice 3 | B-76–B-78 implemented and automated checks passed; live acceptance pending | A fresh external agent reads a task's parts, layers and rules |
| 5 | Slice 4; accepted API request for task phases | B-79–B-81, F-93–F-94 prepared; API request accepted; implementation pending | Mixed dependency cycles refused; both graph modes work |

**Dropped on 2026-11-06:** the former slices 3 (elements), 4 (planning
detail), 6 (outcome coverage on elements), 8 (trace) and 9 (code
observation). Outcome coverage on tasks and code observation of the layer
rules are deferred, with no plan.

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
- **Shadows checks principles, never a folder layout.** No rule may refuse a
  map because of where a layer's place sits.
- **This work covers planning only.** Task execution, phase completion and
  verified completion of a stage wait for §17.

## Review Focus

1. **A Laravel-shaped map.** The database part is a child of the backend,
   and the connection's place is outside the database part's folder. It must
   be approved. (Task 3.2:
   `a_laravel_map_with_a_nested_database_part_is_approved`.)
2. **Two layers of one part at the same place.** It must still be refused
   with `layer_place_shared`, in any letter case. (Task 3.2.)
3. **A project whose old map passed the Structure step.** After the
   removal it reads as Map approved, and Roadmap opens. (Task 3.3:
   `a_project_past_the_old_structure_step_opens_roadmap`.)
4. **The daemon on the daily database after the `files` field is removed.**
   It starts, and parts read back. (Task 3.1:
   `parts_with_starter_files_read_back_after_the_migration`.)
5. **Phase waits that form a cycle through another plan.** Refuse them,
   naming the whole path. (Task 5.2:
   `a_cycle_through_another_plan_is_refused`.)

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
- Each part kind has a catalog of layers, and v2 replaces v1 for every
  project.
- A map card carries each part's one folder and its layers.
- Shadows refuses a map that misses a required layer.
- Create structure makes the layer folders.
- The Project map page is a canvas.

**Base:** main at `26bf739e` (slice 1 merged in PR #59).
**Owners:**
- `core-standards`: the catalog;
- `core-design`: parts, map rules and structure;
- `core-harness`: the standards text and the prompt;
- `shadows-mcp`: the tool description;
- `web/`: F- tasks.

### Decisions for this slice

D1 to D4 close the former spec §10 OPEN 2 (stack profile mapping).
Mohammed left the choice to the controller on 2026-11-05. The spec records
them in §§5.1–5.3.

- **D1. A layer has one place.**
  - `place` is a project-relative path inside the part's folder.
  - **One folder per layer:** a layered stack writes
    `backend/app/Domain`.
  - **One place per module:** a module-per-folder stack (service-per-crate
    Rust) writes one `{module}` marker in the place, for example
    `crates/core-{module}/src/model.rs`.
  - A place with `{module}` is a pattern. It may name a file or a folder.
    Shadows creates nothing for it until modules exist as elements
    (slice 3).
  - A place without `{module}` is a folder, and Create structure makes it.
  - S9 reads: "Each part owns one folder. Each layer owns one place inside
    its part: a folder, or one path per module."
- **D2. A required layer may be marked not used,** with a written reason.
  - Its `place` is then empty.
  - The person approves the reason with the map card, like a part waiver.
  - The base rules stay mandatory. A project's additions cannot remove a
    required layer.
- **D3. A grouping part needs no layers.** A part whose children include a
  part of the same kind is a grouping part, for example a `frontend` that
  holds `web` and `mobile`. The map shows this, and nothing new is stored.
- **D4. One frontend catalog** serves web and mobile clients alike. Each
  client is its own part (this answers the slice's former question 1).
- **Out of this slice:**
  - layers added by project additions (additions keep adding parts and
    rules only, and an added part kind has no catalog layers);
  - S7 and S8 checks, which need element links (slice 3).

### Task 2.1 (B-63): the v2 catalog in `core-standards`

**Files:**
- Modify: `crates/core-standards/src/standards.yaml`, `src/model.rs`,
  `src/base.rs`, `src/validation.rs`, `src/lib.rs`, `contract.yaml`
- Modify: `crates/core-harness/src/standards.rs`, `contract.yaml`:
  - drop the contract template from the Planner's standards text;
  - print each kind's presence (`required`, `waivable with a reason`,
    `optional`) in place of `waivable`.
  The layers are added to this text in Task 2.5.
- Modify: `crates/core-design/src/store/map_proposal_shape.rs`
  (`MapProposalShapeRows::validate_args` reads `presence == Waivable` in
  place of `waivable`), `crates/core-design/contract.yaml`
- Modify: `crates/shadows-core/src/lib.rs` (re-exports)
- Test: `crates/core-standards/tests/base.rs`,
  `crates/shadows-core/tests/design_vision/standards.rs`,
  `crates/shadows-core/tests/design_parts/map_proposals.rs` (the waiver
  tests, unchanged in meaning)

**Interfaces (produces):**
- `pub struct PartKind { name: String, owns: String, presence: Presence, layers: Vec<LayerSpec> }`.
  It replaces `MandatoryPart`.
- `pub enum Presence { Required, Waivable, Optional }`, with serde
  `snake_case`.
- `pub struct LayerSpec { id: String, role: String, required: bool, elements: Vec<String>, may_use: Vec<LayerRef> }`
- `pub struct LayerRef { part_kind: Option<String>, layer: String }`.
  - `None` means the same part.
  - The YAML writes it as `"application"` or `"api.contract"`.
- `BaseStandards { version: i64, parts: Vec<PartKind>, rules: Vec<StandardRule> }`.
  `contract_template` is removed.
- `EffectiveStandards::mandatory_parts()` keeps its signature and leaves
  out `Optional` kinds.
- `EffectiveStandards::layers(&self, kind: &str) -> &[LayerSpec]`. An added
  or unknown kind returns `&[]`.

**v2 content:** `version: 2`, and the five kinds and their layers exactly as
spec §5.2 tables. Presence:
- backend: required;
- database: required;
- api: waivable;
- frontend: waivable;
- integration: optional.

Rules:
- S1 to S6 are kept. Change S3 to "Every part has a contract head in
  Shadows." Change S6 to "The backend and the database are separate parts,
  each with its own folder."
- Add S7, S8 and S9, with the texts from spec §5.2. S9 uses the D1 wording.

- [ ] **Step 1: Write the failing tests.** Add them to
  `crates/core-standards/tests/base.rs`:
  - `v2_names_required_backend_layers`: `base().version == 2`. Backend's
    required layer ids are
    `["domain","application","interface","infrastructure","composition"]`,
    in that order.
  - `every_may_use_points_at_a_known_layer`: each `LayerRef` resolves to a
    kind and layer in the base.
  - `api_contract_has_no_implementation_dependency`: the `may_use` of
    `api.contract` is empty, and no `domain` layer has any `may_use`.
  - `integration_is_optional_and_not_mandatory`: `mandatory_parts()` on
    base-only standards is
    `["backend","database","api","frontend"]`.
  - Rename `the_base_standards_parse_with_the_four_mandatory_parts` to
    `the_base_standards_parse_with_five_kinds_and_rules_s1_to_s9`.

- [ ] **Step 2: Run them and see them fail.**
  Run `cargo test -p core-standards --test base`. The new names fail.

- [ ] **Step 3: Rewrite the YAML and the parser.** The parser reads
  `presence`, `layers` and `may_use`.
  - A `may_use` text with one `.` is `kind.layer`. Without a `.`, it is
    the same part.
  - Remove the `contract_template` parsing.

- [ ] **Step 4: Update the callers.** No crate may still read `waivable`
  or `contract_template` when this task commits.
  - In `core-harness/src/standards.rs`, delete the "Contract template"
    block and print the presence.
  - In `map_proposal_shape.rs`, a first-map waiver is accepted only for a
    kind whose presence is `Waivable`. An `Optional` kind needs no waiver,
    and a waiver naming it is refused with the existing message.
  - In `shadows-core/src/lib.rs`, re-export `PartKind`, `Presence`,
    `LayerSpec` and `LayerRef` in place of `MandatoryPart` and
    `ContractTemplate`.
  - Update `design_vision/standards.rs` to the new base.

- [ ] **Step 5: Run the tests and see them pass.**
  - `cargo test -p core-standards`
  - `cargo test -p shadows-core --test design_vision --test design_parts`
  - `cargo test -p core-harness`
  - `cargo test -p core-design`

- [ ] **Step 6: Update the contracts and commit.** Rewrite the shapes and
  the test list in `core-standards/contract.yaml`. Update the waiver line in
  `core-design/contract.yaml` and the standards text line in
  `core-harness/contract.yaml`. Then commit:
  `feat(backend): B-63 standards v2 names each kind's layers`.

### Task 2.2 (B-64): one `folder`, `layers`, and the migration

**Files:**
- Modify: `crates/core-design/src/model.rs`; every reader of `folders` in
  `core-design/src`:
  - `map_rules.rs`, `structure.rs`, `structure_create.rs` and
    `structure_write.rs`;
  - `store/structure.rs` and `store/roadmap_seed.rs`.
- Create: `crates/core-db/migrations/0047_part_folder.sql`
- Modify: `crates/core-design/contract.yaml`;
  `crates/shadows-mcp/src/workspace_tools.rs` (the word "folders");
  `api/openapi.json`; `web/src/api/schema.d.ts`.
- Create: `docs/shared/requests/2026-11-05-part-folder-and-layers.md`.
  - Status `asked`.
  - Fields: the new `PartContent` shape, the v2 `BaseStandards` shape, and
    the new refusal rules from Task 2.3.
- Fixtures only, with `folders: vec![..]` becoming `folder: ".."`:
  - `crates/core-design/tests/map_rules.rs`, `structure.rs`;
  - `crates/core-turns/tests/planning_context.rs`;
  - `crates/shadows-core/tests/agreement_bindings.rs`,
    `agreement_review.rs`, `code_index.rs`, `design_migration.rs`,
    `design_outcomes.rs`, `design_parts.rs`, `design_parts/map.rs`,
    `design_parts/map_proposals.rs`, `design_parts/stage.rs`,
    `design_parts/structure.rs`, `plan_migration.rs`;
  - `crates/shadows/tests/agreement_journey.rs`, `design_routes.rs`,
    `map_proposal_coverage.rs`, `openapi/read_only.rs`,
    `planning_context.rs`, `project_events.rs`, `workspace_mcp.rs`,
    `workspace_mcp/approval.rs`, `workspace_mcp/roadmap.rs`.
- Test: `crates/shadows-core/tests/design_migration.rs`.

**Interfaces (produces):**
- `PartContent.folder: String`, with `#[serde(default)]`. It replaces
  `folders: Vec<String>`.
  - An empty folder is refused by the existing `one_folder` rule. Its text
    changes to "part {title} has no folder. Give the part one folder."
  - An old JSON value that still has `folders` reads with `folder` empty,
    because unknown fields are ignored.
- `PartContent.layers: Vec<PartLayer>`, with `#[serde(default)]`.
- `pub struct PartLayer { layer: String, place: String, not_used: Option<String> }`.
  It derives the same traits as `InitialFile`, including `JsonSchema`.
- `MapRules::normalise` normalises `folder` and each non-empty `place`.
  - It keeps one `{module}` marker literally.
  - It refuses a second marker.

**Migration** (`0047_part_folder.sql`). Crude on purpose, because the data
is test data.
- `design_part.content_json`:
  - set `$.folder` to `$.folders[0]`;
  - when that is missing, use `lower(replace(trim($.title),' ','-'))`;
  - remove `$.folders`;
  - set `$.layers` to `[]`.
- `design_map_version.parts_json`: an array of rows, each holding its
  part as a JSON *string* in `content_json`. Rewrite that string with the
  same change, through `json(...)` and back to text, for each element. Keep the element order with `json_group_array(... ORDER BY key)`
  over `json_each`.
- `design_map_proposal`: every `Pending` row becomes `Superseded`. The
  Planner proposes again under v2. Old payloads still read, with an empty
  folder.
- Existing `standards_version` columns keep `1`, because they record what
  was used. New writes record `2`.

- [ ] **Step 1: Write the failing tests** in `design_migration.rs`:
  - `a_stored_part_with_folders_reads_back_with_its_first_folder`: insert
    a v1 row whose `folders` is `["api","api/v2"]`. Run the migrations.
    Then `folder == "api"` and `layers` is empty.
  - `a_part_without_folders_gets_its_title_slug`: a part titled `Web App`
    with `folders` `[]` gets the folder `web-app`.
  - `the_daemon_starts_on_a_database_with_old_parts`: open a database file
    seeded at migration 0046 with a part, a map version and a pending map
    proposal. Then:
    - `AppCore::start` succeeds;
    - the workspace read returns the part;
    - the proposal reads as `Superseded`.
  - `a_part_round_trips_its_layers`: a part with two layers, one of them
    `not_used: Some("no clients yet")`, reads back equal.

- [ ] **Step 2: Run them and see them fail.**
  Run `cargo test -p shadows-core --test design_migration`.

- [ ] **Step 3: Change the model, the readers and the fixtures,** and write
  the migration.

- [ ] **Step 4: Regenerate the API files.**
  - `cargo test -p shadows --test openapi` rewrites `api/openapi.json`, as
    the repository already does.
  - Then run `npm run gen:api` in `web/`.

- [ ] **Step 5: Run the touched tests and see them pass.**
  - `cargo test -p core-design`
  - `cargo test -p shadows-core --test design_migration --test design_parts --test design_outcomes --test agreement_bindings --test agreement_review --test code_index --test plan_migration`
  - `cargo test -p core-turns --test planning_context`
  - `cargo test -p shadows --test design_routes --test workspace_mcp --test map_proposal_coverage --test agreement_journey --test openapi --test planning_context --test project_events`

- [ ] **Step 6: Commit.** Commit the request file and the contract with
  the code:
  `feat(backend): B-64 a part has one folder and its layers; old parts are migrated`.

### Task 2.3 (B-65): map rules for layers

**Files:**
- Modify: `crates/core-design/src/map_rules.rs`, `contract.yaml`
- Test: `crates/core-design/tests/map_rules.rs`;
  `crates/shadows/tests/design_routes.rs` (one write-path regression)

**Interfaces:**
- `MapRules::check_shape` keeps its signature. It reads the catalog
  through `standards.layers(kind)`.
- The checks run on every part in the candidate map, not only `written`.
  Proposal, approval and person edit all reach them, as in slice 1.

**New refusals.** Each message names the part and the layer, and says the
fix.

| Rule | When |
|---|---|
| `missing_layer` | A part of a catalog kind, which is not a grouping part (D3), lacks a required layer and has no `not_used` entry for it |
| `unknown_layer` | A layer id is not in its kind's catalog, appears twice, or the part has no catalog kind |
| `layer_outside_part` | A place, or any one of a pattern place's expansions, is not inside the part's folder |
| `layer_place_shared` | Two layers of one part can name the same path, or one can lie inside another, for some module name (case-insensitive) |
| `layer_not_used_without_reason` | `not_used` is blank, or `not_used` is set while `place` is not empty |
| `layer_inside_child` | A place, or any expansion of a pattern place, can lie inside a child part's folder |

**Pattern places: two different questions.** `{module}` stands for any
non-empty text without `/`. It may be a whole segment (`backend/{module}`)
or part of one (`crates/core-{module}`).

1. **Conflict: is there one module name that makes the paths overlap?**
   - It is used for `layer_place_shared` and `layer_inside_child`.
   - One possible overlap is enough to refuse.
   - Example: `backend/modules/{module}/src` and `backend/modules/users/src`
     conflict, because the module `users` makes them equal. So do
     `crates/core-{module}` and `crates/core-users`.
2. **Ownership: does every module name keep the path inside the folder?**
   - It is used for `layer_outside_part`.
   - One possible match is not enough. Every expansion must lie inside the
     part's folder.
   - It holds only when the part folder's segments are a literal prefix of
     the place, with no marker among them, and the place has at least one
     more segment.
   - Example: part folder `backend` and place `{module}/src` are refused.
     The module `backend` would put the place inside the folder, but the
     module `billing` puts it outside.

Neither question needs a list of modules. Slice 3 records modules as
elements. Its Task 3.2 checks the expanded places of each recorded module
again, on proposal and approval, with these same functions.

- [ ] **Step 1: Write the failing tests** in `map_rules.rs`, one per row
  above:
  - `a_missing_required_layer_is_refused`
  - `an_unknown_layer_is_refused`
  - `a_layer_outside_its_part_is_refused`
  - `two_layers_sharing_a_place_are_refused`
  - `a_not_used_layer_needs_a_reason_and_no_place`
  - `a_layer_inside_a_child_part_is_refused`

  Then the cases the slice must hold:
  - `a_grouping_part_needs_no_layers`: a frontend with children `web` and
    `mobile` of kind frontend, each with its four layers. The map passes.
  - `module_patterns_are_one_place_per_module`: these places pass:
    - domain `crates/core-{module}/src/model.rs`;
    - application `crates/core-{module}/src/lib.rs`;
    - infrastructure `crates/core-{module}/src/store`.
  - `a_second_module_marker_is_refused`
  - `a_pattern_conflicts_with_a_place_it_can_expand_to`:
    `backend/modules/{module}/src` and `backend/modules/users/src` in one
    part are refused with `layer_place_shared`.
  - `a_marker_inside_a_folder_name_conflicts_too`: `crates/core-{module}`
    and `crates/core-users` in one part are refused with
    `layer_place_shared`.
  - `a_pattern_that_can_reach_a_child_folder_is_refused`: the parent's
    place `backend/{module}` and a child part's folder `backend/billing`
    are refused with `layer_inside_child`.
  - `a_pattern_must_stay_inside_its_part_for_every_module`: part folder
    `backend` with place `{module}/src` is refused with
    `layer_outside_part`. Place `backend/{module}/src` passes.
  - `layer_places_use_windows_case_rules`: `Backend/App/Domain` and
    `backend/app/domain` count as one place.
  - `moving_a_parent_rechecks_child_layer_places`: a person edit moves a
    parent's folder, and a child's layer place then falls outside it.

  Add `a_map_without_a_required_layer_is_refused_on_every_write_path` to
  `design_routes.rs`, in the shape of slice 1's folder regression.

- [ ] **Step 2: Run them and see them fail.**
  - `cargo test -p core-design --test map_rules`
  - `cargo test -p shadows --test design_routes`

- [ ] **Step 3: Write the checks.**
  - Split the folder and place checks out of `check_shape` into
    `pub fn check_places(parts: &[MapPart<'_>], standards: &EffectiveStandards) -> Result<(), MapRefusal>`.
    This covers slice 1's `one_folder`, `folder_outside_parent` and
    `folder_claimed_twice`, and this task's layer rules.
    `check_shape` calls it, so there is still one policy.
  - Write two private functions over lowercase segments:
    - `fn may_overlap(a: &str, b: &str) -> Overlap { Equal, AInsideB, BInsideA, None }`
      answers question 1. A segment with `{module}` matches a literal
      segment that starts with the text before the marker and ends with the
      text after it, with at least one character between them. Two
      segments that both hold a marker match when their prefixes agree and
      their suffixes agree, meaning one is a prefix (or suffix) of the
      other. `layer_place_shared` and `layer_inside_child` use it.
    - `fn always_inside(place: &str, folder: &str) -> bool` answers
      question 2, as defined above. `layer_outside_part` uses it.
    - No rule uses one function for the other's question.

- [ ] **Step 4: Run the same commands and see them pass.**
  Then update the `obligations` in `contract.yaml`.

- [ ] **Step 5: Commit:**
  `feat(backend): B-65 the map refuses a missing, unknown or misplaced layer`.

### Task 2.4 (B-66): layer folders on disk

**Files:**
- Modify: `crates/core-design/src/structure.rs`,
  `src/store/structure.rs` (`Snapshot` and its loader), `contract.yaml`
- Test: `crates/core-design/tests/structure.rs`;
  `crates/shadows-core/tests/design_parts/structure.rs`

**Interfaces:**
- `Structure::render` adds every non-pattern, non-empty layer place to
  `Planned.folders`, with its parent folders as it already does for parts.
- Render runs the place policy, not the whole `check_shape`. Vision and
  served items were checked when the map was approved, and the snapshot is
  immutable. The folder and layer rules are what the disk write depends
  on.
- **Where the inputs come from:** the snapshot loader in
  `store/structure.rs` already reads the map version's `MapRow`s (`id`,
  `parent_id`) and the effective standards.
  - `Snapshot` gains `parents: Vec<(String, Option<String>)>`, in the same
    order as `parts`, and `standards: EffectiveStandards`.
  - `RenderInput` gains `parents: &[(String, Option<String>)]` and
    `standards: &EffectiveStandards`.
  - Render builds `MapPart`s from these and calls
    `MapRules::check_places` (Task 2.3) before its disk checks. A refusal
    becomes its message.
  - No default or empty value stands in for a missing input.
- The `SHADOWS.md` text says `Standards: base v2`.

- [ ] **Step 1: Write the failing tests:**
  - `structure_makes_each_layer_folder`: a backend with
    `backend/app/Domain` and `backend/app/Http` plans both folders.
  - `a_module_pattern_place_makes_no_folder`: no planned folder contains
    `{module}`.
  - `a_layer_folder_never_holds_a_generated_contract`: no planned file
    ends in `contract.yaml`.
  - `render_refuses_a_snapshot_that_breaks_the_map_rules`: a snapshot
    part with an empty folder is refused with the `one_folder` message,
    before any disk check.
  - `render_checks_child_folders_against_their_parents`: a snapshot whose
    child folder lies outside its parent's folder is refused with
    `folder_outside_parent`. This proves the parents reached render.

- [ ] **Step 2: Run and see them fail.**
  Run `cargo test -p core-design --test structure`.

- [ ] **Step 3: Implement it.** Then run
  `cargo test -p core-design --test structure` and
  `cargo test -p shadows-core --test design_parts`.

- [ ] **Step 4: Commit:**
  `feat(backend): B-66 create structure makes each layer's folder`.

### Task 2.5 (B-67): the Planner sees layers

**Files:**
- Modify: `crates/core-harness/src/standards.rs`, `src/prompt.txt`,
  `contract.yaml`
- Modify: `crates/shadows-mcp/src/workspace_tools.rs` (the `map_propose`
  description)
- Test: `crates/shadows/tests/planner_mcp.rs`

**Behaviour:**
- The standards text lists each kind with its presence. Under each kind it
  lists its layers, one line each: `id (required|optional): role. May use:
  ...`.
- In the prompt's map line, "folders" becomes "one folder". It adds:
  - "every required layer of each part's kind, each with a place inside
    the part's folder, chosen for the stack";
  - "write `{module}` in a place when the stack gives each module its own
    folder";
  - "mark a required layer not used only with a reason the person can
    approve";
  - "a grouping part needs no layers".
- The `map_propose` description names `folder` and `layers`.

- [ ] **Step 1: Write the failing tests** in `planner_mcp.rs`:
  - `the_map_tool_names_layers`: the listed `map_propose` description
    contains `layers` and `folder`, and does not contain `folders`.
  - `the_standards_text_lists_backend_layers`: the Planner's first-turn
    standards text contains `domain (required)` and `composition
    (required)`, and does not contain `Contract template`.

- [ ] **Step 2: Run and see them fail.**
  Run `cargo test -p shadows --test planner_mcp`.

- [ ] **Step 3: Implement it.** Then run `cargo test -p shadows --test
  planner_mcp` and `cargo test -p core-harness`.

- [ ] **Step 4: Commit:**
  `feat(backend): B-67 the Planner is told each kind's layers`.

### Task 2.6 (F-90): the part form, the map card and the standards page

These tasks need the shared request from B-64 to be `accepted`.

**Files:**
- Modify: `web/src/app/design/part-editor.tsx`
  - One "Folder" input.
  - A "Layers" table with one row per catalog layer of the chosen kind.
    Each row first asks **Used** or **Not used** (a two-option control).
    "Used" shows the place input. "Not used" shows the reason input, and is
    offered only for a required layer. An optional layer has no row until
    the person adds it. The two inputs never show together.
  - A grouping part (D3) shows "Groups: web, mobile", with a link to each
    child, in place of the layers table. The map card and part details show
    the same line, so missing layers never look like an error.
- Modify: `web/src/app/design/map-part-details.tsx` (folder and layers,
  read-only); `web/src/app/conversation/map-proposal-card.tsx` (layers on
  the card).
- Modify: `web/src/app/design/standards-view.tsx`
  - Kinds with their presence, and their layers as a table: id, role,
    required, may use.
  - The contract template section is removed.
- Modify: `web/src/test/fake-daemon.ts`,
  `web/src/test/contract-fixtures.ts`
- Test: `standards-view.test.tsx`, `parts-view.test.tsx`,
  `map-proposal-card.test.tsx`, `composer-bar.test.tsx`,
  `roadmap-proposal-card.test.tsx`, `create-structure.test.tsx` (fixtures)

**Tests:**
- `the_part_form_sends_one_folder_and_its_layers`
- `a_not_used_layer_needs_a_reason_before_save`
- `choosing_not_used_hides_the_place_and_asks_for_a_reason`
- `a_grouping_part_links_its_children_instead_of_layers`
- `the_standards_page_lists_each_kinds_layers`
- `the_map_card_shows_each_parts_layers`

**Done when:** `npx vitest run src/app/design src/app/conversation` passes
in `web/`.

### Task 2.7 (F-91): the Project map canvas
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
- Each node also shows its layer ids.

### Slice 2 acceptance (Mohammed, in the app)

1. Start the daemon on the daily database. The old test projects open, and
   each part shows one folder.
2. In a new project, the Planner proposes a map with layers. Approve it.
3. Removing a required layer in the part form is refused, with the message
   from Task 2.3.
4. Create structure makes the layer folders and no `contract.yaml`.
5. The Standards page shows v2 with its layers. The Project map shows the
   canvas.

---

## Slice 3: principles, not layout

**Outcome.**
- The standards are base v3: the same kinds and layers, no `elements`, and
  rules S6, S9 and S10 as in spec §5.2.
- The map is refused only on principle (spec §5.3). Where a place sits is
  free.
- No command writes the structure. Create structure, the Structure step,
  `SHADOWS.md` and part starter files are removed.
- The Planner is told to follow the stack's own layout.

**Build order:** 3.1 → 3.2 → 3.3 → 3.4 on the backend; 3.5 on the web once
the shared request is accepted and 3.3 is committed.

**Shared request first.** Three serialized shapes change:
- `LayerSpec` loses `elements` (the standards route);
- `PartContent` loses `files` (every route and MCP tool that returns parts);
- the Create structure route goes, and the stage view loses `Structure`
  and its file count.

Before 3.1 starts, the backend lead writes
`docs/shared/requests/2026-11-06-principles-not-layout.md` with these three
changes. The web side accepts it before 3.5 starts.

### Task 3.1: base v3, and parts without starter files

**Execution ruling (2026-11-07):** base v3 is B-71. Starter-file removal and
migration 0048 move to Task 3.3 (B-73), together with the structure writer
that consumes them. This avoids adapting code and tests only to delete
them two commits later. The completed slice's interfaces are unchanged.

**Files:**
- Modify: `crates/core-standards/src/standards.yaml`, `src/model.rs`,
  `src/base.rs`, `contract.yaml`
- Modify: `crates/core-design/src/model.rs` (remove `files` and
  `InitialFile`), `src/lib.rs`, `src/map_rules.rs` (the `files` checks),
  `contract.yaml`
- Create: `crates/core-db/migrations/0048_part_files_removed.sql`
- Modify: every reader the compiler names, among them
  `crates/core-harness/src/standards.rs`, `src/prompt.txt` (the "files"
  argument line only), `crates/shadows-mcp/src/workspace_tools.rs`, and the
  test fixtures
- Regenerate: `api/openapi.json`, `web/src/api/schema.d.ts`
- Test: `crates/core-standards/tests/base.rs`,
  `crates/shadows-core/tests/design_migration.rs`

**Interfaces:**
- `LayerSpec { id: String, role: String, required: bool, may_use: Vec<LayerRef> }`.
- `PartContent` has no `files`.
- `standards.yaml` is `version: 3`:
  - every `elements:` line goes;
  - the backend's `owns` reads "The business logic, in modules, reached
    through one owner object.";
  - S6, S9 and S10 read as spec §5.2. S10 says the owner builds the
    services and is the one way in from outside; services reach each other
    through their declared dependencies, never through the owner.

- [x] **Step 1: Write the failing tests.**
  - `base_v3_has_no_element_kinds_and_states_s10` in `tests/base.rs`:
    - `Standards::base().version == 3`;
    - the rule `S10` exists, and its text contains "one owner object"
      and "declared dependencies";
    - the rule `S6`'s text does not contain "own folder".
  - `parts_with_starter_files_read_back_after_the_migration` in
    `design_migration.rs`:
    - seed a `design_part` row whose `content_json` has
      `"files":[{"path":"backend/composer.json","content":"{}"}]`, and a
      map version row holding the same part;
    - migrate;
    - neither JSON has a `files` key, and the part reads back through
      `Design::parts`.
  - Extend `a_map_version_keeps_its_row_order` to run through 0048.
- [x] **Step 2: Run them and see them fail.**
  `cargo test -p core-standards --test base` and
  `cargo test -p shadows-core --test design_migration`.
- [x] **Step 3: Change the catalog and the model.**
  - Remove `elements` from `LayerSpec` and its parsing in `base.rs`.
  - Remove `files` and `InitialFile` from `core-design`, and every
    `files` check in `MapRules`. `check_reserved` loses its last caller in
    3.3; leave it until then.
- [x] **Step 4: Write migration 0048.**
  - `design_part.content_json`: `json_remove(content_json, '$.files')`.
  - `design_map_version.parts_json`: rebuild the array as 0047 did, with
    the inner `content_json` string passed through
    `json_remove(json(...), '$.files')` and back to text, ordered by `key`.
- [x] **Step 5: Fix every caller the compiler names,** regenerate
  `api/openapi.json` and `web/src/api/schema.d.ts` (`npm run gen:api` in
  `web/`), and run:
  - `cargo test -p core-standards`
  - `cargo test -p shadows-core --test design_migration --test design_parts --test design_vision`
  - `cargo test -p core-design`
  - `cargo test -p core-harness`
  - `cargo test -p shadows --test openapi --test workspace_mcp --test planner_mcp`
- [x] **Step 6: Update the contracts and commit.**
  `core-standards`, `core-design` and, if they name the fields,
  `core-harness` and `shadows-mcp`. Commit:
  `feat(backend): base v3 standards and parts without starter files`.

### Task 3.2: map rules on principle only

**Files:**
- Modify: `crates/core-design/src/map_rules.rs`, `contract.yaml`
- Test: `crates/core-design/tests/map_rules.rs`

**Interfaces:**
- `MapRules::check_places` keeps the slice 1 folder rules (`one_folder`,
  `folder_outside_parent`, `folder_claimed_twice`, a literal part folder)
  and these layer rules:
  - `missing_layer`, `unknown_layer`, `layer_not_used_without_reason`;
  - `invalid_path` for a place that is absolute, holds `..` or `\`, or is
    empty while used;
  - `layer_place_shared` for two layers of one part whose normalised places
    are equal, ignoring case.
- Removed: `layer_outside_part`, `layer_inside_child`, `may_overlap`,
  `always_inside`, the `Overlap` type and the one-marker limit.
- `{module}` may appear any number of times in a layer place.

- [x] **Step 1: Write the failing tests.**
  - `a_laravel_map_with_a_nested_database_part_is_approved`:
    - part `Backend`, kind `backend`, folder `backend`, places
      `backend/app/Domain`, `backend/app/Application`, `backend/app/Http`,
      `backend/app/Infrastructure`, `backend/app/Providers`;
    - part `Database`, kind `database`, a child of `Backend`, folder
      `backend/database`, places `backend/database/schema`,
      `backend/database/migrations` and `backend/config/database.php`;
    - `check_places` returns `Ok`.
  - `a_place_outside_its_part_is_accepted`: the connection place above,
    alone, passes.
  - `two_layers_at_the_same_place_are_refused`: `backend/app/Domain` and
    `Backend/App/domain` in one part give `layer_place_shared`.
  - `a_marker_may_appear_more_than_once`:
    `backend/{module}/src/{module}` passes.
- [x] **Step 2: Run them and see them fail.**
  `cargo test -p core-design --test map_rules`.
- [x] **Step 3: Remove the rules and the matchers.** Delete the tests of
  the removed rules; the commit message names each deleted test.
- [x] **Step 4: Run the tests.**
  - `cargo test -p core-design`
  - `cargo test -p shadows-core --test design_parts`
  - `cargo test -p shadows --test design_routes`
- [x] **Step 5: Update the contract and commit.** The MapRules section
  lists the kept refusals only. Commit:
  `feat(backend): map rules check principles, not places`.

### Task 3.3: remove Create structure and the Structure step

**Files:**
- Delete: `crates/core-design/src/structure.rs`, `src/structure_create.rs`,
  `src/structure_write.rs`, and the structure code in
  `src/store/structure.rs`
- Modify: `crates/core-design/src/stage.rs` (no `Stage::Structure`),
  `src/store/stage.rs` (no structure file count, no
  `design_structure_write` read), `src/map_rules.rs` (remove
  `check_reserved`), `src/lib.rs`, `contract.yaml`
- Modify: `crates/shadows-http/src/design.rs`, `src/lib.rs` (the route
  goes), and `crates/shadows-core/src/app.rs` if it names the method
- Create: `crates/core-db/migrations/0049_structure_write_dropped.sql`
- Regenerate: `api/openapi.json`, `web/src/api/schema.d.ts`
- Delete or rewrite: `crates/core-design/tests/structure.rs`,
  `crates/shadows-core/tests/design_parts/structure.rs`, and the structure
  cases in `crates/shadows/tests/design_routes.rs`
- Update: `docs/codebase/README.md` (remove the rows of the deleted
  modules)
- Amend: `docs/core/specs/2026-10-08-guided-planning-design.md` §23, the
  owner of the planning gates, in the same commit:
  - the step list and stage line drop Structure: Idea, Vision, Map,
    Roadmap, Plans;
  - the gate table drops the "Create structure" row, and the roadmap row
    reads "Propose or approve a roadmap | The map is not approved";
  - §23.7 "Writing the structure" is replaced by one paragraph: no command
    writes the structure; the approved map records the layout, and a plan's
    tasks or the Planner in Agent Mode write it (link to the structure
    spec §5.4);
  - the delivery and acceptance lines that name the structure write follow.

**Interfaces:**
- `Stage` is `Idea | Vision | Map | Roadmap | Plans`. Roadmap opens once
  the map is approved.
- `StageView` has no structure fields.
- Migration 0049 drops `design_structure_write`. Nothing on disk is
  deleted.

- [x] **Step 1: Write the failing tests.**
  - `roadmap_opens_once_the_map_is_approved`: a project whose map is
    approved reads `Stage::Roadmap` from `Design::stage`.
  - `a_project_past_the_old_structure_step_opens_roadmap`: a database with
    a completed `design_structure_write` row migrates, and the project
    reads `Stage::Roadmap`.
  - In `crates/shadows/tests/openapi.rs`, the route list holds no
    `create-structure` path.
- [x] **Step 2: Run them and see them fail.**
  `cargo test -p shadows-core --test design_parts` and
  `cargo test -p shadows --test openapi`.
- [x] **Step 3: Delete the modules, the route, the stage and the table.**
  Follow the compiler. No caller may keep a structure field.
- [x] **Step 4: Regenerate and run.**
  - `npm run gen:api` in `web/`, then `git diff --exit-code api/` shows only
    the expected change;
  - `cargo test -p core-design`
  - `cargo test -p shadows-core --test design_parts --test design_migration`
  - `cargo test -p shadows --test openapi --test design_routes --test codemap --test contract_tests --test architecture`
- [x] **Step 5: Update the contracts and the code map, and commit:**
  `feat(backend): no command writes the structure`.

### Task 3.4: the Planner follows the stack

**Files:**
- Modify: `crates/core-harness/src/prompt.txt` (the map section, as spec
  §7), `src/standards.rs` (no element kinds; S10 printed), `contract.yaml`
- Test: `crates/shadows/tests/planner_mcp.rs`

- [x] **Step 1: Write the failing assertions** in `planner_mcp.rs`:
  - the appended prompt contains "follow the framework's own layout" and
    "Shadows writes no folders";
  - it contains neither "Create structure" nor "SHADOWS.md";
  - the standards text contains "S10" and no "Elements:".
- [x] **Step 2: Run them and see them fail.**
  `cargo test -p shadows --test planner_mcp`.
- [x] **Step 3: Rewrite the map section** with the five bullets of spec
  §7, and remove every Create structure line from the stage text.
- [x] **Step 4: Run** `cargo test -p shadows --test planner_mcp` and
  `cargo test -p core-harness`.
- [x] **Step 5: Update the contract and commit:**
  `feat(backend): the Planner follows the stack's layout`.

### Task 3.5 (web, F-): the form, the standards page and the stage line

**Files:**
- Modify: `web/src/app/design/part-editor.tsx` (no starter files; the place
  hint), `map-part-details.tsx`, `map-proposal-card.tsx`,
  `standards-view.tsx`, `map-canvas.tsx` (the "Recorded elements" sentence
  goes), the stage line component, `web/src/api/design.ts`
- Delete: `web/src/app/design/create-structure.tsx` and its test
- Modify: `web/src/test/fake-daemon.ts`, the contract fixtures, and the
  design `contract.yaml`
- Test: the files' own tests

**Interfaces:**
- The place input's hint: "Where this layer lives in your stack's own
  layout, for example backend/app/Domain."
- The stage line shows Idea, Vision, Map, Roadmap, Plans.

- [x] **Step 1: Write the failing tests.**
  - `the_part_form_has_no_starter_files`
  - `the_place_hint_names_the_stack_layout`
  - `the_standards_page_shows_s10_and_no_element_kinds`
  - `the_stage_line_has_no_structure_step`
  - `the_map_page_has_no_create_structure_button`
- [x] **Step 2: Run them and see them fail.**
  `npx vitest run src/app/design src/app/workspace`.
- [x] **Step 3: Change the views,** delete the panel, and update
  `fake-daemon.ts` to the regenerated schema.
- [x] **Step 4: Run** `npm run typecheck`, `npm run lint` and
  `npx vitest run src/app/design src/app/workspace src/codemap.test.ts src/architecture`.
- [x] **Step 5: Update the contract and commit:**
  `feat(web): the map follows the stack, with no structure step`.

**Slice 3 acceptance:**
Manual acceptance belongs to Mohammed (2026-11-07). Automated test results
do not close this checklist. Use an existing project; create no new project
for this acceptance.

1. Start `shadows serve` on the daily database. It starts.
2. In the existing project, agree a Laravel map with the Planner: the database part
   inside the backend. The map is approved.
3. Roadmap opens. There is no Create structure button and no Structure
   step.
4. If the backend target is empty, in Agent Mode ask the Planner to run
   `composer create-project laravel/laravel backend`. It runs. If the
   existing project already has its scaffold, inspect it instead; never
   recreate the framework over existing files.

---

## Slice 4: `task_context`

**Outcome.** The agent building a task asks Shadows where it works and
which rules hold, instead of reading files on disk.

**Build order:** 4.1 → 4.2 → 4.3. No web task: the read is for agents.

**Shared request:** none. `task_context` is an MCP tool, not an HTTP route.

### Task 4.1: the layout read (`core-design`)

**Files:**
- Create: `crates/core-design/src/layout.rs`
- Modify: `crates/core-design/src/lib.rs`, `crates/core-design/src/store/mod.rs`,
  `crates/core-design/contract.yaml`, `crates/core-design/src/map_rules.rs`,
  `crates/shadows-core/src/lib.rs`,
  `crates/shadows-core/tests/design_parts.rs`, `docs/codebase/README.md`.
- Create: `crates/core-design/src/store/layout.rs`. Layout value types and
  pure path ownership remain in `layout.rs`, re-exported by the owner.
- Test: `crates/shadows-core/tests/design_parts/layout.rs` (new module of
  `design_parts`)

**Interfaces:**
- `Design::layout(&self, project: &ProjectId) -> Result<ProjectLayout, DesignError>`.
- `Design::layout_in(conn, project)` is the transaction-local owner API;
  `layout` opens one read transaction and delegates. Plans calls `layout_in`
  in its own read snapshot; it never opens Design storage.
- `ProjectLayout { standards_base: i64, standards_additions: Option<i64>, parts: Vec<LayoutPart> }`,
  in map order.
- `LayoutPart { id: PartId, parent: Option<PartId>, title, kind: Option<String>, folder, grouping: bool, layers: Vec<LayoutLayer>, rules: Vec<RuleText> }`.
- `LayoutLayer { id, role, place: Option<String>, not_used: Option<String>, may_use: Vec<LayerRef> }`,
  the part's recorded layer joined with its catalog line.
- `RuleText { id, text }`: the rules whose `parts` name this kind, and the
  rules with no `parts`.
- It reads accepted `design_part` rows through Design’s private layout store,
  using `MapVersionsRows::rows` and `effective` in the same transaction. Person
  edits are accepted map state; pending proposals never enter it. Require a
  live project via `Projects::require_live_in`. Include `map_revision: i64`
  from `design_workspace` so the answer identifies its layout snapshot.
- Standards additions use the public version number, never the private row id.
  Join base and additions rules, selecting kind-specific and global rules.
  Empty kind has global rules only. Unknown catalog layer is an explicit
  named refusal, never silently omitted. No filesystem reads or writes.
- `grouping` is `MapRules::is_grouping`, the one existing rule.
- `ProjectLayout::owners(&self, path: &str) -> Vec<PathOwner>`: every part
  that claims the path, so the rule "who owns this path" has one owner.
  - `PathOwner { part: PartId, via: OwnedVia }`, with
    `OwnedVia::Folder` or `OwnedVia::Layer(String)` (the layer id).
  - **By folder:** the deepest part whose folder holds the path.
  - **By layer place:** every part with a used layer whose place equals
    the path or holds it. In a place, `{module}` matches any one
    non-empty segment text, also inside a segment such as `core-{module}`.
  - Paths compare by normalized segments, ignoring case. Segment boundaries
    prevent `backend` matching `backend2`; repeated/trailing separators and
    `.` normalize, absolute/traversal/backslash paths are unowned. A marker
    matches nonempty text within one segment; repeated markers work.
  - Use one deepest folder owner first, then layer owners in map/layer order;
    preserve distinct folder/layer claims, including two claims by one part.
    Map order is the explicit `(ordinal,id)` order of current accepted rows.

- [x] **Step 1: Layout tests.** `layout_lists_the_rules_for_each_kind`,
  `layout_joins_layers_and_owns_nested_and_external_places`,
  `layout_snapshot_keeps_parts_and_standards_consistent`,
  `layout_marks_grouping_parts_without_copying_the_rule`; proposal exclusion
  is asserted in `equivalent_map_retries_replay_the_original_card`.
- [x] **Step 2: Run them and see them fail.**
  `cargo test -p shadows-core --test design_parts layout`.
- [x] **Step 3: Implement `Design::layout`** in `layout.rs`, reading the
  approved map's parts and the effective standards in one transaction.
- [x] **Step 4: Run** `cargo test -p shadows-core --test design_parts`
  and `cargo test -p core-design`.
- [x] **Step 5: Add its contract line and commit:**
  `feat(backend): Design reads a project's layout`.

**Verified:** design_parts 41 passing; core-design, architecture, codemap,
contract_tests, affected all-target clippy and fmt passed. Manual acceptance pending.

### Task 4.2: `task_context` (`core-plans`)

**Files:**
- Modify: `crates/core-plans/src/scope.rs` (shared named-workflow guidance).
- Create: `crates/core-plans/src/task_context.rs`
- Modify: `crates/core-plans/src/lib.rs`, `crates/core-plans/src/store/mod.rs`,
  `crates/core-plans/src/store/design_parts.rs`, `crates/core-plans/contract.yaml`,
  `crates/shadows-core/src/lib.rs`, `docs/codebase/README.md`.
- Create: `crates/core-plans/src/store/task_context.rs`,
  `crates/shadows-core/tests/task_context.rs` (standalone integration target).

**Interfaces:**
- `Plans::task_context(&self, grant: &Grant, workflow: Option<&WorkflowId>, task: u32) -> Result<TaskContext, PlansError>`.
  - Explicit `workflow_id` remains required, as `task_for` requires. Preserve
    its same-project scope and named missing-task refusal. Additionally require
    `WorkflowState::Frozen`; a named Draft refuses rather than selecting another
    version. An old approved version remains readable after a new Draft exists.
  - Load plan, live project, tied part ids, accepted Design layout and standards
    in one reader transaction. Add an owner-private snapshot read; do not call
    `in_scope` and `Design::layout` as separate snapshots. Reuse scope wording,
    but keep new transactional SQL limited to Plans-owned plan/task/association
    rows. Call `Design::layout_in` for every Design fact.
- `TaskContext`:
  - `plan: { plan_id, workflow_id, version, title }`;
  - `task: TaskContent`, as frozen in that version;
  - `parts: Vec<TaskPart>`: the plan's tied parts, and every owner
    `ProjectLayout::owners` names for one of the task's `reads` or
    `writes`;
  - `unowned_paths: Vec<String>`: task paths no part owns;
  - `standards: { base: i64, additions: Option<i64> }`, plus
    `map_revision: i64`; all versions identify the facts actually read.
  - Tied parts come from existing Plans-owned `design_part_plan` association
    via `store/design_parts.rs`, never Design SQL in Plans. Keep tied parts
    without matching paths; order all selected parts by layout map order.
    Deduplicate task path strings in stable reads-then-writes order, preserving
    distinct ownership reasons. Invalid paths stay unowned, never guessed.
- `TaskPart { part: LayoutPart, paths: Vec<TaskPath> }`, with
  `TaskPath { path: String, via: OwnedVia }`: the task's paths this part
  owns, and how. One path can appear under two parts.
- The method calls `Design::layout_in`. It never reads Design tables.
- It grants nothing and writes nothing.

- [x] **Step 1: Context integration cases.**
  `task_context_refuses_drafts_then_returns_frozen_content_without_writes`
  covers explicit approved versions after a new Draft, missing task/workflow,
  foreign scope, tied parts, and command/event immutability.
  `task_context_joins_layout_rules_nested_owners_and_stable_paths`
  covers nested ownership, out-of-folder layer ownership, catalog rules,
  stable duplicate removal and unowned paths. Design's layout snapshot test
  holds the shared transaction-local API's snapshot behavior.
- [x] **Step 2: Run them and see them fail.**
- [x] **Step 3: Implement `Plans::task_context`** in `task_context.rs`.
- [x] **Step 4: Run** `cargo test -p shadows-core --test task_context` and
  `cargo test -p core-plans`.
- [x] **Step 5: Add its contract line and commit:**
  `feat(backend): task_context tells an agent where it works`.

### Task 4.3: the MCP tool

**Files:**
- Modify: `crates/shadows-mcp/src/plan_tools.rs`,
  `crates/shadows-mcp/src/contract.yaml`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt` (one line: the building
  agent reads `task_context` first)
- Modify: `crates/core-harness/contract.yaml`.
- Test: `crates/shadows/tests/planner_mcp.rs`,
  `crates/shadows/tests/mcp_tools.rs`, `crates/shadows/tests/mcp_server.rs`.

**Interfaces:**
- Tool `task_context`, arguments `{ workflow_id?: string, task_number: u32 }`,
  for the thread and project grants. Add it to both `Grants::tools_for`
  allowlists; router registration alone does not authorize it. It calls
  `Plans::task_context` and
  returns its JSON.
- Its description says it is the first read before building a task, and
  that it grants no permission to execute.

- [x] **Step 1: Write the failing tests.**
  - `both_grants_read_approved_task_context_without_execution_authority`
    covers listing and successful/refused reads through both grant kinds.
  - `the_planner_follows_the_stacks_layout` verifies the prompt instruction.
- [x] **Step 2: Run them and see them fail.**
  `cargo test -p shadows --test planner_mcp --test mcp_tools --test mcp_server`.
- [x] **Step 3: Add the tool and the prompt line.**
- [x] **Step 4: Run** those three test targets and
  `cargo test -p shadows --test contract_tests --test codemap`.
- [x] **Step 5: Update the contracts and commit:**
  `feat(backend): the task_context MCP tool`.

**Slice 4 acceptance:**
1. Approve a plan with one task in the Laravel project.
2. A fresh external agent reads the task's context.
3. It names the backend part, its layers with their places, and S10.

---

## Slice 5: phases inside tasks

**Outcome.** A task has ordered phases. A phase can wait for a phase of
another task, in this plan or another plan of the project. The plan graph
shows phases (spec §6.2).

**Build order:** 5.1 → 5.2 → 5.3 on the backend; 5.4 → 5.5 on the web once
the shared request is accepted and B-80’s generated response is committed.
B-79–B-81 are blocked only until controller records API acceptance; F-93
remains blocked until B-80 is built, F-94 until F-93. Use the existing owner
spec; execution status remains deferred to §17.

**Shared request first:** `TaskContent` gains `phases`, and `plan_edit`
gains `phases_set`. The backend lead writes
`docs/shared/requests/2026-11-07-task-phases.md` before 5.1 starts.
The controller accepts this request for both sides under Mohammed’s instruction
to implement the existing design on this branch.

### Task 5.1: the phase model (`core-plans`)

**Files:**
- Modify: `crates/core-plans/src/model.rs`, `crates/core-plans/src/lib.rs`,
  `crates/core-plans/src/rules.rs`, `crates/core-plans/contract.yaml`,
  `crates/shadows-core/src/lib.rs`, `api/openapi.json`,
  `web/src/api/schema.d.ts`, `docs/codebase/README.md`.
- Create: `crates/core-plans/src/phases.rs` for pure public phase values.
- Test: `crates/core-plans/tests/workflow_rules.rs`.
- Constructor fixture inventory must be completed by the controller before
  B-79 dispatch: adding a Rust struct field breaks every `TaskContent` literal.
  The exact compiler-identified fixture files are appended to B-79 Touch; no
  blanket permission to edit arbitrary tests or unrelated product modules.

**Interfaces:**
- `TaskContent.phases: Vec<Phase>`, `#[serde(default)]`. It is stored in
  the task's existing `contract_json`, so no migration is needed.
- `Phase { id: PhaseId, number: u32, title: String, goal: String, acceptance: Vec<u32>, waits_for: Vec<PhaseRef> }`.
  - `id` is a stable string, given by the server on `phases_set` when
    absent.
  - `number` is the display order, 1-based and contiguous in a task.
    The server derives it from the phase's position in the list; a client
    never sends it.
  - `acceptance` names the task's acceptance item numbers.
- `PhaseRef`:
  - `Local { task: u32, phase: PhaseId }`;
  - `Plan { plan_id: PlanId, version: i64, task: u32, phase: PhaseId }`.
  Use an explicitly tagged snake_case wire enum (`kind: local | plan`),
  documented in the shared request. Versions must be positive. A cross-plan
  wait pins an exact approved version; PhaseId is an opaque server identity.
  Phase numbers must be positive/contiguous; repeated phase IDs, blank
  titles/goals, unknown acceptance references and self-waits are named
  validation problems. Old JSON missing phases defaults to empty.
- `task_add` creates a task without phases; `task_update` preserves the
  existing phase list and refuses an attempted phase change, directing the
  caller to `phases_set`. This prevents forged IDs, number edits and bypasses.
  Because the new serialized TaskContent changes HTTP today, regenerate
  OpenAPI and web schema in B-79, not only at 5.3. Preserve old command
  fingerprints for absent/empty phases (omit empty serialization).

- [ ] **Step 1: Write the failing tests.**
  - `a_task_without_phases_reads_back_unchanged`: old `contract_json`
    without `phases` reads as an empty list.
  - `phase_acceptance_must_name_the_tasks_items`
  - `phase_numbers_follow_list_order`
- [ ] **Step 2: Run them and see them fail.**
  `cargo test -p core-plans --test workflow_rules`.
  Also prove old contract JSON and a phase-bearing JSON round-trip, and that
  omitted/empty phases preserve existing command-fingerprint assertions.
- [ ] **Step 3: Add the types and the checks** to the existing task rules.
- [ ] **Step 4: Run** `cargo test -p core-plans`.
- [ ] **Step 5: Update the contract and commit:**
  `feat(backend): tasks have phases`.

### Task 5.2: the combined graph and the edit operations

**Files:**
- Modify: `crates/core-plans/src/dependency_graph.rs`, `src/ops.rs`,
  `crates/core-plans/src/rules.rs`, `crates/core-plans/src/store/graph.rs`,
  `crates/core-plans/src/store/edit.rs`, `crates/core-plans/src/store/read.rs`,
  `crates/core-plans/src/model.rs`, `crates/core-plans/src/lib.rs`,
  `crates/core-plans/src/store/mod.rs`, `crates/core-plans/contract.yaml`,
  `crates/shadows-core/src/lib.rs`, `api/openapi.json`,
  `web/src/api/schema.d.ts`, `docs/codebase/README.md`.
- Create: `crates/core-plans/src/store/phase_projection.rs` and
  `crates/shadows-core/tests/phase_dependencies.rs`.
- Test: `crates/core-plans/tests/workflow_rules.rs`,
  `crates/shadows-core/tests/cross_plan_resolution.rs`,
  `crates/shadows-core/tests/cross_plan_links.rs`.

**Interfaces:**
- `DependencyGraph` gains a start and a complete event per phase:
  - a task's start precedes its first phase's start;
  - each phase's complete precedes the next phase's start;
  - the last phase's complete precedes the task's complete;
  - a `waits_for` adds the target phase's complete → this phase's start.
  The existing `needs` and `completes_after` edges stay as they are.
- Graph identity includes exact workflow/version, task and optional stable
  phase id, with start/complete event identity distinct from display labels.
  Existing GraphRows currently keys PlanId and follows latest versions;
  replace that keying before mixing pinned and latest edges. A node at v1
  must not alias the same phase at v2. Existing whole-task links retain their
  latest-version semantics. Cross-plan phase waits stay in the same project,
  even if a project link permits whole-task links elsewhere.
- `cycle()` returns a deterministic actual directed cycle path (not merely
  sorted SCC members), naming plan/version and phases as `T3 · P2`.
  Cover mixed legacy latest links and exact phase pins, including two
  reachable versions of the same plan.
- `PlanOp::PhasesSet { task: u32, phases: Vec<PhaseInput> }` replaces the
  task's whole phase list in one operation. This is how phases are added,
  removed and reordered, so a reorder is never two edits.
  - `PhaseInput { id: Option<PhaseId>, title, goal, acceptance, waits_for }`.
  - Allocate IDs inside the write transaction, after command replay and
    revision/writer checks. IDs must not be regenerated during read,
    deserialization, normalization or fingerprint construction. UUIDs use
    the existing dependency; immutable command replay returns the original
    outcome and the stored IDs remain identical after reopen.
  - An `id` that exists keeps its identity at its new position. A missing
    `id` gets a new one. An `id` left out of the list is removed.
  - An unknown or repeated `id` is refused.
  - Its `Object` is the task, so a batch cannot also update that task.
- A cross-plan `PhaseRef` must name an approved version of a plan in the
  same project, and an existing task and phase in it. Otherwise the edit
  is refused naming the target.
- Validate the candidate combined graph before `TaskRows::write_content`,
  not the previous stored root. Check ALL waits in the final candidate,
  including refs introduced through task operations; malformed targets are
  refused before any row/event/revision/command commits.
- Approval reloads every exact cross-plan target inside the same serialized
  `write_txn_as` that freezes. Missing/unapproved/foreign targets are named
  blockers on Draft reads and approval. Newer versions and removed phases
  in later versions do not invalidate an older Frozen target. Archiving is
  not deletion and does not redirect a pin. No preflight-only race check.
- Add minimal derived `Plan.linked_phases` metadata for both incoming and
  outgoing foreign-task waits: source and target each have `plan_id`,
  `workflow_id`, `version`, `task`, phase id/number/title; `broken` is a
  nullable named reason. A missing target keeps its reference without
  invented title/number/workflow. Local-other-task edges use the same shape.
  Read through Plans, in the current snapshot; never recurse into full Plan
  JSON or expose unrelated projects. Incoming cross-plan previews use the
  current latest source version, while target resolution follows its exact
  pin. Preview edges are projections, excluded from canonical PlanContent.
  An incoming edge shown on v1 must actually pin v1.
- Existing project WorkflowEdited/WorkflowFrozen invalidation already
  refreshes the workflow query family. Verify phase edits use those events;
  add no second event stream or client completion state.

- [ ] **Step 1: Write the failing tests.**
  - `implicit_phase_order_and_task_links_can_form_a_cycle`: T1 · P2 waits
    for T2 · P1, and T2 `needs` T1: refused, naming the path.
  - `a_cycle_through_another_plan_is_refused`
  - `a_wait_on_an_unapproved_plan_is_refused`
  - `a_new_target_version_does_not_redirect_a_frozen_phase_wait`
  - `reordering_phases_preserves_their_identity`: one `phases_set` that
    swaps P1 and P2 keeps both `id`s, renumbers them, and waits on them
    still resolve.
  - `phases_set_refuses_an_unknown_or_repeated_id`
  - `a_removed_phase_is_a_named_blocker`
  - `concurrent_approvals_cannot_commit_a_combined_cycle`: two real
    connections contend; assert no committed invalid graph. Since new
    cross-plan phase waits require an already Frozen target, two drafts
    cannot manufacture reciprocal new pins; separately cover legacy
    latest-version task links mixed with an existing Frozen phase pin.
  - `phase_edit_failure_rolls_back_content_revision_command_and_event`
  - `phase_ids_survive_replay_and_database_reopen`
  - `linked_phase_previews_keep_pins_and_name_missing_targets`
  - `phases_set_and_a_task_update_on_one_task_are_refused`
- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Extend the graph, the operations and the approval check.**
- [ ] **Step 4: Run** `cargo test -p core-plans`;
  `cargo test -p shadows-core --test phase_dependencies --test cross_plan_resolution --test cross_plan_links`;
  `cargo test -p shadows --test core_plans --test openapi`; regenerate both
  generated files after the new operation/projection, then typecheck web.
- [ ] **Step 5: Update the contract and commit:**
  `feat(backend): phase waits join one dependency graph`.

### Task 5.3: the tools and `task_context`

**Files:**
- Modify: `crates/shadows-mcp/src/plan_tools.rs` (the `plan_edit`
  description lists `phases_set`), `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-harness/contract.yaml`, `crates/core-plans/contract.yaml`,
  `crates/core-plans/src/store/task_context.rs`,
  `crates/core-plans/src/lib.rs`, `crates/shadows-core/src/lib.rs`
- Modify: `crates/core-harness/src/prompt.txt` (phases: when to use them,
  and that the Planner never marks a phase complete)
- Modify: `crates/core-plans/src/task_context.rs`: an optional
  `phase: Option<&PhaseId>`
- Test: `crates/shadows/tests/planner_mcp.rs`,
  `crates/shadows/tests/mcp_tools.rs`, `crates/shadows-core/tests/task_context.rs`.
- B-79/B-80 own HTTP schema regeneration as their public shapes change.
  B-81 verifies those files remain current; MCP-only phase_id is not an HTTP
  route and does not alone warrant changing OpenAPI.

**Interfaces:**
- `Plans::task_context(grant, workflow, task, phase: Option<&PhaseId>)`.
  With a phase, the context adds the phase and the targets it waits for,
  at their pinned versions, loaded in its existing single snapshot through
  the shared Plans-owned exact loader. The task’s acceptance stays whole.
  A missing phase in the named task refuses by identity; a broken pinned
  target is returned as a named blocker, never substituted with latest.
  No completion field, checkbox, permission or execution method is added.
- The MCP tool gains `phase_id?: string`.

- [ ] **Step 1: Write the failing tests.**
  - `the_planner_adds_a_phase_with_plan_edit`
  - `phase_context_keeps_task_acceptance_and_pinned_dependencies`
  - `a_phase_outside_the_named_task_is_refused`
  - the prompt contains "never mark a phase complete".
- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Implement,** and regenerate the API files.
- [ ] **Step 4: Run** `cargo test -p shadows --test planner_mcp --test openapi`
  and `cargo test -p core-plans`; `cargo test -p shadows --test mcp_tools`;
  `cargo test -p shadows-core --test task_context`.
- [ ] **Step 5: Update the contracts and commit:**
  `feat(backend): the Planner edits phases`.

### Task 5.4 (web, F-93): the task view

**Files:**
- Create: `web/src/app/workflows/task-view.tsx`, `task-view.test.tsx`
- Modify: `web/src/app/workflows/plan-page.tsx` (route state for the
  opened task), `plan-graph.tsx` (open a task), `contract.yaml`
- Modify: `web/src/test/fake-daemon.ts`

**Interfaces:**
- The existing `/projects/$projectId/workflows/$workflowId` route validates
  optional positive integer `task`; its workflow id pins the version. Invalid
  task/phase references show an explicit unavailable message. Use route search
  state instead of only PlanView’s current local `inspected` state. Browser
  Back/Escape restore prior graph selection, filters and viewport; opening
  a foreign reference uses its captured workflow id, never latest.
- Keep HTTP calls and query identities in `api/workflows.ts`: alias generated
  Phase/PhaseRef/LinkedPhase types; existing `planQuery` reads the exact
  workflow. Components render the server’s linked_phases and blockers; no
  client validation graph or fetch/URL construction. No new endpoint needed.
- The view uses `PlanGraph`'s React Flow look: the task is the start
  node, its phases are task-card nodes, edges carry labels, and the legend,
  zoom and minimap stay.
- A foreign phase (another task's or another plan's) is a dashed node with
  `T2 · P1` and, for another plan, its version. Clicking it opens that
  task’s view. Include incoming as well as outgoing waits. Phase status is
  absent; no mock done percentage or completion toggle.
- A task with no phases shows "This task has no phases" and its
  acceptance.

- [ ] **Step 1: Write the failing tests.**
  - `opening_a_task_shows_its_phases_in_order`
  - `a_foreign_phase_is_dashed_and_opens_its_task`
  - `back_returns_to_the_plan_with_the_selection_kept`
  - `a_task_without_phases_says_so`
  - `a_pinned_foreign_phase_never_opens_the_latest_draft`
  - `task_view_has_loading_empty_error_and_keyboard_paths`
  - `task_view_survives_workflow_event_refresh`
- [ ] **Step 2: Run them and see them fail.**
  `npx vitest run src/app/workflows`.
- [ ] **Step 3: Build the view.**
- [ ] **Step 4: Run** `npm run typecheck`, `npm run lint` and
  `npx vitest run src/app/workflows src/codemap.test.ts src/architecture`.
- [ ] **Step 5: Update the contract and commit:**
  `feat(web): the task view shows its phases`.

### Task 5.5 (web, F-94): Show phases on the plan graph

**Files:**
- Modify: `web/src/app/workflows/plan-graph.tsx`, `layout.ts`,
  `plan-page.tsx` (the header button), `task-node.tsx`, `plan-camera.ts`,
  `plan-edge.tsx`, `linked-layout.ts`, `contract.yaml`,
  `web/src/test/fake-daemon.ts`, `docs/frontend/codebase/README.md`
- Create: `web/src/app/workflows/phase-node.tsx`,
  `web/src/app/workflows/phase-layout.ts`, `phase-layout.test.ts`
- Test: `plan-graph.test.tsx`, `layout.test.ts`

**Interfaces:**
- **The button:** Show phases / Hide phases, in the plan header beside
  Versions.
- **When shown:**
  - every task with phases is a React Flow group node: a frame with its
    number and title;
  - its phases are child nodes, with badges such as `T3 · P2`;
  - explicit waits join phase nodes;
  - whole-task constraints keep their start and completion endpoints and
    their labels;
  - tasks with no phases remain plain nodes.
- **Layout:** left to right by dependency rank, as today. Frames size to
  their phases.
- Selection, filters and viewport are kept on toggle. Keep one ReactFlow
  instance; automatic `useCamera` effects must not reset a person’s viewport
  on expansion. Every phase node id includes workflow/version/task/stable
  phase id. Whole-task start/complete handles preserve semantics independently
  of the first/last drawn card. Use server waits and graph labels, no domain
  cycle policy in web. The accessible list has identical navigation.
- Checks per web task also include `npm run build` and
  `npx vitest run src/codemap.test.ts src/architecture`. Browser acceptance
  requires 1024px and 375px, light/dark, keyboard/Back, and real empty/error
  states. Automated fixture evidence never substitutes for live acceptance.

- [ ] **Step 1: Write the failing tests.**
  - `show_phases_draws_a_frame_per_task_with_its_phases`
  - `an_edge_points_from_prerequisite_to_waiting_phase`
  - `hide_phases_returns_to_task_cards`
  - `expansion_preserves_whole_task_constraints_and_tasks_without_phases`
  - `foreign_phase_nodes_show_the_pinned_version`
  - `expansion_keeps_selection_and_viewport`
  - `keyboard_list_and_graph_open_the_same_task`
  - in `layout.test.ts`: `a_plan_of_100_tasks_and_400_phases_lays_out`
- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Build the frames, the phase nodes and the toggle.**
- [ ] **Step 4: Run** `npm run typecheck`, `npm run lint` and
  `npx vitest run src/app/workflows src/codemap.test.ts src/architecture`.
- [ ] **Step 5: Update the contract and commit:**
  `feat(web): Show phases on the plan graph`.

**Slice 5 acceptance:**
1. Open a plan, toggle Show phases, follow a foreign phase, and come back
   without losing the selection.
2. Exercise a task without phases, and both task dependency kinds.
3. Check the 1024px and 375px layouts, and a fixture of 100 tasks and 400
   phases.

## Acceptance inventory

| Concern | Tasks |
|---|---|
| Missing folder, moves and renames | 1.1, 3.2 |
| No contract files; no command writes the structure | 1.2, 3.3 |
| Migrations on the daily database | 2.2, 3.1, 3.3 |
| Standards fit any stack | 3.1, 3.2, 3.4 |
| The stage line without Structure | 3.3, 3.5 |
| Agent context without the old chat | 4.1, 4.2, 4.3 |
| Task and phase ordering, both graph modes | 5.1–5.5 |
| Theme, accessibility, loading and navigation | 2.6, 3.5, 5.4, 5.5 |

Phase completion, execution evidence and verified completion of a project
stage are tracked by the §17 delivery. This plan shows their absence
honestly.
