# Executors, slices 0 and 1: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. In this repository a task is dispatched by its stages
> entry (B-n, F-n) to `backend-builder` or `frontend-builder`.

**Goal:** Mohammed opens a task of an approved plan on its own page, chooses a model, presses
Start, and an executor agent does that task inside its write scope and commits it; he reads the
commit on the Diff tab and marks the task done by hand.

**Architecture:** A new service, Runs (`core-runs`), owns attempts and git. An executor session
is a Thread of kind `executor`, opened by Harness the way a Planner session is, with its own
built-in instructions, tool list, permission policy and a grant scoped to its task. The executor
reads its task through the existing `task_context` and commits through a new `task_report` MCP (named `task_commit` in the first draft; B-95 at `617d806f` renamed it)
tool. The web gets a task page with tabs.

**Tech Stack:** Rust 1.94+, SQLx 0.9/SQLite, Tokio, ACP over the Claude adapter 0.101.0,
`shadows-process` for git, React/TypeScript with TanStack Router.

**Spec:** [Executors design](../specs/2026-11-08-executors-design.md), slices 0 and 1 (§10).
Slices 2 (critic), 3 (ladder) and 4 (together) get their own plans after slice 1 runs in the
browser.

## Global constraints

- Every task starts at `crates/shadows-core/src/app.rs` (CLAUDE.md strict rule). Runs is the one
  new owner; no other owner is added. A change to a module updates its `contract.yaml` and the code
  map (`docs/codebase/README.md`) in the same commit.
- Nothing is written into the person's project folder except the task's own code (D12). The
  executor instructions live in `crates/core-harness/src/executor.md`, compiled in.
- Executor sessions use `setting_sources: []` and `strictMcpConfig: true`, as the Planner's do:
  the project's `.claude/` settings, agents and hooks are not loaded.
- Commits go to the branch the folder has checked out; Shadows never switches, creates or pushes
  a branch (§12.2).
- Live keys: only `OPENROUTER` and `API_KEY_MINIMAX`, read with a loop, never printed or committed.
- Focused tests per task; the full gate runs in CI. No `cargo clean`, no second `target/`.
- Migrations: take the next unused number on `main` at dispatch. The Codex branch plans `0050`;
  whichever lands second renumbers and records it in its stages entry.
- Stage numbers: B-89 to B-94 and F-98 (B-85 to B-88 and F-97 are reserved by the Codex plan).

## Review focus

1. **Start pressed twice, or a replayed request**: one attempt, one session. The second Start with
   the same `command_id` returns the first attempt; a new `command_id` while one runs is refused
   `TaskBusy`. Pinned in B-91.
2. **Daemon restart mid-attempt**: the attempt reads `Interrupted`, its grant is revoked, its
   thread holds no session, and nothing restarts on its own (§12.3). Pinned in B-91.
3. **Executor threads in the conversation list**: the sidebar and `Threads::list` show planning
   threads only; executor threads are reached from their task. Pinned in B-90.
4. **The person staged a file of their own** before the commit: it stays staged and uncommitted;
   a changed file outside `writes` is left in the folder, named in the result, never committed.
   Pinned in B-92.
5. **The plan changed between page load and Start**: a superseded or archived version, or a `needs`
   task not marked done, refuses Start with the reason named. Pinned in B-91.

---

### B-89: Probe the write boundary (slice 0) — done 2026-11-08

Run by the controller on Claude Code 2.1.319 through the pinned adapter 0.101.0:
[`docs/evidence/2026-11-08-executor-write-boundary-probe.md`](../../evidence/2026-11-08-executor-write-boundary-probe.md).
Mohammed's ruling on reading it: the executor is built as the Planner is, with only the tools
and permissions its task gives it; shell commands are not policed; the commit takes only paths
in `writes` (design §7). A last run of that exact setup confirmed it: edits in `backend/` ran
unasked, a Write to `web/` was refused, Bash ran. B-90 and B-92 below are written to that.

---

### B-90: The executor session

**Files:**
- Create: `crates/core-harness/src/executor.md`, `crates/core-harness/src/executor.rs`,
  `crates/shadows-core/tests/executor_session.rs`
- Modify: `crates/core-threads/src/model.rs` (`ThreadKind`), `src/store/` (kind column read and
  written; `list` and `list_for_profile` return `planning` threads only), `contract.yaml`;
  `crates/core-db/migrations/00NN_executor_sessions.sql` (next unused number):
  `threads.kind TEXT NOT NULL DEFAULT 'planning'`, and `mcp_grant` rebuilt (SQLite cannot alter a
  CHECK) with `kind IN ('thread','project','executor')` and nullable `scope_workflow TEXT`,
  `scope_version INTEGER`, `scope_task INTEGER`, every existing row and index kept;
  `crates/core-db/contract.yaml`, the storage-contract schema and checksum tests;
  `crates/core-grants/src/store/mod.rs` (startup revocation and per-grant revoke cover
  `executor` as they cover `thread`);
  `crates/core-harness/src/setup.rs` (setup by kind), `src/sessions.rs`, `contract.yaml`;
  `crates/shadows-agent/src/acp/session.rs` (`SessionSetup.permissions`), `crates/shadows-agent/contract.yaml`;
  `crates/core-grants/src/model.rs`, `src/tools.rs`, store (the executor kind and its scope),
  `contract.yaml`; `crates/shadows-mcp/src/server.rs` (a third router for the executor kind),
  `contract.yaml`; `crates/core-plans/src/task_context.rs` and its store query (the task check),
  `crates/core-plans/contract.yaml`

**Interfaces:**
- Consumes: design §7 (the tool list, the task's permission rules, mode `default`). Nothing from Runs.
- Produces:
  - `pub enum ThreadKind { Planning, Executor }` (`as_str`: `planning`, `executor`) on `PlanningThread.kind`.
  - `Threads::create_executor_in(conn, project: &ProjectId, title: String, harness: &str) -> ThreadId`.
  - `GrantKind::Executor` (`as_str`: `executor`), beside `Thread` and `Project`.
    `Grants::tools_for(GrantKind::Executor)` returns exactly `task_context`, `where_is`,
    `who_uses` and `outline`; B-92 adds the commit tool when it implements it (`task_commit` then; `task_report` since B-95 at `617d806f`). `server.rs` builds
    its router from that list, as it does for the other two kinds. Every other tool is absent.
  - Startup revocation (`revoke_thread_grants`, called in `AppCore::start`) revokes live
    `executor` grants with the `thread` ones: every executor's adapter died with the old daemon.
    Closing an executor session revokes its grant, as closing a Planner session does.
  - `pub struct TaskScope { workflow: String, version: i64, task: u32 }` in `core-grants`, opaque
    there: the workflow is a string, so `core-grants` imports nothing from `core-plans` (which
    already depends on it). `Grants::issue_executor_grant_in(conn, project: &ProjectId,
    thread: &ThreadId, scope: TaskScope) -> (Grant, Token)`; `Grant.scope: Option<TaskScope>`.
  - `Plans::task_context` interprets the scope: for an executor grant, `workflow`, `version` and
    `task` must equal the scope's, else `PlansError::OutsideGrant`. Project and thread grants keep
    today's project-level check.
  - `Harness::open_executor(thread: &ThreadId, root: &Path, writes: &[String], grant: TaskScope,
    model: String)`: the session opens with `EXECUTOR_TOOLS` (`Read, Grep, Glob, Edit, Write,
    MultiEdit, Bash, TodoWrite`), its grant's MCP server pre-approved as the Planner's is,
    `executor.md` appended, `setting_sources: []`, `strictMcpConfig: true`, and the task's rules
    in `_meta.claudeCode.options.settings.permissions.allow`: `Read`, `Grep`, `Glob`, `TodoWrite`,
    `Bash`, and `Edit(p)`, `Write(p)`, `MultiEdit(p)` for each pattern `p` of `writes`. Its policy
    is `PermissionPolicy::Refuse`, as the Planner's. The mode is set to `default` explicitly.
  - `SessionSetup` gains `permissions: Vec<String>` (the allow rules), sent as above; `None` of
    it changes for the Planner.

- [ ] **Step 1: Write the failing tests** in `crates/shadows-core/tests/executor_session.rs`
  (fake-acp):
  - `an_executor_session_opens_with_its_instructions_tools_and_rules`: the setup's append starts
    with `executor.md`, `tools == EXECUTOR_TOOLS`, `setting_sources` empty, the allow rules hold
    `Edit(backend/**)`, `Write(backend/**)`, `MultiEdit(backend/**)` and `Bash` for a task whose
    `writes` is `backend/**`, the policy is `Refuse`, and the mode is set to `default`.
  - `a_request_outside_the_rules_is_refused`: fake-acp asks to write `web/a.ts` → refused, and
    the refusal is a thread entry.
  - `an_executor_grant_reads_only_its_own_task`: `task_context` for its task → Ok; another task, or
    the same task in another version → `OutsideGrant`.
  - `an_executor_grant_lists_only_its_tools`: `tools/list` returns exactly `task_context`,
    `where_is`, `who_uses`, `outline`.
  - `an_executor_grant_from_before_a_restart_is_refused`: issue an executor grant, restart the
    core over the same database → a call with its token is refused as revoked.
  - `existing_grants_survive_the_migration`: a database with `thread` and `project` grants before
    the migration → the same rows, kinds and tokens after it.
  - `an_executor_grant_cannot_write_the_plan`: a plan-writing tool → unknown tool.
  - `executor_threads_are_not_listed_as_conversations`: `Threads::list` returns the planning thread only.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test executor_session`. Expected: FAIL.
- [ ] **Step 3:** Write `executor.md` from the design §8 (stay in `writes`; build nothing you do
  not own; no mocks for another task's code; a test failing on another task's missing code is
  expected and is named in your report; if the plan is wrong, stop and say why; read your task
  with `task_context` first; ask `where_is` before searching; commit, then report with `task_report`).
- [ ] **Step 4:** Implement the kind, the setup branch and the grant scope.
- [ ] **Step 5:** Run the new test and `cargo test -p shadows-core --test planner_isolation --test sessions --test thread_session`
  and `cargo test -p shadows --test planner_mcp --test mcp_tools`. Expected: PASS; Planner behaviour unchanged.
- [ ] **Step 6: Commit** `feat(harness): executor sessions scoped to one task`.

---

### B-91: Runs, attempts and Start/Stop

**Files:**
- Create: `crates/core-runs/Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/mod.rs`,
  `src/start.rs`, `src/ready.rs`, `src/recover.rs`, `contract.yaml`, `tests/attempts.rs`
- Create: `crates/core-db/migrations/00NN_runs.sql` (next unused number)
- Modify: `crates/shadows-core/src/app.rs` (owner row, field, accessor, `start` wiring, recovery
  before serving), `crates/shadows-core/Cargo.toml`, `crates/core-db/contract.yaml`,
  `crates/core-db/tests/storage_contract/` (schema list, checksum count),
  `docs/codebase/README.md`

**Interfaces:**
- Consumes: `Plans` (frozen version read, `TaskContent`, links), `Threads` (create an executor
  thread, B-90's `Threads::create_executor_in`), `Harness` (open and close the session, B-90), `Turns`
  (send the first turn and cancel it), `Operations` (command replay, recovery).
- Produces:
  - `pub struct Runs` with `Runs::new(storage, plans, threads, harness, turns, operations)`.
  - `pub struct AttemptId(String)`; `pub enum AttemptState { Running, Committed, Stopped, Interrupted, Failed }`
    (`as_str`: `running`, `committed`, `stopped`, `interrupted`, `failed`).
  - `pub struct Attempt { id, project_id, workflow_id, version: i64, task: u32, number: u32,
    thread_id: ThreadId, operation_id: OperationId, model: String, state: AttemptState,
    commit: Option<String>, reason: Option<String>, started_at: String, ended_at: Option<String> }`.
    `operation_id` is the operation of the attempt's turn; Runs reads its end from Operations.
  - **Transitions** (the only ones; each sets `ended_at` when it leaves `Running`):

    | From | Event | To | `reason` |
    |---|---|---|---|
    | — | Start accepted | `Running` | — |
    | `Running` | session fails to open | `Failed` | `session did not open: {error}`; grant revoked |
    | `Running` | `task_report` accepted | `Committed` | — |
    | `Running` | turn ends, no commit | `Failed` | `ended without a commit` |
    | `Running` | turn fails | `Failed` | the turn's failure |
    | `Running` | Stop confirmed | `Stopped` | — |
    | `Running` | found at startup, commit with its trailer in git | `Committed` | — |
    | `Running` | found at startup, no such commit | `Interrupted` | — |
    | `Committed` | turn ends or is stopped | `Committed` | unchanged |

    One commit per attempt; a second `task_commit` (before B-95 at `617d806f`) is `AlreadyCommitted`.
  - `pub struct TaskRun { task: u32, done: bool, done_at: Option<String>, latest: Option<Attempt> }`.
  - `Runs::start(&self, command: CommandId, workflow: &WorkflowId, version: i64, task: u32, model: String) -> Result<Attempt, RunsError>`
  - `Runs::stop(&self, attempt: &AttemptId) -> Result<Attempt, RunsError>` (waits for the turn's
    confirmed end, as the Planner's Stop does)
  - `Runs::task(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<TaskRun, RunsError>`
  - `Runs::attempts(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<Vec<Attempt>, RunsError>` (newest first)
  - `Runs::mark_done(&self, command: CommandId, workflow: &WorkflowId, version: i64, task: u32, done: bool) -> Result<TaskRun, RunsError>`
  - `Runs::recover(&self) -> Result<RecoverReport, RunsError>` with
    `RecoverReport { committed: u32, interrupted: u32 }`. In B-91 every `Running` row found at
    startup becomes `Interrupted`; B-92 adds the git lookup that turns a row whose commit exists
    into `Committed` first.
  - **Overlap, decided in the Start transaction:** Start reads the project's attempts that **hold
    their scope**, and refuses `Overlap { task: u32 }` when one's `writes` may intersect this
    task's. An attempt holds its scope while its session can still write: from Start until its
    turn's operation has ended and its session is closed, whatever its state (a `Committed`
    attempt whose turn still runs holds it). Runs stores `released_at`, set when the turn ends,
    Stop is confirmed, the session fails to open, or recovery finds it; the check reads
    `released_at IS NULL`. Two
    patterns may intersect unless their literal prefixes (each pattern cut before its first glob
    character, then at its last `/`) are disjoint: neither is a path prefix of the other. Unproven
    means intersecting. The check and the insert are one write transaction, so two concurrent Starts
    cannot both pass.
    **One matcher:** `core-runs/src/scope.rs` owns pattern matching for the overlap check and the
    commit, built on `globset` with `literal_separator(true)`; its commit names `globset` as an
    infrastructure library. Its tests carry over the earlier project's
    (`shadow-verify/src/scope.rs`): `*` does not cross `/`, an empty `writes` admits nothing, an
    invalid pattern is an error, Windows separators are normalised.
  - `RunsError`: `NotFrozen`, `Superseded`, `Archived`, `Waiting { tasks: Vec<u32> }`, `TaskBusy`,
    `Overlap { task: u32 }`, `CommandConflict`, `NotFound`, `Storage`. B-92 adds `NotGit`,
    `DirtyScope`, `LinkInScope`, `AlreadyCommitted`, `NothingToCommit`.

- [ ] **Step 1: Write the failing tests** in `crates/core-runs/tests/attempts.rs`:
  - `start_opens_one_attempt_on_an_executor_thread`: start T1 → `state == Running`, `number == 1`,
    the thread's kind is `executor`, the model is the one given.
  - `the_same_command_returns_the_same_attempt`: start twice with one `command_id` → equal ids, one row.
  - `a_second_start_while_running_is_busy`: new `command_id` → `Err(TaskBusy)`.
  - `start_waits_for_needs_links`: T3 `needs` T1, T1 not done → `Err(Waiting { tasks: vec![1] })`;
    after `mark_done(T1, true)` → Ok.
  - `start_refuses_a_draft_a_superseded_or_an_archived_version`: each → its error.
  - `stop_ends_the_attempt_and_closes_its_session`: `state == Stopped`, `ended_at` set, no live session.
  - `a_session_that_fails_to_open_fails_the_attempt`: fake-acp refuses `session/new` → `Failed`,
    reason starts `session did not open`, grant revoked, a new Start is allowed.
  - `a_turn_that_ends_without_a_commit_fails_the_attempt`: fake ends the turn → `Failed`,
    reason `ended without a commit`.
  - `recover_interrupts_running_attempts_without_restarting_them`: a `Running` row at startup, no
    commit in git → `Interrupted`, grant revoked, no turn started; `interrupted == 1`.
  - `overlapping_tasks_cannot_start_together`: T3 writes `backend/app/**`, T4 writes
    `backend/app/Models/**`; T3 running → T4 `Err(Overlap { task: 3 })`.
  - `disjoint_tasks_start_together`: T3 `backend/app/**`, T5 `web/**` → both `Running`.
  - `a_committed_attempt_holds_its_scope_until_its_turn_ends`: T3 commits while its turn is still
    running → T4 (overlapping) `Err(Overlap { task: 3 })`; after T3's turn ends → T4 starts.
  - `two_concurrent_starts_of_overlapping_tasks_admit_one`: start T3 and T4 at once (`tokio::join!`)
    → exactly one `Running`, the other `Overlap`.
- [ ] **Step 2:** Run `cargo test -p core-runs --test attempts`. Expected: fails to compile (no crate).
- [ ] **Step 3:** Migration: tables `run_attempts` (columns of `Attempt`, plus `released_at`,
  `command_id`, `fingerprint`; unique `(workflow_id, version, task, number)`) and `run_task_marks`
  (`workflow_id, version, task, done, done_at, command_id`). Ordering by `number`, never `rowid`.
- [ ] **Step 4:** Implement the crate in its fixed shape (`model.rs` pure; SQL only in `store/`).
  The ready rule (`ready.rs`) reads the version's links and each `needs` target's mark.
  `start` creates the executor thread (B-90), records the attempt, opens the session and sends the
  first turn with the prompt `Do task T{n}. Read it with task_context first.` in one operation.
- [ ] **Step 5:** Wire `Runs` in `AppCore` with its owner-table row; call `runs.recover()` in
  `AppCore::start` after `operations.recover`, before serving.
- [ ] **Step 6:** Run `cargo test -p core-runs --test attempts` and
  `cargo test -p core-db embedded_migrations_match_the_base_catalog`, then
  `cargo test -p shadows --test architecture --test codemap --test contract_tests`. Expected: PASS.
- [ ] **Step 7: Commit** `feat(runs): attempts with start, stop and recovery`.

---

### B-92: Git and the commit check

Removed by B-95 at `617d806f`: `Runs::commit`, its commit queue, the temporary index and the Shadows-Attempt trailer. `Runs::report` and `task_report` replace them. The lines below describe the first build.

**Files:**
- Create: `crates/core-runs/src/git.rs`, `crates/core-runs/src/commit.rs`, `crates/core-runs/tests/commit.rs`
- Modify: `crates/core-runs/Cargo.toml` (`shadows-process`), `contract.yaml`;
  `crates/shadows-mcp/src/plan_tools.rs` (the tool), `contract.yaml`

**Interfaces:**
- Consumes: design §7; B-91's `Attempt`, transitions and matcher; B-90's `TaskScope` on the
  caller's grant.
- Produces:
  - `Runs::commit(&self, grant: &Grant, message: String) -> Result<CommitResult, RunsError>` with
    `CommitResult { attempt: Attempt, files: Vec<String>, left: Vec<String> }`: one commit at a
    time per project folder (a per-project `tokio::sync::Mutex`).
    1. `git status --porcelain=v1 -z -uall` lists the changed paths; ignored files never appear.
       A rename's origin is listed with its destination.
    2. A changed path matching `writes` is committed. A changed path outside `writes` is left in
       the folder, uncommitted, and returned in `left`; the Report names it.
    3. A path in `writes` whose parent folders include a link or junction refuses the commit
       (`LinkInScope { paths }`): git would read another folder's files through it.
    4. Each committed file is written as a blob (`git hash-object -w`), the tree is built in a
       temporary index (`GIT_INDEX_FILE`: `git read-tree HEAD`, `git update-index --add
       --cacheinfo` per path, `--remove` for a deleted one, `git write-tree`), and
       `git commit-tree <tree> -p <HEAD>` writes the commit with the message
       `T{n}: {title}

{message}

Shadows-Attempt: {attempt id}`.
    5. `git update-ref HEAD <new> <old>`: refused if HEAD moved, and the attempt reports it.
    6. Only those paths are updated in the folder's real index, so the person's other staged
       entries are untouched and the committed paths show clean.

    **Git first, then SQLite**: the SHA and `Committed` are written after `update-ref`; a crash
    between the two is repaired by `recover`.
  - Errors: `LinkInScope { paths }` commits nothing and fails the attempt with the paths as
    reason. `AlreadyCommitted`, `NothingToCommit`.
  - `recover` gains its git step: `git log --all --format=%H%x00%(trailers:key=Shadows-Attempt,valueonly)`
    finds the attempt's commit; a `Running` row with one becomes `Committed` with that SHA.
  - `Runs::diff(&self, attempt: &AttemptId) -> Result<Vec<FileDiff>, RunsError>` where
    `FileDiff { path: String, added: u32, removed: u32, patch: String }` from `git show --numstat`
    and `git show` of the attempt's commit; bounded at 2,000 lines per file, then truncated with a marker.
  - Start preconditions (§12.2) added to B-91's `start`: `NotGit`; `DirtyScope` when `git status
    --porcelain` lists paths inside `writes`.
  - MCP tool `task_commit { message: string }` → `{ commit, files, left }`; B-95 at `617d806f` replaced it with `task_report`.

- [ ] **Step 1: Write the failing tests** in `crates/core-runs/tests/commit.rs` (a temp git repo):
  - `commit_takes_only_the_paths_in_writes`: `backend/a.php` and `web/b.ts` are modified → the
    commit holds `backend/a.php` only; `web/b.ts` stays modified and is in `left`.
  - `a_persons_staged_file_stays_staged`: `notes.md` staged before → not in the commit, still staged.
  - `files_a_generator_wrote_are_committed`: `backend/composer.json` and
    `backend/app/Models/User.php` appear with no tool call behind them → both are committed.
  - `ignored_files_never_enter_the_commit`: `backend/.gitignore` lists `vendor/` and `.env`;
    `backend/vendor/x.php` and `backend/.env` appear → the commit holds neither.
  - `a_rename_from_outside_writes_leaves_its_origin`: `web/a.ts` renamed to `backend/a.ts` →
    `backend/a.ts` is committed, `web/a.ts` (deleted) is in `left`.
  - `a_junction_inside_writes_refuses_the_commit`: `backend/link` is a junction (a symlink off
    Windows) to `web/` → `Err(LinkInScope { paths: ["backend/link/..."] })`, HEAD unchanged.
  - `head_moving_during_the_commit_is_refused`: a test hook commits elsewhere before step 5 →
    `update-ref` refuses; nothing is lost; the attempt reports it.
  - `a_crash_between_git_and_sqlite_recovers_as_committed`: a test hook fails the SQLite write after
    git commits; `recover()` → the attempt is `Committed` with the commit's SHA, `committed == 1`.
  - `two_commits_at_once_are_serialized`: two attempts commit concurrently → two commits, each with its own paths.
  - `start_refuses_a_folder_that_is_not_git` and `start_refuses_uncommitted_changes_in_writes`.
  - `diff_returns_each_files_counts_and_patch`.
  - In `crates/shadows-core/tests/executor_session.rs`, `an_executor_grant_lists_only_its_tools`
    now expects the five names, `task_commit` included (B-95 at `617d806f` renamed it `task_report`).
- [ ] **Step 2:** Run `cargo test -p core-runs --test commit`. Expected: FAIL.
- [ ] **Step 3:** Implement `git.rs` (each git call through `shadows-process`, with the project
  folder as `cwd`; no shell) and `commit.rs`; add the MCP tool.
- [ ] **Step 4:** Run `cargo test -p core-runs` and `cargo test -p shadows --test mcp_tools`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runs): task commits through Shadows`.

---

### B-93: Runs over HTTP

**Files:**
- Create: `crates/shadows-http/src/runs.rs`, `crates/shadows/tests/runs_routes.rs`
- Modify: `crates/shadows-http/src/lib.rs` (mount), `src/openapi.rs`, `contract.yaml`;
  `api/openapi.json` (regenerated, never hand-edited); `crates/core-events/` only if the attempt
  state needs a project event (B-91's attempt changes go through the existing project stream)

**Interfaces:**
- Consumes: B-91 and B-92; the accepted shared request
  `docs/shared/requests/2026-11-08-task-run-page.md` at `c71b263d`, which fixes the read's shape.
- Produces (all under `/api/projects/{project}/workflows/{workflow}/versions/{version}/tasks/{task}`):
  - `GET` → `TaskRun` with the request's added fields: `needs` (each `{ task, title, done }`),
    `start` (`{ allowed, code, reason }`, Core's answer to "may Start run now", using the same
    checks as `Runs::start` without writing) and `suggested_model` (the plan thread's remembered
    model, §12.6). `Runs::start_answer` serves it (`Runs::can_start` was removed at `5a52a06a`).
  - `GET /attempts` → `Attempt[]`.
  - `POST /attempts` `{ command_id, model }` → `Attempt` (201); errors map `Waiting`, `TaskBusy`,
    `Overlap`, `NotGit`, `DirtyScope`, `NotFrozen`, `Superseded`, `Archived` to 409 with Core's
    `code` and reason, the same codes `start.code` uses.
  - `POST /attempts/{attempt}/stop` → `Attempt`.
  - `GET /attempts/{attempt}/diff` → `FileDiff[]`.
  - `PUT /done` `{ command_id, done }` → `TaskRun`.

- [ ] **Step 1: Write the failing tests** in `crates/shadows/tests/runs_routes.rs`: one per route,
  plus `start_while_waiting_is_409_with_the_waiting_tasks` and `a_replayed_start_returns_the_same_attempt`.
- [ ] **Step 2:** Run `cargo test -p shadows --test runs_routes`. Expected: FAIL.
- [ ] **Step 3:** Implement the routes; each reads the request, calls one Runs method, shapes the answer.
- [ ] **Step 4:** Regenerate `api/openapi.json`; run `cargo test -p shadows --test runs_routes --test openapi`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(http): task attempts routes`.

---

### F-98: The task page

**Files:**
- Create: `web/src/app/tasks/task-page.tsx`, `task-header.tsx`, `details-tab.tsx`, `run-tab.tsx`,
  `diff-tab.tsx`, `report-tab.tsx`, `use-task-run.ts`, `task-page.test.tsx`, `contract.yaml`;
  `web/src/api/runs.ts`
- Modify: `web/src/router.tsx` (route), `web/src/app/workflows/inspect-panel.tsx` (an "Open task"
  link), `web/src/test/fake-daemon.ts`, `docs/frontend/codebase/README.md`, `docs/frontend/stages.md`

**Interfaces:**
- Consumes: B-93's routes and generated types; the accepted shared request
  `docs/shared/requests/2026-11-08-task-run-page.md` at `c71b263d`. The page computes no readiness: Start's
  enabled state and reason are `start.allowed` and `start.reason`; the `needs` row is `needs`.
- Produces: route `/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task`.
  - Header: plan title link, `T{n} · {title}`, the latest attempt's state label, each `needs` task
    with ✓ when marked done, Start (disabled with Core's reason) or Stop while running, and
    "Mark done" / "Mark not done".
  - Details: goal, writes, reads, acceptance (no marks yet), executor model select starting on the
    suggested model; the critic row is absent until slice 2.
  - Run: the attempt's executor thread rendered with the conversation's existing entry components.
  - Diff: the attempt's files with counts; a file opens its patch.
  - Report: each attempt's state, model, start/end, commit and reason.

- [ ] **Step 1: Write the failing tests** in `task-page.test.tsx` (fake daemon):
  `start_sends_the_chosen_model_and_shows_running`, `start_is_disabled_with_the_waiting_tasks`,
  `stop_shows_stopped`, `a_refused_start_shows_cores_reason_and_keeps_the_model`,
  `diff_lists_files_and_opens_a_patch`, `mark_done_updates_the_header`,
  `a_retry_after_a_network_error_reuses_the_command_id`.
- [ ] **Step 2:** Run `npx vitest run src/app/tasks/task-page.test.tsx`. Expected: FAIL.
- [ ] **Step 3:** Implement with tabs (design mockup A); no layout or rule beyond Core's answers.
- [ ] **Step 4:** Run the test, `npm run typecheck`, `npm run lint`, `npm run build`,
  `npx vitest run src/codemap.test.ts src/architecture`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): task page with start, run and diff`.

---

### B-94: Run it

- [ ] **Step 0: Prepare the folder.** The Laravel test project's folder
  (`F:\shadows_projects_testing\testing` on 2026-11-08) is not a git repository and already holds
  a hand-scaffolded `backend/` with a `.env`: Start would refuse it `NotGit`, then `DirtyScope`.
  Move `backend/` aside (keep its `.env` out of the repository), `git init`, and commit an
  initial `.gitignore` so the folder is clean and T1 scaffolds `backend/` itself.
- [ ] **Step 1:** Start `shadows serve` with the branch build on that folder. Open the plan's T1 page.
- [ ] **Step 2:** Mohammed chooses a model and presses Start. Observe: the Run tab streams; the
  executor reads `task_context` first; the commit holds `backend/` paths only, and anything it
  changed elsewhere is left uncommitted and named in the Report; the Diff tab shows the commit.
- [ ] **Step 3:** Stop a second attempt mid-run; restart the daemon during a third and confirm
  `Interrupted`. Read the server log for each.
- [ ] **Step 4:** Mark T1 done; T3's Start becomes enabled.
- [ ] **Step 5:** Record the run in `docs/evidence/2026-11-08-executors-slice-1.md` and mark the
  stages entries done. Commit `docs: record executors slice 1 acceptance`.
