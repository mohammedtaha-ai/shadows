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
4. **The person staged a file of their own** before the commit: `task_commit` commits only the
   attempt's paths (`git commit --only -- <paths>`), and the person's staged file stays staged and
   uncommitted. Pinned in B-92.
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
- [ ] **Step 5:** Write the evidence and amend §7 with the rule chosen from it. The rule names:
  the session mode; which tools raise requests carrying a path; how a shell command is treated
  (allowlist of command prefixes, or refused); and how an attempt's changed paths are attributed
  in a shared folder (the paths its permitted requests named, plus `git status` paths inside its
  `writes` that no other running attempt claims).
- [ ] **Step 6:** Delete the throwaway source. Commit the evidence, §7 and stages:
  `docs: measure the executor write boundary`.

**Verification:** each of the four cases has a recorded observation; §7 has no OPEN block left,
or B-90 stays blocked with the measured gap stated.

---

### B-90: The executor session

**Files:**
- Create: `crates/core-harness/src/executor.txt`, `crates/core-harness/src/executor.rs`,
  `crates/shadows-core/tests/executor_session.rs`
- Modify: `crates/core-threads/src/model.rs` (`ThreadKind`), `src/store/` (kind column read and
  written; `list` and `list_for_profile` return `planning` threads only), `contract.yaml`;
  `crates/core-db/migrations/00NN_thread_kind.sql` (next unused number:
  `threads.kind TEXT NOT NULL DEFAULT 'planning'`), `crates/core-db/contract.yaml`, the
  storage-contract schema and checksum tests;
  `crates/core-harness/src/setup.rs` (setup by kind), `src/sessions.rs`, `contract.yaml`;
  `crates/shadows-agent/src/acp/permissions.rs` (scoped policy), `crates/shadows-agent/contract.yaml`;
  `crates/core-grants/src/model.rs` and store (task scope on a thread grant), `contract.yaml`;
  `crates/shadows-mcp/src/plan_tools.rs` (which tools an executor grant sees), `contract.yaml`

**Interfaces:**
- Consumes: B-89's §7 rule (mode, tool list, shell treatment). Nothing from Runs.
- Produces:
  - `pub enum ThreadKind { Planning, Executor }` (`as_str`: `planning`, `executor`) on `PlanningThread.kind`.
  - `Threads::create_executor_in(conn, project: &ProjectId, title: String, harness: &str) -> ThreadId`.
  - `pub struct TaskScope { workflow_id: WorkflowId, version: i64, task: u32, writes: Vec<String> }`;
    `Grants::issue_thread_grant_in` gains `scope: Option<TaskScope>`; `Grant.scope: Option<TaskScope>`.
  - `PermissionPolicy::Scoped { root: PathBuf, writes: Vec<String> }`: allow a request whose paths
    all fall inside `writes` under `root`; refuse anything else, including a request with no path,
    except what §7 allows.
  - `EXECUTOR_TOOLS` in `setup.rs`, from §7.
  - An executor grant sees exactly: `task_context` (its own task only), `where_is`, `who_uses`,
    `outline`, and `task_commit` (added by B-92). Every planning write tool is refused for it.

- [ ] **Step 1: Write the failing tests** in `crates/shadows-core/tests/executor_session.rs`
  (fake-acp):
  - `an_executor_session_opens_with_its_instructions_tools_and_policy`: the setup's append starts
    with `executor.txt`, `tools == EXECUTOR_TOOLS`, `setting_sources` empty, policy `Scoped`.
  - `an_edit_inside_writes_is_allowed_and_outside_is_refused`: fake requests `backend/a.php` → allow;
    `web/a.ts` → refuse, and the refusal is a thread entry.
  - `an_executor_grant_reads_only_its_own_task`: `task_context` for its task → Ok; another task → refused.
  - `an_executor_grant_cannot_write_the_plan`: a plan-writing tool → refused.
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
    thread_id: ThreadId, model: String, state: AttemptState, commit: Option<String>,
    reason: Option<String>, started_at: String, ended_at: Option<String> }`.
  - `pub struct TaskRun { task: u32, done: bool, done_at: Option<String>, latest: Option<Attempt> }`.
  - `Runs::start(&self, command: CommandId, workflow: &WorkflowId, version: i64, task: u32, model: String) -> Result<Attempt, RunsError>`
  - `Runs::stop(&self, attempt: &AttemptId) -> Result<Attempt, RunsError>` (waits for the turn's
    confirmed end, as the Planner's Stop does)
  - `Runs::task(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<TaskRun, RunsError>`
  - `Runs::attempts(&self, workflow: &WorkflowId, version: i64, task: u32) -> Result<Vec<Attempt>, RunsError>` (newest first)
  - `Runs::mark_done(&self, command: CommandId, workflow: &WorkflowId, version: i64, task: u32, done: bool) -> Result<TaskRun, RunsError>`
  - `Runs::recover(&self) -> Result<u32, RunsError>` (count of attempts set `Interrupted`)
  - `RunsError`: `NotFrozen`, `Superseded`, `Archived`, `Waiting { tasks: Vec<u32> }`, `TaskBusy`,
    `CommandConflict`, `NotFound`, `Storage`. B-92 adds `NotGit`, `DirtyScope`, `OutsideScope`,
    `NothingToCommit`.

- [ ] **Step 1: Write the failing tests** in `crates/core-runs/tests/attempts.rs`:
  - `start_opens_one_attempt_on_an_executor_thread`: start T1 → `state == Running`, `number == 1`,
    the thread's kind is `executor`, the model is the one given.
  - `the_same_command_returns_the_same_attempt`: start twice with one `command_id` → equal ids, one row.
  - `a_second_start_while_running_is_busy`: new `command_id` → `Err(TaskBusy)`.
  - `start_waits_for_needs_links`: T3 `needs` T1, T1 not done → `Err(Waiting { tasks: vec![1] })`;
    after `mark_done(T1, true)` → Ok.
  - `start_refuses_a_draft_a_superseded_or_an_archived_version`: each → its error.
  - `stop_ends_the_attempt_and_closes_its_session`: `state == Stopped`, `ended_at` set, no live session.
  - `recover_interrupts_running_attempts_without_restarting_them`: a `Running` row at startup →
    `Interrupted`, grant revoked, no turn started; `recover()` returns 1.
- [ ] **Step 2:** Run `cargo test -p core-runs --test attempts`. Expected: fails to compile (no crate).
- [ ] **Step 3:** Migration: tables `run_attempts` (columns of `Attempt`, plus `command_id`,
  `fingerprint`; unique `(workflow_id, version, task, number)`) and `run_task_marks`
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
- Consumes: B-89's attribution rule; B-91's `Attempt`; B-90's `TaskScope` on the caller's grant.
- Produces:
  - `Runs::commit(&self, grant: &Grant, message: String) -> Result<Attempt, RunsError>`: one commit
    at a time per project folder (a per-project `tokio::sync::Mutex`); stages and commits only the
    attempt's attributed paths with `git commit --only -- <paths>`; message
    `T{n}: {title}\n\n{message}\n\nShadows-Attempt: {attempt id}`; sets `commit` and `Committed`.
  - Errors: `OutsideScope { paths: Vec<String> }` (any attributed path outside `writes`; nothing
    committed; attempt `Failed` with the paths as reason), `NothingToCommit`.
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
  - `two_commits_at_once_are_serialized`: two attempts commit concurrently → two commits, each with its own paths.
  - `start_refuses_a_folder_that_is_not_git` and `start_refuses_uncommitted_changes_in_writes`.
  - `diff_returns_each_files_counts_and_patch`.
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
- Consumes: B-91 and B-92.
- Produces (all under `/api/projects/{project}/workflows/{workflow}/versions/{version}/tasks/{task}`):
  - `GET` → `TaskRun` plus the suggested starting model (the plan thread's remembered model, §12.6).
  - `GET /attempts` → `Attempt[]`.
  - `POST /attempts` `{ command_id, model }` → `Attempt` (201); errors map `Waiting`, `TaskBusy`,
    `NotGit`, `DirtyScope`, `NotFrozen`, `Superseded`, `Archived` to 409 with Core's reason.
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
- Consumes: B-93's routes and generated types.
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
