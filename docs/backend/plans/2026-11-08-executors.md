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
reads its task through the existing `task_context` and commits through a new `task_commit` MCP
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
  executor instructions live in `crates/core-harness/src/executor.txt`, compiled in.
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
4. **The person staged or edited a file of their own** before the commit: a staged file outside
   the attempt's paths stays staged and uncommitted; a hand edit to a file the attempt wrote
   refuses the commit (`UnprovenChange`) instead of committing it. Pinned in B-92.
5. **The plan changed between page load and Start**: a superseded or archived version, or a `needs`
   task not marked done, refuses Start with the reason named. Pinned in B-91.

---

### B-89: Probe the write boundary (slice 0)

Closes the design's §7 OPEN. A throwaway probe; no product source changes.

**Files:**
- Create: `docs/evidence/2026-11-08-executor-write-boundary-probe.md`
- Modify: `docs/backend/specs/2026-11-08-executors-design.md` §7 (replace the OPEN with the
  measured rule), `docs/backend/stages.md`
- Throwaway: a `#[ignore]` test in `crates/shadows-core/tests/` or a scratch client outside the
  repository; deleted after the evidence is written. The evidence names the commit.

**Interfaces:**
- Consumes: the existing ACP `Connection` (`crates/shadows-agent/src/acp/`), the adapter 0.101.0,
  a disposable git folder.
- Produces: the measured facts B-90 and B-92 build on, written into §7.

- [ ] **Step 1:** Open a Claude session through Shadows' ACP connection in a disposable git folder
  with tools `Read, Grep, Glob, Edit, Write, MultiEdit, Bash, TodoWrite`, mode `default`, and a
  permission handler that records every `session/request_permission` (tool kind, title, raw input,
  `locations`) and answers allow.
- [ ] **Step 2:** Prompt it to (a) edit a file inside `src/`, (b) create a file outside `src/`,
  (c) run `echo x > outside.txt` in the shell, (d) run `git status`. Record, for each, whether a
  permission request arrived and which path it named.
- [ ] **Step 3:** Repeat with the handler refusing (b) and (c). Record whether the refused write is
  absent from disk afterwards, and what the model reports.
- [ ] **Step 4:** Repeat (a) in mode `acceptEdits`. Record whether a permission request still
  arrives (expected: no; then executors must open in `default`).
- [ ] **Step 5: Ownership cases.** Record what the permission requests and `git status` show when:
  (e) the person edits `src/a.txt` by hand while the session runs, after the agent edited it;
  (f) the agent runs a command whose child process writes a file (`git checkout -- x`, a script
  that writes, a package manager); (g) the agent edits through `src/link/`, where `link` is a
  symlink, and on Windows a junction, pointing outside `src/`. For (g), record whether the request
  names the link path or the target, and whether `std::fs::canonicalize` resolves it.
  For (a) and a `Write` of a new file, record what the tool call itself carries once it completes
  (`content`, `rawInput`, `rawOutput`, a diff): whether the written bytes, or an edit Shadows can
  apply to the pre-image it recorded, can be taken **from the agent's own call**. The ownership
  hash comes from that, never from reading the file afterwards, which could pick up a hand edit
  made before the read.
- [ ] **Step 6:** Write the evidence and amend §7 with the rule chosen from it. The rule names:
  the session mode; which tools raise requests carrying a path; how a shell command is treated
  (allowlist of command prefixes, or refused); how a path is checked (canonical, links resolved,
  inside the project root and `writes`); and **how ownership is proven**: a changed file is the
  attempt's only when its current content equals the content the attempt's own last permitted
  write produced, as derived from that tool call (above). If no tool call yields its written
  content, the rule says so and B-90 stays blocked. A changed path in `writes` that the attempt cannot prove is refused at
  commit, never committed on a guess.
- [ ] **Step 7:** Delete the throwaway source. Commit the evidence, §7 and stages:
  `docs: measure the executor write boundary`.

**Verification:** each of the seven cases has a recorded observation; §7 has no OPEN block left,
or B-90 stays blocked with the measured gap stated.

---

### B-90: The executor session

**Files:**
- Create: `crates/core-harness/src/executor.txt`, `crates/core-harness/src/executor.rs`,
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
  `crates/shadows-agent/src/acp/permissions.rs` (scoped policy), `crates/shadows-agent/contract.yaml`;
  `crates/core-grants/src/model.rs`, `src/tools.rs`, store (the executor kind and its scope),
  `contract.yaml`; `crates/shadows-mcp/src/server.rs` (a third router for the executor kind),
  `contract.yaml`; `crates/core-plans/src/task_context.rs` and its store query (the task check),
  `crates/core-plans/contract.yaml`

**Interfaces:**
- Consumes: B-89's §7 rule (mode, tool list, shell treatment, path check). Nothing from Runs.
- Produces:
  - `pub enum ThreadKind { Planning, Executor }` (`as_str`: `planning`, `executor`) on `PlanningThread.kind`.
  - `Threads::create_executor_in(conn, project: &ProjectId, title: String, harness: &str) -> ThreadId`.
  - `GrantKind::Executor` (`as_str`: `executor`), beside `Thread` and `Project`.
    `Grants::tools_for(GrantKind::Executor)` returns exactly `task_context`, `where_is`,
    `who_uses` and `outline`; B-92 adds `task_commit` when it implements it. `server.rs` builds
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
  - `pub struct ExecutorScope { root: PathBuf, writes: Vec<String>, grant: TaskScope }`, passed by
    the caller to `Harness::open_executor(thread: &ThreadId, scope: ExecutorScope, model: String)`.
  - `PermissionPolicy::Scoped { root: PathBuf, writes: Vec<String> }`: canonicalize each requested
    path (links and junctions resolved); allow only when every canonical path is inside `root` and
    matches `writes`; refuse anything else, including a request with no path, except what §7 allows.
    It records, per allowed path, the hash of the content the agent's own completed tool call
    wrote, derived from the call as B-89's rule says, never from re-reading the file (the
    ownership record B-92 checks). After the attempt's `task_commit` succeeds it refuses every
    further write in that session.
  - `EXECUTOR_TOOLS` in `setup.rs`, from §7.

- [ ] **Step 1: Write the failing tests** in `crates/shadows-core/tests/executor_session.rs`
  (fake-acp):
  - `an_executor_session_opens_with_its_instructions_tools_and_policy`: the setup's append starts
    with `executor.txt`, `tools == EXECUTOR_TOOLS`, `setting_sources` empty, policy `Scoped`.
  - `an_edit_inside_writes_is_allowed_and_outside_is_refused`: fake requests `backend/a.php` → allow;
    `web/a.ts` → refuse, and the refusal is a thread entry.
  - `a_path_through_a_link_outside_writes_is_refused`: `backend/link/x.php`, where `link` points to
    `web/` → refused.
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
- [ ] **Step 3:** Write `executor.txt` from the design §8 (stay in `writes`; build nothing you do
  not own; no mocks for another task's code; a test failing on another task's missing code is
  expected and is named in your report; if the plan is wrong, stop and say why; read your task
  with `task_context` first; ask `where_is` before searching; commit with `task_commit`).
- [ ] **Step 4:** Implement the kind, the setup branch, the scoped policy and the grant scope.
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
    | `Running` | `task_commit` succeeds | `Committed` | — |
    | `Running` | turn ends, no commit | `Failed` | `ended without a commit` |
    | `Running` | turn fails | `Failed` | the turn's failure |
    | `Running` | Stop confirmed | `Stopped` | — |
    | `Running` | found at startup, commit with its trailer in git | `Committed` | — |
    | `Running` | found at startup, no such commit | `Interrupted` | — |
    | `Committed` | turn ends or is stopped | `Committed` | unchanged |

    One commit per attempt; a second `task_commit` is `AlreadyCommitted`.
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
  - `RunsError`: `NotFrozen`, `Superseded`, `Archived`, `Waiting { tasks: Vec<u32> }`, `TaskBusy`,
    `Overlap { task: u32 }`, `CommandConflict`, `NotFound`, `Storage`. B-92 adds `NotGit`,
    `DirtyScope`, `OutsideScope`, `UnprovenChange`, `AlreadyCommitted`, `NothingToCommit`.

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

### B-92: Git and `task_commit`

**Files:**
- Create: `crates/core-runs/src/git.rs`, `crates/core-runs/src/commit.rs`, `crates/core-runs/tests/commit.rs`
- Modify: `crates/core-runs/Cargo.toml` (`shadows-process`), `contract.yaml`;
  `crates/shadows-mcp/src/plan_tools.rs` (the tool), `contract.yaml`

**Interfaces:**
- Consumes: B-89's ownership rule; B-91's `Attempt` and transitions; B-90's `TaskScope` on the
  caller's grant and the ownership record kept by `PermissionPolicy::Scoped`.
- Produces:
  - `Runs::commit(&self, grant: &Grant, message: String) -> Result<Attempt, RunsError>`: one commit
    at a time per project folder (a per-project `tokio::sync::Mutex`). **It commits the bytes it
    checked, not the file as git later reads it:**
    1. For each changed path inside `writes`, read the file's bytes once; the path is the
       attempt's only when the hash of those bytes equals the ownership record.
    2. Write exactly those bytes as blobs (`git hash-object -w --stdin`).
    3. Build the tree in a temporary index (`GIT_INDEX_FILE`): `git read-tree HEAD`, then
       `git update-index --add --cacheinfo <mode>,<blob>,<path>` per path (`--remove` for a
       deleted path); `git write-tree`.
    4. `git commit-tree <tree> -p <HEAD>` with the message
       `T{n}: {title}\n\n{message}\n\nShadows-Attempt: {attempt id}`.
    5. `git update-ref HEAD <new> <old>`: refused if HEAD moved, and the attempt reports it.
    6. Update only those paths in the folder's real index to the committed blobs, so the person's
       other staged entries are untouched and the committed paths show clean.

    The working tree is never read by git during the commit. **Git first, then SQLite**: the SHA
    and `Committed` are written after `update-ref`; a crash between the two is repaired by
    `recover`.
  - Errors: `OutsideScope { paths }` (an attributed path outside `writes`), `UnprovenChange { paths }`
    (a changed path in `writes` whose content is not what the attempt last wrote: the person or
    another process changed it). Both commit nothing and fail the attempt with the paths as reason.
    `AlreadyCommitted`, `NothingToCommit`.
  - `recover` gains its git step: `git log --all --format=%H%x00%(trailers:key=Shadows-Attempt,valueonly)`
    finds the attempt's commit; a `Running` row with one becomes `Committed` with that SHA.
  - `Runs::diff(&self, attempt: &AttemptId) -> Result<Vec<FileDiff>, RunsError>` where
    `FileDiff { path: String, added: u32, removed: u32, patch: String }` from `git show --numstat`
    and `git show` of the attempt's commit; bounded at 2,000 lines per file, then truncated with a marker.
  - Start preconditions (§12.2) added to B-91's `start`: `NotGit`; `DirtyScope` when `git status
    --porcelain` lists paths inside `writes`.
  - MCP tool `task_commit { message: string }` → `{ commit, files }`.

- [ ] **Step 1: Write the failing tests** in `crates/core-runs/tests/commit.rs` (a temp git repo):
  - `commit_takes_only_the_attempts_paths`: attempt edits `backend/a.php`; another file
    `web/b.ts` is modified → the commit holds `backend/a.php` only; `web/b.ts` stays modified.
  - `a_persons_staged_file_stays_staged`: `notes.md` staged before → not in the commit, still staged.
  - `a_path_outside_writes_refuses_the_commit`: attributed `web/x.ts` → `Err(OutsideScope { paths: ["web/x.ts"] })`, HEAD unchanged.
  - `a_hand_edit_after_the_agents_write_refuses_the_commit`: the agent writes `backend/a.php`, the
    test then appends a line to it → `Err(UnprovenChange { paths: ["backend/a.php"] })`, HEAD unchanged.
  - `a_file_written_by_a_child_process_is_unproven`: `backend/gen.php` appears with no permitted
    write recorded → `Err(UnprovenChange)`.
  - `a_change_between_the_check_and_the_commit_is_not_committed`: a test hook rewrites
    `backend/a.php` after step 1 → the commit's blob for it equals the checked bytes
    (`git cat-file -p HEAD:backend/a.php`), and the later bytes remain an uncommitted change.
  - `head_moving_during_the_commit_is_refused`: a test hook commits elsewhere before step 5 →
    `update-ref` refuses; nothing is lost; the attempt reports it.
  - `a_crash_between_git_and_sqlite_recovers_as_committed`: a test hook fails the SQLite write after
    git commits; `recover()` → the attempt is `Committed` with the commit's SHA, `committed == 1`.
  - `two_commits_at_once_are_serialized`: two attempts commit concurrently → two commits, each with its own paths.
  - `start_refuses_a_folder_that_is_not_git` and `start_refuses_uncommitted_changes_in_writes`.
  - `diff_returns_each_files_counts_and_patch`.
  - In `crates/shadows-core/tests/executor_session.rs`, `an_executor_grant_lists_only_its_tools`
    now expects the five names, `task_commit` included.
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
  `docs/shared/requests/2026-11-08-task-run-page.md`, which fixes the read's shape.
- Produces (all under `/api/projects/{project}/workflows/{workflow}/versions/{version}/tasks/{task}`):
  - `GET` → `TaskRun` with the request's added fields: `needs` (each `{ task, title, done }`),
    `start` (`{ allowed, code, reason }`, Core's answer to "may Start run now", using the same
    checks as `Runs::start` without writing) and `suggested_model` (the plan thread's remembered
    model, §12.6). `Runs::can_start(&self, workflow, version, task) -> StartAnswer` serves it.
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
  `docs/shared/requests/2026-11-08-task-run-page.md`. The page computes no readiness: Start's
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

- [ ] **Step 1:** Start `shadows serve` with the branch build on the Laravel test project (a git
  folder). Open the plan's T1 page.
- [ ] **Step 2:** Mohammed chooses a model and presses Start. Observe: the Run tab streams; the
  executor reads `task_context` first; an attempted write outside `backend/**` is refused and
  shown; the commit holds `backend/` paths only; the Diff tab shows it.
- [ ] **Step 3:** Stop a second attempt mid-run; restart the daemon during a third and confirm
  `Interrupted`. Read the server log for each.
- [ ] **Step 4:** Mark T1 done; T3's Start becomes enabled.
- [ ] **Step 5:** Record the run in `docs/evidence/2026-11-08-executors-slice-1.md` and mark the
  stages entries done. Commit `docs: record executors slice 1 acceptance`.
