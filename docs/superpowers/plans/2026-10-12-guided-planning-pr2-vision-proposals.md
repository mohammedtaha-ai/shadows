# §23 PR 2 — Vision Proposals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The vision becomes five lists of items with stable ids.
- The Planner proposes one item change per card with `vision_propose`.
- The person approves each card with its button, and an approved card writes a new vision version.
- The Vision page lists the items with their origin, and says how many proposals are waiting.

**Architecture:**
- **Items:** `VisionContent` changes from five strings to five `Vec<VisionItem>`, still stored as JSON in `design_workspace.vision_content`. A migration converts the old text, and a new table keeps every vision version.
- **Hand edits:** three item ops replace `VisionPut`, through the existing `Design::edit` and its revision check.
- **Proposals:** they live in a new table and are drawn as a `VisionProposal` thread entry whose `card` holds the proposal.
- **Approval:** it applies the same item op inside one write. It checks staleness per item (§23.5, decided 2026-10-12), not the workspace revision.

**Tech Stack:** Rust (SQLx 0.9 / SQLite, axum + utoipa, rmcp), React + TanStack Query.

**Spec:** `docs/superpowers/specs/2026-10-08-guided-planning-design.md`
- §23.3, with its 2026-10-12 block;
- §23.5, with its "Vision proposals" block;
- §23.4, for drift, which stays out of this PR;
- §23.8, for the entry `card` column.

## Global Constraints

- **Cargo commands:** prefix every one with `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0`.
- **Line length:** no Rust line over 100 columns.
- **Migrations:** never edit an existing one. This PR adds `0022_vision_items.sql` only.
- **Integration tests:** add no new test file under `tests/`; each is a ~49 MB binary. Extend the files named here, including their existing submodule directories (`tests/design_vision/`, `tests/design_parts/`).
- **API:** when `api/openapi.json` changes, run `npm run gen:api` in `web/` and commit both files. Between Task 1 and Task 5 the web client does not type-check against the new schema; that is expected, and the web gate runs in Task 7.
- **Edit tools:** no prettier, and no Python or sed edits to source files.
- **Tests:** targeted tests per task; the full gate once, in Task 7.
- **Contracts:** every service change updates its `contract.yaml` in the same commit.
- **Navigation:** use the Rust LSP (`documentSymbol`, `findReferences`) for signatures and callers. The `shadows` MCP server is not running; say so in the report.
- **Approval scope:** in this PR, only the person approves, through HTTP. No MCP tool approves.

## Rulings made while planning

Record each as a ledger `Ruling:` line at Task 1.

1. **`VisionPut` is removed.** It is replaced by `VisionItemAdd { field, text }`, `VisionItemPut { id, text }` and `VisionItemRemove { id }`.
   - Why: a whole-content put lets a client invent or reuse ids, which §23.3 forbids.
   - Cost if wrong: one more op variant.
2. **`vision_propose` is a thread-grant tool only.** An external agent's project grant has no conversation to draw a card in, as with `plan_show`.
3. **One item change per call.** The Planner calls `vision_propose` once per card; three goals are three calls in one turn.
   - The arguments are `{ action: "add" | "edit" | "remove", field?, item_id?, text?, replaces? }`.
     - `add` needs `field` and `text`;
     - `edit` needs `item_id` and `text`;
     - `remove` needs `item_id`;
     - `replaces` (a proposal id) is allowed on any action and supersedes that pending proposal.
4. **Proposal status is stored:** `Pending`, `Approved` or `Superseded`.
   - **Outdated:** this is computed when read (`outdated: bool`), never stored, because a hand edit can make a proposal outdated without touching the proposal row.
   - **Approval:** approving an outdated proposal is refused with `INVALID_COMMAND` "G2 changed after this proposal; ask the Planner to propose again".
5. **Card status is live.** The card's entry holds the proposal as it was proposed; the card reads status and `outdated` from `GET /api/projects/{id}/vision-proposals`. Events invalidate that query because it sits under the `'design'` key.
6. **No history route.** Vision versions are stored, but no route reads them (§23.3, decided 2026-10-12). A store reader exists for tests and for the next-id rule.
7. **No drift in the stage.** It moves to PR 3 (§23.4 as amended). The stage only changes "field filled" from "trimmed text non-empty" to "list non-empty".

## Review Focus

1. **Two pending adds in one field, approved in either order.** Expected: they get consecutive ids in approval order, and neither is refused. Test: `two_adds_approve_in_any_order_with_consecutive_ids` (Task 2).
2. **A hand edit of G1 while an edit proposal for G1 is pending.** Expected: the card reads Outdated, approval is refused, and nothing is written. Test: `an_edit_proposal_is_outdated_after_a_hand_edit_of_its_item` (Task 2).
3. **A removed id after a new add.** Expected: remove G2 (of G1–G2), add one: it is G3, not G2. Test: `a_removed_id_is_never_reused` (Task 1).
4. **A replayed approval** (same command id) after the proposal was approved. Expected: the stored outcome is returned, with no second version and no second event. Test: `approving_twice_with_one_command_writes_once` (Task 2).
5. **Old databases with text vision.** Expected: migration 0022 makes each non-empty field one item with the field's first id, makes an empty or whitespace field `[]`, and writes version rows. Test: `migration_0022_turns_vision_text_into_items` (Task 1).

---

### Task 1: The vision as items

**Files:**
- Create: `crates/shadows-core/migrations/0022_vision_items.sql`
- Create: `crates/shadows-core/src/design/vision_items.rs` (pure: prefixes, next id, applying an item op to `VisionContent`)
- Modify: `crates/shadows-core/src/design/model.rs` (`VisionItem`, `VisionField`, `VisionContent`, the `DesignOp` vision variants)
- Modify: `crates/shadows-core/src/design/ops.rs` (normalize the new ops: trim nothing, refuse an empty `text`)
- Modify: `crates/shadows-core/src/design/store/vision.rs` (apply item ops, write a version row on every vision change)
- Create: `crates/shadows-core/src/design/store/vision_versions.rs` (version rows: insert, read all for a project)
- Modify: `crates/shadows-core/src/design/stage.rs` (filled = non-empty list)
- Modify: `crates/shadows-core/src/design/mod.rs`, `crates/shadows-core/src/lib.rs` (re-exports)
- Modify: `crates/shadows-core/src/design/contract.yaml`, `docs/codebase/README.md`
- Modify: generated `api/openapi.json`, `web/src/api/schema.d.ts`
- Test:
  - `crates/shadows-core/tests/design_vision.rs`;
  - `crates/shadows-core/tests/design_parts/stage.rs`;
  - `crates/shadows-core/tests/plan_migration.rs`, holding the migration test beside `migration_0020_…`;
  - `crates/shadows/tests/design_routes.rs`;
  - `crates/shadows/tests/workspace_mcp.rs`.

**Interfaces:**
- Produces:
  ```rust
  // design/model.rs
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "snake_case")]
  pub enum VisionField { Purpose, Users, Goals, Boundaries, TechnicalDirection }
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct VisionItem {
      pub id: String,
      pub text: String,
      /// The conversation the item was last approved from; `None` for a hand edit.
      pub approved_from: Option<ThreadId>,
  }
  #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct VisionContent {
      pub purpose: Vec<VisionItem>, pub users: Vec<VisionItem>, pub goals: Vec<VisionItem>,
      pub boundaries: Vec<VisionItem>, pub technical_direction: Vec<VisionItem>,
  }
  // DesignOp: VisionPut removed;
  VisionItemAdd { field: VisionField, text: String },
  VisionItemPut { id: String, text: String },
  VisionItemRemove { id: String },
  // design/vision_items.rs
  impl VisionField {
      pub fn prefix(self) -> &'static str   // "PU" | "U" | "G" | "B" | "T"
      pub fn of_id(id: &str) -> Option<VisionField>   // "G12" -> Goals; "PU1" -> Purpose
      pub const ALL: [VisionField; 5]
  }
  impl VisionContent {
      pub fn items(&self, field: VisionField) -> &Vec<VisionItem>
      pub fn item(&self, id: &str) -> Option<&VisionItem>
  }
  pub(crate) fn next_id(field: VisionField, history: &[VisionContent]) -> String
  /// Applies one item op; `from` becomes the item's `approved_from`.
  pub(crate) fn apply(content: &mut VisionContent, op: &DesignOp, id_for_add: Option<String>,
      from: Option<&ThreadId>) -> Result<(), String>
  // design/store/vision_versions.rs
  pub(crate) async fn insert_version(conn: &mut SqliteConnection, project: &ProjectId,
      number: i64, content: &VisionContent, proposal: Option<&str>, ts: &str)
      -> Result<(), StorageError>
  pub(crate) async fn all_versions(conn: &mut SqliteConnection, project: &ProjectId)
      -> Result<Vec<VisionContent>, StorageError>
  ```
  `next_id(field, history)` is one past the highest number with that field's prefix in any version, the current one included. `PU` is matched before `U`, and the id after the prefix must be all digits.

- [ ] **Step 1: Write the failing tests**

In `design_vision.rs`:
1. Replace the `content(text)` helper and the existing `VisionPut` tests with item-op equivalents. Keep their assertions on atomicity, replay and revision.
2. Add:

```rust
fn add(field: VisionField, text: &str) -> DesignOp {
    DesignOp::VisionItemAdd { field, text: text.into() }
}

#[tokio::test]
async fn a_removed_id_is_never_reused() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, p) = core_and_project(&tmp).await;
    let d = core.design();
    d.edit("a".into(), &p, 0, vec![add(VisionField::Goals, "one"), add(VisionField::Goals, "two")])
        .await
        .unwrap();
    let ids = |v: &VisionView| v.content.goals.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&d.vision(&p).await.unwrap()), ["G1", "G2"]);
    d.edit("r".into(), &p, 1, vec![DesignOp::VisionItemRemove { id: "G2".into() }])
        .await
        .unwrap();
    d.edit("b".into(), &p, 2, vec![add(VisionField::Goals, "three")]).await.unwrap();
    assert_eq!(ids(&d.vision(&p).await.unwrap()), ["G1", "G3"]);
}

#[tokio::test]
async fn every_vision_change_is_a_stored_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, p) = core_and_project(&tmp).await;
    let d = core.design();
    d.edit("a".into(), &p, 0, vec![add(VisionField::Purpose, "Sell books")]).await.unwrap();
    let put = DesignOp::VisionItemPut { id: "PU1".into(), text: "Sell used books".into() };
    d.edit("b".into(), &p, 1, vec![put]).await.unwrap();
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT number, content_json FROM design_vision_version WHERE project_id = ?
          ORDER BY number",
    )
    .bind(p.as_str())
    .fetch_all(core_storage(&core).reader())
    .await
    .unwrap();
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), [1, 2]);
    assert!(rows[0].1.contains("\"Sell books\"") && rows[1].1.contains("\"Sell used books\""));
}

#[tokio::test]
async fn item_ops_refuse_an_unknown_id_and_empty_text() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, p) = core_and_project(&tmp).await;
    for op in [
        DesignOp::VisionItemPut { id: "G9".into(), text: "x".into() },
        DesignOp::VisionItemRemove { id: "X1".into() },
        add(VisionField::Users, "   "),
    ] {
        let refused = core.design().edit(format!("{op:?}"), &p, 0, vec![op]).await;
        assert!(
            matches!(&refused, Err(CoreError::Refused { code: ErrorCode::InvalidCommand, .. })),
            "{refused:?}"
        );
    }
    assert_eq!(core.design().vision(&p).await.unwrap().revision, 0);
}
```

(`core_storage` is whatever the file already uses to reach `Storage`; the helper `core_and_project` exists from PR 1. If either has another name, use the file's own, and find it with LSP `documentSymbol`.)

In `tests/design_parts/stage.rs`, change its `VisionContent { … }` literals to item lists. One item per field means filled; `vec![]` means missing.

In `plan_migration.rs`, add, next to `migration_0020_…` and using the same `migrated_to(db, last)` helper:

```rust
#[tokio::test]
async fn migration_0022_turns_vision_text_into_items() {
    // migrate to 0021, insert a project + design_workspace row with
    // vision_content = {"purpose":"Sell books","users":"  ","goals":"G",
    //   "boundaries":"","technical_direction":"Rust"}, vision_revision 3,
    // then migrate to the end.
    // Assert vision_content parses as VisionContent with
    //   purpose = [PU1 "Sell books"], users = [], goals = [G1 "G"],
    //   boundaries = [], technical_direction = [T1 "Rust"], approved_from None;
    // and design_vision_version has one row, number 3, with that content.
}
```

Write the test body in full, following the `0020` test's setup exactly.

Update `design_routes.rs` `vision_routes_round_trip_conflict_and_preserve_conversations` and `workspace_mcp.rs` to the item shape:
- the request ops become `VisionItemAdd`;
- the expected `content` becomes lists of `{ id, text, approved_from: null }`.

- [ ] **Step 2: Run them to see them fail**

Run: `… cargo test -p shadows-core --test design_vision`
Expected: compile errors on `VisionField` / `VisionItemAdd`.

- [ ] **Step 3: Write the migration** `0022_vision_items.sql`

```sql
-- §23.3: the vision's five fields become lists of items with stable ids, and
-- every vision change is a stored version. Text becomes one item per field.
CREATE TABLE design_vision_version (
    project_id   TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    number       INTEGER NOT NULL CHECK (number >= 1),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    -- The approved proposal this version applied; NULL for a hand edit.
    proposal_id  TEXT NULL,
    created_at   TEXT NOT NULL,
    PRIMARY KEY (project_id, number)
);

UPDATE design_workspace SET vision_content = json_object(
    'purpose', CASE WHEN trim(coalesce(json_extract(vision_content, '$.purpose'), '')) = ''
        THEN json_array() ELSE json_array(json_object('id', 'PU1',
        'text', json_extract(vision_content, '$.purpose'), 'approved_from', NULL)) END,
    'users', CASE WHEN trim(coalesce(json_extract(vision_content, '$.users'), '')) = ''
        THEN json_array() ELSE json_array(json_object('id', 'U1',
        'text', json_extract(vision_content, '$.users'), 'approved_from', NULL)) END,
    'goals', CASE WHEN trim(coalesce(json_extract(vision_content, '$.goals'), '')) = ''
        THEN json_array() ELSE json_array(json_object('id', 'G1',
        'text', json_extract(vision_content, '$.goals'), 'approved_from', NULL)) END,
    'boundaries', CASE WHEN trim(coalesce(json_extract(vision_content, '$.boundaries'), '')) = ''
        THEN json_array() ELSE json_array(json_object('id', 'B1',
        'text', json_extract(vision_content, '$.boundaries'), 'approved_from', NULL)) END,
    'technical_direction', CASE
        WHEN trim(coalesce(json_extract(vision_content, '$.technical_direction'), '')) = ''
        THEN json_array() ELSE json_array(json_object('id', 'T1',
        'text', json_extract(vision_content, '$.technical_direction'),
        'approved_from', NULL)) END
);

INSERT INTO design_vision_version (project_id, number, content_json, proposal_id, created_at)
SELECT project_id, vision_revision, vision_content, NULL, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
  FROM design_workspace WHERE vision_revision >= 1;
```

Check that `json_object(…, 'approved_from', NULL)` stores JSON `null`; the migration test asserts it.

- [ ] **Step 4: Write the model and `vision_items.rs`.** `apply`:
  - **`VisionItemAdd`:** needs `id_for_add` and refuses a blank `text`. It pushes `VisionItem { id, text, approved_from: from.cloned() }` onto the field's list.
  - **`VisionItemPut`:** finds the item by id in the field `VisionField::of_id` names; when the id is unknown it returns `Err("G9 is not in the vision")`. It replaces `text` and sets `approved_from = from.cloned()`.
  - **`VisionItemRemove`:** removes the item; an unknown id is an error.
  - **Other ops:** `unreachable!`, because the caller only passes vision ops.

  `next_id`:
  ```rust
  pub(crate) fn next_id(field: VisionField, history: &[VisionContent]) -> String {
      let highest = history
          .iter()
          .flat_map(|c| c.items(field))
          .filter_map(|i| number_of(field, &i.id))
          .max()
          .unwrap_or(0);
      format!("{}{}", field.prefix(), highest + 1)
  }

  fn number_of(field: VisionField, id: &str) -> Option<u32> {
      let rest = id.strip_prefix(field.prefix())?;
      (!rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())).then(|| rest.parse().ok())?
  }
  ```
  `of_id` tries `PU` first, then `U`, `G`, `B`, `T`, and needs a digits-only rest.

- [ ] **Step 5: Wire the store**

In `edit_design`, replace the `VisionPut` arm by collecting the vision ops in order. If there are any, after the part and outcome ops:
1. Load the current content.
2. Load `all_versions`.
3. For each op, call `apply`, with `id_for_add = Some(next_id(field, &[history…, current]))` recomputed after each add, so two adds in one edit get G1 and G2.
4. Write `vision_content` and `vision_revision + 1`.
5. Call `insert_version(conn, project, new_vision_revision, &content, None, &ts)`.

An `apply` error becomes `StorageError::Constraint(message)`; `Design::edit` already maps that to `INVALID_COMMAND`.

In `stage.rs`, change the "empty" test to `items(field).is_empty()`.

- [ ] **Step 6: Run the tests**

Run:
1. `… cargo test -p shadows-core --test design_vision`
2. `… cargo test -p shadows-core --test design_parts`
3. `… cargo test -p shadows-core --test plan_migration`
4. `… cargo test -p shadows --test design_routes`
5. `… cargo test -p shadows --test workspace_mcp`
6. `… cargo test -p shadows-core --test storage_contract`, which keeps the schema inventory: add `design_vision_version` there.
7. `… cargo test -p shadows --test openapi`
8. `cd web && npm run gen:api`

Expected: all pass. `schema.d.ts` changes.

- [ ] **Step 7: Contract, code map, commit**
  - **`design/contract.yaml`:**
    - shapes: `VisionItem`, `VisionField`, the new `VisionContent`;
    - `DesignOp` vision variants;
    - obligations, each naming its test:
      - "ids are per field, one past the highest ever issued, never reused";
      - "every vision change is a stored version";
      - "item ops refuse unknown ids and blank text without writing".
  - **Code-map rows:**
    - `design/vision_items.rs` | "applying vision item changes";
    - `design/store/vision_versions.rs` | "stored vision versions".

```bash
git add crates api web/src/api/schema.d.ts docs/codebase/README.md
git commit -m "§23.3: the vision as items with stable ids, every change a stored version"
```

---

### Task 2: Proposals, supersession and approval in the core

**Files:**
- Modify: `crates/shadows-core/migrations/0022_vision_items.sql`: append the proposal table. The file has not shipped, so this is not editing a released migration.
- Create: `crates/shadows-core/src/design/proposals.rs` (types + service methods)
- Create: `crates/shadows-core/src/design/store/proposals.rs`
- Modify: `crates/shadows-core/src/threads/model.rs` (`ThreadEntryKind::VisionProposal`)
- Modify: `crates/shadows-core/src/design/mod.rs`, `design/store/mod.rs`, `lib.rs`
- Modify: `crates/shadows-core/src/design/contract.yaml`, `crates/shadows-core/src/threads/contract.yaml`
- Test: `crates/shadows-core/tests/design_vision/` — add `proposals.rs` as a submodule of the existing `design_vision` test binary, the way `standards.rs` is. This is a module, not a new test binary.

Ruling 8: the proposal table goes in `0022` with the items, since neither is on `main` yet. One migration per PR keeps the upgrade a single step. Ledger it.

**Interfaces:**
- Consumes: `apply`, `next_id`, `all_versions`, `insert_version` (Task 1); `writer(grant)` from `agreement_scope.rs` (make it `pub(super)`, already); `self.handles.running_for(thread)`.
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "snake_case")]
  pub enum VisionAction { Add, Edit, Remove }
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub enum ProposalStatus { Pending, Approved, Superseded }
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct VisionProposal {
      pub id: String,
      pub thread_id: ThreadId,
      pub action: VisionAction,
      pub field: VisionField,
      /// `None` for an add until approved; then the id it was given.
      pub item_id: Option<String>,
      /// The new text; `None` for a removal.
      pub text: Option<String>,
      /// The item's text when proposed; `None` for an add.
      pub old_text: Option<String>,
      pub status: ProposalStatus,
      /// Computed: an edit/remove whose item changed or went since (§23.5).
      pub outdated: bool,
      pub created_at: String,
  }
  pub struct VisionPropose {
      pub action: VisionAction, pub field: Option<VisionField>, pub item_id: Option<String>,
      pub text: Option<String>, pub replaces: Option<String>,
  }
  impl Design {
      pub async fn propose_vision(&self, grant: &Grant, args: VisionPropose)
          -> Result<VisionProposal, CoreError>
      pub async fn vision_proposals(&self, project: &ProjectId)
          -> Result<Vec<VisionProposal>, CoreError>
      pub async fn approve_vision_proposal(&self, command_id: String, project: &ProjectId,
          proposal: &str) -> Result<DesignChange, CoreError>
  }
  ```

- [ ] **Step 1: Write the failing tests** in `tests/design_vision/proposals.rs`.
  - **Setup:**
    - Reuse the fixture the `planner_mcp`/`plan_in_conversation` tests use to get a thread grant with a running turn, if one is reachable from `shadows-core` tests through `shadows_core::testing`. Find it with LSP `workspaceSymbol` "running_for" or the test files.
    - If none is reachable, set the turn running by writing the handle directly through `testing::LiveHandles` the way `plans`' show tests do.
    - Record the choice as a ruling.
  - **The tests:**

```rust
#[tokio::test]
async fn a_proposal_is_a_pending_card_in_its_conversation() {
    // propose add Goals "Ship in a week" through the thread grant.
    // assert: returned status Pending, item_id None, outdated false;
    // the thread's last entry is kind VisionProposal, body "Add goal: Ship in a week",
    // card == serde_json::to_value(&proposal) minus status/outdated (the card holds
    // id, action, field, item_id, text, old_text);
    // vision unchanged (revision 0).
}

#[tokio::test]
async fn two_adds_approve_in_any_order_with_consecutive_ids() {
    // propose add Goals "A", then add Goals "B"; approve B first, then A.
    // assert goals == [G1 "B", G2 "A"], each approved_from == Some(thread);
    // both proposals Approved with item_id G1 / G2; revision 2.
}

#[tokio::test]
async fn an_edit_proposal_is_outdated_after_a_hand_edit_of_its_item() {
    // hand-add G1 "Old" (edit). propose edit G1 -> "New" (old_text "Old").
    // hand-edit G1 -> "Mine". vision_proposals: outdated true.
    // approve -> Refused INVALID_COMMAND containing "G1 changed after this proposal".
    // vision G1 still "Mine"; proposal still Pending.
}

#[tokio::test]
async fn approving_one_card_does_not_outdate_its_neighbours() {
    // hand-add G1 "x". propose add Goals "y" and edit G1 -> "z".
    // approve the add; the edit is not outdated and approves.
}

#[tokio::test]
async fn a_newer_proposal_supersedes_by_item_or_by_replaces() {
    // hand-add G1. propose edit G1 "a" (p1), then edit G1 "b" (p2): p1 Superseded.
    // propose add Users "u" (p3), then add Users "v" replaces p3 (p4): p3 Superseded.
    // approving a Superseded proposal -> Refused INVALID_COMMAND "superseded".
}

#[tokio::test]
async fn approving_twice_with_one_command_writes_once() {
    // propose add; approve with "ok" twice -> same DesignChange; one version row
    // for it; ProjectDesignChanged count +1 only.
}

#[tokio::test]
async fn a_proposal_names_a_known_item_and_carries_text() {
    // edit of G9 (unknown) -> INVALID_COMMAND; add with no text -> INVALID_COMMAND;
    // remove with text -> INVALID_COMMAND "a removal has no text".
    // a project grant (not thread) -> GRANT_SCOPE; no running turn -> INVALID_COMMAND
    // "no turn is running for this conversation".
}
```

Write each body in full in the test file. The comments above are the exact assertions.

- [ ] **Step 2: Run them to see them fail**

Run: `… cargo test -p shadows-core --test design_vision proposals`
Expected: compile errors, unknown `propose_vision`.

- [ ] **Step 3: Append to `0022_vision_items.sql`**

```sql
CREATE TABLE design_vision_proposal (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id     TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id  TEXT NOT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id      TEXT NOT NULL,
    action        TEXT NOT NULL CHECK (action IN ('add','edit','remove')),
    field         TEXT NOT NULL,
    item_id       TEXT NULL,
    text          TEXT NULL,
    old_text      TEXT NULL,
    status        TEXT NOT NULL CHECK (status IN ('Pending','Approved','Superseded')),
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    created_at    TEXT NOT NULL,
    seq           INTEGER NOT NULL,
    UNIQUE (project_id, seq)
);
```

`seq` is the project's proposal order, MAX+1 inside the write, so listings never rely on insertion order (CLAUDE.md "Ordering is explicit"). A proposal on a project with no `design_workspace` row creates that row first, as `edit_design` does. Use the planning-thread table name that `thread_entry` references; check it in `0001`.

- [ ] **Step 4: Write `ThreadEntryKind::VisionProposal`.** Add it to `as_str` and `parse`, with the doc "a vision proposal's card (§23.5); its `card` is the proposal as proposed".

- [ ] **Step 5: Write the store.** It holds `propose_vision_in` and `approve_vision_proposal_in`, each one `write_txn`, plus `list_vision_proposals`.
  - **Propose** (the command is classified on Thread scope, like `show_plan`):
    1. `check_writer`.
    2. Validate the action's required fields, and for edit/remove that the item exists; read `old_text` from it.
    3. For edit/remove, set every Pending proposal of the same `item_id` to Superseded. With `replaces`, set that proposal (same project, Pending) to Superseded.
    4. Append the `VisionProposal` entry under `turn`:
       - author `writer.actor()`;
       - body `"{Add|Edit|Remove} {field label}: {text or item id}"`;
       - `card: Some(&json)`.
    5. Insert the row with `seq`, the current base standards version and the current additions id.
    6. Emit the event `VisionProposed { project_id, proposal_id }`, with project and thread, then `record_command`.
  - **Approve** (the command is classified on Project scope; a replay returns the recorded `DesignChange` through `design_command_result`, as `edit_design` does):
    1. Load the proposal; it must be Pending, or the refusal is `"this proposal was superseded"` or `"this proposal was already approved"`.
    2. Run the outdated check, which is the same function `list` uses.
    3. Build the op, with `id_for_add = next_id(…)`, and `apply` it with `from = Some(thread)`.
    4. Bump `revision` and `vision_revision`, then call `insert_version(…, Some(proposal_id))`.
    5. Set the proposal to Approved with the given `item_id`.
    6. Write the `design_command_result`, then emit `ProjectDesignChanged` with `vision_changed: true` and `record_command`.
  - **Outdated:**
    ```rust
    fn outdated(p: &Row, current: &VisionContent) -> bool {
        p.status == "Pending" && p.action != "add"
            && current.item(p.item_id.as_deref().unwrap_or_default())
                .map(|i| Some(&i.text) != p.old_text.as_ref())
                .unwrap_or(true)
    }
    ```

- [ ] **Step 6: Write the service** (`design/proposals.rs`).
  - **`propose_vision`:**
    - Derive the writer from the grant; a `Writer::External` is refused `GRANT_SCOPE` "vision_propose needs a Planner's conversation".
    - The running turn comes from `self.handles.running_for(thread)`; none is refused `INVALID_COMMAND`.
    - Build the `CommandContext` the way `Plans::show` does: `derived_id(Anchor::Operation(&op), &fp)` with fingerprint params `{action, field, item_id, text, replaces}`. So a retried tool call in the same turn is a replay.
  - **`approve_vision_proposal`:** built with `user_command(command_id, "VisionProposalApprove", json!({ "project": project, "proposal": proposal }))`.
  - **`vision_proposals`:** returns all of them, ordered by `seq`.

- [ ] **Step 7: Run the tests**

Run:
1. `… cargo test -p shadows-core --test design_vision`
2. `… cargo test -p shadows-core --test storage_contract` (add the table to the inventory)
3. `… cargo test -p shadows-core --test contracts`

Expected: all pass.

- [ ] **Step 8: Contracts and commit**
  - **`design/contract.yaml`:**
    - shapes and the three methods;
    - obligations, each with its test from Step 1:
      - "a vision proposal is a pending card and changes nothing";
      - "approval applies one item op, with ids in approval order";
      - "staleness is per item; neighbours stay valid";
      - "supersession by item or by `replaces`";
      - "approval replays";
    - `not_the_caller's`: "compute outdated yourself; read `vision_proposals`".
  - **`threads/contract.yaml`:** the new kind.

```bash
git add crates docs/codebase/README.md
git commit -m "§23.5: vision proposals as cards, approved one item at a time"
```

---

### Task 3: The Planner proposes — MCP tool, workspace read, prompt

**Files:**
- Modify: `crates/shadows-mcp/src/workspace_tools.rs` (`vision_propose`)
- Modify: `crates/shadows-mcp/src/server.rs` (`THREAD_TOOLS` gains `vision_propose`; size 12 → 13)
- Modify: `crates/shadows-core/src/harness/prompt.txt`
- Modify: `crates/shadows-core/src/design/workspace.rs` (doc only: items now carry ids)
- Test: `crates/shadows/tests/workspace_mcp.rs`, `crates/shadows/tests/mcp_tools.rs` (the tool list), `crates/shadows/tests/planner_mcp.rs` (prompt version expectations, if any)

**Interfaces:**
- Consumes: `Design::propose_vision(grant, VisionPropose)` (Task 2).
- Produces: the MCP tool `vision_propose` with arguments `{ action, field?, item_id?, text?, replaces? }`, answering the `VisionProposal` JSON.

- [ ] **Step 1: Write the failing tests.**
  - **In `workspace_mcp.rs`:** a thread grant with a running turn calls `vision_propose` `{ "action": "add", "field": "goals", "text": "Ship" }`. The answer has `status: "Pending"`, and a project grant's tool list has no `vision_propose`.
  - **In `mcp_tools.rs`:** update the expected thread tool list.

- [ ] **Step 2: Run to see them fail**

Run: `… cargo test -p shadows --test workspace_mcp`
Expected: FAIL, unknown tool.

- [ ] **Step 3: Implement the tool** in `workspace_tools.rs`:

```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct VisionProposeArgs {
    /// "add", "edit" or "remove".
    action: VisionAction,
    /// For an add: "purpose", "users", "goals", "boundaries" or "technical_direction".
    field: Option<VisionField>,
    /// For an edit or a removal: the item's id, such as "G2".
    item_id: Option<String>,
    /// The item's text, for an add or an edit.
    text: Option<String>,
    /// A pending proposal this one replaces, when the person asked for a change.
    replaces: Option<String>,
}
```
- **Description:** "Propose one change to the vision, which the person approves with the card's button: add an item to a field, or edit or remove an item by its id. One call is one card; several calls in one reply are several cards. Nothing changes until the person approves."
- **Body:** it calls `self.core.design().propose_vision(&grant, …)` and goes through `answer(…map_err(Refusal::from))`.
- **Schema derive:** if `VisionAction`/`VisionField` do not derive `schemars::JsonSchema`, take them as `String` and parse, refusing `INVALID_COMMAND`. Follow what `PlanShowArgs` does for `place`.

- [ ] **Step 4: Update the prompt.**
  - **Tool list:** add a `vision_propose` line under "Shadows' tools".
  - **New paragraph** after "Standards and stage":
    ```text
    The vision
    - The vision is five lists of items with ids: purpose PU1…, users U1…, goals G1…, boundaries B1…, technical direction T1…. Read them with workspace_get.
    - At "vision", walk the missing fields one at a time. When the person agrees on an item, propose it with vision_propose: one call per item, several in one reply if you agreed on several. Then wait: nothing is written until the person approves the card.
    - When the person asks to change a pending card, propose again with replaces set to that proposal's id. To change or remove an approved item, propose an edit or a removal by its id.
    ```

- [ ] **Step 5: Run the tests**

Run:
1. `… cargo test -p shadows --test workspace_mcp`
2. `… cargo test -p shadows --test mcp_tools`
3. `… cargo test -p shadows --test mcp_server`
4. `… cargo test -p shadows --test planner_mcp`

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates
git commit -m "§23.5: the Planner proposes vision items through vision_propose"
```

---

### Task 4: HTTP — list and approve

**Files:**
- Modify: `crates/shadows-http/src/design.rs`, `crates/shadows-http/src/lib.rs`
- Modify: generated `api/openapi.json`, `web/src/api/schema.d.ts`
- Test: `crates/shadows/tests/design_routes.rs`

**Interfaces:**
- Produces:
  - `GET /api/projects/{id}/vision-proposals` → `Vec<VisionProposal>`;
  - `POST /api/projects/{id}/vision-proposals/{proposal_id}/approve`, with body `{ command_id }` → `DesignChange`.
  - Refusals:
    - 400 `INVALID_COMMAND` for outdated, superseded or already approved;
    - 404 for an unknown project or proposal;
    - 409 `COMMAND_CONFLICT`.

- [ ] **Step 1: Write the failing route test** `vision_proposal_routes_list_and_approve`:
  1. Create a proposal through the core service in the test, as Task 2's tests do.
  2. GET the list: one entry, Pending.
  3. POST approve: 200 with `{ revision }`.
  4. GET the list: Approved, with `item_id` "G1".
  5. POST again under a new command id: 400 `INVALID_COMMAND`.
  6. Check the journal: the events include `VisionProposed` and `ProjectDesignChanged`.

- [ ] **Step 2: Run, see it fail**

Run: `… cargo test -p shadows --test design_routes vision_proposal`
Expected: 404 on the new path.

- [ ] **Step 3: Write the two handlers.** Model them on `design::edit` and the standards routes; each calls one service method.

- [ ] **Step 4: Run, regenerate**

Run:
1. `… cargo test -p shadows --test design_routes`
2. `… cargo test -p shadows --test openapi`
3. `cd web && npm run gen:api`

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates api web/src/api/schema.d.ts
git commit -m "§23.5: routes to list vision proposals and approve one"
```

---

### Task 5: Web — the Vision page as items

**Files:**
- Modify: `web/src/api/design.ts` (types `VisionItem`, `VisionField`, `VisionProposal`; `listVisionProposals`, `approveVisionProposal`)
- Modify: `web/src/api/queries.ts` (`visionProposalsQuery(projectId)` with key `['projects', projectId, 'design', 'vision-proposals']`)
- Modify: `web/src/app/design/vision-editor.tsx` (rewrite: items per field)
- Modify: `web/src/app/design/vision-editor.test.tsx`
- Modify: `web/src/stream/use-project-events.ts` (+ test): `VisionProposed` invalidates `['projects', id, 'design']`
- Modify: `web/src/test/fake-daemon.ts`

**Interfaces:**
- Consumes: the routes of Task 4 and the item ops of Task 1.
- Produces:
  - `visionProposalsQuery`;
  - `fieldLabel(field)`: "Purpose", "Users", "Goals", "Boundaries", "Technical direction";
  - a shared `ItemOrigin` that renders "from <thread title link>" or "added by hand".

- [ ] **Step 1: Write the failing tests** in `vision-editor.test.tsx`, replacing the textarea-form tests.
  - Each field lists its items with ids, such as `G1 · Sell books`.
  - An item approved from a conversation links to `/projects/p1/threads/t1`; a hand item says "added by hand".
  - **Add:**
    1. Type in the field's input.
    2. Press Add.
    3. Check: it POSTs `ops: [{ kind: 'VisionItemAdd', field: 'goals', text }]` with `expected_revision`.
  - **Edit:**
    1. Open the editor on G1.
    2. Change the text.
    3. Check: Save sends `VisionItemPut`.
  - **Remove:** Remove sends `VisionItemRemove`.
  - **Conflict:** a `REVISION_CONFLICT` keeps the typed text and offers Reload. Mirror the existing conflict test.
  - **Pending banner:** with two pending proposals in thread `t1` and one in `t2`, the page shows "3 proposals await your approval", with links to both conversations, each with its count.
  - **`use-project-events.test.tsx`:** a `VisionProposed` event invalidates the design key.

- [ ] **Step 2: Run, see them fail**

Run: `cd web && npx vitest run src/app/design/vision-editor.test.tsx src/stream/use-project-events.test.tsx`
Expected: FAIL.

- [ ] **Step 3: Implement.**
  - **Header comment:** `// One job: the vision as five lists of items (§23.3): read, edited by hand one item at a time, with what awaits approval.`
  - **One item edit at a time:**
    - an edit in progress keeps its text across refetches;
    - a conflict shows the existing "changed elsewhere" line with Reload;
    - each save uses `attemptFor` for its command id.
  - **Text:** `dir="auto"` on all item text.
  - **Thread titles for links:** read them from the existing project threads query. Find it in `queries.ts`; a missing title shows "a conversation".

- [ ] **Step 4: Run, pass; type-check**

Run:
1. The Step 2 command;
2. `npx tsc -b --noEmit`;
3. `npm run lint`.

Expected: pass. Other type errors left from the schema change are fixed in this task. `grep -rn "VisionPut\|VisionContent" web/src` must show no stale use.

- [ ] **Step 5: Commit**

```bash
git add web
git commit -m "§23.3 web: the Vision page lists items, edits them by hand, names pending proposals"
```

---

### Task 6: Web — the proposal card in the conversation

**Files:**
- Create: `web/src/app/conversation/vision-proposal-card.tsx`
- Create: `web/src/app/conversation/vision-proposal-card.test.tsx`
- Modify: `web/src/app/conversation/entry.tsx` (branch for `VisionProposal`)
- Modify: `web/src/test/contract-fixtures.ts` (`visionProposalEntry`)

**Interfaces:**
- Consumes: `visionProposalsQuery`, `approveVisionProposal`, `fieldLabel` (Task 5).

- [ ] **Step 1: Write the failing tests** in `vision-proposal-card.test.tsx`.
  - **Pending add:** shows "Add goal" and the text, with an **Approve** button.
    1. Press Approve.
    2. Check: it POSTs to `/approve` with a command id.
    3. After the list refetches, the card shows **Approved · G1** and no button.
  - **Pending edit:** shows the old text struck through beside the new text, with the item id.
  - **Pending remove:** shows the item's text and "Remove".
  - **Superseded:** shows **Superseded** and no button.
  - **Outdated:** shows **Outdated — ask the Planner to propose again** and no button.
  - **Refused approval:** the daemon's message shows through `ErrorLine`.
  - **Copy:** the message actions copy the body line, like other entries. Follow `subagent-card.test.tsx`.

- [ ] **Step 2: Run, see them fail**

Run: `cd web && npx vitest run src/app/conversation/vision-proposal-card.test.tsx`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement.**
  - **Data:** the card reads its proposal from `entry.card`, which holds the proposal as proposed, and its live status from `visionProposalsQuery(projectId)`, found by id. While that query loads, it shows the card without a button.
  - **Header comment:** `// One job: a vision proposal's card (§23.5): what the Planner proposed, its live status, and the person's Approve.`
  - **Entry:** in `entry.tsx`, `if (entry.kind === 'VisionProposal') return <VisionProposalCard entry={entry} projectId={…} />`. Use the project id the conversation already passes to its entries; add the prop if it does not.

- [ ] **Step 4: Run, pass**

Run:
1. `npx vitest run src/app/conversation`
2. `npx tsc -b --noEmit`
3. `npm run lint`

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add web
git commit -m "§23.5 web: vision proposal cards with Approve, Superseded and Outdated"
```

---

### Task 7: Status, browser run, gate

**Files:**
- Modify: `docs/status.md` (§23 PR 2 line)
- Create: `docs/evidence/2026-10-12-guided-planning-pr2.md` (what ran, as PR 1's evidence did)

- [ ] **Step 1: Browser run on a copy of the development database**
  1. Copy the database with SQLite's online backup, as in PR 1's evidence, to an ignored trial path.
  2. Point `.claude/launch.json` at the copy and start the preview.
  3. Check the migration: the existing projects' vision text shows as one item per field.
  4. In a new project, ask the Planner how to start, then agree on three goals.
     - Check: three cards appear.
  5. Approve the second card, then the first.
     - Check: the ids are G1 and G2, in approval order.
  6. Ask the Planner to reword the third.
     - Check: the third card shows Superseded and a new card appears.
  7. Approve the new card.
     - Check: the Vision page lists G1–G3, each linking to the conversation.
  8. Leave a fourth proposal pending.
     - Check: the Vision page names it, and the header stage moves as fields fill.
  9. Edit G1 by hand while an edit proposal for G1 is pending.
     - Check: its card shows Outdated.
  10. Read the daemon logs for errors.
  11. Restore `.claude/launch.json` and delete the trial copy.

  Record it in the evidence file, including any step not observed. If the Claude limit is exhausted, use the MiniMax wrapper approach from PR 1's evidence only with Mohammed's say-so.

- [ ] **Step 2: Full gate, once**

Run, from the root, with the cargo prefix:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
3. `cargo test --workspace`
4. `cargo clippy --workspace -- -D warnings`
5. `cargo tree -e features,no-dev --workspace | grep test-support`, which prints nothing
6. `git diff --exit-code api/`
7. In `web/`: `npm test`, `npm run lint`, `npm run build`
8. Check: no Rust line over 100 columns.

Expected: everything passes, except step 5, whose exit 1 is the expected empty result.

- [ ] **Step 3: Commit**

```bash
git add docs
git commit -m "§23 PR 2: vision proposals built and run"
```
