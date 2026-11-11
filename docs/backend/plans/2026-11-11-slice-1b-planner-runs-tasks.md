# Slice 1b: the Planner runs and judges tasks — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); the entry's Touch and Done when are binding, and so are this
> file's Interfaces and steps. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Planner can make an empty folder a git repository, start a task, hear how its
attempt ended, judge the commit, and either mark the task done or send the executor an addition
it carries on with in its own session. After a done mark it starts the next task that never
ran and can start now, on its own.

**Architecture:** Everything is Runs' (`crates/core-runs`): git calls in a project folder,
attempts, done marks. The Planner reaches Runs through four MCP tools that call one Runs method
each, behind a Planner-grant check in Runs. The notice to the Planner is a waiting message on the
plan's own conversation (`WrittenBy::Planner.thread_id`) through `Turns::queue`, so a busy
Planner hears it when its turn ends and a restart does not lose it. An addition is a new attempt
on the same executor thread, carrying the addition text; the frozen plan version never changes.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, rmcp, git through `shadows-process`.

**Spec:** [executors design §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)
(Mohammed's rulings of 2026-11-11). Program: [local delivery](2026-11-09-local-delivery-program.md).
Web request: [planner review](../../shared/requests/2026-11-11-planner-review.md).

## Global Constraints

- Start at `crates/shadows-core/src/app.rs`; the owner is `Runs`. No new service, table owner or
  second way in. Adapters translate only: one Runs method per tool.
- New tests are live (the real adapter, real git) or run no agent at all. No new `fake-acp`
  test. Existing fake tests stay; one is renamed in B-125 because the rule it holds changes.
- A live test that needs `auto` runs on the person's signed-in Claude Code subscription, model
  `sonnet` (B-121, part 5); its `#[ignore]` reason says so.
- Builders find symbols with the LSP, run only their own tests with `-p <crate> --test <target>`
  (never `--workspace`), build in the one `target/` (no `CARGO_TARGET_DIR`, `cargo clean`,
  `--release`), and keep every `crates/**/*.rs` line within 100 columns.
- Each task updates its contracts and code-map rows in its own commit; `api/openapi.json`
  changes only in B-126, regenerated, never hand-edited.
- Commit messages: `feat(backend): B-<n> <goal>`; stage files by name.

## Review Focus

1. **Stop on a committed attempt waiting for judgment** frees its session and scope and sends
   the Planner nothing new. Test: `stopping_a_committed_attempt_frees_its_scope` (B-125).
2. **A restart with a committed attempt waiting** keeps it waiting (scope held), and an
   addition after the restart reopens the executor's thread. Test:
   `recovery_keeps_a_committed_attempt_waiting_for_its_judgment` (B-125); the reopening is
   `open_executor` at the top of `run_turn` (B-126).
3. **An addition while the executor is still running** (no commit yet) is refused, not queued
   into the running turn. Test: `an_addition_needs_a_committed_attempt_waiting` (B-126).
4. **A grant that is not a Planner's of the plan's project** (an executor's, an external
   agent's) cannot initialise git, start, mark or amend. Tests:
   `git_init_is_refused_to_an_executor_grant` (B-123),
   `the_planner_tools_refuse_a_grant_that_is_not_a_planners` (B-124).
5. **A folder inside a parent repository** is not initialised a second time. Test:
   `git_init_is_refused_inside_a_repository` (B-123).

---

## Task B-123: The Planner initialises git in a folder that has none

**Files:**
- Create: `crates/core-runs/src/init.rs` (one job: giving a project folder its git repository)
- Modify: `crates/core-runs/src/git.rs`, `crates/core-runs/src/lib.rs` (`mod init;`),
  `crates/core-runs/src/error.rs`, `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/init.rs` (new module, declared in
  `crates/shadows-core/tests/runs_attempts.rs` as `report` is), `crates/shadows/tests/mcp_server.rs`,
  `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- Produces, in `git.rs`:
  - `pub(crate) async fn init(&self) -> Result<(), String>`: runs `git init -q`.
  - `pub(crate) async fn commit_empty(&self, message: &str) -> Result<String, String>`: runs
    `git -c user.name=Shadows -c user.email=shadows@localhost commit -q --allow-empty -m <message>`
    (the `-c` pairs before `commit`), then answers `self.head()`, or
    `Err("the first commit was not recorded")` when that is `None`. The identity is fixed so the
    commit does not depend on the person's git configuration, and it says honestly who made it.
- Produces, in `error.rs`: `AlreadyGit` (text "the project folder is already inside a git
  repository", code `already_git`) and `NoFolder` (text "the project has no folder", code
  `no_folder`). Neither is in `refuses_start`.
- Produces, in `init.rs`: `pub async fn init_git(&self, grant: &Grant) -> Result<String, RunsError>`.
  In order: `grant.kind` must be `GrantKind::Thread`, else `NotFound`; the folder from
  `Projects::get(&grant.project_id)`'s `directory`, `NoFolder` when it has none (do not use
  `Runs::folder`, whose `None` means `NotGit`); `Git::at(folder).prefix()` is `Some` →
  `AlreadyGit` (a folder inside any work tree, its own or a parent's); `init()` failing →
  `Unavailable(text)`; `commit_empty("Initial commit")` failing → `Unavailable(text)`. Answers
  the full SHA and logs `runs.git_initialised` with the project. Writes no file.
- Produces, in `run_tools.rs`: the tool `project_git_init`, no arguments (the shape other
  argument-less tools use). Description: "Make this project's folder a git repository when Start
  says it is not one: git init and one empty first commit, made in Shadows' name. Refused when
  the folder is already inside a repository. It writes no file; starter files are a task's
  work." Answers `{ "commit": <sha> }`.
- `THREAD_TOOLS` gains `"project_git_init"` (23 tools).
- `prompt.txt`, in "Shadows' tools" after the `project_files` line, this exact line:
  `- project_git_init: when Start is refused because the project folder is not a git repository, make it one with project_git_init: git init and an empty first commit. It is refused inside an existing repository and writes no file; starter files are a task's work.`

- [ ] **Step 1: Write the failing tests** in `runs_attempts/init.rs` (`use super::*;`). A helper
  in `runs_attempts.rs`, `async fn planner_grant(f: &Fixture) -> shadows_core::Grant`, reads the
  plan's `written_by_thread` as `planner_turn` does and answers
  `shadows_core::Grants::issue_thread_grant_for_test(&f.storage, &thread)`'s grant.
  - `the_planner_initialises_git_in_a_folder_without_one`: remove `f.root/.git`; call
    `init_git(&grant)`; assert the answer is 40 hex characters, equals `git rev-parse HEAD`,
    `git log -1 --format=%an` is `Shadows`, `git ls-tree -r HEAD` is empty,
    `git status --porcelain` contains `?? README.md`, and `runs().task(&f.workflow, 1, 3)`'s
    `start.allowed` is true.
  - `git_init_is_refused_inside_a_repository`: on the fixture's repository,
    `Err(RunsError::AlreadyGit)`, and `git rev-list --count HEAD` is still `1`.
  - `git_init_is_refused_to_an_executor_grant`: a `Grant` built as B-121's live test builds one
    (kind `Executor`, scope T3), after removing `.git`: `Err(RunsError::NotFound)` and no `.git`.
  - In `mcp_tools.rs`, a new test `the_planner_is_told_how_to_run_and_judge_tasks` asserts
    `shadows_core::testing::PROMPT` contains `"project_git_init: when Start is refused"`. Later
    tasks add their sentences to this test.
- [ ] **Step 2: Run them and see them fail** for the missing method and line:
  `cargo test -p shadows-core --test runs_attempts git_init` and
  `cargo test -p shadows-core --test runs_attempts the_planner_initialises`.
- [ ] **Step 3: Implement** the Interfaces above; add the tool to both lists in `mcp_server.rs`.
- [ ] **Step 4: Run** `cargo test -p shadows-core --test runs_attempts`,
  `cargo test -p shadows --test mcp_server --test mcp_tools --test planner_mcp`, then
  `cargo test -p shadows --test architecture --test codemap --test contract_tests`: all pass.
- [ ] **Step 5: Contracts and map.** `core-runs`: `modules.init.rs`, `functions.init_git`, the
  two errors in `RunsError`, an obligation tested by the three tests; `shadows-mcp`: the tool;
  `core-grants`: 23 thread tools; `core-harness`: the prompt line if it lists them; a code-map
  row `crates/core-runs/src/init.rs | giving a project folder its repository`.
- [ ] **Step 6: Commit** `feat(backend): B-123 the Planner initialises git in a folder that has none`.

## Task B-124: The Planner starts a task and marks it done or not done

**Files:**
- Create: `crates/core-runs/src/planner.rs` (one job: the Planner's calls on its project's tasks)
- Modify: `crates/core-runs/src/lib.rs`, `crates/core-runs/src/error.rs`,
  `crates/core-runs/src/model.rs`, `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/refusal.rs`,
  `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/planner.rs` (new module),
  `crates/shadows/tests/fixtures/live.rs`, `crates/shadows/tests/planner_runs_live.rs` (new),
  `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- Consumes: `Runs::start` and `Runs::mark_done`, unchanged; `Runs::suggested_model`.
- Produces, in `lib.rs`: `async fn read_plan(&self, workflow: &WorkflowId) -> Result<Plan, RunsError>`
  holds the error mapping `plan()` has today; `plan()` becomes `read_plan` plus its version check.
- Produces, in `planner.rs`:
  - `async fn planner_plan(&self, grant: &Grant, workflow: &WorkflowId) -> Result<Plan, RunsError>`
    (private to the crate): `NotFound` unless `grant.kind == GrantKind::Thread` and the plan
    `read_plan` answers has `project_id == grant.project_id`.
  - `pub async fn start_for(&self, grant: &Grant, workflow: &WorkflowId, task: u32, model: Option<String>) -> Result<Attempt, RunsError>`:
    `planner_plan`; the model is the trimmed `model` when not empty, else
    `self.suggested_model(&plan)`, else `NoModel`; then
    `self.start(format!("planner-start-{}", AttemptId::generate()), workflow, plan.version, task, model)`.
    Start's own checks and answers are unchanged: the HTTP Start and this tool run one method.
  - `pub async fn mark_done_for(&self, grant: &Grant, workflow: &WorkflowId, task: u32, done: bool) -> Result<DoneMark, RunsError>`:
    `planner_plan`; then `self.mark_done(format!("planner-mark-{}", AttemptId::generate()), workflow, plan.version, task, done)`;
    then, only when `done` is true, `next` is `self.next_ready(&plan)`, else `None`.
  - `async fn next_ready(&self, plan: &Plan) -> Result<Option<u32>, RunsError>` (private): in
    task-number order, the first task with no done mark, no attempt at all, and an allowed
    `start_answer(plan, n)`. A task that already ran (done, waiting, failed, stopped) is never
    offered: a failure goes to the person, and a done task is never started twice.
- Produces, in `model.rs`: `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] pub struct DoneMark { pub task: TaskRun, pub next: Option<u32> }`:
  what the Planner's mark answers; MCP only, no route serves it. Exported from `lib.rs` beside `TaskRun`.
- The rule (Mohammed, 2026-11-11): after the Planner marks a task done, it starts the task
  `next` names on its own, without waiting for the person.
- Produces, in `error.rs`: `NoModel` (text "no model to run the executor on: name one the
  executor's harness (claude-code) offers", code `no_model`).
- Produces, in `run_tools.rs`:
  - `task_start { workflow_id: WorkflowId, task_number: u32, model: Option<String> }`.
    `model`'s doc: "A model the executor's harness (claude-code) offers; leave it out for the
    task's suggested model." Description: "Start an executor on one task of an approved plan
    version, only when the person asked you to run it. Shadows checks the task as the Start
    button does (an approved version, its needs links done, a git folder, no overlapping task
    running) and answers the attempt, or the refusal with its code. The executor works on its
    own; Shadows tells you in this conversation when its attempt ends." Answers the `Attempt`.
  - `task_done { workflow_id: WorkflowId, task_number: u32, done: bool }`. Description: "Mark a
    task done (done true) only after you judged its committed work against every acceptance
    item, or not done (done false) to take a mark back. A done task frees the tasks whose needs
    links wait for it. With done true it answers next: the lowest-numbered task of this
    version that has never run and can start now, or null; start it with task_start." Answers
    the `DoneMark`.
- `refusal.rs`: `RunsError::NotFound`'s text becomes "not found: no such plan version or task in
  this project, or no running attempt holds this grant's task".
- `THREAD_TOOLS` gains `"task_start"` and `"task_done"` (25 tools).
- `prompt.txt`, a new paragraph after the tools list, headed `Running tasks`, with these lines:
  `- task_start: start a task of an approved plan version when the person asks you to run it. Leave model out to use the task's suggested model. A refusal names its reason (waiting, task_busy, overlap, not_git, dirty_scope); tell the person, and fix what is yours to fix.`
  `- task_done: mark a task done (done true) or not done (done false). Mark it done only after you judged its work yourself.`
  `- After you mark a task done, start the task task_done names as next at once with task_start, without waiting for the person. When next is null, tell the person nothing more can start now. Start no task that next does not name unless the person asks.`
- `crates/shadows/tests/fixtures/live.rs` gains `pub const SUBSCRIPTION_MODEL: &str = "sonnet";`,
  `pub fn subscription_adapter() -> LiveAdapter` (the launcher as `live_adapter` writes it,
  over this process's environment without any `ANTHROPIC_*`, `API_KEY_*` or
  `CLAUDE_CONFIG_DIR` name, as `subscription_fixture` in
  `crates/shadows-core/tests/runs_attempts/live.rs` does; `live_adapter` and it share one
  private launcher function), and
  `pub async fn planner_turn_on(app: &App, command: &str, prompt: &str, model: &str) -> Operation`;
  `planner_turn` becomes `planner_turn_on(app, command, prompt, PICKED)`.

- [ ] **Step 1: Write the failing tests.** In `runs_attempts/planner.rs` (no agent runs):
  - `the_planner_marks_a_task_done_and_not_done`: `mark_done_for(&grant, &f.workflow, 1, true)`
    answers `task.done == true` and `next == Some(3)` (T3: never run, no needs);
    `task(&f.workflow, 1, 6).start.allowed` is true (T6 needs T1);
    `mark_done_for(.., 1, false)` answers `task.done == false`, `next == None`, and T6's
    `start.code` is `Some("waiting")`.
  - `the_next_task_is_one_that_never_ran`: insert a `failed` attempt row for T3 (as
    `report.rs`'s `begin` inserts one, state `failed`, `ended_at` and `released_at` set); then
    `mark_done_for(.., 1, true)` answers `next == Some(4)`; mark T4 done by hand with
    `mark_done`, and the next `mark_done_for(.., 1, true)` answers `next == Some(5)`.
  - `the_planner_start_needs_a_model`: on a fresh fixture (no Planner turn recorded, no model
    remembered), `start_for(&grant, &f.workflow, 3, None)` is `Err(RunsError::NoModel)` and
    `attempts(&f.workflow, 1, 3)` is empty.
  - `the_planner_tools_refuse_a_grant_that_is_not_a_planners`: with an executor `Grant` (as in
    B-123) and with a project grant from `shadows_core::Grants::issue_project_grant_for_test`,
    `start_for`, `mark_done_for` each answer `Err(RunsError::NotFound)`; nothing is written
    (`task(.., 1).done` stays false, no attempt).
  In `crates/shadows/tests/planner_runs_live.rs` (module doc: the Planner and its executors on
  the real adapter, on the person's Claude Code subscription, reaching `/mcp` on a real port;
  CI skips it; run with `-- --ignored --test-threads=1`; it declares the `app`, `listening`,
  `live` and `plan` fixture modules as `planner_live.rs` does):
  - `live_the_planner_starts_the_task_it_was_asked_to_run`, `#[ignore = "live: needs the
    signed-in Claude Code subscription"]`: a listening app on `subscription_adapter()`; its
    project folder made a git repository with one commit (as `runs_routes/live.rs` makes it);
    an approved v1 whose T1 writes `src/**` and whose goal says "Create src/t1.rs containing
    `pub fn t1() {}` with the Write tool."; then
    `planner_turn_on(app, "live-start", "Run T1 of plan version <workflow id> with task_start, with model sonnet. Do nothing else.", SUBSCRIPTION_MODEL)`.
    Assert the turn `Completed`; the Planner's entries hold a `ToolCall` naming `task_start`;
    `runs().attempts(&v1, 1, 1)` has exactly one attempt, model `sonnet`, on a thread of kind
    `executor`. Stop it with `runs().stop` and print `live start: <state>`.
  In `mcp_tools.rs`, `the_planner_is_told_how_to_run_and_judge_tasks` also asserts
  `"start the task task_done names as next at once with task_start, without waiting for the person"`.
- [ ] **Step 2: Run them and see them fail:** `cargo test -p shadows-core --test runs_attempts the_planner_`.
- [ ] **Step 3: Implement** the Interfaces; add both tools to `mcp_server.rs`'s two lists.
- [ ] **Step 4: Run** `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test mcp_server --test mcp_tools --test planner_mcp --test runs_routes`;
  `cargo test -p shadows --test planner_live --test planner_runs_live` (builds; live ones
  ignored); live, signed in:
  `cargo test -p shadows --test planner_runs_live live_the_planner_starts -- --ignored --test-threads=1`,
  its printed line recorded in the stages entry; then architecture, codemap, contract_tests.
- [ ] **Step 5: Contracts and map.** `core-runs`: `modules.planner.rs`, `functions.start_for`
  and `mark_done_for` ("the Planner's grant checked, then `start` / `mark_done`"),
  `DoneMark` and `next_ready` ("the lowest-numbered task never run that Start allows now"),
  an obligation for the Planner's own start after a done mark tested by the two mark tests,
  `NoModel`, an agreement `[start, start_for]` "HTTP Start and the Planner's task_start run one
  method" tested by `the_planner_tools_refuse_a_grant_that_is_not_a_planners` and the live test;
  `shadows-mcp`: two tools and the NotFound text; `core-grants`: 25; `core-harness`: the
  paragraph; code-map rows for `planner.rs` and `planner_runs_live.rs` if `codemap` asks.
- [ ] **Step 6: Commit** `feat(backend): B-124 the Planner starts a task and marks it done or not done`.

## Task B-125: A finished attempt reaches its Planner; a committed one waits for its judgment

**Files:**
- Create: `crates/core-runs/src/notify.rs` (one job: telling a plan's Planner how an attempt ended)
- Modify: `crates/core-runs/src/lib.rs`, `crates/core-runs/src/watch.rs`,
  `crates/core-runs/src/store/mod.rs`, `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-mcp/src/run_tools.rs` (task_done's description),
  `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts.rs` (one renamed test),
  `crates/shadows-core/tests/runs_attempts/planner.rs`,
  `crates/shadows/tests/planner_runs_live.rs`, `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- Produces, in `lib.rs`: `async fn last_invocation(&self, thread: &ThreadId) -> Option<InvocationView>`:
  the invocation of the thread's latest operation by `created_at`, read through
  `Turns::operations`; a read failure is logged `runs.planner_unread` and is `None`.
  `suggested_model` uses it; its answers do not change.
- Produces, in `store/mod.rs`:
  `pub async fn awaiting(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<Option<(AttemptId, ThreadId)>, RunsError>`:
  the task's highest-`number` attempt when its state is `committed` and `released_at IS NULL`,
  else `None`.
- Changes `mark_done`: when `done` is true, after the mark is written,
  `awaiting(..)` → `self.release(&id, &thread)` (closes the executor's session, frees the scope).
  `done` false releases nothing. A replayed mark command releases nothing a second time
  (release is idempotent; no extra check).
- Changes `watch.rs`'s `observe`, after the outcome write, by the attempt as it now stands:
  - `Committed`: no release. When the turn ended `Completed` or `Failed`, `self.notify(&now)`.
    (A `Cancelled` turn means Stop, which released it itself.)
  - `Failed` written by this watcher (`finish` answered `true`): release, then `notify`.
  - anything else: release, as today.
- Produces, in `notify.rs`: `pub(crate) async fn notify(&self, attempt: &Attempt)`. Never fails
  the watcher: every failure is a `tracing::warn!` and a return.
  1. `read_plan(&attempt.workflow_id)`; its `written_by` must be `WrittenBy::Planner` with
     `thread_removed == false`, else log `runs.notify_no_planner` and return.
  2. `last_invocation(&thread_id)`; `None` → log `runs.notify_no_turn` and return.
  3. The prompt: for `Committed`, `COMMITTED` with the task's title, the files from
     `self.diff(&attempt.id)` as `path (+added -removed)` joined by `, ` (`none` when empty) and
     the report (`(none)` when absent); for `Failed`, `FAILED` with its reason.
  4. `self.inner.turns.queue(thread_id, SendTurn { command_id: format!("notify-{}", attempt.id), prompt, attachments: vec![], model: invocation.requested_model, mode: invocation.requested_mode, effort: invocation.requested_effort, focus: None, plan: None, client_tab: None })`;
     an error is logged `runs.notify_failed`. A waiting message on a busy Planner thread starts
     when its turn ends; on an idle one it starts now. The fixed command id makes a repeat a replay.
  The two texts, exactly (`{…}` filled in):
  ```text
  [Shadows] T{n} "{title}" of plan version {workflow}: attempt {number} committed {sha}.
  Files: {files}
  The executor's report:
  {report}
  Judge this commit against T{n}'s acceptance items before you mark it done.
  ```
  ```text
  [Shadows] T{n} "{title}" of plan version {workflow}: attempt {number} failed: {reason}
  Tell the person; start it again only if they ask.
  ```
- `task_done`'s description adds: "Marking it done also closes the executor's session."
- `prompt.txt`, `Running tasks` gains:
  `- When an executor's attempt ends, Shadows tells you in a message that starts with [Shadows]: the commit, the files it changed and the executor's report, or why it failed. Judge a commit yourself against the task's acceptance items: read the files it changed, and count anything the report says was skipped or could not run as not done. Mark the task done with task_done only when every item holds; otherwise tell the person what is missing.`
- Recovery keeps a committed attempt's scope across a restart: `RunsStore::settle_running`
  (`store/recover.rs`) frees every other held scope but not a `committed` one's. (Corrected
  2026-11-11 during the build: the first text said recovery did not change, and it released
  them.)

- [ ] **Step 1: Change and write the tests.**
  - Rename `a_committed_attempt_holds_its_scope_until_its_turn_ends` to
    `a_committed_attempt_holds_its_scope_until_it_is_marked_done`: after `f.release()`, wait
    until the attempt's operation in `turns().operations(&attempt.thread_id)` has `finished_at`;
    then `f.start("s3", 4)` is still `Err(RunsError::Overlap { task: 3 })`;
    `runs().mark_done("m1".into(), &f.workflow, 1, 3, true)`; then `f.start("s4", 4)` is `Ok`.
  - In `runs_attempts/planner.rs` (no agent; rows inserted as `report.rs`'s `begin` does):
    - `stopping_a_committed_attempt_frees_its_scope`: a `committed` row with `ended_at`, no
      operation, `released_at` NULL; `task(.., 3).start.code == Some("task_busy")`;
      `runs().stop(&id)` keeps the state `Committed`; then `start.allowed` is true and the
      Planner thread's operations are as many as before.
    - `recovery_keeps_a_committed_attempt_waiting_for_its_judgment`: the same row;
      `recover()` answers `interrupted == 0`; `task(.., 3).start.code == Some("task_busy")`.
  - In `crates/shadows/tests/planner_runs_live.rs` (B-124's file and setup: a listening app on
    `subscription_adapter()`, so the Planner's session and the executor's `task_report` reach
    the real `/mcp`), each first recording the Planner's turn with
    `planner_turn_on(app, "p1", "Answer with the word ready.", SUBSCRIPTION_MODEL)`, then
    starting T1 with `core.runs().start(..)` on `SUBSCRIPTION_MODEL` and the production first
    prompt:
    - `live_a_committed_attempt_reaches_its_planner_and_waits_for_its_judgment`: T1's goal is
      B-124's (`src/t1.rs`). Poll until the attempt is `Committed` and its operation has
      `finished_at` (10 minutes at most). Assert: `task(&v1, 1, 1).start.code ==
      Some("task_busy")`; within 3 minutes the Planner thread's entries hold one whose body
      contains the attempt's sha and `attempt 1 committed`; then
      `mark_done("m1".into(), &v1, 1, 1, true)`; `start.code` is no longer `task_busy` and
      `harness().offered(&attempt.thread_id)` is `None`. Print `live notice: <the entry's first
      line>`.
    - `live_an_attempt_without_a_commit_reaches_its_planner`: T1's goal is "Change no file and
      make no commit: answer that there is nothing to do, and stop."; the attempt ends `Failed`
      with `ended without a commit`; within 3 minutes a Planner entry contains `attempt 1
      failed: ended without a commit`; `task(&v1, 1, 1).start.allowed` is true.
  - `the_planner_is_told_how_to_run_and_judge_tasks` also asserts
    `"count anything the report says was skipped or could not run as not done"`.
- [ ] **Step 2: Run and see the renamed test and the two no-agent tests fail:**
  `cargo test -p shadows-core --test runs_attempts committed`.
- [ ] **Step 3: Implement** the Interfaces.
- [ ] **Step 4: Run** `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test runs_routes --test mcp_tools --test mcp_server`; any existing
  test there that expects a committed attempt's scope freed at its turn's end now expects it
  held until the mark (change only that expectation, and say which in the report); live, signed
  in: `cargo test -p shadows --test planner_runs_live live_a_committed_attempt_reaches -- --ignored --test-threads=1`
  and `cargo test -p shadows --test planner_runs_live live_an_attempt_without_a_commit -- --ignored --test-threads=1`,
  printed lines recorded; then architecture, codemap, contract_tests.
- [ ] **Step 5: Contracts and map.** `core-runs`: `modules.notify.rs`; the header's scope
  sentence becomes "from Start until its task is judged (marked done) or it is stopped, and for
  an attempt without a commit until its turn has ended"; `transitions` gain "turn ends after an
  accepted report: stays Committed, holds its scope, the Planner is told" and "marked done:
  the awaiting attempt's session closes and its scope is freed"; `functions.mark_done` and
  `recover` say so; obligations for the five tests; `dependencies.turns` adds "queues the
  Planner's notice". `shadows-mcp`, `core-harness`: the texts. Code-map row for `notify.rs`.
- [ ] **Step 6: Commit** `feat(backend): B-125 a finished attempt reaches its Planner, and a committed one waits for its judgment`.

## Task B-126: An addition continues the executor in its session, recorded on the task

**Files:**
- Create: `crates/core-runs/src/amend.rs` (one job: carrying an attempt on with the Planner's addition)
- Create: `crates/core-db/migrations/0055_attempt_addition.sql`
- Modify: `crates/core-runs/src/start.rs`, `crates/core-runs/src/lib.rs`,
  `crates/core-runs/src/model.rs`, `crates/core-runs/src/error.rs`,
  `crates/core-runs/src/store/mod.rs`, `crates/core-runs/src/notify.rs` (the last-addition
  line), `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-core/tests/runs_attempts.rs` (review fix R-125-2 only)
- Modify: `crates/core-db/tests/fixtures/base_migrations.sha384` and
  `crates/core-db/tests/migration_checksums.rs` (as B-96 did, if the checksum test asks),
  `crates/core-db/contract.yaml` (if it lists migrations)
- Modify: `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Modify: `api/openapi.json` (regenerated)
- Test: `crates/shadows-core/tests/runs_attempts/planner.rs`,
  `crates/shadows-core/tests/runs_attempts/live.rs`, `crates/shadows/tests/planner_runs_live.rs`,
  `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- Migration `0055_attempt_addition.sql`: a comment saying an attempt that carries on another
  in its session holds the Planner's addition, NULL for one Start began; then
  `ALTER TABLE run_attempts ADD COLUMN addition TEXT NULL;`
- `Attempt` gains, after `report`:
  `/// The Planner's addition this attempt carried on with, in the session of the attempt before it; null for an attempt Start began.`
  `#[schema(required)] pub addition: Option<String>`. `load` reads it.
- `error.rs`: `NotAwaiting { task: u32 }` (text "T{task} has no committed attempt waiting for
  your judgment", code `not_awaiting`), `EmptyAddition` (text "an addition says what is
  missing or must be extended; it cannot be empty", code `empty_addition`) and
  `AdditionsSpent { task: u32 }` (text "T{task} had its 3 additions: it is stopped and goes
  back to the person; tell them what is still missing", code `additions_spent`).
- **The cap (Mohammed, 2026-11-11): three additions per task start.** `amend.rs` holds
  `const ADDITIONS: u32 = 3;`. `store/mod.rs` gains
  `pub async fn additions_since_start(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<u32, RunsError>`:
  how many of the task's attempts, counted from the highest `number` down, carry an
  `addition` before the first that does not (the one a Start began). A new Start by the
  person or the Planner begins a new count.
- `store/mod.rs`: `pub(crate) struct Continued { pub(crate) addition: String, pub(crate) start_head: Option<String> }`
  and `pub async fn continue_attempt(&self, previous: &AttemptId, next: Continued) -> Result<Attempt, RunsError>`,
  one write: `previous` must be `committed` with `released_at IS NULL`, else
  `NotAwaiting { task }`; set its `released_at`; insert a `running` row copying its project,
  workflow, version, task, `thread_id`, model and `writes_json`, with the next `number`,
  `command_id` `amend-<new id>`, fingerprint `amend`, `start_head` and `addition`; answer it.
  No new thread: the executor's thread carries on.
- `start.rs`: `pub(crate) async fn run_turn(&self, attempt: Attempt, prompt: String) -> Result<Attempt, RunsError>`
  is today's tail of `start`, moved, not changed: `open_executor` with the attempt's
  `TaskScope` and model (failure: `abandon` "session did not open: …"), the still-Running
  check, the turn with `prompt` in `MODE`, `set_operation`, the late Stop, `watch`. `start`
  calls it with `self.first_prompt(task)`. One rule, one place, for both.
- `amend.rs`: `pub async fn amend(&self, grant: &Grant, workflow: &WorkflowId, task: u32, addition: String) -> Result<Attempt, RunsError>`:
  `planner_plan`; trimmed `addition` empty → `EmptyAddition`; `store.awaiting(workflow,
  plan.version, task)` `None` → `NotAwaiting { task }`; when `additions_since_start` is
  already `ADDITIONS`, `self.stop(&awaiting)` (the attempt keeps its commit and state
  `Committed`; its session closes and its scope is freed) and `AdditionsSpent { task }`, with
  nothing else written; else the previous attempt; `start_head` = `Git::at(folder).head()`;
  `continue_attempt`; then `run_turn(next, ADDITION.replace(..))`. `open_executor` reopens the
  thread whether its session is live, closed for idleness or gone after a restart.
- `notify.rs`: a committed notice for an attempt whose `additions_since_start` is
  `ADDITIONS` gains, before its last line, exactly: `This attempt carried T{n}'s third and
  last addition: mark it done, or the task stops and goes back to the person.`
- **One review path.** An attempt's end reaches its judge by one road only: `watch` sees the
  turn end → `notify` hands the attempt on → the Planner judges → `task_done` (`mark_done`)
  or `task_amend` (`amend`). Nothing else marks or continues a task. `core-runs`' contract
  says so in its header, and names `notify` as the one place a reviewer would be added. No
  trait, hook or field is built for it now.
- Review fixes carried here (Hamoud's review of B-123 to B-125, 2026-11-11):
  - **R-125-1:** `task_start`'s description says "only when the person asked you to run it",
    which contradicts the done-mark rule. It becomes: "Start an executor on one task of an
    approved plan version: one the person asked you to run, or the task task_done named as
    next." The rest of the description stays.
  - **R-125-2:** `a_committed_attempt_holds_its_scope_until_it_is_marked_done` checks the
    held scope as soon as the turn's operation has `finished_at`, before the watcher has
    necessarily acted, so the check would pass even if the watcher still released it. Before
    `f.start("s3", 4)`, sleep 500 ms (twenty of the watcher's 25 ms polls).
  `ADDITION`, exactly:
  ```text
  The Planner reviewed your commit {sha} for task T{n} and asks for more:

  {addition}

  Do it inside the task's writes. When it is done, commit your own files again with git and call task_report with the new commit's sha and an honest report of what you did, ran and found.
  ```
- `run_tools.rs`: `task_amend { workflow_id: WorkflowId, task_number: u32, addition: String }`;
  `addition`'s doc: "What is missing or must be extended, as the executor should act on it."
  Description: "Send the executor of a task you judged what is missing or must be extended.
  The task's committed attempt must be waiting for your judgment (not marked done). The
  executor carries on in its own session as a new attempt, and the addition is recorded on the
  task; the plan version does not change and needs no approval. Shadows tells you again when it
  ends." Answers the new `Attempt`.
- `THREAD_TOOLS` gains `"task_amend"` (26 tools).
- `prompt.txt`: in B-125's line, "otherwise tell the person what is missing." becomes
  "otherwise send task_amend with what is missing or must be extended." and `Running tasks`
  gains: `- task_amend: the executor carries on in its own session; the addition is recorded on the task, and the plan version does not change. A task takes at most three additions after its Start; a fourth is refused, the task is stopped, and it goes back to the person.`
  The prompt test also asserts `"A task takes at most three additions after its Start"`.

- [ ] **Step 1: Write the failing tests.**
  - `runs_attempts/planner.rs`, `an_addition_needs_a_committed_attempt_waiting` (no agent):
    with no attempt, `amend(&grant, &f.workflow, 3, "x".into())` is `NotAwaiting { task: 3 }`;
    with a `running` row, the same; `amend(.., "   ".into())` is `EmptyAddition`; with an
    executor grant, `NotFound`; no row was added (`attempts` length unchanged).
  - `runs_attempts/planner.rs`, `the_fourth_addition_stops_the_task` (no agent): rows for T3
    numbered 1 to 4, as `report.rs`'s `begin` inserts them: 1 with no `addition`, 2 to 4 each
    with one, all `committed` with `ended_at`, 1 to 3 released, 4 not. `amend(.., "more".into())`
    is `AdditionsSpent { task: 3 }`; attempt 4 is still `Committed`; `task(.., 3).start.allowed`
    is true; `attempts(.., 3)` still has 4 rows. Then a row 5 with no `addition` and a row 6
    with one, `committed`, not released: `additions_since_start` is 1, and `amend` with an
    executor grant is still `NotFound` (the cap does not open the tool to others).
  - `runs_attempts/live.rs`, `live_an_addition_continues_the_executor_in_its_session`
    (subscription): as B-125's committed test up to the commit's report and the turn's end; then
    `amend(&planner_grant, &f.workflow, 3, "Add a second line with the word B to the file you created in this task.".into())`
    answers number 2, the same `thread_id`, `addition` equal to that text, `Running`; wait for
    a HEAD after the first commit; report it with the executor grant and "Added B."; wait for the
    turn's end. Assert: attempt 1 is still `Committed` with its first sha; attempt 2 is
    `Committed` with another sha; `git show <second>:backend/app/note.txt` holds `A` and `B`;
    `attempts(.., 3)` lists numbers `[2, 1]`; then `mark_done(.., true)` and
    `amend(.., "more".into())` is `NotAwaiting { task: 3 }`. Print `live addition: <second sha>`.
  - `planner_runs_live.rs`, `live_the_planner_runs_judges_and_amends_a_task` (subscription, the
    slice's acceptance): the setup of B-124's live test; one Planner turn with "Run T1 of plan
    version <id> with task_start, model sonnet. When Shadows tells you T1's attempt 1
    committed, call task_amend for T1 with the addition: add the line `// two` at the end of
    src/t1.rs. When Shadows tells you T1's attempt 2 committed, read src/t1.rs and call
    task_done for T1 with done true."; poll `runs().task(&v1, 1, 1)` up to 15 minutes until
    `done`. Assert two attempts, both `Committed`, on one thread; attempt 2's `addition`
    contains `// two`; `git show <attempt 2 sha>:src/t1.rs` contains `pub fn t1` and `// two`;
    the Planner's entries hold `ToolCall`s naming `task_start`, `task_amend` and `task_done`.
    Print `live journey: <attempt 1 sha> <attempt 2 sha>`.
  - `the_planner_is_told_how_to_run_and_judge_tasks` also asserts
    `"otherwise send task_amend with what is missing or must be extended."`.
- [ ] **Step 2: Run and see them fail:** `cargo test -p shadows-core --test runs_attempts an_addition`.
- [ ] **Step 3: Implement** the migration, the Interfaces and the tool; add it to
  `mcp_server.rs`'s lists; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` (the only
  change: `Attempt.addition`).
- [ ] **Step 4: Run** `cargo test -p core-db --test migration_checksums`;
  `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test runs_routes --test mcp_server --test mcp_tools --test planner_mcp --test openapi`;
  live, signed in: `cargo test -p shadows-core --test runs_attempts live_an_addition -- --ignored --test-threads=1`
  and `cargo test -p shadows --test planner_runs_live live_the_planner_runs_judges -- --ignored --test-threads=1`,
  printed lines recorded; then architecture, codemap, contract_tests.
- [ ] **Step 5: Contracts and map.** `core-runs`: `modules.amend.rs`, `functions.amend` and
  `run_turn`, `additions_since_start`, `Attempt.addition`, the three errors, a transition
  "an addition: the waiting attempt is released into a new Running attempt on the same
  thread" and "a fourth addition: the waiting attempt is stopped, the task goes back to the
  person", obligations for the two no-agent tests, the one-review-path header note, an
  agreement
  `[start, amend]` "one `run_turn` opens the session and sends the turn" tested by the live
  addition test; `core-db`: 0055; the tool lists; code-map row for `amend.rs`.
- [ ] **Step 6: Commit** `feat(backend): B-126 an addition continues the executor in its session, recorded on the task`.

## Task B-127: A task stopped on the cap leaves a lessons record

Mohammed, 2026-11-11: when a task stops on the cap, Shadows keeps a JSON record that can be
read to improve Shadows: the plan and version, the task, each addition's text (what was
missing), what stopped the work, what the plan should have said so the executor need not have
come back, and the commands the executor ran.

**Owner:** Runs. It owns the attempts, their additions and the cap; the record is about one
task's run, as the done marks are. It reads the executor's entries through Threads, which
`Runs::new` now receives from `AppCore` (a service gets the services it calls through its
constructor).

**What Shadows has for the commands.** Every tool the executor ran is a `ToolCall` entry on its
executor thread (`Threads::entries`, in ordinal order): `body` is the title the adapter gave the
call, and `card` is a `ToolCallCard` with `kind`, `status` and `error`. For a shell call
(`kind` `execute`) the title is the command line as Claude Code shows it. The raw input and the
output are not stored, so the record holds each shell call's title, status and error, and no
output. All of a chain's attempts share one thread, so the commands are the thread's, in order,
not split by attempt.

**Files:**
- Create: `crates/core-runs/src/lessons.rs` (one job: the record a task leaves when it stops on
  the cap), `crates/core-runs/src/store/lessons.rs`,
  `crates/core-db/migrations/0056_run_lessons.sql`
- Modify: `crates/core-runs/src/lib.rs` (`Inner.threads`, `Runs::new`),
  `crates/core-runs/src/model.rs`, `crates/core-runs/src/error.rs`,
  `crates/core-runs/src/amend.rs`, `crates/core-runs/src/store/mod.rs`,
  `crates/core-runs/Cargo.toml` (only if the entry or card type needs a dependency it does not
  list), `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-core/src/app.rs` (pass `threads.clone()` to `Runs::new`)
- Modify: `crates/core-db/tests/fixtures/base_migrations.sha384`,
  `crates/core-db/tests/migration_checksums.rs` (if asked), `crates/core-db/contract.yaml` (if it
  lists migrations), `crates/shadows/tests/architecture/table-owners.txt` (`run_lessons`, runs)
- Modify: `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/lessons.rs` (new module),
  `crates/shadows-core/tests/runs_attempts.rs` (module line),
  `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- Migration `0056_run_lessons.sql`, with a comment saying one row is the record a task left
  when it stopped on the cap, keyed by the attempt that was stopped:
  `CREATE TABLE run_lessons (attempt_id TEXT PRIMARY KEY, project_id TEXT NOT NULL, workflow_id TEXT NOT NULL, version INTEGER NOT NULL, task INTEGER NOT NULL, record_json TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);`
  and an index on `(workflow_id, version, task)`.
- `model.rs`, all `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]`,
  no `utoipa` (no route serves them):
  - `pub struct TaskLesson { pub workflow_id: WorkflowId, pub version: i64, pub task: u32, pub title: String, pub attempts: Vec<LessonAttempt>, pub commands: Vec<LessonCommand>, pub stopped_by: Option<String>, pub plan_should_have_said: Option<String>, pub recorded_at: String }`;
    the two Planner fields are `null` until it writes them.
  - `pub struct LessonAttempt { pub number: u32, pub addition: Option<String>, pub commit: Option<String>, pub report: Option<String> }`:
    the chain from the Start to the stopped attempt, oldest first; `addition` is what was missing.
  - `pub struct LessonCommand { pub command: String, pub status: String, pub error: Option<String> }`.
- `error.rs`: `NoLesson { task: u32 }` (text "T{task} has no record of a stop on the cap",
  code `no_lesson`) and `EmptyLesson` (text "stopped_by and plan_should_have_said must both say
  something", code `empty_lesson`).
- `store/lessons.rs`:
  `put_lesson(&self, attempt: &AttemptId, project: &ProjectId, lesson: &TaskLesson) -> Result<(), RunsError>`
  (insert, or replace the row of the same attempt);
  `latest_lesson(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<Option<(AttemptId, ProjectId, TaskLesson)>, RunsError>`
  (by `created_at`, then `attempt_id`);
  `lessons(&self, project: &ProjectId) -> Result<Vec<TaskLesson>, RunsError>` (by `created_at`).
- `lessons.rs`:
  - `pub(crate) async fn record_lesson(&self, plan: &Plan, stopped: &Attempt)`: the chain
    (the task's attempts from the newest down to and including the first with no `addition`,
    then oldest first), the commands from `self.inner.threads.entries(&stopped.thread_id)`
    (each `ToolCall` whose card's `kind` is `execute`; `command` the entry's body, `status` the
    card's status as its wire text, `error` the card's), `title` from the plan, both Planner
    fields `None`, `recorded_at` now. A failure is logged `runs.lesson_failed` and never
    changes the cap's answer; success logs `runs.lesson_recorded` with the task.
  - `pub async fn write_lesson(&self, grant: &Grant, workflow: &WorkflowId, task: u32, stopped_by: String, plan_should_have_said: String) -> Result<TaskLesson, RunsError>`:
    `planner_plan`; both trimmed and non-empty, else `EmptyLesson`; `latest_lesson`, else
    `NoLesson { task }`; sets the two fields, stores, answers the record. Writing again
    replaces the two fields.
  - `pub async fn lessons(&self, project: &ProjectId) -> Result<Vec<TaskLesson>, RunsError>`:
    every record of the project, for reading; no route or tool serves it yet.
- `amend.rs`: on the cap, after `self.stop(..)`, `self.record_lesson(&plan, &stopped)` before
  answering `AdditionsSpent`; that error's text gains "; then call task_lessons with what
  stopped the work and what the plan should have said".
- `run_tools.rs`: `task_lessons { workflow_id: WorkflowId, task_number: u32, stopped_by: String, plan_should_have_said: String }`.
  Docs: `stopped_by` "What kept the work from being done, in a few sentences: the cause, not
  the additions again."; `plan_should_have_said` "What the task should have said so the
  executor would not have needed an addition: its acceptance, contract, setup or reads."
  Description: "After a task was stopped on its three additions, record what stopped the work
  and what the plan should have said. Shadows keeps it with the task's additions and the
  commands the executor ran, to improve how plans are written." Answers the `TaskLesson`.
- `THREAD_TOOLS` gains `"task_lessons"` (27 tools).
- `prompt.txt`, `Running tasks` gains: `- task_lessons: when task_amend is refused because the task had its three additions, call task_lessons once: what stopped the work, and what the plan should have said so the executor would not have needed an addition. Then tell the person.`

- [ ] **Step 1: Write the failing tests** in `runs_attempts/lessons.rs` (no agent; attempt rows
  as in B-126's cap test, on one executor thread made with `Threads::create_executor_in`;
  entries appended with `Threads::append_thread_entry`, the test-only write: two `ToolCall`
  entries with cards of kind `execute`, `go test ./...` completed and `docker compose up -d`
  failed with error `docker: not found`, and one of kind `edit`):
  - `a_task_stopped_on_the_cap_leaves_a_record`: the fourth `amend` is `AdditionsSpent`; then
    `lessons(&f.project)` has one record for T3: `attempts` numbered 1 to 4 with the additions
    as inserted (`None` for 1); `commands` exactly the two `execute` calls in order, the second
    with its error; both Planner fields `None`.
  - `the_planner_writes_what_the_plan_should_have_said`: then `write_lesson(&grant, .., 3,
    "Docker is not installed here.", "T3 needs Docker; the setup plan should install or check it.")`
    answers the record with both fields; `lessons` reads them back; `write_lesson` with `"  "`
    is `EmptyLesson`; for T4 (no record) `NoLesson { task: 4 }`; with an executor grant,
    `NotFound`.
  - The prompt test also asserts `"call task_lessons once"`.
- [ ] **Step 2: Run them and see them fail:** `cargo test -p shadows-core --test runs_attempts lesson`.
- [ ] **Step 3: Implement** the migration, the Interfaces, the wiring in `app.rs` and the tool;
  add it to `mcp_server.rs`'s lists.
- [ ] **Step 4: Run** `cargo test -p core-db --test migration_checksums`;
  `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test mcp_server --test mcp_tools --test planner_mcp --test openapi`
  (no change to `api/`); `cargo test -p shadows --test architecture --test codemap --test contract_tests`.
- [ ] **Step 5: Contracts and map.** `core-runs`: `owns_tables` gains `run_lessons`;
  `dependencies.threads` adds "reads the executor's entries for a lessons record"; `new` takes
  `threads`; `modules.lessons.rs` and `store/lessons.rs`; the functions and shapes above;
  obligations for the two tests. `core-db`: 0056. The tool lists. Code-map rows for both new
  files.
- [ ] **Step 6: Commit** `feat(backend): B-127 a task stopped on the cap leaves a lessons record`.
