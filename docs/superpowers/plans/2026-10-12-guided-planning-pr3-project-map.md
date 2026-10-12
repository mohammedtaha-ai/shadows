# §23 PR 3 — Project Map, Contracts and Gates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** With the vision complete, the Planner proposes the project map and the person approves it.
- The first map is proposed whole with `map_propose`; later the Planner proposes one part at a time.
- The code refuses a map that misses a mandatory part, leaves a goal unserved, or comes before the vision is complete (`STAGE_BLOCKED`).
- After the first map, the person edits parts by hand through the same gates, and the stage reports drift.

**Architecture:**
- **The map is the parts.** `PartContent` gains `serves`, `folders`, `files` and `contract`. They are `#[serde(default)]` inside the existing `design_part.content_json`, so old rows still parse and no part column is added.
- **Map state:** a new `design_workspace.map_revision` (0 means no approved map), a `design_map_version` table (a snapshot of every part on each map change), and a `design_part_waiver` table.
- **Rules in one pure module:** `design/map_rules.rs` checks a whole proposed set of parts: identity, folders, served ids, rule ids, mandatory kinds, goal coverage. Proposal, approval and hand edits all call it, so the gates are one code path.
- **Proposals** follow PR 2's vision proposals exactly: a table, a thread entry kind with the card JSON, a `MapProposed` event, a list route and an approve route. Staleness is computed on read.

**Tech Stack:** Rust (SQLx 0.9 / SQLite, axum + utoipa, rmcp), React + TanStack Query.

**Execution (2026-10-12):** Tasks 1–6 implemented in local commits
`30f5919`, `0841e21`, `c265416`, `dd1c116`, `b1fbb73`, `a3c4133`.
Browser acceptance exposed and fixed Vision save/SSE cancellation in
`1266b23`. Final review and Task 7 are in progress. The ledger is
`.superpowers/sdd/2026-10-12-guided-planning-pr3-project-map/progress.md`.
Metadata-only edits preserve served-item baselines; explicit content review
refreshes them. Standards-added gaps permit no-worsening incremental repair.
Preexisting legacy parts require explicit adoption (owner spec OPEN), so a
first-map approval never discards their identities silently.

**Spec:** `docs/superpowers/specs/2026-10-08-guided-planning-design.md`
- §23.5, with its "Map proposals: decided with Mohammed on 2026-10-12" block;
- §23.6, gates and `STAGE_BLOCKED`;
- §23.4, the PR 3 paragraph on the stage and drift;
- §23.2, mandatory parts and rules (`standards.yaml`).

**Pattern to copy:** PR 2 (merged as #25, `50146a9`) built vision proposals the same way. Read these before writing the map versions of them:
- `crates/shadows-core/src/design/proposals.rs`
- `crates/shadows-core/src/design/store/proposals.rs`, `proposal_approval.rs`, `proposal_read.rs`
- `crates/shadows-mcp/src/workspace_tools.rs`
- `crates/shadows-http/src/design.rs`
- `web/src/app/conversation/vision-proposal-card.tsx`
- `crates/shadows-core/tests/design_vision/proposals.rs`

## Global Constraints

- **Cargo commands:** prefix every one with `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0`.
- **Line length:** no Rust line over 100 columns.
- **Migrations:** never edit an existing one. This PR adds `0023_project_map.sql` only.
- **Integration tests:** add no new test file under `tests/`; each is a ~49 MB binary. New core tests go in the new submodule `crates/shadows-core/tests/design_parts/map.rs`, declared from `tests/design_parts.rs`. Everything else extends files named here.
- **API:** when `api/openapi.json` changes, run `npm run gen:api` in `web/` and commit both files.
- **Edit tools:** no prettier, and no Python, sed or Node scripts that edit source or docs. Use the editor.
- **Tests:** targeted tests per task; the full gate once, in Task 7.
- **Contracts:** every service change updates its `contract.yaml` in the same commit.
- **Navigation:** ask `where_is` / `who_uses` / `outline` when the `shadows` MCP server is connected; otherwise use the Rust LSP, and say which in the report.
- **Approval scope:** only the person approves, through HTTP. No MCP tool approves; Planner approval is PR 3b.
- **Out of scope:** writing anything to disk (PR 4), plan gates (PR 5), a map history page, a reject button.

## Rulings made while planning

Record each as a ledger `Ruling:` line at Task 1.

1. **Proposed parts use a `key`, not an id.** `PartId` is a generated UUID (`id.rs`), so a proposal cannot name one. A first map's parts carry a `key` (a slug unique inside that proposal) and `parent_key`. Approval issues a `PartId::generate()` for each, in tree order. A change to an existing part names its real `part_id`.
2. **Identity is the title under its parent,** compared after `trim` and lowercasing. Two siblings with the same title conflict. Spec: "two parts with the same title under the same parent are one identity".
3. **Folders are normalised before any check:** forward slashes, no leading `./`, no trailing `/`, no empty segment. An absolute path, a drive letter or a `..` segment is refused. PR 4 writes them; refusing them now keeps a bad path out of an approved map.
   - Ownership: folder `a` overlaps `b` when one equals the other or is a prefix followed by `/`.
   - An overlap is allowed only when the inner folder's part is a descendant of the outer folder's part.
4. **Waivers live on the first map only** in PR 3. A change proposal carries no waiver. A waiver stays recorded if a part of that kind is added later; it is then simply unused.
   - Cost if wrong: one more argument on the change action.
5. **Map change actions are `add` and `change`; there is no `remove`.** Parts have no delete op today (`DesignOp` has none), and §23.5 lists "add or change a part".
6. **Hand edits after the first map** may change `serves`, `folders` and `contract` as well as the existing fields. Initial `files` are read-only on the page; the Planner changes them through a proposal.
   - Why: a file skeleton editor is a code editor, which this PR does not build.
   - Cost if wrong: one more editor field later.
7. **`STAGE_BLOCKED` is a new `ErrorCode::StageBlocked`** carried by a new `StorageError::StageBlocked(String)`.
   - HTTP: `409` with code `STAGE_BLOCKED` and the reason as the message.
   - MCP: a tool result with `isError` and text `STAGE_BLOCKED: <reason>`, through `refusal.rs`.
   - Spec §23.6 as amended. Other refusals (bad arguments, conflicts, outdated, superseded) stay `422 INVALID_COMMAND`, as in PR 2.
8. **Tests that need parts before a map** call a test-support seam, `Storage::seed_map_for_tests(project)`. It sets `map_revision` to 1 and writes a version of the current parts, with no gate. It is behind the `test-support` feature, like `InvocationVersions`.
   - Why: the hand-create gate (spec: "before it is approved, a hand create, edit or move of a part is refused") breaks every existing test that creates a part by hand. A seam costs one line per test; walking each test through a real Planner turn costs a fake session per test.
   - The seam never runs in product code; `cargo tree` step 5 of the gate already proves `test-support` stays out of the product build.
9. **The stage is `map` until `map_revision >= 1`, then `structure`.** `missing` at `map` lists the mandatory kinds not yet present or waived. `StageView` gains `drift: Vec<String>`, empty before the first map.
10. **Codex review of the plan (2026-10-12), accepted:**
   - **Only the written part's links are checked.** `check_shape` checks `serves` ids exist only for the parts the edit or proposal writes. A link in another part to a since-removed item is drift (`notifications serves G2, which was removed`), never a refusal: removing G2 must not block an edit of an unrelated part.
   - **Approval re-checks the vision is complete,** inside its write, for every action (§23.6: "propose or approve a map"). A field emptied by hand after the proposal refuses with `StageBlocked("vision incomplete: …")`.
   - **Standards versions come from the Planner's turn,** not from the current state: the proposal records the base version and the additions version that turn's `agent_invocation` row recorded, and staleness compares those with the current ones. The current versions may hold rules the Planner never saw.
   - **Drift is per part, not per map version.** A part records the revision of each item it serves when it is written (`design_part.served_revisions`). Drift names each part whose served item changed or was removed since then, and each unserved goal. There is no "the vision changed since the map" line, so a small edit of one part cannot clear another part's drift.
   - **Paths are compared as Windows sees them:** case-insensitively (`src/API` conflicts with `src/api`). An initial file's path lies inside one of its part's folders, and two files with the same path (case-insensitively) conflict.
11. **`design/contract.yaml` may pass 500 lines.** It is one contract per service by rule, and `contracts.rs` reads one file per service. The commit that crosses 500 says so.

## Review Focus

1. **The first map with a goal no part serves.** Expected: approval refused with `STAGE_BLOCKED: unserved goals: G3`, and nothing written. Test: `first_map_refuses_an_unserved_goal` (Task 2).
2. **A change to `notifications` serving G2, then a hand edit of G2, and separately a new G4.** Expected: the G2 edit makes the card Outdated and approval is refused; adding G4 leaves it valid. Test: `a_part_change_is_outdated_by_its_served_item_only` (Task 2).
3. **Two pending adds of a part with the same title under the same parent.** Expected: both are accepted as proposals; the first approval writes; the second is refused as a conflict. Test: `a_second_add_with_the_same_identity_is_refused_at_approval` (Task 2).
4. **A child folder inside its parent's folder, and a sibling inside another's.** Expected: `backend/src/core` under part `core` (child of `backend`, which owns `backend`) is accepted; the same folder on a sibling is refused. Test: `folders_nest_only_under_their_parent` (Task 1).
5. **A hand edit that moves the only `backend` part under another part, or changes its kind.** Expected: `STAGE_BLOCKED: missing part: backend`, nothing written. Test: `a_hand_edit_cannot_drop_a_mandatory_part` (Task 1).

---

### Task 1: The map in the parts, its versions and its gates

**Files:**
- Create: `crates/shadows-core/migrations/0023_project_map.sql`
- Create: `crates/shadows-core/src/design/map_rules.rs` (pure: normalising folders, and checking a whole set of parts)
- Create: `crates/shadows-core/src/design/store/map_versions.rs` (insert a version; read `map_revision`, waivers and the current parts)
- Modify: `crates/shadows-core/src/design/model.rs` (`PartContent` fields, `InitialFile`, `ContractHead`, `PartWaiver`)
- Modify: `crates/shadows-core/src/design/store/vision.rs` (`edit_design`: the hand-edit gate, and a map version when a part changed)
- Modify: `crates/shadows-core/src/design/stage.rs`, `crates/shadows-core/src/design/store/stage.rs` (map_revision, drift)
- Modify: `crates/shadows-core/src/db/error.rs` (`StorageError::StageBlocked`), the `ErrorCode` enum (`StageBlocked`)
- Modify: `crates/shadows-http/src/failure/storage.rs` (`409 STAGE_BLOCKED`), `crates/shadows-mcp/src/refusal.rs`
- Modify: `crates/shadows-core/src/design/contract.yaml`, `docs/codebase/README.md` (owner rows for the new modules)
- Modify: generated `api/openapi.json`, `web/src/api/schema.d.ts`
- Test:
  - `crates/shadows-core/tests/design_parts.rs` and `tests/design_parts/stage.rs`;
  - `crates/shadows-core/tests/plan_migration.rs` (the migration test beside `migration_0022_…`);
  - every existing test that creates a part by hand (`grep -rln PartCreate crates/*/tests`): add the seam call (Ruling 8).

**Interfaces:**
- Produces:
  ```rust
  // design/model.rs — PartContent keeps its four fields and gains:
  #[serde(default)] pub serves: Vec<String>,          // vision item ids, e.g. "G2"
  #[serde(default)] pub folders: Vec<String>,         // normalised relative paths
  #[serde(default)] pub files: Vec<InitialFile>,
  #[serde(default)] pub contract: ContractHead,
  pub struct InitialFile { pub path: String, pub content: String }
  #[derive(Default)]
  pub struct ContractHead { pub header: String, pub rules: Vec<String> } // "S1", "P2"
  pub struct PartWaiver { pub part: String, pub reason: String,
      pub approved_by: String, pub created_at: String }

  // design/map_rules.rs
  pub(crate) fn normalise_folder(raw: &str) -> Result<String, String>;
  /// One part as the rules see it: its place in the tree and its content.
  pub(crate) struct MapPart<'a> { pub key: &'a str, pub parent: Option<&'a str>,
      pub content: &'a PartContent }
  /// Identity, folder and file ownership (case-insensitive), and the rule ids
  /// of every part; served ids only for the keys in `written` (Ruling 10).
  /// Any failure is `StorageError::Constraint` (422).
  pub(crate) fn check_shape(parts: &[MapPart], written: &[&str], vision: &VisionContent,
      standards: &EffectiveStandards) -> Result<(), String>;
  /// Per-part drift: served items changed or removed since the part's
  /// `served_revisions`, then goals no part serves.
  pub(crate) fn drift(parts: &[(MapPart, &BTreeMap<String, i64>)],
      vision: &VisionContent) -> Vec<String>;
  /// Mandatory kinds on top-level parts, minus waivers: what is missing.
  pub(crate) fn missing_kinds(parts: &[MapPart], waived: &[String],
      standards: &EffectiveStandards) -> Vec<String>;
  /// Goal ids no part serves.
  pub(crate) fn unserved_goals(parts: &[MapPart], vision: &VisionContent) -> Vec<String>;

  // stage.rs
  pub struct StageView { pub stage: Stage, pub missing: Vec<String>,
      #[serde(default)] pub drift: Vec<String> }
  ```

**Migration `0023_project_map.sql`:**
```sql
-- §23.5 (map proposals, 2026-10-12): the map is the parts. Each change to the
-- parts after the first approved map is a stored version; waivers are recorded.
ALTER TABLE design_workspace ADD COLUMN map_revision INTEGER NOT NULL DEFAULT 0
    CHECK (map_revision >= 0);
-- Each served item's revision when the part was last written: drift per part.
ALTER TABLE design_part ADD COLUMN served_revisions TEXT NOT NULL DEFAULT '{}'
    CHECK (json_valid(served_revisions));

CREATE TABLE design_map_version (
    project_id      TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    number          INTEGER NOT NULL CHECK (number >= 1),
    -- Every part: id, parent, ordinal, revision, content.
    parts_json      TEXT NOT NULL CHECK (json_valid(parts_json)),
    -- The vision version when this map version was written; a record only.
    -- Drift reads design_part.served_revisions (Ruling 10).
    vision_revision INTEGER NOT NULL,
    -- The approved map proposal; NULL for a hand edit or the test seam.
    proposal_id     TEXT NULL,
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    created_at      TEXT NOT NULL,
    PRIMARY KEY (project_id, number)
);

CREATE TABLE design_part_waiver (
    project_id   TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    part_name    TEXT NOT NULL,
    reason       TEXT NOT NULL CHECK (length(trim(reason)) > 0),
    approved_by  TEXT NOT NULL,
    proposal_id  TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    PRIMARY KEY (project_id, part_name)
);
```
The proposal table is added in Task 2 by the same migration file. Task 1 writes the file; Task 2 appends its `CREATE TABLE` before the branch's first push. The file is new in this PR, so this edits no shipped migration.

- [x] **Step 1: Write the failing tests**

In `tests/design_parts.rs` (or a new `tests/design_parts/map.rs` submodule for the map ones):
- `folders_nest_only_under_their_parent` (Review Focus 4);
- `a_hand_edit_cannot_drop_a_mandatory_part` (Review Focus 5): seed a map with one top-level part of each mandatory kind, then try `PartPut` with `kind: None` on `backend`, and `PartMove` of `backend` under `api`. Both are `StageBlocked("missing part: backend")`, and the revision is unchanged;
- `a_hand_part_create_before_the_first_map_is_stage_blocked`: refused, and after `seed_map_for_tests` the same create succeeds;
- `a_hand_part_edit_writes_a_map_version`: after the seam, one `PartPut` makes version 2 with `proposal_id` NULL;
- `a_folder_with_dot_dot_or_absolute_is_refused`: `../x`, `/x`, `C:/x`, `a//b` are refused; `./a/b/` is stored as `a/b`.

In `tests/design_parts/stage.rs`:
- `the_stage_is_map_until_a_map_is_approved_then_structure`;
- `drift_names_an_unserved_goal_and_a_changed_served_item`: after the seam with part `core` serving G1 and part `web` serving nothing, add G2 and edit G1 by hand. Drift is `["G2 is served by no part", "core: G1 changed since the part was written"]`. Then a hand edit of `web` alone leaves both lines (Ruling 10).
- `removing_a_served_goal_does_not_block_another_part`: remove G1, which `core` serves; a hand edit of `web` succeeds, and drift names `core serves G1, which was removed`.
- `paths_conflict_case_insensitively_and_files_stay_in_their_folders`: folders `src/API` and `src/api` on two siblings conflict; a file `other/x.rs` on a part whose folders are `src` is refused.

In `tests/plan_migration.rs`:
- `migration_0023_keeps_parts_and_starts_without_a_map`: a part row written before 0023 still parses with empty `serves`, `folders`, `files` and contract; `map_revision` is 0.

- [x] **Step 2: Run them; each fails**

Run: `cargo test -p shadows-core --test design_parts --test plan_migration`
Expected: FAIL (compile errors on the new fields and the seam).

- [x] **Step 3: Implement**

- Migration, model fields, `map_rules.rs`, `map_versions.rs`, `StageBlocked` end to end, and the seam.
- In `edit_design`, after the revision check and before applying ops:
  1. If any op is a part op and `map_revision == 0`: `StageBlocked("the first map comes from the Planner: ask it to propose one")`.
  2. After applying the ops, read every part, then call `check_shape` (served ids checked only for the parts this edit wrote) and `missing_kinds`, inside the same transaction. A missing kind that was not missing before the edit is `StageBlocked("missing part: <kind>")`. Set each written part's `served_revisions`. Then insert a map version and set `map_revision = number`.
- `stage::compute` takes `map_revision`, the waivers and the parts' `serves` instead of the bare kinds.
- `stage::line` at `structure` appends drift, for example `Drift: G4 is served by no part.`

- [x] **Step 4: Run the tests and the existing parts and agreement tests**

Run: `cargo test -p shadows-core --test design_parts --test plan_migration --test design_outcomes --test agreement_bindings --test agreement_review --test design_migration`
Then: `cargo test -p shadows --test design_routes --test project_events --test workspace_mcp --test agreement_journey`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add -A crates api web/src/api docs/codebase
git commit -m "feat(design): the map in the parts, its versions and its hand-edit gates"
```

---

### Task 2: Map proposals in the core

**Files:**
- Modify: `crates/shadows-core/migrations/0023_project_map.sql` (append the proposal table)
- Create: `crates/shadows-core/src/design/map_proposals.rs` (values and `Design` entry points)
- Create: `crates/shadows-core/src/design/store/map_proposals.rs` (propose)
- Create: `crates/shadows-core/src/design/store/map_approval.rs` (approve)
- Create: `crates/shadows-core/src/design/store/map_proposal_read.rs` (load, list, outdated)
- Modify: `crates/shadows-core/src/threads/model.rs` (`ThreadEntryKind::MapProposal`)
- Modify: `crates/shadows-core/src/events/store.rs` (`MapProposed` reaches project subscribers: PR 2's bug, `e1188bd`)
- Modify: contracts (`design`, `threads`, `events`), `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/design_parts/map.rs`

**Interfaces:**
- Produces:
  ```rust
  #[serde(rename_all = "snake_case")]
  pub enum MapAction { First, Add, Change }
  pub struct ProposedPart {
      pub key: String,                 // first: unique in the proposal; add: unused
      pub parent_key: Option<String>,  // first only
      pub parent: Option<PartId>,      // add only: an existing part, or top level
      pub content: PartContent,
  }
  pub struct ProposedWaiver { pub part: String, pub reason: String }
  pub struct MapPropose {
      pub action: MapAction,
      pub parts: Vec<ProposedPart>,        // first: all; add/change: exactly one
      pub part_id: Option<PartId>,         // change only
      pub waivers: Vec<ProposedWaiver>,    // first only (Ruling 4)
      pub replaces: Option<String>,
  }
  pub struct MapProposal {
      pub id: String, pub thread_id: ThreadId, pub action: MapAction,
      pub parts: Vec<ProposedPart>, pub part_id: Option<PartId>,
      pub old: Option<PartContent>,        // change: the part as it was
      pub waivers: Vec<ProposedWaiver>,
      pub status: ProposalStatus, pub outdated: bool, pub created_at: String,
  }
  impl Design {
      pub async fn propose_map(&self, grant: &Grant, args: MapPropose)
          -> Result<MapProposal, CoreError>;
      pub async fn map_proposals(&self, project: &ProjectId)
          -> Result<Vec<MapProposal>, CoreError>;
      pub async fn approve_map_proposal(&self, command_id: String,
          project: &ProjectId, proposal: &str) -> Result<DesignChange, CoreError>;
  }
  ```

**The proposal table** (append to `0023_project_map.sql`):
```sql
CREATE TABLE design_map_proposal (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id     TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id  TEXT NOT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id      TEXT NOT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    action        TEXT NOT NULL CHECK (action IN ('first','add','change')),
    part_id       TEXT NULL,
    payload_json  TEXT NOT NULL CHECK (json_valid(payload_json)),
    -- What staleness compares (§23.5): the first map's vision revision and
    -- map revision; a change's part revision and its served items' revisions.
    vision_revision INTEGER NOT NULL,
    map_revision    INTEGER NOT NULL,
    expected_part_revision INTEGER NULL,
    expected_items_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(expected_items_json)),
    status        TEXT NOT NULL CHECK (status IN ('Pending','Approved','Superseded')),
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    created_at    TEXT NOT NULL,
    seq           INTEGER NOT NULL,
    UNIQUE (project_id, seq)
);
CREATE INDEX map_proposal_part ON design_map_proposal(project_id,part_id,status);
```

**Propose** (one write, in this order, copying `propose_vision_in`):
1. Planner writer only; replay by `classify` returns the stored card.
2. `check_running_turn_in` (PR 2's race fix).
3. The vision is complete, or `StageBlocked("vision incomplete: <fields>")`.
4. By action:
   - `first`: `map_revision == 0`, or `Constraint("a map is approved; propose a change to one part")`.
   - `add` / `change`: `map_revision >= 1`, or `StageBlocked("no map yet: propose the first map")`. `change` needs an existing `part_id`.
5. Shape: build the whole resulting set (the current parts with the change applied, or the proposal's parts for `first`) and run `check_shape`. Waivers: a waiver of a non-waivable kind is `Constraint`, and a waiver needs a non-blank reason. Coverage and missing kinds are **not** checked at proposal; they are approval gates (§23.6 table), so the Planner can show a draft.
6. Supersede: `first` supersedes every pending `first`; `change` supersedes pending changes of the same `part_id`; `replaces` supersedes the named pending proposal (any action).
7. Record the expectations: the current `vision_revision` and `map_revision`; for `change` the part's revision; and for `add`/`change` the revision of each id in `serves` (`expected_items_json`). The two standards versions are the ones the turn's `agent_invocation` row recorded (Ruling 10), as `propose_vision_in` already does for the additions version.
8. Append the `MapProposal` entry with the card, the row, the `MapProposed` event, and `record_command`.

**Outdated** (computed on read, Pending only):
- `first`: the vision revision changed, or the map revision is not 0, or either standards version differs from the current.
- `change`: the part's revision changed, or any expected item is missing or has another revision, or a standards version differs.
- `add`: any expected item is missing or changed, or a standards version differs.

**Approve** (one write; check order as in PR 2):
1. Replay; the live project; the proposal belongs to this project; status Pending (Superseded and Approved refuse); outdated refuses with `Constraint("the map changed after this proposal; ask the Planner to propose again")`.
2. The vision is complete, or `StageBlocked("vision incomplete: <fields>")` (Ruling 10). Then build the resulting set and run `check_shape` again (conflicts may have appeared since: Review Focus 3).
3. The gates, as `StageBlocked`:
   - `first`: `missing_kinds` with the proposal's waivers is empty, else `missing part: database, api`; and `unserved_goals` is empty, else `unserved goals: G3`.
   - `add`/`change`: `missing_kinds` is no worse than before (a change may not drop a kind).
4. Write: create the parts (`first`: a `PartId::generate()` per key, parents before children, ordinal by proposal order) or update the one part, setting each written part's `served_revisions` from the current vision; the waivers with `approved_by` = the person's principal id; a map version with `proposal_id`; `map_revision`; the design revision +1; the proposal `Approved`; `ProjectDesignChanged` with `changed_parts`; `design_command_result`; `record_command`.

- [x] **Step 1: Write the failing tests** in `tests/design_parts/map.rs`. Use PR 2's helpers for a running Planner turn (`tests/design_vision/proposals.rs` shows how).
  - `first_map_is_stage_blocked_while_the_vision_is_incomplete`
  - `first_map_approval_needs_every_mandatory_part_or_a_waiver`: missing `database` refuses; a waiver of `frontend` with a reason passes and is recorded with the approver; a waiver of `backend` is refused at proposal.
  - `first_map_refuses_an_unserved_goal` (Review Focus 1)
  - `first_map_is_outdated_by_any_vision_change`
  - `a_part_change_is_outdated_by_its_served_item_only` (Review Focus 2)
  - `a_second_add_with_the_same_identity_is_refused_at_approval` (Review Focus 3)
  - `a_newer_first_map_supersedes_the_pending_one`
  - `approving_a_map_twice_with_one_command_writes_once`
  - `a_proposal_from_an_ended_turn_is_refused`

- [x] **Step 2: Run them; each fails.** Run: `cargo test -p shadows-core --test design_parts map::`. Expected: FAIL.
- [x] **Step 3: Implement** as above.
- [x] **Step 4: Run.** Run: `cargo test -p shadows-core --test design_parts --test design_vision`. Expected: PASS.
- [x] **Step 5: Commit** `feat(design): Planner map proposals and atomic person approval`.

---

### Task 3: The `map_propose` MCP tool and the Planner's instructions

**Files:**
- Modify: `crates/shadows-mcp/src/workspace_tools.rs` (the tool), `crates/shadows-mcp/src/server.rs` (`THREAD_TOOLS` 13 → 14)
- Modify: `crates/shadows-core/src/harness/prompt.txt`, `crates/shadows-core/src/harness/contract.yaml`, `crates/shadows-core/src/grants/contract.yaml`
- Test: `crates/shadows/tests/workspace_mcp.rs`, `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/planner_mcp.rs`

- [x] **Step 1: Failing tests:**
  - `map_propose_draws_a_card_and_writes_nothing`
  - `map_propose_before_a_complete_vision_answers_stage_blocked`: the tool text begins `STAGE_BLOCKED: vision incomplete`.
  - The tool list counts grow by one.
- [x] **Step 2: Run; they fail.** Run: `cargo test -p shadows --test workspace_mcp --test mcp_server --test planner_mcp`.
- [x] **Step 3: Implement.**
  - The tool description: "Propose the project map, or one change to it. With `first`, give every part (each with a `key`, and `parent_key` for a child), the vision ids it serves, its folders, its initial files (manifests and skeletons, no logic) and its contract head. Add a waiver only for `api` or `frontend`, with the reason. With `add` or `change`, give one part. One call is one card. Nothing changes until the person approves it."
  - `prompt.txt` gains a paragraph for the map stage: read `workspace_get` and the standards, propose the whole first map in one call, every goal served, mandatory parts as top-level parts with their `kind`, and on `STAGE_BLOCKED` go back to what the reason names.
- [x] **Step 4: Run.** Expected: PASS.
- [x] **Step 5: Commit** `feat(mcp): let the Planner propose the project map`.

---

### Task 4: HTTP routes

**Files:**
- Modify: `crates/shadows-http/src/design.rs`, `crates/shadows-http/src/lib.rs`
- Modify: generated `api/openapi.json`, `web/src/api/schema.d.ts`
- Test: `crates/shadows/tests/design_routes.rs`, `crates/shadows/tests/openapi.rs`

Routes, copying the vision proposal pair:
- `GET /api/projects/{id}/map-proposals` → `Vec<MapProposal>`
- `POST /api/projects/{id}/map-proposals/{proposal_id}/approve` with `{ command_id }` → `DesignChange`.
  - Refusals: `409 STAGE_BLOCKED` for a gate; `422 INVALID_COMMAND` for the rest.

- [x] **Step 1: Failing test** `map_proposal_routes_list_and_approve`: a refused approval answers 409 with code `STAGE_BLOCKED`, and a valid one 200.
- [x] **Step 2: Run; it fails.** Run: `cargo test -p shadows --test design_routes --test openapi`.
- [x] **Step 3: Implement**, then `npm run gen:api` in `web/`.
- [x] **Step 4: Run.** Expected: PASS.
- [x] **Step 5: Commit** `feat(http): list and approve map proposals`.

---

### Task 5: The Project map page

**Files:**
- Modify: `web/src/app/design/parts-view.tsx`, `part-editor.tsx`, `workspace-page.tsx`
- Modify: `web/src/api/design.ts`, `web/src/api/queries.ts`
- Modify: `web/src/stream/use-project-events.ts` (`MapProposed` invalidates `'design'`)
- Modify: `web/src/test/fake-daemon.ts`, `web/src/test/contract-fixtures.ts`
- Test: `web/src/app/design/parts-view.test.tsx`, `web/src/stream/use-project-events.test.tsx`

Behaviour:
- **Before the first map:** no "Add part" button. A line instead: "The first map comes from the Planner. Ask for it in a conversation." It names the waiting first-map proposal with a link, if one exists.
- **Each part shows:** kind, the goals it serves (as ids with their text on hover), its folders, its files (names; content opens read-only), and its contract head.
- **Drift:** a notice above the tree lists `stage.drift`; each unserved goal is named.
- **Pending changes:** "N map proposals await your approval", with links per conversation, as on the Vision page.
- **The part editor** edits title, responsibility, design, kind, serves (checkboxes of vision items), folders (one per line) and the contract header and rules. Files are read-only (Ruling 6).
- **Errors:** a `STAGE_BLOCKED` answer is shown as the reason text, in place.

- [x] **Step 1: Failing tests:**
  - `hides_add_part_before_the_first_map`;
  - `shows_drift_and_the_pending_count`;
  - `edits_serves_and_folders`;
  - `shows_a_stage_blocked_reason`;
  - and, in the events test, `MapProposed` invalidates design.
- [x] **Step 2: Run; they fail.** Run: `npx vitest run src/app/design/parts-view.test.tsx src/stream/use-project-events.test.tsx` in `web/`.
- [x] **Step 3: Implement.**
- [x] **Step 4: Run them, then `npx tsc -b --noEmit` and `npm run lint`.** Expected: PASS.
- [x] **Step 5: Commit** `feat(web): the project map with served goals, folders, contracts and drift`.

---

### Task 6: The map proposal card

**Files:**
- Create: `web/src/app/conversation/map-proposal-card.tsx`, `map-proposal-card.test.tsx`
- Modify: `web/src/app/conversation/entry.tsx` (the `MapProposal` kind)
- Reuse: the side panel pattern of `plan-side-panel.tsx` for a contract

Behaviour:
- **First map:** the part tree with each part's kind, served goals and folders.
  - Waivers sit in their own highlighted block above Approve: "Without frontend — reason: …".
  - Each part's contract opens in the side panel.
- **Change:** the part before and after, with changed fields marked.
- **Add:** the new part.
- **Status** is read live from the list query, as the vision card does: Approve, Approved, Superseded, or "Outdated — ask the Planner to propose again".
- **A refused approval** shows its reason under the button, `STAGE_BLOCKED` included.

- [x] **Step 1: Failing tests:**
  - `shows_waivers_apart_before_approve`;
  - `approve_posts_and_shows_approved`;
  - `outdated_has_no_approve_button`;
  - `a_stage_blocked_refusal_shows_its_reason`;
  - `a_contract_opens_in_the_side_panel`.
- [x] **Step 2: Run; they fail.** Run: `npx vitest run src/app/conversation/map-proposal-card.test.tsx`.
- [x] **Step 3: Implement.**
- [x] **Step 4: Run; then type-check and lint.** Expected: PASS.
- [x] **Step 5: Commit** `feat(web): approve the project map from its conversation card`.

---

### Task 7: Evidence, the browser run and the full gate

- [x] **Step 1: Browser run on a copy of the development database.**
  - Start a separate daemon and web on free ports. Leave `.claude/launch.json` and the person's daemon alone.
  - Use the real Planner: complete a vision, then ask for the map. Check:
    1. a map missing `database` is refused with its reason, in the card;
    2. a full map with a waived `frontend` is approved, and the waiver shows;
    3. Project map shows served goals, folders and contracts;
    4. adding G4 by hand shows drift;
    5. a change proposal for a part serving G2 goes Outdated after a hand edit of G2.
  - Screenshots go into `docs/evidence/2026-10-12-guided-planning-pr3/`.
  - **Stop the trial daemon and web server when done.**
- [ ] **Step 2: Full gate, once,** from the root, as CLAUDE.md lists it, plus the web gate (`npm test`, `npx tsc -b --noEmit`, `npm run lint`, `npm run build`).
- [ ] **Step 3: Write `docs/evidence/2026-10-12-guided-planning-pr3.md`** with the gate counts, the browser checks and what was not tried. Update `docs/status.md`, and tick this plan's boxes by hand.
- [ ] **Step 4: Commit** `docs: record PR 3 browser acceptance and gate`. Do not push.
