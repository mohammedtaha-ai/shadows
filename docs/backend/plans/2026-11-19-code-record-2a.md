# The code record, slice 2a: tables and screens, their state, and `project_state`

Design: [`../specs/2026-11-19-project-code-record-design.md`](../specs/2026-11-19-project-code-record-design.md)
§5.1 to §5.4, §8, §9 (2a); the elements are
[§18.13](../../core/specs/2026-10-03-project-planning-workspace-design.md#1813-tables-and-screens-design-elements-mohammed-2026-11-19);
the pins are [§17 "The task contract"](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#the-task-contract).
Shared request: [`../../shared/requests/2026-11-19-project-state-and-diagrams.md`](../../shared/requests/2026-11-19-project-state-and-diagrams.md).
Branch: `ccr-9e1b02d8-b0ylm1`; plan 2b ([`2026-11-19-code-record-2b.md`](2026-11-19-code-record-2b.md),
`code_where`) follows on the same branch. Dispatch: the controller runs the builders and fixers
(Mohammed, 2026-11-19); the backend lead prepares the tasks and checks the reports.

## Rulings for every task

From Mohammed, 2026-11-19, unless marked otherwise.

1. **Design owns tables and screens** as project elements (§18.13): id `E{n}`, revisions, a part of
   the approved map, at least one roadmap outcome, a table only in a part of kind `database` and a
   screen only in a part of kind `frontend` (the standards' kinds).
2. **A contract pins, never describes:** `ElementPin { id, revision, action, file, symbol_id? }`.
   Plans owns the pin rules. Ruling 1 named the reference `{element_id, action, file, symbol?}`;
   this plan adds `revision` (the agreements and decisions pattern pins exact revisions, and the
   state rule in design §5.2 orders by it). The field is named `id`, as `DecisionPin.id` is.
3. **The Planner writes elements directly** in its running turn (MCP), with no card and no
   approval; the person sees them through the pins in the plan they approve.
4. **Read only in this plan.** No route writes an element. A later hand edit writes a new revision
   of the same element.
5. **States** (design §5.2): `planned`, `built`, `gap`, `unchecked`, `removed`, with `behind`,
   `confirmed`, `done` and `planned` pins. Revisions order everything; never time or rowid.
6. **Draft plans add nothing.** A plan's live version is its highest Frozen version, and only if
   the plan is Active.
7. **`ProjectState` is a new service** (crate `project-state`). It composes and holds no table.
   CodeRecord cannot own `project_state`: task progress is Runs', and Runs depends on CodeRecord.
   Mohammed approved it on 2026-11-19, over the earlier "CodeRecord owns project state".
8. **The frontend sees nothing before the shared request is accepted.** A task that changes
   `api/openapi.json` (a route, or a type a route serializes, such as `TaskContract` and
   `TaskContext`) or the project event stream's kinds is blocked until it is.
9. **Bounds** (design §5.3): 500 live elements per project; 200 columns per table; 50 links per
   screen; 200 parts, 20 plans per part, 100 gaps in `project_state`; 50 plans in a part's detail.
10. **Old plans unchanged:** `TaskContract.elements` and `TaskContext.elements` are
    `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so a contract or a saved brief
    without them reads, serializes and fingerprints exactly as before. Commands keep
    `command_schema_ver: 1`.
11. **Every change a state read shows reaches the project stream** (2026-11-20, from Masada's
    review; Mohammed and the controller accepted it). The changes and their events:

    | Change | Event | Journaled by |
    |---|---|---|
    | an element | `DesignElementChanged` | Design (B-175) |
    | a done task's record | `CodeRecordChanged` | CodeRecord (B-178) |
    | a task's progress: an attempt inserted or its state changed, a done or not-done mark | `TaskProgressChanged` | Runs (B-187) |

    Each is journaled in the write that makes the change. B-180 puts all three on the project tail.
    No existing attempt event fits: Runs journals none today (only `TaskExecutorChosen`), and an
    executor thread's operation events neither carry the attempt's state (Runs decides it after the
    turn) nor belong to the project tail.

## What is reused

| Existing piece | Where | How 2a uses it |
|---|---|---|
| Project decisions: `E{n}`-style ids per project, revisions, the Planner's write through `proposal_writer` and a running turn, replay through `CommandContext::classify` and `record_command`, an identifier-only event in the write | `crates/core-design/src/decisions.rs`, `src/store/decisions.rs`, migration `0071_design_decisions.sql` | The element write path copies it step for step: `ElementPut` and `ElementDelete` commands, `next_id`, `latest`, `recorded`, `DesignElementChanged` |
| `DecisionPin` in `TaskContract`, checked by `pins` in the edit's write, resolved into `TaskContext.decisions` | `crates/core-plans/src/task_contract.rs`, `src/store/edit.rs` (`pins`), `src/store/task_context.rs` | `ElementPin` rides the same field rules (`serde(default, skip_serializing_if)`, `schema(required = false)`); `pins` checks elements too; `TaskContext.elements` is filled beside `decisions` |
| `AgreementBinding` and `AgreementContent` | `crates/core-plans/src/bindings.rs`, `crates/core-design/src/agreement.rs` | The pattern (an exact pinned version) only. Not reused as code: a binding is a plan-level edit (`BindingPut`) with a role, operations and a part, checked against an Agreed state; an element has no role, no operations and no approval, and its pin lives in the task contract. `AgreementContent` is an OpenAPI document; a table has columns |
| `ContractRules::problems` (the pure per-task rules) | `crates/core-plans/src/task_contract.rs` | The pin's pure rules join it, with its `"T{n} contract: ..."` messages |
| `Design::layout_in` and `ProjectLayout::owners` | `crates/core-design/src/layout.rs` | A pin's file must be owned by the element's part |
| `Design::require_roadmap_in`; the store's `part_in`; the `design_outcome` table | `crates/core-design/src/store/` | An element is written only at the Plans stage, in a part and for outcomes of its project |
| Standards' part kinds `database` and `frontend` | `crates/core-standards/src/standards.yaml` | The kind a table's or a screen's part must have |
| `Design::seed_roadmap_for_tests` | `crates/core-design/src/store/roadmap_seed.rs` | `seed_elements_for_tests` builds on it |
| `Design::stage`, `Design::layout` | `crates/core-design` | The stage and the parts in map order in `project_state` |
| `Plans::list` / its store query `list_plans` (Active plans, in `created_at` then id order); `Plans::plan_parts_in` | `crates/core-plans` | `live_versions` reads the same plans in the same order, and each plan's parts |
| `Plans::design_part` | `crates/core-plans` | Not called by `ProjectState` (one part per call, no tasks); an agreement test holds that the plans `project_state` lists under a part are `design_part`'s Active plans |
| `Runs::progress` (PR #72) | `crates/core-runs` | Each live version's task statuses: done, started (an attempt and not done), not started |
| `CodeRecord::record_done_in` / `forget_done_in`, `DoneRecord`, the `code_record_symbol` states, `delete_project_in` | `crates/code-record/src/store/mod.rs` | The done task's pins are written and removed in the same marking write; a pin's symbol state is read from the symbol rows by join; project removal removes the pin rows |
| `Codebase::missing_inputs` | `crates/codebase` | Whether each built pin's file is in the folder now, one call for all |
| The project event tail's kind list | `crates/core-events/src/store.rs` (`read_project_events_after`) | Two new kinds |
| MCP tool translation (`decision_tools.rs`), `Grants::tools_for`, the refusal mapping | `crates/shadows-mcp/src/`, `crates/core-grants` | Three Design tools and `project_state` |
| HTTP decisions routes | `crates/shadows-http/src/design.rs` | The elements routes beside them |
| `runs_attempts` fixtures and `CodeRecord::record_answer_for_tests` | `crates/shadows-core/tests/runs_attempts.rs` | `ProjectState`'s tests mark tasks done with recorded answers |
| React Flow (`@xyflow/react`) canvases | `web/src/app/design/map-canvas.tsx`, `web/src/app/workflows/` | Masada's diagrams (the shared request); not touched here |

**New, and why nothing existing fits:**
- **Table `design_elements`:** no Design table holds columns or links. Decisions are text, agreements
  are OpenAPI documents, and a part's `design` is free text.
- **Table `code_record_element`:** the record's tables hold files and symbols. A pin is neither, and
  the state rule needs its revision and action.
- **`Plans::live_versions`:** nothing answers every Active plan with its parts and its live version's
  pinned tasks in one snapshot. `list` has no parts or tasks, and `design_part` covers one part and
  no tasks.
- **Service `ProjectState`:** nothing above Runs composes reads (Ruling 7).

## Gate files every task here touches

These are the ones B-170 found:
- **A new migration** updates:
  - `crates/core-db/tests/fixtures/base_migrations.sha384` (its `sha384sum` line, as the last line);
  - the count in `crates/core-db/tests/migration_checksums.rs` (`assert_eq!(expected.len(), N)`);
  - the table list in `crates/core-db/tests/storage_contract/schema.rs`;
  - `crates/shadows/tests/architecture/table-owners.txt` (`<table> :: <owner>`).
- **A crate that comes to depend on another service crate** is added to that crate's allowed
  dependents in `crates/shadows/tests/architecture/main.rs`.
- **Test modules** name their imports (the ratchet refuses `use super::*`).
- **A new module** gets its row in `docs/codebase/README.md` (the codemap test).

Every task's Done when includes:
- `cargo test -p shadows --test architecture --test codemap`;
- `cargo test -p shadows-core --features test-support --test contracts`;
- `cargo test -p core-db --test migration_checksums` (when it adds a migration);
- `cargo fmt --all --check`;
- both clippy commands of `docs/backend/AGENTS.md` rule 5;
- the 100-character line check there.

The LSP was not available while this plan was written. Struct literals that gain a field are found
by the compiler: every `TaskContract { .. }` and `TaskContext { .. }` literal it names gains
`elements: Vec::new()`. Those files are in Touch by that rule and no other change is made in them.

---

## Task B-175 · Design owns tables and screens

**Files:**
- **Create:**
  - `crates/core-db/migrations/0077_design_elements.sql`;
  - `crates/core-design/src/elements.rs`;
  - `crates/core-design/src/store/elements.rs`;
  - `crates/shadows-core/tests/design_elements.rs`.
- **Modify:**
  - `crates/core-design/src/model.rs`, `src/error.rs`, `src/lib.rs`, `src/store/mod.rs`,
    `src/store/roadmap_seed.rs` (the seed below), `contract.yaml`;
  - `crates/shadows-core/src/error.rs` and `src/lib.rs` (re-exports);
  - `crates/shadows-mcp/src/refusal.rs` (the two new codes, through the existing `DesignError`
    mapping);
  - `docs/codebase/README.md`;
  - the gate files.
- **Not:** Plans, Runs, CodeRecord, any route or MCP tool.

**Migration `0077_design_elements.sql`:**

```sql
-- A project's tables and screens (design elements). Each write is a new revision; nothing is
-- overwritten. A deleted element's last revision has deleted = 1 and takes no further revision.
CREATE TABLE design_elements (
    project_id TEXT NOT NULL,
    element_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    kind TEXT NOT NULL CHECK (kind IN ('table', 'screen')),
    name TEXT NOT NULL,
    part_id TEXT NOT NULL,
    outcomes_json TEXT NOT NULL CHECK (json_valid(outcomes_json)),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    written_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id, element_id, revision)
);
```

Table owner line: `design_elements :: design`.

**Types** (`model.rs`). Every type derives `Debug, Clone, PartialEq, Eq, serde::Serialize,
serde::Deserialize, utoipa::ToSchema, schemars::JsonSchema`, except `ElementSeed`, which derives
`Debug, Clone`:

```rust
#[serde(rename_all = "snake_case")]
pub enum ElementKind { Table, Screen }

/// A table's or a screen's content at one revision.
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ElementContent { Table(TableContent), Screen(ScreenContent) }

#[serde(deny_unknown_fields)]
pub struct TableContent { pub name: String, pub purpose: String, pub columns: Vec<TableColumn> }

#[serde(deny_unknown_fields)]
pub struct TableColumn {
    pub name: String,
    /// As the project's language or database writes it: `TEXT`, `uuid`, `int`.
    pub type_name: String,
    #[serde(default)]
    pub meaning: String,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<ColumnRef>,
}

#[serde(deny_unknown_fields)]
pub struct ColumnRef { /** An element id: `E3`. */ pub table: String, pub column: String }

#[serde(deny_unknown_fields)]
pub struct ScreenContent {
    pub name: String,
    pub purpose: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default)]
    pub leads_to: Vec<ScreenLink>,
}

#[serde(deny_unknown_fields)]
pub struct ScreenLink { /** An element id. */ pub screen: String, pub when: String }

/// One revision of a table or a screen as Design holds it.
pub struct DesignElement {
    pub id: String,
    pub revision: i64,
    pub project_id: core_projects::ProjectId,
    pub part_id: PartId,
    pub outcomes: Vec<OutcomeId>,
    pub content: ElementContent,
    pub deleted: bool,
    /// The Planner's thread id.
    pub written_by: String,
    pub created_at: String,
}

/// The Planner's write: a new element (no id), or the next revision of `id` when
/// `expected_revision` is its latest.
#[serde(deny_unknown_fields)]
pub struct ElementInput {
    #[serde(default)] pub id: Option<String>,
    #[serde(default)] pub expected_revision: Option<i64>,
    pub part_id: PartId,
    pub outcomes: Vec<OutcomeId>,
    pub content: ElementContent,
}

#[serde(deny_unknown_fields)]
pub struct ElementDelete { pub id: String, pub expected_revision: i64 }

/// test-support only: the parts and the outcome `seed_elements_for_tests` made.
pub struct ElementSeed { pub database: PartId, pub frontend: PartId, pub outcome: OutcomeId }
```

`ElementContent::kind(&self) -> ElementKind` and `ElementContent::name(&self) -> &str` are
methods.

**Errors** (`error.rs`): `InvalidElement(String)` (`#[error("{0}")]`, code `INVALID_ELEMENT`) and
`ElementStale(String)` (`#[error("{0}")]`, code `ELEMENT_STALE`). In
`crates/shadows-core/src/error.rs` and the MCP refusal, map them beside `InvalidDecision` and
`DecisionStale`, the same way. The HTTP status mapping (422 and 409) is B-180's.

**Methods** (`elements.rs`, `impl Design`; a private `ElementRules` there holds the pure rules, as
`DecisionRules` does):

```rust
pub async fn put_element_for(&self, grant: &Grant, command_id: String, input: ElementInput)
    -> Result<DesignElement, DesignError>;
pub async fn delete_element_for(&self, grant: &Grant, command_id: String, input: ElementDelete)
    -> Result<DesignElement, DesignError>;
/// Every element of `project` at its latest revision, deleted ones included, by id number.
pub async fn elements(&self, project: &ProjectId) -> Result<Vec<DesignElement>, DesignError>;
/// Every revision of `id`, oldest first; empty when the project has no such element.
pub async fn element_revisions(&self, project: &ProjectId, id: &str)
    -> Result<Vec<DesignElement>, DesignError>;
/// One revision as `conn` sees it; `revision` None is the latest.
pub async fn element_in(conn: &mut sqlx::SqliteConnection, project: &ProjectId, id: &str,
    revision: Option<i64>) -> Result<Option<DesignElement>, DesignError>;
/// test-support only.
pub async fn seed_elements_for_tests(&self, project: &ProjectId)
    -> Result<ElementSeed, DesignError>;
```

**Order of a write** (store `put_element_in` / `delete_element_in`, one `write_txn_as`, as
`propose_decision_in`):
1. `proposal_writer(grant, "design_element_put needs a Planner's conversation", "no turn is
   running for this conversation")` (`design_element_delete` for a delete).
2. `Grants::check_writer`.
3. Replay: `CommandContext::classify(conn, ctx, "Project", project)`; a recorded command answers its
   recorded `"{id}@{revision}"` revision.
4. `Operations::require_running_in`.
5. `Design::require_roadmap_in`.
6. `ElementRules::normalise`.
7. The store rules below.
8. The insert.
9. `JournalEntry::new("DesignElementChanged", writer.actor())`, with project, thread and operation,
   and payload `{ "project_id", "element_id", "revision" }`.
10. `record_command(conn, ctx, "Project", project, "Element", "{id}@{revision}")`.

The command kinds are `ElementPut` and `ElementDelete`. Each fingerprint is
`CommandContext::fingerprint(kind, &serde_json::to_value(&input)?)`, with `command_schema_ver: 1`.

A new id is `E{n}`, where n is one more than the highest number the project has used. The query
reads `MAX(CAST(SUBSTR(element_id, 2) AS INTEGER))` over all its rows, deleted ones included, so a
number is never reused. The list order is the same number.

**Pure rules** (`ElementRules::normalise(input) -> Result<ElementInput, DesignError>`). Texts are
trimmed first. Each refusal is `InvalidElement` with exactly this text:

| Rule | Text |
|---|---|
| name 1..=100 chars | `a {table\|screen}'s name needs 1 to 100 characters, not {n}` |
| purpose 1..=1000 | `a {kind}'s purpose needs 1 to 1000 characters, not {n}` |
| columns 1..=200 | `a table needs 1 to 200 columns, not {n}` |
| column name 1..=100 | `column {i} needs a name of 1 to 100 characters` (i from 1) |
| type 1..=100 | `` the column `{name}` needs a type of 1 to 100 characters `` |
| meaning ≤ 500 | `` the column `{name}`'s meaning is over 500 characters `` |
| unique column, ignoring case | `` the column `{name}` is named twice `` |
| route, when given: starts with `/`, ≤ 200 | `` a route starts with '/' and has up to 200 characters: `{route}` `` |
| links ≤ 50 | `a screen leads to at most 50 screens, not {n}` |
| when 1..=300 | `the link to {screen} needs a when of 1 to 300 characters` |
| unique (screen, when) | `{screen} is linked twice with the same when` |
| outcomes ≥ 1 | `an element serves at least one roadmap outcome` |
| outcomes unique | `outcome {id} is named twice` |
| new with expected_revision | `a new element has no expected_revision` |
| id without expected_revision | `ElementStale`: `{id} needs the expected_revision of its latest revision` |

**Store rules** (inside the write; `InvalidElement` unless marked):

| Rule | Text |
|---|---|
| the id is the project's | `{id} is not an element of this project` |
| expected is the latest | `ElementStale`: `{id} is at revision {latest}, not {expected}` |
| not deleted | `{id} is deleted; write a new element` |
| the kind is unchanged | `{id} is a {kind}; its kind does not change` |
| the part is the project's (store `part_in`) | `part {part} is not a part of this project` |
| a table's part has kind `database` (trimmed, exact) | `a table belongs to a part of kind database; {part} is {kind}` (`{kind}` is `of no kind` when none) |
| a screen's part has kind `frontend` | `a screen belongs to a part of kind frontend; {part} is {kind}` |
| each outcome is a `design_outcome` row of the project | `outcome {id} is not in this project's roadmap` |
| unique live name per kind, ignoring case | `` a {kind} named `{name}` already exists: {other} `` |
| unique live route | `` the route `{route}` is already {other}'s `` |
| foreign key target is a live table of the project, or this element | `` `{column}` references {target}, which is not a table of this project `` |
| the target has the column (this input's columns when self) | `` `{column}` references {target}.{col}, which {target} does not have `` |
| a link target is a live screen of the project, or this element | `{screen} is not a screen of this project` |
| a put keeps every column another live table's latest revision refers to | `{name}.{col} is referenced by {other_name}.{other_col} ({other}); change {other} first` |
| a delete: no other live element refers to it | `{id} is referenced by {other} ({detail}); change {other} first`, where detail is `{table}.{column}` or `leads to it` |
| at most 500 live elements, on a new id | `this project has 500 tables and screens; delete one first` |

"Live" means the element's latest revision has `deleted = 0`.

**The seed** (test-support, `roadmap_seed.rs` beside `seed_roadmap_for_tests`):
`seed_elements_for_tests` runs `seed_roadmap_for_tests`. If they do not exist yet, it inserts two
parts: kind `database` with folder `db`, and kind `frontend` with folder `web`. It also inserts one
outcome tied to both parts. It answers `ElementSeed`, and is idempotent: a second call answers the
same ids.

**Contract** (`crates/core-design/contract.yaml`):
- shapes: the types above;
- `DesignError` values: + `InvalidElement`, `ElementStale`;
- functions under `service`: the six methods, each with its rule in one comment, the write order
  and the bound;
- `shared_in_transaction`:
  `element_in: "Plans checks a contract's element pins in its edit's write and reads them into
  TaskContext. Design owns design_elements."`;
- obligations, each with the tests below;
- modules: `elements.rs: project table and screen entry points`,
  `store/elements.rs: persisting table and screen revisions`.

The same two module rows go in `docs/codebase/README.md`.

**Tests:**

Unit tests in `src/elements.rs`:
- `names_and_texts_are_trimmed_and_bounded`: a name of 101 characters is refused with its text;
  `"  users "` becomes `users`.
- `a_column_name_is_unique_ignoring_case`: `id` and `ID` are refused.
- `a_route_starts_with_a_slash`: `login` is refused and `/login` is kept.

`crates/shadows-core/tests/design_elements.rs`, with a Planner grant and a running turn as
`design_decisions.rs` builds them, and `seed_elements_for_tests`:
- `the_planner_writes_a_table_and_a_new_revision_keeps_its_id`: E1 r1, then a put with
  `expected_revision: 1` gives E1 r2; `element_revisions` answers both, oldest first.
- `a_replayed_put_returns_the_first_revision`: the same command id and input answer r1 again with
  no second row and no second event; the same id with other input is `CommandConflict`.
- `a_stale_revision_is_refused`: `expected_revision: 1` after r2 is `ElementStale`
  `E1 is at revision 2, not 1`.
- `a_table_needs_a_database_part_and_a_screen_a_frontend_part`: both texts.
- `an_element_serves_an_outcome_of_the_project`: an empty list and a foreign outcome are refused.
- `a_second_table_with_the_same_name_is_refused`: `Users` after `users`; a screen named `users`
  is allowed.
- `a_foreign_key_names_a_live_table_and_its_column`: to a screen, to a missing column, and to a
  deleted table are refused; a self-reference is accepted.
- `a_referenced_column_cannot_be_dropped_and_a_referenced_table_cannot_be_deleted`: both texts
  name the referrer.
- `a_screen_leads_only_to_screens_and_its_route_is_unique`.
- `a_deleted_element_takes_no_new_revision`: the delete writes r2 with `deleted: true`; a put on it
  is refused.
- `ids_are_never_reused`: E1 and E2 are written and E2 deleted; the next is E3.
- `only_a_planners_running_turn_writes`: an executor grant is `GrantScope`; a Planner whose turn
  ended is refused as decisions are.
- `the_write_journals_design_element_changed`: the payload holds exactly `project_id`,
  `element_id` and `revision`.
- `the_project_has_at_most_500_elements`: the 501st new element is refused; a revision of an
  existing one is not.

**Done when:**
- `cargo test -p core-design`;
- `cargo test -p shadows-core --features test-support --test design_elements`;
- the gate list.

---

## Task B-176 · The Planner writes tables and screens

**Files:**
- **Create:** `crates/shadows-mcp/src/element_tools.rs`; `crates/shadows/tests/element_mcp.rs`.
- **Modify:**
  - `crates/shadows-mcp/src/server.rs` (combine `element_tool_router()` beside
    `decision_tool_router()`);
  - `crates/shadows-mcp/src/lib.rs` (the module);
  - `crates/shadows-mcp/src/contract.yaml`;
  - `crates/core-grants/src/` (the file holding `tools_for`'s Thread list);
  - `crates/core-grants/contract.yaml` (its tool count comment: 41 becomes 44);
  - `crates/core-harness/src/prompt.txt`;
  - `crates/core-harness/contract.yaml` (a comment naming the prompt lines and their test);
  - `crates/shadows/tests/mcp_server.rs` (the expected tool lists);
  - `crates/shadows/tests/mcp_tools.rs`;
  - `docs/codebase/README.md` (`element_tools.rs: Design elements MCP translation`).
- **Needs:** B-175.

**Tools** (Planner only: the Thread list of `Grants::tools_for`, never the executor's or the
project grant's):
- `design_element_put`:
  - **args:** `{ command_id: String, id: Option<String>, expected_revision: Option<i64>, part_id:
    String, outcomes: Vec<String>, content: ElementContent }`.
  - **calls:** `Design::put_element_for`.
  - **description:** "Write a table or a screen of the project's design, or a new revision of one:
    a table in a database part, a screen in a frontend part, serving at least one roadmap outcome.
    Leave id out for a new element; with id, give the expected_revision of its latest revision.
    Pin the revision a task builds in its contract's elements."
- `design_element_delete`:
  - **args:** `{ command_id, id, expected_revision }`.
  - **calls:** `Design::delete_element_for`.
  - **description:** "Delete a table or a screen from the project's design: a last revision a task
    then builds with action delete. Refused while another element refers to it."
- `design_elements`:
  - **args:** `{ id: Option<String> }`.
  - **calls:** without id, `Design::elements(&grant.project_id)`; with id,
    `Design::element_revisions`.
  - **description:** "List this project's tables and screens at their latest revision, or every
    revision of one by id. Read only."

**Prompt** (`prompt.txt`, the "Shadows' tools" list, after the `decision_propose` line), exactly:

```text
- design_element_put, design_element_delete, design_elements: the project's tables and screens live in Shadows' design, not in a task. Before a task builds a table or a screen, write it with design_element_put: a table in a database part, a screen in a frontend part, each serving the roadmap outcomes it is for. A table has its columns with their types, primary key and the column each foreign key refers to; a screen has its route and the screens it leads to, and when. A change is a new revision of the same id, never a second element with the same name. Read them with design_elements before you write one.
```

**Tests:**

`crates/shadows/tests/element_mcp.rs`, over the MCP server as `workspace_mcp.rs` drives it:
- `the_planner_writes_a_table_and_reads_it_back`: `design_element_put` then `design_elements`
  answers E1 with its columns.
- `a_refused_element_keeps_its_code`: a screen in the database part answers `INVALID_ELEMENT` with
  the rule's text.

`mcp_server::the_tool_list_depends_on_the_grant_kind`: the Planner's list has the three tools; the
executor's and the project grant's do not.

`mcp_tools::the_planner_is_told_to_write_tables_and_screens`: the prompt holds the line above.

**Done when:**
- `cargo test -p shadows --test element_mcp --test mcp_server --test mcp_tools`;
- the gate list.

---

## Task B-177 · A task pins the elements it builds; the executor receives them

**Status at writing:** blocked until the shared request is accepted (`TaskContract` and
`TaskContext` change in `api/openapi.json`).

**Files:**
- **Modify:**
  - `crates/core-plans/src/task_contract.rs`;
  - `crates/core-plans/src/store/edit.rs` (`pins`);
  - `crates/core-plans/src/store/task_context.rs`;
  - `crates/core-plans/src/model.rs` (`TaskContext`);
  - `crates/core-plans/src/lib.rs` (re-export);
  - `crates/core-plans/contract.yaml`;
  - `crates/shadows-core/src/lib.rs` (re-export `ElementPin`, `ElementAction`);
  - `crates/core-harness/src/prompt.txt` ("The task contract" section);
  - `crates/core-harness/src/executor.md`;
  - `crates/core-harness/contract.yaml`;
  - tests: `crates/shadows-core/tests/task_contract.rs`, `crates/shadows-core/tests/task_context.rs`,
    `crates/shadows-core/tests/attempt_briefs.rs`, `crates/shadows/tests/mcp_tools.rs`;
  - `api/openapi.json` (regenerated);
  - every file the compiler names for a `TaskContract { .. }` or `TaskContext { .. }` literal (it
    gains `elements: Vec::new()` and nothing else).
- **Not:** Design's element rules, CodeRecord, Runs.
- **Needs:** B-175; the shared request accepted.

**Types** (`task_contract.rs`):

```rust
/// One revision of a project table or screen that the task builds, changes or deletes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
    schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElementPin {
    /// The element: `E3`.
    pub id: String,
    /// Its exact revision, the latest when the task is written.
    pub revision: i64,
    pub action: ElementAction,
    /// The file that holds it: one of the task's create, modify or delete files.
    pub file: String,
    /// The task's symbol in that file that is it (a class, a component), when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
    utoipa::ToSchema, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ElementAction { Create, Modify, Delete }
```

`TaskContract` gains, after `decisions`:

```rust
/// The project's tables and screens this task builds, changes or deletes, each at one revision.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
#[schema(required = false)]
pub elements: Vec<ElementPin>,
```

`TaskContext` gains `elements: Vec<core_design::DesignElement>` with the same three attributes:
each pinned revision as Design holds it, in pin order.

**Pure rules** (in `ContractRules::problems`, after the decision pins' place, each
`"T{n} contract: ..."`):
- `` the element `{id}` is pinned more than once ``
- `` the element `{id}` needs a revision of 1 or more ``
- `` the element `{id}` names `{file}`, which the task does not create, modify or delete ``
- `` the element `{id}` names the symbol `{s}`, which the task does not describe in `{file}` ``
- `` the element `{id}` is deleted by this task, so it names no symbol `` (action `delete` with
  `symbol_id`)

**Store rules** (`pins` in `store/edit.rs`, for each changed task, beside the decision pins; read
with `Design::element_in`). All `PlanInvalid` problems:
- `T{n} pins {id} r{revision}, which this project's design does not have`
- `T{n} pins {id} r{revision}, but its latest revision is r{latest}; pin r{latest}`
- `T{n} deletes {id}, but r{revision} does not delete it; delete it with design_element_delete first`
- `T{n} builds {id} r{revision}, which deletes it; use action delete`
- `` T{n} pins {id} in `{file}`, which its part {part} does not own ``: none of
  `Design::layout_in(conn, project)?.owners(file)` is the element's part. Read the layout once per
  edit.

Approval does not check pins again: element rows are never changed or removed, so a pin that held
at the edit still names an existing revision. A newer revision shows as `behind` in the state.

**Task context** (`store/task_context.rs`, after `decisions`): for each pin of `task.contract`,
`Design::element_in(tx, &plan.project_id, &pin.id, Some(pin.revision))`; push when `Some`.

**Prompt** (`prompt.txt`, "The task contract", after the `uses` line), exactly:

```text
- elements: every table or screen the task builds, changes or deletes, pinned by id: {"id": "E3", "revision": 2, "action": "create", "file": "db/migrations/0003_users.sql", "symbol_id": "s1"}. Pin the element's latest revision. file is one of the task's create, modify or delete files, owned by the element's part; symbol_id, when the element is a class or a component, is the task's symbol in that file; a delete pin names the revision design_element_delete wrote and no symbol. Never copy columns or links into the contract: Shadows gives the executor the pinned revision.
```

**Executor** (`executor.md`, after its `decisions` line), exactly:

```text
- Your brief's elements are the project's tables and screens this task builds, each at its pinned revision: build its columns, keys, route and links as written there. If you cannot, report it with task_conflict.
```

**Contract** (`crates/core-plans/contract.yaml`):
- shapes: `ElementPin`, `ElementAction`; `TaskContract.elements`;
- `TaskContext`'s note: elements;
- a rule block `element_pins:` beside `decision_pins:`, with the texts above and its tests.

**Tests:**

`crates/shadows-core/tests/task_contract.rs`:
- `a_task_pins_an_elements_latest_revision`: r1 after r2 exists is refused with its text; r2 is
  accepted.
- `a_pin_names_a_file_the_task_works_and_a_symbol_in_it`: the three pure texts.
- `a_pin_file_belongs_to_the_elements_part`: `web/x.sql` for a table in the `db` part is refused.
- `a_delete_pin_needs_a_deleting_revision_and_a_build_pin_a_live_one`.
- `a_contract_without_elements_keeps_its_fingerprint`: the JSON and the fingerprint of a contract
  with no elements equal the ones stored before this change (a fixed literal in the test).
- `a_frozen_pin_keeps_its_revision_after_the_element_changes`: approve; write r2; the task still
  pins r1.

`crates/shadows-core/tests/task_context.rs`:
- `task_context_gives_the_executor_its_pinned_elements`: the context holds E1 r1 with its columns
  after r2 was written.

`crates/shadows-core/tests/attempt_briefs.rs`:
- `a_saved_brief_without_elements_still_reads`: a brief JSON stored without `elements` reads with
  none.

`crates/shadows/tests/mcp_tools.rs`:
- `the_planner_is_told_how_a_contract_pins_elements`: the prompt line.
- `the_executor_is_told_to_build_its_elements_as_pinned`: the executor line.

Then regenerate the document: `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.

**Done when:**
- those tests;
- `cargo test -p core-plans`;
- `cargo test -p shadows --test openapi`;
- `cargo test -p shadows-core --features test-support --test harness_schema`;
- the gate list.

---

## Task B-178 · The record keeps the pinned elements, and CodeRecord judges their state

**Files:**
- **Create:**
  - `crates/core-db/migrations/0078_code_record_element.sql`;
  - `crates/code-record/src/elements.rs`;
  - `crates/code-record/src/store/elements.rs`;
  - `crates/code-record/tests/elements.rs`.
- **Modify:**
  - `crates/code-record/src/lib.rs`, `src/model.rs`, `src/store/mod.rs` (`record_done_in`,
    `forget_done_in`, `delete_project_in` call into `store/elements.rs`);
  - `crates/code-record/Cargo.toml` (+ `core-design`, types only: `DesignElement`);
  - `crates/code-record/contract.yaml`;
  - `crates/code-record/tests/record.rs` (one test, and the new `forget_done_in` argument);
  - `crates/core-runs/src/store/marks.rs` and `crates/core-runs/contract.yaml` (that argument only);
  - `docs/codebase/README.md`;
  - the gate files (architecture: `code-record` may depend on `core-design`).
- **Not:** Runs, beyond the one argument below in `crates/core-runs/src/store/marks.rs` (its
  `DoneRecord` already carries the contract the attempt's saved brief holds, pins included);
  Design; Plans.
- **Needs:** B-177.

**Migration `0078_code_record_element.sql`:**

```sql
-- The tables and screens each done task's contract pinned, with what the task did to them.
CREATE TABLE code_record_element (
    project_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    task INTEGER NOT NULL,
    element_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    action TEXT NOT NULL CHECK (action IN ('create', 'modify', 'delete')),
    path TEXT NOT NULL,
    symbol_id TEXT NULL,
    PRIMARY KEY (workflow_id, version, task, element_id)
);

CREATE INDEX code_record_element_project ON code_record_element (project_id, element_id);
```

Table owner line: `code_record_element :: code_record`.

**Writing:**
- `record_done_in` writes one row per `done.contract.elements` pin, after its symbol rows.
- `forget_done_in` and `delete_project_in` delete the task's (the project's) rows.
- Both `record_done_in` and `forget_done_in` journal
  `JournalEntry::new("CodeRecordChanged", Actor::system())`, with
  `.with_project(project)` and payload `{ "project_id", "workflow_id", "version", "task" }`.
  `forget_done_in` gains a `project: &ProjectId` first parameter after `tx` for this. Its one
  caller, `crates/core-runs/src/store/marks.rs`, passes the mark's project (in Touch for that one
  argument, with the `forget_done_in` line of `crates/core-runs/contract.yaml`).

**Types** (`model.rs`; Serialize, ToSchema; `PlannedPin` also Deserialize):

```rust
#[serde(rename_all = "snake_case")]
pub enum BuildState { Planned, Built, Gap, Unchecked, Removed }
#[serde(rename_all = "snake_case")]
pub enum Confirmation { FileAndSymbol, FileOnly }

/// A pin of a task not done in a plan's live version: a fact the caller supplies.
pub struct PlannedPin { pub workflow_id: WorkflowId, pub version: i64, pub task: u32,
    pub pin: core_plans::ElementPin }

/// One task's pin behind an element's state.
pub struct ElementTask {
    pub workflow_id: WorkflowId, pub version: i64, pub task: u32, pub revision: i64,
    pub action: core_plans::ElementAction, pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub symbol_id: Option<String>,
    /// The symbol's state when the task was marked done; none for a planned pin or no symbol.
    #[serde(skip_serializing_if = "Option::is_none")] pub symbol_state: Option<SymbolState>,
}

pub struct ElementState {
    pub element_id: String,
    pub state: BuildState,
    pub built_revision: Option<i64>,
    pub behind: bool,
    #[serde(skip_serializing_if = "Option::is_none")] pub confirmed: Option<Confirmation>,
    pub done: Vec<ElementTask>,
    pub planned: Vec<ElementTask>,
    #[serde(skip_serializing_if = "Option::is_none")] pub why: Option<String>,
}
```

**Method** (`elements.rs`):

```rust
/// Each element's state from the record's pins, the supplied planned pins and the folder.
pub async fn element_states(&self, project: &ProjectId, elements: &[DesignElement],
    planned: &[PlannedPin]) -> Result<Vec<ElementState>, CodeRecordError>;
```

1. Read the project's pin rows. Each row's `symbol_state` comes from a `LEFT JOIN
   code_record_symbol` on `(workflow_id, version, task, symbol_id)`.
2. Collect the files of the pins at each element's built revision. Ask
   `self.code.missing_inputs(Asker::Person(project), &files)` once. If it errors, every element with
   such a pin is `unchecked` with why `the folder could not be read: {error}`.
3. Call the private pure `judge(latest: &DesignElement, done: &[ElementTask], planned:
   Vec<ElementTask>, absent: &BTreeSet<String>, folder_error: Option<&str>) -> ElementState`
   per element. Answer in the order of `elements`.

**`judge`, exactly design §5.2:**
- **No pins:** the latest revision is deleted and no done pin exists → `removed`. No done pin
  otherwise → `planned`.
- **Built revision:** otherwise `built_revision` is the highest done pin revision.
  - If that revision is the latest and deleted, and a done `delete` pin has it → `removed`.
  - Else judge the done pins at `built_revision`, in this order:
    1. a pin's file in `absent` → `gap`, why `` `{file}` is absent; T{task} of {workflow_id} v{version} built it ``;
    2. `symbol_state == Missing` → `gap`, why `{symbol_id} in {file} was missing when T{task} was marked done`;
    3. any `Unchecked`, or a `folder_error` → `unchecked`, why
       `no language server confirmed {symbol_id} in {file}`, or the folder text;
    4. otherwise `built`.
  - Only `delete` pins skip the file check.
- **behind:** `built_revision < latest.revision`.
- **confirmed:** for `built` only. `file_and_symbol` when every judged pin has a symbol, else
  `file_only`.
- **done:** the judged pins. **planned:** every supplied pin for the element.

**Contract** (`crates/code-record/contract.yaml`):
- the job comment: "whether the code holds what a contract promised, and each design element's
  state from it";
- shapes: the types above;
- `element_states` with the rule;
- `record_done_in` / `forget_done_in` comments: the element rows and the event;
- the new table in the store comment;
- obligations with the tests below;
- dependencies: `design: crates/core-design/contract.yaml # types only: DesignElement`;
- modules: `elements.rs: judging a design element's build state`,
  `store/elements.rs: the record's element pins`.

**Tests:**

Unit tests in `src/elements.rs`, over `judge`:
- `an_element_no_task_built_is_planned`
- `the_highest_done_revision_gives_the_state_and_a_newer_revision_is_behind`
- `an_absent_file_is_a_gap_before_an_unchecked_symbol`
- `a_missing_symbol_is_a_gap`
- `an_unchecked_symbol_is_unchecked_and_a_found_one_built`
- `a_pin_without_a_symbol_is_built_on_its_file_only`
- `a_built_deletion_is_removed_and_a_planned_one_keeps_the_built_state`
- `a_folder_that_cannot_be_read_makes_the_element_unchecked_with_the_reason`

`crates/code-record/tests/elements.rs`, with a project folder as `tests/verify.rs` builds one:
- `done_records_the_contracts_element_pins_and_not_done_forgets_them`, each with a
  `CodeRecordChanged` event.
- `element_states_read_the_record_and_the_folder`: one built pin with its file present, one with
  its file absent; the states are `built` and `gap`.

`crates/code-record/tests/record.rs`:
- `removing_the_project_removes_its_element_rows`.

**Done when:**
- `cargo test -p code-record` (with `--features test-support` where its tests need it);
- `cargo test -p shadows-core --features test-support --test runs_attempts` (the marking write
  still passes);
- `cargo test -p core-db --test migration_checksums`;
- the gate list.

---

## Task B-179 · `ProjectState`: the project's state, and the `project_state` tool

**Status at writing:** waits on B-178. Its owner was approved by Mohammed on 2026-11-19 (Ruling 7).

**Files:**
- **Create:**
  - `crates/project-state/{Cargo.toml, contract.yaml, src/lib.rs, src/model.rs, src/state.rs,
    src/error.rs}`;
  - `crates/core-plans/src/live_versions.rs`;
  - `crates/core-plans/src/store/live_versions.rs`;
  - `crates/shadows-core/tests/runs_attempts/project_state.rs` (registered in
    `crates/shadows-core/tests/runs_attempts.rs`);
  - `crates/shadows-mcp/src/state_tools.rs`.
- **Modify:**
  - `crates/core-plans/src/lib.rs`, `src/model.rs`, `src/store/mod.rs`, `contract.yaml`;
  - `crates/shadows-core/Cargo.toml`, `src/app.rs` (build it in `assemble_services` after `runs`;
    field `project_state`; accessor `pub fn project_state(&self) -> &ProjectState`; the doc table
    row ``| `ProjectState` | the project's state as Shadows knows it | `crates/project-state/src` |
    `crates/project-state/contract.yaml` |``), `src/error.rs`, `src/lib.rs`;
  - `crates/shadows-mcp/src/server.rs`, `src/lib.rs`, `src/refusal.rs`, `src/contract.yaml`;
  - `crates/core-grants/src/` (`tools_for`) and its contract (44 becomes 45);
  - `crates/core-harness/src/prompt.txt` and its contract;
  - `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/mcp_tools.rs`,
    `crates/shadows/tests/element_mcp.rs`;
  - `docs/codebase/README.md`;
  - the architecture gate (`project-state` in the services list and as an allowed dependent of
    `core-design`, `core-plans`, `core-runs`, `code-record`).
- **Not:** a route (B-180); the events (B-180).
- **Needs:** B-178.

**Plans' read** (`live_versions.rs`, `impl Plans`):

```rust
/// Every Active plan of `project`, in `list`'s order, with its parts and its live version: its
/// highest Frozen version and that version's tasks with their element pins. A Draft is named,
/// never read.
pub async fn live_versions(&self, project: &ProjectId) -> Result<Vec<LiveVersion>, PlansError>;

pub struct LiveVersion { pub plan_id: PlanId, pub title: String, pub parts: Vec<PartId>,
    pub live: Option<LiveFrozen>, pub draft: Option<WorkflowId> }
pub struct LiveFrozen { pub workflow_id: WorkflowId, pub version: i64, pub tasks: Vec<LiveTask> }
pub struct LiveTask { pub number: u32, pub title: String, pub elements: Vec<ElementPin> }
```

One reader transaction. Each plan's parts come from `Plans::plan_parts_in`. `title` is the live
version's title, else the Draft's. Plans' contract records the rule and the test
`live_versions_name_the_highest_frozen_version_and_skip_drafts`.

**`ProjectState`** (`lib.rs`): `pub struct ProjectState { design: Design, plans: Plans, runs: Runs,
code_record: CodeRecord }`; `pub fn new(design: Design, plans: Plans, runs: Runs, code_record:
CodeRecord) -> Self`.

**Shapes** (`model.rs`; Serialize, ToSchema). The bounds are constants in `state.rs`:
`PARTS_MAX = 200`, `PLANS_PER_PART = 20`, `GAPS_MAX = 100`, `DETAIL_PLANS_MAX = 50`.

```rust
pub struct ProjectStateView {
    pub project_id: ProjectId,
    pub stage: core_design::Stage,
    pub missing: Vec<String>,          // StageView.missing
    pub map_revision: i64,
    pub parts: Vec<PartState>,         // layout order, at most PARTS_MAX
    pub parts_more: u32,
    pub unassigned: Vec<PlanState>,    // Active plans with no part, at most PLANS_PER_PART
    pub unassigned_more: u32,
    pub gaps: Vec<ElementGap>,         // gap or unchecked, part order then element number
    pub gaps_more: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<PartDetail>,      // only when asked with part_id
}
pub struct PartState { pub part_id: PartId, pub parent: Option<PartId>, pub title: String,
    pub kind: Option<String>, pub plans: Vec<PlanState>, pub plans_more: u32,
    pub elements: StateCounts }
pub struct PlanState { pub plan_id: PlanId, pub title: String,
    pub workflow_id: Option<WorkflowId>, pub version: Option<i64>,
    pub draft: Option<WorkflowId>, pub tasks: TaskCounts }
pub struct TaskCounts { pub total: u32, pub done: u32, pub started: u32, pub not_started: u32 }
pub struct StateCounts { pub planned: u32, pub built: u32, pub gap: u32, pub unchecked: u32 }
pub struct ElementGap { pub element_id: String, pub kind: ElementKind, pub name: String,
    pub part_id: PartId, pub state: BuildState, pub why: String }
pub struct PartDetail { pub part_id: PartId, pub plans: Vec<PlanTasks>, pub plans_more: u32,
    pub elements: Vec<ElementView> }
pub struct PlanTasks { pub plan_id: PlanId, pub title: String, pub workflow_id: WorkflowId,
    pub version: i64, pub tasks: Vec<TaskLine> }
pub struct TaskLine { pub number: u32, pub title: String, pub done: bool,
    pub attempt: Option<AttemptState> }
pub struct ElementView { pub element: DesignElement, pub state: ElementState }
```

**Methods** (`state.rs`):

```rust
pub async fn state(&self, project: &ProjectId, part: Option<&PartId>)
    -> Result<ProjectStateView, ProjectStateError>;
/// Every element of the project with its state, by element number (at most 500).
pub async fn elements(&self, project: &ProjectId) -> Result<Vec<ElementView>, ProjectStateError>;
```

**Composition, in this order and no other** (one private `facts` method shared by both):
1. `design.stage`, then `design.layout`.
2. `design.elements`.
3. `plans.live_versions`.
4. For each live version, `runs.progress(workflow, version)`.
5. Planned pins: the pins of live tasks whose progress is not done.
6. `code_record.element_states(project, &elements, &planned)`.

Counts and bounds then follow the shapes above:
- **Task counts:** `done` is `done: true`; `started` is not done and `state` is some; `not_started`
  is the rest.
- **Part counts:** `removed` elements are not counted.
- **Parts:** a plan is listed under each of its parts.
- **The `part` argument:** a part that is not in the layout is
  `ProjectStateError::UnknownPart("part {id} is not a part of this project")`, which maps to
  `INVALID_COMMAND`.

**Errors** (`error.rs`): `ProjectStateError { UnknownPart(String), Design(#[from] DesignError),
Plans(#[from] PlansError), Runs(#[from] RunsError), CodeRecord(#[from] CodeRecordError) }`. Map it
in `CoreError` (`crates/shadows-core/src/error.rs`) and `Refusal` by its inner error.

**MCP** (`state_tools.rs`; Planner list only):
- `project_state`:
  - **args:** `{ part_id: Option<String> }`.
  - **calls:** `ProjectState::state(&grant.project_id, part)`.
  - **description:** "Read what Shadows knows of this project before you plan: its stage, each
    part with its plans and their tasks (done, started, not started), its tables and screens by
    state (planned, built, gap, unchecked), and the gaps. Give part_id for one part in full: its
    plans' tasks and its elements. Read only; no language server is asked."

**Prompt** (`prompt.txt`, the "Shadows' tools" list, before `workspace_get`), exactly:

```text
- project_state: read it before you plan a change to a project that has plans: what each part holds, which tasks are done, and which tables and screens exist, are planned, or have a gap. Pin an existing table or screen by its id; never write a second one with the same name.
```

**Contract** (`crates/project-state/contract.yaml`, in the template's shape):
- the job "the project's state as Shadows knows it";
- dependencies: design, plans, runs, code_record;
- the two methods with their composition and bounds;
- an `agreements` entry `between: [project_state::state, plans::design_part]`, rule "a part's plans
  in the state are `design_part`'s plans of that part that are Active", `tested_by:
  a_parts_plans_in_the_state_are_its_design_part_plans`;
- obligations with the tests below.

Codemap rows: the crate and each module. Plans' new modules:
`live_versions.rs: the Active plans' live versions`,
`store/live_versions.rs: reading live versions in one snapshot`.

**Tests:**

`crates/shadows-core/tests/runs_attempts/project_state.rs`, with the `runs_attempts` fixture, a
project folder and `seed_elements_for_tests`:
- `project_state_counts_each_parts_plans_tasks_and_elements`:
  - **setup:** one plan in both parts; T1 pins E1 (a table, `db/0001_users.sql`, no symbol) and T2
    pins E2 (a screen); approve; write the file; mark T1 done with no attempt.
  - **asserts:** the `database` part counts built 1 with `confirmed: file_only`; the `frontend`
    part counts planned 1; the plan's tasks count 1 done and 1 not started.
- `a_draft_adds_nothing_to_the_state`: a Draft-only plan that pins E3 lists the plan with
  `workflow_id: None` and `draft: Some`; E3 has an empty `planned`.
- `an_absent_file_is_a_gap_in_the_state`: delete the file after done; E1 is in `gaps` with why.
- `a_newer_revision_is_behind`: write E1 r2 after done; E1 stays `built` with `behind: true`.
- `a_part_asked_in_full_lists_its_plans_tasks_and_elements`.
- `the_state_is_bounded`: 21 plans tied to one part give 20 and `plans_more: 1`.
- `an_unknown_part_is_refused`.
- `a_parts_plans_in_the_state_are_its_design_part_plans`.
- `a_running_attempt_shows_started_with_its_state`: Start with `wait-for-release`. The part's
  detail gives the task `attempt: "running"` and the counts `started: 1`. After the release and
  the turn's end without a commit, it gives `attempt: "failed"`, still `started: 1`, `done: 0`.

`crates/shadows/tests/element_mcp.rs`:
- `project_state_tool_answers_the_method`: the tool's JSON equals `serde_json::to_value` of the
  method's answer.

`mcp_server::the_tool_list_depends_on_the_grant_kind`: the Planner's list holds `project_state`.

`mcp_tools::the_planner_is_told_to_read_the_project_state`: the prompt line.

**Done when:**
- those tests;
- `cargo test -p core-plans`;
- `cargo test -p project-state`;
- the gate list.

---

## Task B-187 · Runs journals each change of a task's progress

**Files:**
- **Create:** `crates/shadows-core/tests/runs_attempts/progress_events.rs` (registered in
  `crates/shadows-core/tests/runs_attempts.rs`).
- **Modify:**
  - `crates/core-runs/src/store/mod.rs` (a private helper and its calls in `begin_attempt`,
    `continue_attempt`, `finish_owed`, `finish`, `record_report`);
  - `crates/core-runs/src/store/conflict.rs` (`settle_ended`);
  - `crates/core-runs/src/store/recover.rs` (`settle_running`);
  - `crates/core-runs/src/store/marks.rs` (`put_mark`);
  - `crates/core-runs/contract.yaml`.
- **Not:** Events (B-180 adds the kind to the tail); `ProjectState`; any route.
- **Needs:** nothing. It is independent of the elements, and the journal rows are not
  frontend-visible until B-180.

**The event:**
- **Kind:** `TaskProgressChanged`.
- **Actor:** the write's own actor where it has one (the mark's command actor, the person's Stop),
  else `Actor::system()`.
- **Built with:** `JournalEntry::new(kind, actor).with_project(project_id)`.
- **Payload, exactly:** `{ "project_id", "workflow_id", "version", "task", "attempt_id" }`.
  `attempt_id` is a string for an attempt's change and `null` for a mark. Never any state, reason
  or content: a reader asks `Runs::progress` or `ProjectState`.

**Helpers** (private, in `store/mod.rs`):

```rust
/// Journals TaskProgressChanged for `attempt`, its identities read from its run_attempts row.
async fn attempt_progress_in(conn: &mut SqliteConnection, attempt: &AttemptId, actor: Actor,
    at: &str) -> Result<(), RunsError>;
/// Journals TaskProgressChanged for a mark: attempt_id null.
async fn mark_progress_in(conn: &mut SqliteConnection, project: &ProjectId,
    workflow: &WorkflowId, version: i64, task: u32, actor: Actor, at: &str)
    -> Result<(), RunsError>;
```

**Where, each inside the existing write, only when the row changed, and never on a replay:**

| Write | When |
|---|---|
| `begin_attempt` | after its `INSERT INTO run_attempts` (also an attempt recorded `failed` because its session did not open) |
| `continue_attempt` | after its insert of the new Running attempt |
| `finish_owed` | when its `UPDATE ... state='stopped'` moved a row |
| `finish` | when `moved.rows_affected() == 1` |
| `record_report` | when the update to `committed` moved a row |
| `settle_ended` | when it settled the attempt (`failed` or `stopped`) |
| `settle_running` | once per attempt it makes `interrupted`, from the ids it reads before the update |
| `put_mark` | after the mark row, done or not done; a replay returns before it |

Writes that do not change the state or the mark add nothing: a release, a notice or a conflict's
delivery.

**Contract** (`crates/core-runs/contract.yaml`):
- a top comment line: "Every write that changes what `progress` answers journals
  TaskProgressChanged { project_id, workflow_id, version, task, attempt_id } in that write (an
  attempt's insert or state change; a done or not-done mark, attempt_id null); a replay journals
  nothing.";
- an obligation with the tests below.

**Tests** (`progress_events.rs`). They use the `runs_attempts` fixture: `fake_acp` with
`wait-for-release`, and a git folder. They read `kind, payload_json FROM durable_event WHERE
kind = 'TaskProgressChanged' ORDER BY seq` through the fixture's storage.
- `starting_an_attempt_journals_progress_once`: one event, with the attempt's id and the project,
  workflow, version and task. A replayed Start adds none.
- `an_attempt_ending_without_done_journals_progress`: after the release, the turn ends with no
  commit (`failed`) and a second event names the same attempt.
- `a_report_journals_progress`: an accepted `task_report` adds one.
- `a_stop_and_a_startup_interruption_journal_progress`: a person's Stop adds one; `recover` over a
  Running attempt adds one per attempt.
- `a_done_and_a_not_done_mark_journal_progress_with_no_attempt`: each adds one with `attempt_id`
  null; a replayed mark adds none.

**Done when:**
- `cargo test -p shadows-core --features test-support --test runs_attempts progress_events`;
- `cargo test -p core-runs --features test-support`;
- the gate list.

---

## Task B-180 · Routes and events for the web

**Status at writing:** blocked until the shared request is accepted.

**Files:**
- **Create:** `crates/shadows-http/src/project_state.rs`;
  `crates/shadows/tests/project_state_routes.rs`.
- **Modify:**
  - `crates/shadows-http/src/design.rs` (the revisions route);
  - `crates/shadows-http/src/lib.rs` (the routes);
  - `crates/shadows-http/src/openapi.rs`;
  - `crates/shadows-http/src/failure/cases.rs` (`INVALID_ELEMENT` 422, `ELEMENT_STALE` 409);
  - `crates/shadows-http/src/contract.yaml`;
  - `crates/core-events/src/store.rs` (`read_project_events_after`'s kind list gains
    `'DesignElementChanged'`, `'CodeRecordChanged'` and `'TaskProgressChanged'`);
  - `crates/core-events/contract.yaml`;
  - `crates/shadows/tests/design_routes.rs`, `crates/shadows/tests/project_events.rs` (its
    `next_frame` moves to a new `crates/shadows/tests/fixtures/stream.rs`, used through
    `#[path = "fixtures/stream.rs"] mod stream;`);
  - `crates/shadows/tests/runs_routes.rs` (one test, using that fixture);
  - `api/openapi.json` (regenerated);
  - `docs/codebase/README.md` (`project_state.rs: ProjectState service HTTP translation`).
- **Needs:** B-179, B-187; the shared request accepted.

**Routes** (person, like the decisions routes; no grant):

| Route | Calls | Answers |
|---|---|---|
| `GET /api/projects/{id}/state?part_id=` | `ProjectState::state` | `ProjectStateView`; an unknown part is 422 `INVALID_COMMAND` |
| `GET /api/projects/{id}/elements` | `ProjectState::elements` | `[ElementView]` |
| `GET /api/projects/{id}/elements/{element}/revisions` | `Design::element_revisions` | `[DesignElement]`; `[]` for an element the project does not have, as the MCP tool answers |

An unknown or removed project is 404, as the decisions routes answer.

**Events:** the three kinds of Ruling 11 reach the project tail with their identifier-only
payloads. `Plans::project_event_payload_in` keeps them verbatim (it changes only plan events).
`crates/core-events/contract.yaml` gains the obligation: "DesignElementChanged, CodeRecordChanged
and TaskProgressChanged reach the project event tail with identifier-only payloads, in sequence
order", tested by the two stream tests below.

**Tests:**
- `design_routes::element_routes_list_states_and_revisions`
- `project_state_routes::the_state_route_answers_the_method`
- `project_state_routes::an_unknown_part_is_422`
- `project_events::element_and_record_changes_reach_the_project_stream`: one
  `DesignElementChanged` after a put, one `CodeRecordChanged` after a done mark, in sequence order
  after a cursor.
- `runs_routes::an_attempt_starting_and_ending_reaches_the_project_stream_and_the_state` (the test
  Masada asked for):
  - **Setup:** this file's git project and approved plan, whose adapter is missing, so an
    attempt that passes every check is inserted and then ends `failed`. Subscribe to the project's
    stream, then `POST` the task's Start.
  - **Stream:** two `TaskProgressChanged` frames, in order, each naming the attempt, the
    workflow, version and task, and no done mark.
  - **Then** `GET /api/projects/{id}/state` gives the plan's task counts `started: 1,
    not_started: 0, done: 0`.
  - **And** `GET /api/projects/{id}/state?part_id=<the plan's part>` gives the task line `done:
    false, attempt: "failed"`.
  - **Before the Start,** the same reads give `not_started: 1` and `attempt: null`.

  The state between start and end (`running`) is held by B-179's
  `a_running_attempt_shows_started_with_its_state`.

Then `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.

**Done when:**
- those tests;
- `cargo test -p shadows --test openapi`;
- `cargo test -p core-events`;
- the gate list.

The frontend regenerates `web/src/api/schema.d.ts` on its side.

---

## Task B-181 · Acceptance on a real provider (2a)

**Needs:** B-176 to B-180, B-187, and Masada's diagrams (her tasks under the shared request).

The controller runs it with `shadows serve` and a real provider. The keys are `OPENROUTER` or
`API_KEY_MINIMAX`, read with a loop and never printed. Use a fresh project with a git folder, an
approved map with a `database` and a `frontend` part, and an approved roadmap.

1. **First plan:** ask the Planner to plan "people sign up and sign in".
   - **Expected:** it writes tables (users) and screens (Sign up, Sign in, Home) with
     `design_element_put`, and pins them in its tasks.
   - Approve; run the tasks; mark them done.
   - **Record:** each element's state in `project_state`.
2. **Second plan, in a new conversation:** ask for "password reset".
   - **Expected:** before writing, it reads `project_state` or `design_elements`. It writes `users`
     r2 with a reset token column, not a second users table. It pins E-ids of the first plan.
   - **Record:** the tool calls, and that no element was duplicated.
3. **In the browser:**
   - **Expected:** the state view, the ER diagram and the screen flow show the elements with their
     states. `users` shows `behind` until the second plan's task is done. With the state view open,
     starting a task moves it from not started to started, and its end shows its attempt state,
     with no reload.
   - **Record:** screenshots, and the server log lines.

**Evidence:** `docs/evidence/2026-11-2x-code-record-2a-live.md` (the date of the run), with the
commit.

**Done when:** the evidence records the three steps as they happened, failures included.
