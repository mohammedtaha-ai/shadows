# §16 1a — Plans Belong to the Project: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A plan is the project's, not one conversation's. Any conversation reads and carries it on, every version records who wrote it and why, a conversation can be deleted, and Stop on one conversation never waits for another one opening.

**Architecture:** This is spec §16, part 1a.
- **One lock per session.** `Sessions` keeps one slot per thread, and a map lock held only to find a slot (§16.6).
- **The plan entity.** A new `plan` table owns a chain of `workflow` versions. Each version records its writer and, from v2, its reason. One Draft per plan, and an untouchable frozen version, are held by the database itself (§16.2, §16.3, §16.9).
- **The Planner's reach.** It reads and edits every plan of its project (§16.4).
- **Deleting a conversation** is a soft remove. It marks the thread removed and revokes its grant first, then stops the turn and closes the adapter (§16.5).
- **The web client** shows plans per project, writers, reasons, archive, Continue, Delete, and a deleted conversation read only (§16.8, without 1b's map and cross-plan nodes).

**Tech Stack:** Rust 2024, SQLx 0.9 on SQLite, axum 0.8 + utoipa, `rmcp` 3.4.1; React 19 + TanStack Router/Query, `@xyflow/react`, Vitest. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-01-project-plans-design.md` (§16). Where this plan and §16 disagree, §16 is right and the plan is the defect: stop and report. §17 is a draft and is **not** built here.

## Global Constraints

- **Branch** `plans/project-plans`, which already holds the spec commits.
  - No git worktrees.
  - One commit per task.
  - Push only when Mohammed asks.
- **Execution, Mohammed's way:**
  - One opus implementer does Tasks 1–7 in order, in one dispatch. Every subagent runs on opus, stated explicitly.
  - Then one opus reviewer reviews the whole branch. It fixes what it finds, runs the gate once, commits, and reports everything it found, including what it left.
  - The controller verifies the report and does not repeat the review (CLAUDE.md).
  - No PR until Mohammed says.
- **Read first:**
  - `docs/codebase/README.md`, and the `contract.yaml` of every service a task changes;
  - when the `shadows` MCP server is connected, `where_is`, `who_uses` and `outline` before Grep;
  - the Rust LSP (`goToDefinition`, `findReferences`) for callers of a signature you change.

  Then open only the files the task names.
- **Few tests** (Mohammed's rule):
  - Write only the tests a task lists.
  - While working, run the target test (`cargo test -p <crate> --test <file> <name>`, `npx vitest run <file>`).
  - Run the whole gate once, at the end of each task, before its commit.
  - The reviewer runs it once more, at the end.
- **Existing tests that pin a rule §16 replaced** are changed, not deleted. Examples: "a Planner reaches only its own thread's plan", "`PlanListing.thread_id`", "a project with threads cannot be removed". Name each changed test and the §16 line that replaced its rule in the task's commit message. A test whose rule §16 did not touch must pass unchanged.
- **A service change updates its `contract.yaml` in the same commit** (`docs/codebase/contracts/TEMPLATE.yaml`): `plans`, `threads`, `harness`, `grants`, `projects`, `turns`.
- **Code map:** a new module gets its one-job line in `docs/codebase/README.md`, stated without "and".
- **Files:** a file over 300 lines states its one job in the commit message, and at 500 lines it splits (CLAUDE.md). `plans/contract.yaml` is already 515 lines and grows here. It stays one file, because a contract is one service's, and the commit says so.
- **Builds** use the C: target directory:
  `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/Temp/claude/E--Globalprojects-shadows/e86a610a-4c5b-4114-884d-faf050ce6a96/scratchpad/target CARGO_INCREMENTAL=0`.
  - Use Bash, not PowerShell, for anything under `C:\Users\Mohammed\AppData\Local\shadows-dev`: PowerShell's sandbox says that folder does not exist.
  - **Never** open, migrate or delete `shadows-dev\shadows.sqlite3`. Tests make their own databases in temp dirs and delete them.
- **The gate before each commit** (CLAUDE.md):
  1. `cargo fmt --all --check`
  2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
  3. `cargo test --workspace`
  4. `cargo clippy --workspace -- -D warnings`
  5. `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing
  6. `git diff --exit-code api/`, except in the tasks that change a route or a schema (2, 3, 4, 5). Those regenerate `api/openapi.json` and the web's `src/api/schema.d.ts` (`npm --prefix web run gen:api`).
  7. From Task 2 on, also `npm --prefix web run typecheck`, `npm --prefix web run lint` and `npm --prefix web test`, because the schema the web reads changes.

  Report the Rust and web test counts. Take them at the start of Task 1; after that they may only grow.
- **Names and values from §16:**
  - plan states `Active` and `Archived`;
  - command kinds `PlanArchive`, `PlanUnarchive` and `thread.remove`;
  - events `ThreadRemoved`, `PlanArchived` and `PlanUnarchived`;
  - a missing reason is `NULL` and reads "Reason not recorded";
  - the polling interval is 10 s;
  - "(deleted)" and "External agent" in the version line.

## Review Focus

1. **The migration on a real database.** Mohammed's dev database has two conversations with plans, one at v1 Frozen with a v2 Draft. It also has a thread an external agent's from-scratch draft made (`title_source = 'plan'`). He expects every version in a plan after 0012, no version lost, and the app starting. Task 2's migration test builds that shape. Task I runs 0012 on a copy of the real file before anything else.
2. **An old Planner session with an old habit.** A conversation opened before this change holds the old `prompt.txt`, which says "leave workflow_id out". He expects a refusal that says what to do, not a confusing error. Task 3's test: `workflow_get` with no `workflow_id` from a thread grant answers "name the plan version; workflow_list lists the project's plans".
3. **Approving a version an external agent wrote.** No conversation wrote it, so there is no conversation to put "Plan v2 approved" in. He expects the approval to succeed with no entry. Task 2 test `approving_an_external_version_writes_no_entry`.
4. **Deleting the conversation another tab has open.** That tab's next Send gets a 404. He expects a readable error in the composer, not a blank page. On reload, the read-only view shows. Task 6's deleted-conversation test covers the reload. The 404 shows through the composer's existing `ErrorLine`.
5. **Two conversations starting the next version of the same plan at the same moment.** He expects one Draft, and the second `draft_start` answering that same Draft. Task 2 test `two_starts_make_one_draft`.

---

## Execution map

```text
Task 1  harness: one slot per session (§16.6)                          ← lock test
Task 2  plans: migration 0012, plan entity, writers, reasons, DB guards ← migration, one-Draft, frozen, external-approve tests
Task 3  plans: the Planner reaches every plan; tools; prompt; Continue  ← shared-plan, old-habit tests
Task 4  plans: archive, plan read with versions and writers              ← archive test
Task 5  threads: deleting a conversation                                 ← deletion and race test
Task 6  web: sidebar, plan header, Continue, Delete, read-only view      ← 4 web tests
Task 7  documents (§16.13), status, vision
Task R  (opus reviewer) whole branch
Task I  (controller) Mohammed's Windows run on a copy of the dev DB, then the DB; evidence; PR when he says
```

---

### Task 1: One slot per session

**Files:**
- Modify: `crates/shadows-core/src/harness/sessions.rs`
- Modify: `crates/shadows-core/src/harness/context.rs` (its `self.live.lock()` block)
- Modify: `crates/shadows-core/src/harness/contract.yaml`
- Test: `crates/shadows-core/tests/sessions.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces, used by Task 5: `Sessions::terminate(&ThreadId) -> io::Result<()>`, unchanged in signature, now taking only the thread's slot. Every other public method of `Sessions` keeps its signature and behaviour.

The shape:

```rust
/// One thread's place in `Sessions`: empty until an adapter opens, and while
/// one is replaced. Its own lock is held through an opening, a close or a
/// stop; the map's lock only while a slot is found or added.
type Slot = Arc<Mutex<Option<Live>>>;

pub struct Sessions {
    // …
    pub(super) live: Mutex<HashMap<ThreadId, Slot>>,
    // …
}

impl Sessions {
    /// The thread's slot, added empty when it has none. The map's lock is
    /// released before the caller locks the slot.
    async fn slot(&self, thread: &ThreadId) -> Slot {
        self.live.lock().await.entry(thread.clone()).or_default().clone()
    }
}
```

Rules every method follows:
- **Never hold the map's lock across an `.await` on a process, a connection or a timeout.** Take the slot `Arc`, drop the map guard, then lock the slot.
- **`open_live`** locks the thread's slot for the whole opening, exactly as it held the map before. Then it reads `turn_context` while holding the slot. Task 5 relies on this: the read refuses a removed thread.
- **`terminate`, `terminate_adapter`, `give_back_events`, `mark_answered`, `touch`, `lease_events`** and `context.rs` lock only the thread's slot.
- **`reap_idle`** takes a snapshot of `(ThreadId, Slot)` under the map lock, then calls `try_lock` on each slot. A busy slot is in use, so it is not idle and is skipped. It stops an expired adapter under that slot's lock.
- **`close_all`** takes the same snapshot and locks each slot in turn.
- **`live_count`** counts the slots holding `Some`.
- An empty slot may stay in the map. It holds no process, so it is not a leak worth code.

- [ ] **Step 1: Write the failing test** in `crates/shadows-core/tests/sessions.rs`, after `the_default_setup_wait_outlasts_a_slow_harness`. It needs a second thread in the fixture's project. Add `async fn second_thread(fx: &Fixture) -> ThreadId`, created like `fixture`'s, with command id `c3`.

```rust
/// §16.6: an adapter slow to open holds only its own thread. Stop on
/// another thread does not wait for it.
#[tokio::test]
async fn one_thread_opening_does_not_hold_another() {
    let fx = fixture(SessionsConfig::default()).await;
    let other = second_thread(&fx).await;
    fx.sessions.open(&other).await.unwrap();
    // fake-acp sleeps 1.5 s resuming a session whose id starts with `slow-`.
    fx.storage.record_harness_session(&fx.thread, "slow-3").await.unwrap();
    let sessions = fx.sessions.clone();
    let slow = fx.thread.clone();
    let opening = tokio::spawn(async move { sessions.open(&slow).await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let started = std::time::Instant::now();
    fx.sessions.terminate(&other).await.unwrap();
    assert!(
        started.elapsed() < Duration::from_millis(800),
        "terminate waited {:?} for another thread's opening",
        started.elapsed()
    );
    assert!(opening.await.unwrap().is_ok());
    assert_eq!(fx.sessions.live_count().await, 1);
    fx.sessions.close_all().await.unwrap();
}
```

- [ ] **Step 2: Run it to see it fail.**
`cargo test -p shadows-core --features test-support --test sessions one_thread_opening_does_not_hold_another`
Expected: FAIL. `terminate` waited about 1.3 s.

- [ ] **Step 3: Implement the slot map** as described above, in `sessions.rs` and `context.rs`. `Live` and `same_adapter` keep their meaning: `same_adapter` now takes `&mut Option<Live>`.

- [ ] **Step 4: Run the test, then all of `sessions.rs` and `thread_session.rs`.** Expected: PASS, and every existing sessions test unchanged and green.

- [ ] **Step 5: Contract.** In `harness/contract.yaml`, add an obligation: "a thread's opening, close or stop holds only that thread's slot; the map's lock is never held across a process wait", `tested_by: one_thread_opening_does_not_hold_another`.

- [ ] **Step 6: Gate, then commit.**

```bash
git add crates/shadows-core/src/harness crates/shadows-core/tests/sessions.rs docs/codebase/inventory.md
git commit -m "fix(harness): one slot per open session, so Stop never waits on another opening (§16.6)"
```

---

### Task 2: The plan entity

**Files:**
- Create: `crates/shadows-core/migrations/0012_project_plans.sql`
- Modify: `crates/shadows-core/src/plans/model.rs`. Add `PlanId`, `PlanState`, `Writer` as `WrittenBy`, and `change_reason`. `Plan` and `PlanListing` change.
- Modify: `crates/shadows-core/src/plans/store/{read,draft,edit,view,task}.rs`
- Modify: `crates/shadows-core/src/grants/store.rs`. `check_writer` drops its `thread` argument.
- Modify: `crates/shadows-core/src/plans/{mod,scope}.rs`, only what compiling needs. The reach changes in Task 3.
- Modify: `crates/shadows-http/src/workflow.rs` and `crates/shadows-mcp/src/tools.rs`, mechanically, for the new field names.
- Modify: `api/openapi.json` and `web/src/api/schema.d.ts`, regenerated.
- Modify: `web/src/app/workflows/{plan-page.tsx,use-plan.ts}`, only to compile. "Its conversation" links to `written_by.thread_id` when there is one.
- Modify: `web/src/app/sidebar/workflow-list.tsx`, only to compile.
- Modify: `crates/shadows-core/src/plans/contract.yaml`, `crates/shadows-core/src/grants/contract.yaml`
- Test: `crates/shadows/tests/plan_storage.rs`, and a new migration test in `crates/shadows-core/tests/storage_contract.rs`. That file is a named accretion point: if the test passes 80 lines, put it in a new `crates/shadows-core/tests/plan_migration.rs` instead.

**Interfaces:**
- Produces, used by Tasks 3–6:

```rust
newtype_id! { /// Spec §16.2. A plan of a project, owning a chain of versions.
    PlanId }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum PlanState { Active, Archived }

/// Spec §16.3: who wrote a version.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WrittenBy {
    /// The internal Planner: its conversation, and the turn when one was
    /// recorded (none before migration 0012).
    Planner {
        thread_id: ThreadId,
        thread_title: String,
        thread_removed: bool,
        /// The observed model, else the requested one; `None` without a turn.
        model: Option<String>,
        /// `agent_invocation.harness_kind`, e.g. `claude-code`.
        harness: Option<String>,
    },
    /// An external agent's project grant.
    External { grant_id: GrantId },
}

pub struct Plan {          // one version, as today, with:
    pub id: WorkflowId,
    pub plan_id: PlanId,           // replaces `thread_id`
    pub plan_state: PlanState,
    pub project_id: ProjectId,
    pub written_by: WrittenBy,
    pub change_reason: Option<String>,
    // … version, revision, state, title, goal, previous, next, tasks, links,
    //   blockers, last_edit, frozen_at, created_at: unchanged
}

pub struct PlanListing {   // one plan, as a project's list shows it
    pub plan_id: PlanId,
    pub plan_state: PlanState,
    pub id: WorkflowId,            // its latest version
    pub title: String,
    pub version: i64,
    pub state: WorkflowState,
    pub updated_at: String,
}

pub struct DraftStarted {
    pub workflow_id: WorkflowId,
    pub plan_id: PlanId,           // replaces `thread_id`
    pub version: i64,
}
```

- `Storage::list_plans(&ProjectId, archived: bool) -> Vec<PlanListing>`: Active plans, or all of them when `archived`. Ordered by the plan's `created_at`, then id.
- `Storage::start_draft(ctx, writer, project, plan: Option<&PlanId>, fresh: Option<(&str,&str)>, reason: Option<&str>, operation: Option<&OperationId>, draft_ref) -> DraftStarted`. `plan` absent: a new plan with v1 from `fresh`. Present: that plan's Draft if it has one, else its next version copied from the latest, which needs a non-blank `reason`, else `PlanInvalid("a new version needs its reason")`.
- `Storage::start_thread_with_draft` is **deleted**. An external from-scratch start is `start_draft` with `plan: None`.
- `check_writer(conn, writer, project)`. A Planner is in scope when its grant is a live thread grant of its own thread in `project`.
- Each `Writer::Planner` write that has a running turn passes its `OperationId`. `Plans` gets it from `handles.running_for(thread)`.

**The migration**, `0012_project_plans.sql`:

```sql
-- Spec §16.9. A plan belongs to its project; a version to its plan, with its
-- writer and, from v2, its reason. One Draft per plan and an untouchable
-- frozen version are held here, not only in the store.
--
-- `workflow` is rebuilt: SQLite cannot drop a column a foreign key names.
-- Inside sqlx's transaction, `defer_foreign_keys` lets `DROP TABLE workflow`
-- leave `task`, `task_parent` and `draft_intent` pointing at nothing until
-- the new `workflow` holds the same ids again, before the commit checks.
PRAGMA defer_foreign_keys = ON;

CREATE TABLE plan (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    state       TEXT NOT NULL CHECK (state IN ('Active','Archived')),
    created_at  TEXT NOT NULL,
    archived_at TEXT NULL,
    CHECK ((state = 'Archived') = (archived_at IS NOT NULL))
);

-- One plan per thread that has versions. Its id is the thread's: unique,
-- and stable if the migration is ever replayed on a copy.
INSERT INTO plan (id, project_id, state, created_at)
SELECT t.id, t.project_id, 'Active', MIN(w.created_at)
  FROM planning_thread t JOIN workflow w ON w.thread_id = t.id
 GROUP BY t.id, t.project_id;

CREATE TEMP TABLE workflow_before AS SELECT * FROM workflow;
DROP TABLE workflow;

CREATE TABLE workflow (
    id                   TEXT PRIMARY KEY,
    plan_id              TEXT NOT NULL REFERENCES plan(id) ON DELETE RESTRICT,
    state                TEXT NOT NULL CHECK (state IN ('Draft','Approved','Frozen','Running','Completed','Failed')),
    previous_version_id  TEXT NULL,
    source_plan_json     TEXT NULL,
    version              INTEGER NOT NULL CHECK (version >= 1),
    revision             INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
    title                TEXT NOT NULL,
    goal                 TEXT NOT NULL,
    change_reason        TEXT NULL CHECK (change_reason IS NULL OR trim(change_reason) <> ''),
    written_by_thread    TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    written_by_operation TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    written_by_grant     TEXT NULL REFERENCES mcp_grant(id),
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    frozen_at            TEXT NULL,
    UNIQUE (id, plan_id),
    UNIQUE (previous_version_id),
    UNIQUE (plan_id, version),
    FOREIGN KEY (previous_version_id, plan_id) REFERENCES workflow(id, plan_id),
    CHECK (written_by_thread IS NOT NULL OR written_by_grant IS NOT NULL),
    CHECK (written_by_operation IS NULL OR written_by_thread IS NOT NULL),
    CHECK (state NOT IN ('Frozen','Running','Completed','Failed') OR frozen_at IS NOT NULL)
);

INSERT INTO workflow
       (id, plan_id, state, previous_version_id, source_plan_json, version, revision,
        title, goal, change_reason, written_by_thread, written_by_operation,
        written_by_grant, created_at, updated_at, frozen_at)
SELECT id, thread_id, state, previous_version_id, source_plan_json, version, revision,
       title, goal, NULL, thread_id, NULL, NULL, created_at, updated_at, frozen_at
  FROM workflow_before ORDER BY thread_id, version;
DROP TABLE workflow_before;

CREATE UNIQUE INDEX workflow_one_draft ON workflow(plan_id) WHERE state = 'Draft';

-- A frozen version, its tasks and its links never change (§13.2, §16.2).
-- Freezing is the update from Draft, so OLD.state is not yet 'Frozen'.
CREATE TRIGGER workflow_frozen_immutable BEFORE UPDATE ON workflow
WHEN OLD.state = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

CREATE TRIGGER task_frozen_insert BEFORE INSERT ON task
WHEN (SELECT state FROM workflow WHERE id = NEW.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_frozen_update BEFORE UPDATE ON task
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_frozen_delete BEFORE DELETE ON task
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

CREATE TRIGGER task_parent_frozen_insert BEFORE INSERT ON task_parent
WHEN (SELECT state FROM workflow WHERE id = NEW.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_parent_frozen_update BEFORE UPDATE ON task_parent
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_parent_frozen_delete BEFORE DELETE ON task_parent
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

-- §16.5: a deleted conversation stays a row, as a removed project does.
ALTER TABLE planning_thread ADD COLUMN removed_at TEXT NULL;
```

If Step 2 shows that `defer_foreign_keys` does not carry `DROP TABLE` through to the commit, change the approach. Make the file start with `-- no-transaction` and run SQLite's own rebuild:
1. `PRAGMA foreign_keys = OFF;`
2. `BEGIN;` the same body `COMMIT;`
3. `PRAGMA foreign_key_check;` must print nothing.
4. `PRAGMA foreign_keys = ON;`

Say which way it went in the commit message. Do not use any other approach.

**Store changes**, all inside the existing transactions:
- `owner(conn, workflow)` answers `(PlanId, ProjectId)` through `workflow.plan_id → plan.project_id`.
- `latest_version(conn, &PlanId)`.
- `load_plan` joins `plan` for the project and the plan's state. It reads `written_by_*` and joins `planning_thread` for the title and `removed_at`. When `written_by_operation` is set, it joins that operation's `agent_invocation` for `COALESCE(observed_model, requested_model)` and `harness_kind`.
- `edits_of(conn, project, workflow)` reads `WorkflowEdited` events by `project_id`, not `thread_id`.
- `insert_version` writes `plan_id`, the writer columns from `Writer` plus the operation, and `change_reason`.
- **Every plan event** (`WorkflowDraftStarted`, `WorkflowEdited`, `WorkflowFrozen`, and `PlanShown`, which stays as it is) carries the project. It carries `with_thread` and `with_operation` only for a `Writer::Planner` writing in a turn.
- **`approve_plan`** appends its `PlanApproved` entry to the version's `written_by_thread` when that thread is not removed. With no such thread it appends nothing (§16.3). `WorkflowFrozen` then carries that thread, or none.
- **Command scopes**:
  - `draft_start` with a plan classifies under `("Plan", plan_id)`.
  - Without one, it uses `("Project", project_id)`, as the external start from scratch already did.
  - `edit_plan` and `approve_plan` keep `("Workflow", id)`.

- [ ] **Step 1: Write the failing tests.**

In `plan_migration.rs` or `storage_contract.rs`:

```rust
/// §16.9: each thread's chain becomes one Active plan with the same version
/// ids, written by that thread, with no reason; an external from-scratch
/// thread's plan too. Built on the database shape before 0012.
#[tokio::test]
async fn migration_0012_moves_every_version_into_a_plan() {
    // Open a database migrated only to 0011. Use sqlx::migrate::Migrator
    // with the migrations dir filtered to versions <= 11. Insert:
    //   project P; thread A with v1 Frozen and v2 Draft whose previous is v1;
    //   thread B (title_source 'plan') with v1 Draft; thread C with none;
    //   a task and a link on A's v1.
    // Run the rest of the migrations. Assert:
    //   - two plans, ids A and B, both Active, in project P;
    //   - workflow rows: the same three ids, plan_id A/A/B, written_by_thread
    //     A/A/B, change_reason NULL, A's v2 previous = A's v1;
    //   - the task and the link are still there, and
    //     `PRAGMA foreign_key_check` returns no rows;
    //   - an UPDATE of A's v1 title fails with "a frozen plan version never changes".
}
```

In `crates/shadows/tests/plan_storage.rs`:

```rust
/// §16.2: one Draft per plan, held by the database. The second start answers
/// the first's Draft.
#[tokio::test]
async fn two_starts_make_one_draft() {
    // A plan with v1 approved. Two start_draft calls with different command
    // ids, both with a reason, joined with tokio::join!. Both answer the same
    // workflow_id, and the plan has exactly one Draft (v2).
}

/// §16.3: v2 needs its reason; v1 has none.
#[tokio::test]
async fn a_new_version_needs_its_reason() {
    // v1 approved. start_draft(plan, reason: None) is PlanInvalid naming the
    // reason; with reason "the API changed", get_plan(v2).change_reason is
    // Some("the API changed") and get_plan(v1).change_reason is None.
}

/// §16.3: a person's approval of an external agent's version has no
/// conversation to speak in.
#[tokio::test]
async fn approving_an_external_version_writes_no_entry() {
    // External writer: start_draft from scratch, add one task with an
    // acceptance item, approve_plan. It answers Approved; the version's
    // written_by is External; no planning_thread was created; no
    // PlanApproved entry exists anywhere.
}
```

- [ ] **Step 2: Run them to see them fail** (`cargo test -p shadows-core --features test-support --test plan_migration`, `cargo test -p shadows --test plan_storage`). Expected: FAIL, because there is no migration 0012 and no `plan_id`.

- [ ] **Step 3: Write the migration.** Then run only `migration_0012_moves_every_version_into_a_plan` until it passes. Decide here between `defer_foreign_keys` and `-- no-transaction`.

- [ ] **Step 4: Change the model and the store** as above, then the adapters mechanically: field renames and the new `list_plans` argument, `false` from the routes for now.

- [ ] **Step 5: Run the four tests and the existing plan suites** (`plan_storage`, `plan_routes`, `plan_grants`, `planner_mcp`, `mcp_tools`, `plan_in_conversation`, `workflow_rules`). Fix the existing tests that named `thread_id` on a plan, as the Global Constraints say. A Planner still reaches only plans its own thread wrote until Task 3: keep `in_scope` behaving that way, through `written_by_thread`, so those tests stay green here.

- [ ] **Step 6: Regenerate** `api/openapi.json` (the `openapi.rs` test's update path) and the web schema (`npm --prefix web run gen:api`). Fix web type errors only: `thread_id` → `written_by`, with no new UI.

- [ ] **Step 7: Contracts.**
  - `plans/contract.yaml`: one Draft per plan, held by the index; a frozen version held by triggers; the version's writer; v2+ needs a reason; the approval entry's conversation.
  - `grants/contract.yaml`: `check_writer`'s new signature and scope.

- [ ] **Step 8: Gate, then commit.**

```bash
git add -A crates/shadows-core crates/shadows-http crates/shadows-mcp crates/shadows/tests api web/src docs/codebase
git commit -m "feat(plans): a plan is the project's — migration 0012, writers, reasons, one Draft and frozen versions held by the database (§16.2, §16.3, §16.9)"
```

---

### Task 3: The Planner reaches every plan of its project

**Files:**
- Modify: `crates/shadows-core/src/plans/{mod,scope}.rs`
- Modify: `crates/shadows-mcp/src/{tools,server}.rs`
- Modify: `crates/shadows-core/src/harness/prompt.txt`
- Modify: `crates/shadows-core/src/turns/{mod,model}.rs`, `crates/shadows-core/src/turns/store/turn.rs` and `crates/shadows-http/src/conversation.rs`, for `StartTurn.plan`
- Modify: `crates/shadows-core/src/plans/contract.yaml`, `crates/shadows-core/src/turns/contract.yaml`
- Test: `crates/shadows/tests/planner_mcp.rs`

**Interfaces:**
- Consumes: Task 2's `PlanId`, `start_draft(…, plan, fresh, reason, operation, draft_ref)`, `list_plans(project, archived)`.
- Produces:

```rust
pub struct DraftStart {
    pub title: Option<String>,
    pub goal: Option<String>,
    pub plan_id: Option<PlanId>,   // replaces from_workflow_id
    pub reason: Option<String>,
    pub draft_ref: Option<String>,
}
// Plans::list_for(&Grant, archived: bool) -> Vec<PlanListing>
// StartTurn (HTTP body and core): pub plan: Option<PlanId>
```

**Rules:**
- **`in_scope(grant, named, …)`** takes the same path for both kinds of grant:
  - a version must be named;
  - it must be in the grant's project;
  - an edit additionally needs the plan to be `Active`, else `INVALID_COMMAND` "plan X is archived; a person can unarchive it".

  A thread grant with no `workflow_id` gets `GRANT_SCOPE`, "name the plan version with workflow_id; workflow_list lists the project's plans". The `latest_only` rule that edits only the latest version stays, now per plan.
- **`planner_draft`** passes `args.plan_id` and `args.reason` and the running operation to `start_draft`. A plan outside the project is `GRANT_SCOPE`, and an archived one is `INVALID_COMMAND`. The fingerprint params are `{ "project", "plan_id", "title", "goal", "reason" }`.
- **`external_draft`** takes the same path, under its `draft_ref`. From an existing plan, its latest version must be `Frozen`, as today.
- **`THREAD_TOOLS`** gains `workflow_list` and has 6 entries. Its doc comment says why: the plans are the project's now.
- **Tool descriptions and argument docs** in `tools.rs`:
  - `workflow_id` "names the plan version; workflow_list lists each plan's latest";
  - `draft_start`'s `plan_id` "a plan to start its next version, or none for a new plan";
  - `reason` "why the plan changes, required for every version after the first".
- **`prompt.txt`:**
  - Replace the line "This conversation's plan is the only one you reach…" with the four rules of §16.4: shared plans, find before you start, archived is read only, a reason for every new version.
  - Name the plan version with `workflow_id` on every call.
  - The prompt version is a hash of this text, so every open conversation receives it through §13.8's block. No code is needed for that.
- **`StartTurn.plan`:**
  - An optional `plan` on `POST /api/threads/{id}/turns`, in the fingerprint as `focus` is.
  - `Turns::send` checks that the plan is in the thread's project, else `INVALID_COMMAND`.
  - It adds one content block after the person's text, and after a focus block if any: `[Shadows] The person opened this conversation to continue the plan "{title}" (plan_id {id}, latest version workflow_id {latest}). Read it with workflow_get before you plan.`
  - The block is not stored as an entry, as the focus block is not.

- [ ] **Step 1: Write the failing tests** in `crates/shadows/tests/planner_mcp.rs`. Use the existing fake-acp `mcp <tool> <json>` prompt, as the other tests there do.

```rust
/// §16.4: conversation B carries on a plan conversation A wrote, with its
/// reason; the versions name their own conversations.
#[tokio::test]
async fn a_second_conversation_carries_the_plan_on() {
    // A's Planner: draft_start {title, goal}, plan_edit adds T1 with one
    // acceptance item; the person approves v1.
    // B (a new thread, same project): workflow_list shows the plan;
    // draft_start {plan_id} without reason → isError, text names the reason;
    // draft_start {plan_id, reason: "split T1"} → v2.
    // get_plan(v2).written_by is Planner{thread_id: B, model: Some(_)},
    // get_plan(v1).written_by is Planner{thread_id: A}.
}

/// Review Focus 2: an old habit, leaving workflow_id out, is told what to do.
#[tokio::test]
async fn a_planner_without_a_workflow_id_is_told_to_list() {
    // A thread grant's workflow_get {} → isError GRANT_SCOPE whose text
    // contains "workflow_list".
}
```

- [ ] **Step 2: Run them to see them fail.** `cargo test -p shadows --test planner_mcp a_second_conversation_carries_the_plan_on a_planner_without_a_workflow_id_is_told_to_list`

- [ ] **Step 3: Implement** the rules above.

- [ ] **Step 4: Run the two tests, then the plan suites of Task 2 Step 5.** Change the tests whose rule §16.4 replaced, such as "a Planner's source must be its thread's latest" and "workflow_get with no id reads this conversation's plan". Add one assertion to an existing `turn_command.rs` test: a turn with `plan` from another project is `INVALID_COMMAND`. Do not add a new test.

- [ ] **Step 5: Regenerate** `api/openapi.json` and the web schema.

- [ ] **Step 6: Contracts.** `plans`: the reach and the archived refusal. `turns`: `plan` in the fingerprint and its block.

- [ ] **Step 7: Gate, then commit.**

```bash
git commit -am "feat(plans): the Planner reads and carries on every plan of its project (§16.4)"
```

---

### Task 4: Archive, and a plan read with its versions

**Files:**
- Modify: `crates/shadows-core/src/plans/{mod,model}.rs`, `crates/shadows-core/src/plans/store/read.rs`
- Create: `crates/shadows-core/src/plans/store/plan.rs`: archiving a plan, and reading one with its versions. Add its line to `docs/codebase/README.md`.
- Modify: `crates/shadows-http/src/workflow.rs` and `crates/shadows-http/src/lib.rs`, for the routes
- Modify: `crates/shadows-core/src/plans/contract.yaml`
- Test: `crates/shadows/tests/plan_routes.rs`

**Interfaces:**
- Produces, used by Task 6:

```rust
/// One plan with every version, oldest first (§16.10's GET /api/plans/{id}).
pub struct PlanVersions {
    pub plan_id: PlanId,
    pub project_id: ProjectId,
    pub state: PlanState,
    pub archived_at: Option<String>,
    pub versions: Vec<VersionLine>,
}
pub struct VersionLine {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub state: WorkflowState,
    pub title: String,
    pub written_by: WrittenBy,
    pub change_reason: Option<String>,
    pub created_at: String,
}
// Plans::plan(&PlanId) -> PlanVersions
// Plans::archive(command_id, &PlanId) -> PlanVersions     // "PlanArchive", params { "plan" }
// Plans::unarchive(command_id, &PlanId) -> PlanVersions   // "PlanUnarchive", params { "plan" }
```

- **Routes:**
  - `GET /api/plans/{id}` → `PlanVersions`;
  - `POST /api/plans/{id}/archive` and `POST /api/plans/{id}/unarchive` with body `{ command_id }` → `PlanVersions`;
  - `GET /api/projects/{id}/workflows?archived=true` includes archived plans.
- **Behaviour:**
  - Archiving an archived plan, or unarchiving an active one, answers the plan unchanged: the command is idempotent in effect.
  - Each change journals `PlanArchived` or `PlanUnarchived` with the project.
  - A replay answers the plan as it now stands, as `set_project_modes` does.
  - A plan of a removed project is not found.

- [ ] **Step 1: Write the failing test** in `plan_routes.rs`:

```rust
/// §16.2: an archived plan leaves the list, is read only, and comes back.
#[tokio::test]
async fn an_archived_plan_is_read_only_until_unarchived() {
    // A plan with a Draft. POST archive → state Archived. GET workflows → it
    // is absent; GET workflows?archived=true → it is present.
    // A plan_edit through an external grant on its Draft → INVALID_COMMAND,
    // text contains "archived". POST unarchive → Active, and the same edit
    // now succeeds. GET /api/plans/{id} lists v1 with its written_by.
}
```

- [ ] **Step 2: Run it to see it fail.** `cargo test -p shadows --test plan_routes an_archived_plan_is_read_only_until_unarchived`
- [ ] **Step 3: Implement it.** **Step 4:** Run the test and `plan_routes`.
- [ ] **Step 5: Regenerate** openapi and the web schema. Update `plans/contract.yaml`.
- [ ] **Step 6: Gate, then commit.**

```bash
git commit -am "feat(plans): archive and unarchive a plan, and read it with every version's writer (§16.2, §16.10)"
```

---

### Task 5: Deleting a conversation

**Files:**
- Create: `crates/shadows-core/src/threads/store/remove.rs`: marking a thread removed, in one write.
- Modify: `crates/shadows-core/src/threads/{mod,model}.rs`. `PlanningThread` gains `removed_at: Option<String>`.
- Modify: `crates/shadows-core/src/threads/store/{thread,fork}.rs`
- Modify: `crates/shadows-core/src/turns/store/turn.rs` (`start_turn`)
- Modify: `crates/shadows-core/src/turns/mod.rs`: a `ThreadStopper` handle, below.
- Modify: `crates/shadows-core/src/projects/store.rs` (`remove_project`)
- Modify: `crates/shadows-core/src/grants/store.rs`: a `revoke_thread_grants_in(conn, &ThreadId, actor, ts)` in `shared_in_transaction`.
- Modify: `crates/shadows-core/src/app.rs`. Build `Turns`' stopper before `Threads`, and pass it in.
- Modify: `crates/shadows-http/src/thread.rs` and `lib.rs`: `DELETE /api/threads/{id}?command_id=` and `GET /api/threads/{id}`.
- Modify: the `threads`, `turns`, `grants` and `projects` contracts.
- Test: `crates/shadows/tests/thread_routes.rs`

**Interfaces:**

```rust
// turns/mod.rs — owned by Turns; Threads holds one (no dependency on Turns itself).
#[derive(Clone)]
pub(crate) struct ThreadStopper { runtime: Arc<Runtime>, handles: Arc<LiveHandles>, sessions: Arc<Sessions> }
impl ThreadStopper {
    /// Stops the thread's running turn, if any, as Stop does (§12.3), and
    /// waits for its ending. `TerminationFailed` as `Turns::stop` reports it.
    pub(crate) async fn stop_running(&self, thread: &ThreadId) -> Result<StopOutcome, StorageError>;
}

// threads
impl Threads {
    /// §16.5: "thread.remove", params { "thread_id" }.
    pub async fn remove(&self, command_id: String, thread: &ThreadId) -> Result<PlanningThread, CoreError>;
    /// One thread, removed or not.
    pub async fn get(&self, thread: &ThreadId) -> Result<PlanningThread, CoreError>;
}
impl Storage {
    pub async fn remove_thread(&self, ctx: &CommandContext, thread: &ThreadId) -> Result<PlanningThread, StorageError>;
    pub async fn get_thread(&self, thread: &ThreadId) -> Result<PlanningThread, StorageError>;
}
```

**`Threads::remove`, in the order of §16.5:**
1. **`storage.remove_thread(ctx, thread)`.** One write:
   - `classify` first: a replay answers the thread;
   - the thread must exist, and its project must be live;
   - set `removed_at`;
   - `revoke_thread_grants_in`;
   - journal `ThreadRemoved` with project and thread;
   - `record_command`.
2. **`stopper.stop_running(thread)`.** `TerminationFailed` is logged as `thread.remove_stop_failed`, and the remove still answers: the thread is removed and its grant revoked, so the turn cannot write a plan.
3. **`harness.close_session(thread)`.** This goes through `Sessions::terminate`, which takes the slot (Task 1). An error is logged as `thread.remove_close_failed`.

**What refuses a removed thread**, each answering `NotFound("planning_thread")`, which the routes map to 404 as for an unknown thread:
- `start_turn`'s transaction, after `classify`, so a replay of a turn sent before the delete still answers;
- `fork_thread`'s source check;
- `set_thread_harness`;
- `turn_context`, by adding `AND t.removed_at IS NULL`. `Sessions::open_live` reads it while holding the slot, so this refuses a session opening.

**What still reads a removed thread:**
- `list_thread_entries` and `list_operations_for_thread`;
- `get_thread`;
- the plans' `written_by` join.

**`list_threads_for_project`** adds `AND t.removed_at IS NULL`. **`remove_project`** counts only threads with `removed_at IS NULL`.

**A turn's own ending writes** (`mark_operation_*`, its agent entries, `title_from_harness`) are **not** checked: they end work that began before the delete (§16.5, step 2).

- [ ] **Step 1: Write the failing test** in `thread_routes.rs`:

```rust
/// §16.5: a deleted conversation leaves the list and refuses work, its plan
/// stays, it still reads — and nothing starts or writes while it is deleted
/// mid-turn.
#[tokio::test]
async fn a_deleted_conversation_is_gone_but_readable() {
    // Thread A: a turn with fake-acp prompt `hang`, started; A's Planner
    // grant token captured from the session's setup (as planner_mcp tests do).
    // A plan written by A exists (draft_start + one edit, done before the hang).
    // DELETE /api/threads/A?command_id=d1 → 200 with removed_at set.
    // During it, run concurrently: POST /api/threads/A/turns → 404, and an MCP
    // plan_edit with A's token → isError GRANT_INVALID, plan revision unchanged.
    // Afterwards: GET /api/projects/P/threads omits A; GET /api/threads/A has
    // removed_at; GET /api/threads/A/entries answers; the operation is
    // Cancelled; sessions.live_count() == 0; GET /api/projects/P/workflows
    // still lists the plan, its written_by Planner{thread_removed: true}.
    // DELETE again with d1 → 200, the same thread. DELETE /api/projects/P is
    // allowed once P has no other thread.
}
```

- [ ] **Step 2: Run it to see it fail.** `cargo test -p shadows --test thread_routes a_deleted_conversation_is_gone_but_readable`
- [ ] **Step 3: Implement it.** **Step 4:** Run the test, then `project_remove`, `thread_routes`, `fork`, `turn_command`, `recovery` and `thread_session`. `project_remove`'s "refused while it holds threads" test keeps its rule: it counts threads that are not removed.
- [ ] **Step 5: Regenerate** openapi and the web schema. Update the contracts.
- [ ] **Step 6: Gate, then commit.**

```bash
git commit -am "feat(threads): delete a conversation — removed and its grant revoked first, then its turn stopped and its adapter closed (§16.5)"
```

---

### Task 6: The web client

**Files:**
- Modify: `web/src/api/{client,queries}.ts`. Add `getThread`, `removeThread`, `getPlanVersions`, `archivePlan` and `unarchivePlan`, and `listPlans(projectId, archived)`. `planQuery` polls every 10 s, and a new `planVersionsQuery` does too.
- Modify: `web/src/app/sidebar/workflow-list.tsx`: Active plans, then a folded "Archived (N)".
- Modify: `web/src/app/sidebar/thread-list.tsx`: a `⋯` menu per row with **Delete**, using `components/ui/dropdown-menu`.
- Create: `web/src/app/sidebar/delete-thread-dialog.tsx`: asking before a conversation is deleted.
- Modify: `web/src/app/workflows/plan-page.tsx`. Its header gains the version line, the reason, **Versions**, **Continue this plan**, and **Archive** or **Unarchive**. An archived plan shows "Archived · read only" in place of the Approve bar.
- Create: `web/src/app/workflows/version-line.tsx`: one version's writer and reason, as one line.
- Modify: `web/src/app/workflows/use-plan.ts`: `usePlanFrames(plan.data?.written_by)` uses the writer's thread when `kind === 'planner'`, plus the 10 s poll.
- Modify: `web/src/app/conversation/draft.tsx`. `/projects/$projectId/new?plan=<planId>` shows a chip "Continuing: {title} ×", and the first `startTurn` carries `plan`.
- Modify: `web/src/router.tsx`: `validateSearch` for `plan` on the draft route.
- Modify: `web/src/app/conversation/conversation.tsx`. A thread missing from the list is fetched with `getThread`. When `removed_at` is set, render `DeletedConversation`: the banner, `Messages`, no `Composer`, no `useSession`, no `CliPicker`.
- Modify: `web/src/app/project-settings/remove-project.tsx`: "Delete them from the sidebar first."
- Modify: `web/src/test/{fake-daemon,contract-fixtures}.ts`, for the new shapes.
- Test: `web/src/app/sidebar/workflow-list.test.tsx`, `web/src/app/workflows/plan-page.test.tsx`, `web/src/app/conversation/conversation.test.tsx`, and `web/src/app/sidebar/delete-thread-dialog.test.tsx` (new).

**Copy, exactly:**
- Delete dialog title: `Delete "{title}"?`. Its body: `It leaves the list for good. Its plans stay in the project, and their history still names it.` Its buttons: `Cancel`, `Delete`.
- Version line: `v{n} · from {conversation} · {model} · {CLI label}`. A deleted conversation shows `{conversation} (deleted)`. An external one shows `v{n} · External agent`. A version from before the migration has no model and no CLI: `v{n} · from {conversation}`.
- Reason under the line: the text, or `Reason not recorded` when `change_reason` is null and `version > 1`. Nothing for v1.
- Deleted conversation banner: `This conversation was deleted. You can read it, but not write in it.`
- Archived bar: `Archived · read only`.

**Tests, exactly four:**

```tsx
// workflow-list.test.tsx
it('lists active plans and folds the archived ones', …)
// fake daemon: GET workflows → [A Active]; ?archived=true → [A Active, B Archived].
// The list shows A; "Archived (1)" is shown folded; clicking it shows B.

// plan-page.test.tsx
it('heads a version with its writer and reason, and archives the plan', …)
// GET workflow v2 with written_by planner {thread_title "Web fixes", model "opus-5-5",
// harness "claude-code"} and change_reason "the API changed"; GET plans/{id} with v1, v2.
// The header reads "v2 · from Web fixes · opus-5-5 · Claude Code" and "the API changed";
// Archive sends POST plans/{id}/archive with a command_id, then "Archived · read only" shows
// and Approve does not.

// conversation.test.tsx
it('shows a deleted conversation read only', …)
// threads list omits T; GET /api/threads/T → removed_at set; entries → one message.
// The banner shows, the message shows, no textbox "Message", and no
// POST /api/threads/T/session was called.

// delete-thread-dialog.test.tsx
it('deletes a conversation after asking, and leaves its page', …)
// Open T, ⋯ → Delete → dialog with the title; Delete sends DELETE /api/threads/T?command_id=…;
// the app lands on /projects/P/new and the list refetches.
```

- [ ] **Step 1: Write the four tests.** **Step 2:** Run them to see them fail (`npx vitest run <file>` in `web/`).
- [ ] **Step 3: Implement it.** **Step 4:** Run the four, then the whole web suite once.
- [ ] **Step 5: Run it in the browser** against the dev servers of `.claude/launch.json`. Use a **scratch database**, not the dev one: add a temporary launch entry pointing `--db` at a file in the scratchpad. Check that:
  - the sidebar lists the plans;
  - a plan's header shows the version line;
  - Delete leaves the list;
  - a deleted conversation opens read only.

  Delete the scratch database after. Take one screenshot for the report.
- [ ] **Step 6: Gate, then commit.**

```bash
git add -A web
git commit -m "feat(web): plans per project, version writers and reasons, archive, Continue, Delete, and a deleted conversation read only (§16.8)"
```

---

### Task 7: Documents

**Files:** the specs and docs §16.13 lists.

- [x] **Step 1:** Amend in place, each pointing to §16 rather than copying it:
  - §13.2: a plan is §16.2's, with one Draft per plan;
  - §13.6: the tool table and `draft_start` refer to §16.4, and an external draft from scratch creates no thread;
  - §13.10 and §13.11: the plan list, the Workflows section and the page header refer to §16.8 and §16.10;
  - §4.2: a thread can be removed (§16.5), and project removal counts only threads that are not removed;
  - §6.8: lineage within a plan.
- [x] **Step 2:** `docs/vision.md` §8: link §16. Its Today line says what exists now, and that decisions and clarifying messages do not exist yet.
- [x] **Step 3:** `docs/status.md`: a "Where we are" entry for 1a, with the test counts. Next: 1b, then the rest, as `status.md` orders them. The lock and conversation deletion leave Next.
- [x] **Step 4:** Commit (docs only, so no gate):

```bash
git commit -am "docs: §16 1a is built — §13, §4.2, §6.8, vision §8 and status point to it"
```

---

### Task R: Whole-branch review (one opus reviewer)

One dispatch, `model: "opus"`. It reads §16, this plan, the branch's diff against `main`, and the contracts the branch changed. It hunts first in the Review Focus inputs and in these places:
- the migration's data move and its triggers against Mohammed's real database shape;
- every place that read `workflow.thread_id`. None may be left: `grep -rn "thread_id" crates/shadows-core/src/plans` must show only the writer columns;
- the Sessions slot rules: no map lock held across an `.await` on a process;
- the order of `Threads::remove`, and every write that must refuse a removed thread;
- the web's Delete reaching the open conversation.

It fixes what it finds, runs the gate once, commits, and reports what changed, what it left and why, and the test counts.

### Task I: The Windows run (controller with Mohammed)

1. Rebuild the binary from the branch.
2. **Copy** `shadows-dev\shadows.sqlite3` and its `-wal` and `-shm` into the scratchpad, using Bash. Run the branch's daemon on the copy with a `daemon-branch` launch entry and the web client.
3. **Check with Mohammed** in the browser:
   - both existing conversations' plans are under Workflows;
   - "Continue this plan" opens a draft whose first message starts v2 with a reason;
   - the version line names the new conversation, model and CLI, with the reason;
   - Delete a conversation: its plan stays, and its line opens it read only;
   - Stop on one conversation while another opens does not wait.
4. Write `docs/evidence/project-plans/WINDOWS_RUN.md`. It holds the date, the commit, what was checked, and what was not.
5. Delete the copy. Only after Mohammed has seen the run, and only when he says so:
   - stop the `daemon` preview;
   - copy the new exe into `shadows-dev\bin`;
   - start it on the real database, which migration 0012 then migrates;
   - remove the `daemon-branch` entry.
6. Open the PR only when Mohammed says.
