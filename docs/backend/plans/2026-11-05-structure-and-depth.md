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
| 2 | Slice 1; Mohammed approves D1–D4; accepted API request | Written to step level; waits for D1–D4 | Layers on the map card, migration on the daily database, canvas in the app |
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

Spec §10 OPEN 2 (stack profile mapping) is closed by D1 to D4 below. These
are the controller's proposal, and **Mohammed approves them before any task
is `todo`**. When he approves, amend spec §§5.1–5.3 and §10 in the same
commit as the stages entries.

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
- Modify: `crates/core-harness/src/standards.rs`, to drop the contract
  template from the Planner's standards text (the layers are added in
  Task 2.5)
- Modify: `crates/shadows-core/src/lib.rs` (re-exports)
- Test: `crates/core-standards/tests/base.rs`,
  `crates/shadows-core/tests/design_vision/standards.rs`

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

- [ ] **Step 4: Update the callers.**
  - In `core-harness/src/standards.rs`, delete the "Contract template"
    block.
  - In `shadows-core/src/lib.rs`, re-export `PartKind`, `Presence`,
    `LayerSpec` and `LayerRef` in place of `MandatoryPart` and
    `ContractTemplate`.
  - Update `design_vision/standards.rs` to the new base.

- [ ] **Step 5: Run the tests and see them pass.**
  - `cargo test -p core-standards`
  - `cargo test -p shadows-core --test design_vision`
  - `cargo test -p core-harness`

- [ ] **Step 6: Update the contract and commit.** Rewrite the shapes and
  the test list in `core-standards/contract.yaml`, then commit:
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
- `design_map_version.parts_json`: the same rewrite for each element's
  content. Keep the element order with `json_group_array(... ORDER BY key)`
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
| `layer_outside_part` | A place is not inside the part's folder |
| `layer_place_shared` | Two layers of one part have equal places (case-insensitive), or one non-pattern place lies inside another |
| `layer_not_used_without_reason` | `not_used` is blank, or `not_used` is set while `place` is not empty |
| `layer_inside_child` | A non-pattern place lies inside a child part's folder |

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
  - `layer_places_use_windows_case_rules`: `Backend/App/Domain` and
    `backend/app/domain` count as one place.
  - `moving_a_parent_rechecks_child_layer_places`: a person edit moves a
    parent's folder, and a child's layer place then falls outside it.

  Add `a_map_without_a_required_layer_is_refused_on_every_write_path` to
  `design_routes.rs`, in the shape of slice 1's folder regression.

- [ ] **Step 2: Run them and see them fail.**
  - `cargo test -p core-design --test map_rules`
  - `cargo test -p shadows --test design_routes`

- [ ] **Step 3: Write the checks in `check_shape`.**
  - Reuse the segment-based `inside` and the lowercase comparison there.
  - A pattern place is compared by its text before `{module}`, for the
    inside-part check.

- [ ] **Step 4: Run the same commands and see them pass.**
  Then update the `obligations` in `contract.yaml`.

- [ ] **Step 5: Commit:**
  `feat(backend): B-65 the map refuses a missing, unknown or misplaced layer`.

### Task 2.4 (B-66): layer folders on disk

**Files:**
- Modify: `crates/core-design/src/structure.rs`, `contract.yaml`
- Test: `crates/core-design/tests/structure.rs`;
  `crates/shadows-core/tests/design_parts/structure.rs`

**Interfaces:**
- `Structure::render` adds every non-pattern, non-empty layer place to
  `Planned.folders`, with its parent folders as it already does for parts.
- It calls `MapRules::check_shape` on the snapshot first (spec §5.1: "the
  same policy before its disk checks"). It maps a refusal to its message.
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
    Each row has a place input and a "Not used" reason input.
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
