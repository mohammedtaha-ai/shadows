# Slice 5: checks Shadows runs, and messages to a task's agent — Implementation Plan

> **For agentic workers:** work from the named stages entry, one verified task and
> scoped commit at a time. Use superpowers:executing-plans for inline execution, or
> superpowers:subagent-driven-development when the controller selects agents. This
> planning document does not start product implementation.

**Goal:** A task carries the commands that prove it; Shadows runs them after the executor's
commit and records the results, so the Planner judges on evidence. The Planner can also ask a
task's agent a plain question and hear its reply.

**Architecture:** AppCore composes the existing owners. Plans owns the task contract and its
validation; Runs owns running checks, their records, the done mark's rule and agent messages;
Turns delivers turns and notices; Harness opens the executor session. No new service.

**Tech Stack:** existing Rust 1.94+, SQLx 0.9/SQLite, Tokio, `shadows-process`, rmcp,
axum/utoipa, schemars. No new third-party package.

**Spec:** [executors design §5.4](../specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15)
(checks) and [§5.3](../specs/2026-11-08-executors-design.md#53-the-planner-messages-a-tasks-agent-mohammed-2026-11-14)
(`message_agent`).
**Baseline:** `04125e12`, `feature/slice-5-checks`, 2026-11-15.
**Replaces:** `docs/backend/plans/2026-11-09-checks-and-critic.md` at `04125e12`; it was written
for the critic, which §5.1 replaced. Its steps are not executable.
**Order:** B-111 → B-113 → B-114 → B-150 → F-103 → B-151.

## Global Constraints

- Begin at AppCore and the named owner contracts. Stores stay private; cross-owner
  transaction work uses `*_in(&mut core_db::Tx, …)` methods.
- No new service, scheduler, baseline run, second checkout, critic or read-only mode.
- A check is a program and its arguments, run without a shell. Its folder is inside the
  project folder; `..` and absolute paths are refused when the contract is written.
- One check at a time per project. That is Runs' live state, like held scopes; it is not
  persisted, and recovery rebuilds nothing from it.
- Mutations use normalized command fingerprints, schema 1 and replay before current-state
  refusals. A notice's command id names its attempt or message, so a repeat is a replay.
- Migrations reserved on this baseline: `0063_run_checks.sql` (B-113),
  `0064_mark_reason.sql` (B-114), `0065_agent_messages.sql` (B-150). Renumber only an
  unshipped file if another branch takes a number.
- One cache per workspace; no CARGO_TARGET_DIR, worktree build, clean, release/custom
  profile/target or changed RUSTFLAGS. Full gate stays in CI; local tests are focused.
- Public wire types carry serde/utoipa derives and are re-exported through shadows-core.
  Regenerate `api/openapi.json` and `web/src/api/schema.d.ts` with any route or type change.
- Changed owners update their `contract.yaml` and the code map in the same commit.
- New tests use real temporary Git folders, real SQLite and real commands (`git` itself is
  the check program in fixtures). No new fake-acp behavior. Live-provider tests are
  `#[ignore]` and run on the signed-in subscription.

## Review Focus

1. A check never reads as passed when it did not prove the commit: B-113's
   `head_moved_during_check_is_inconclusive` and `missing_program_is_not_run`.
2. Two attempts finishing together never run checks at once in one project: B-113's
   `checks_of_one_project_run_one_at_a_time`.
3. A restart neither loses nor doubles results or notices: B-113's
   `restart_reruns_unfinished_checks_and_notifies_once`.
4. A done mark over a failed check is never silent: B-114's
   `done_over_failed_check_needs_reason`.
5. A message never creates an attempt, counts as an addition or settles an attempt:
   B-150's `message_changes_no_attempt`.

## Tasks

### B-111 · Checks in the task contract

**Owner:** Plans (contract and validation), Harness (instruction text).

**Files:**
- Modify: `crates/core-plans/src/task_contract.rs` (the `TaskCheck` type, the `checks` field, its problems)
- Modify: `crates/core-plans/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt` (how the Planner writes checks)
- Modify: `crates/core-harness/src/executor.md` (Shadows runs the checks after the commit)
- Modify: `api/openapi.json`, `web/src/api/schema.d.ts` (generated)
- Test: `crates/core-plans/tests/task_contract.rs`

**Interfaces:**
- `TaskContract.checks: Vec<TaskCheck>`, `#[serde(default)]`, so existing contracts read unchanged.
- `TaskCheck { id: String, run: Vec<String>, folder: Option<String>, timeout_secs: Option<u32>, proves: Vec<u32> }`:
  `id` unique in the task (`c1`); `run[0]` the program, the rest its arguments;
  `folder` relative to the project folder; `timeout_secs` 1 to 1800, default 600;
  `proves` the task's acceptance items it proves.
- Problems, one each: empty `run` or empty program; duplicate id; `folder` absolute or with `..`;
  timeout out of range; `proves` naming an item the task does not have.

**Steps:**
1. Tests (RED): `check_needs_a_program`, `check_ids_are_unique`, `check_folder_stays_inside`,
   `check_timeout_is_bounded`, `check_proves_existing_items`, and `contract_without_checks_reads`
   (a stored contract with no `checks` key deserializes). Run
   `cargo test -p core-plans --test task_contract`; the new tests fail.
2. Implement in `task_contract.rs` beside the output/acceptance problems. The `plan_edit` schema
   picks the field up through schemars.
3. Planner text (`prompt.txt`, after the task_done lines): write one check per acceptance item a
   command can prove, as a program and arguments (`["go","test","./internal/api/..."]`), never a
   shell line; a check shared with another task's build or database is still written, since
   Shadows runs a project's checks one at a time. Executor text: Shadows runs the task's checks
   after your commit; run them yourself before you commit.
4. GREEN: the same command, plus `cargo test -p shadows --test planner_instructions` and
   `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`, then `npm run gen:api` in `web/`.
5. Commit: `feat(plans): checks in the task contract`.

### B-113 · Runs runs a task's checks after its commit

**Owner:** Runs.

**Files:**
- Create: `crates/core-runs/src/checks.rs` (running one attempt's checks)
- Create: `crates/core-runs/src/store/checks.rs`
- Create: `crates/core-db/migrations/0063_run_checks.sql`
- Modify: `crates/core-runs/src/lib.rs` (the per-project check lock in `Inner`), `model.rs`
  (`CheckOutcome`, `CheckResult`), `watch.rs` (checks before the committed notice),
  `notify.rs` (results in the notice body and card), `recover.rs`, `store/mod.rs`
- Modify: `crates/core-runs/contract.yaml`, `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/attempt_checks.rs` (new; the fixture pattern of `executor_commits.rs`)

**Interfaces:**
- Table `run_checks (attempt_id, ordinal, check_id, run_json, folder, outcome, exit_code,
  head_before, head_after, dirty, output, truncated, started_at, finished_at)`, key
  `(attempt_id, ordinal)`; ordering by `ordinal`.
- `CheckOutcome`: `Passed | Failed | TimedOut | Inconclusive | NotRun`.
- `CheckResult { id, run, folder, proves, outcome, exit_code, head_before, head_after, dirty, output, truncated, started_at, finished_at }`.
- `Runs::run_checks(&self, attempt: &AttemptId) -> Result<Vec<CheckResult>, RunsError>`
  (crate-private; watch and recovery call it) and `Runs::checks(&self, attempt: &AttemptId) -> Result<Vec<CheckResult>, RunsError>` (public read).
- The checks come from the attempt's saved context (`run_attempt_context`), the contract the
  attempt began on, never a later version.

**Rules:**
- Runs when the watcher finds the attempt `Committed`, before `notify`. A task with no checks
  sends its notice as today.
- Takes the project's lock (a `tokio::sync::Mutex` per project id in Runs' live state); the
  attempt's checks run in order while it is held.
- Per check: read HEAD and whether the tree has uncommitted changes (`dirty`); spawn through
  `shadows-process` in `folder`; wait with the time limit (`terminate_tree` on expiry);
  keep stdout and stderr together, the last 65536 bytes (`truncated` when cut); read HEAD again.
  Outcome: spawn error → `NotRun` with the error as output; time limit → `TimedOut`;
  HEAD moved → `Inconclusive`; exit 0 → `Passed`; otherwise `Failed`.
- Each row is written when it starts and again when it finishes, so a restart sees which ran.
- The committed notice lists each check (`c1 passed`, `c2 failed (exit 1)`); its card carries
  the results without output. The Planner reads output through B-114.
- Recovery: a `Committed` attempt with a check row unfinished, or with checks in its context and
  no rows, deletes its unfinished rows and runs its checks again in the background, then sends
  its notice (command id `notify-<attempt>`, so a sent one is a replay).

**Steps:**
1. Tests (RED) in `attempt_checks.rs`, real Git folder, `git` as the program:
   `passed_and_failed_checks_are_recorded` (`git --version`, `git rev-parse no-such-ref`),
   `missing_program_is_not_run`, `check_time_limit_ends_it` (a 1-second limit on a command that
   waits, such as `git` reading stdin), `long_output_keeps_its_end`,
   `head_moved_during_check_is_inconclusive`, `checks_of_one_project_run_one_at_a_time`,
   `restart_reruns_unfinished_checks_and_notifies_once`. Run
   `cargo test -p shadows-core --test attempt_checks`; the new tests fail.
2. Implement in the listed files.
3. GREEN: the same, plus `--test executor_commits --test recovery --test task_conflicts`.
   `cargo test -p shadows --test codemap` for the new module.
4. Commit: `feat(runs): run a task's checks after its commit`.

### B-114 · The Planner reads checks and judges on them

**Owner:** Runs, with its adapters.

**Files:**
- Create: `crates/core-db/migrations/0064_mark_reason.sql` (`reason` on `run_task_marks`)
- Modify: `crates/core-runs/src/lib.rs` (`mark_done` takes a reason), `planner.rs`
  (`checks_for`, `mark_done_for`), `store/marks.rs`, `model.rs` (`TaskRun.done_reason`),
  `error.rs` (`ChecksNotPassed`)
- Modify: `crates/core-grants/src/tools.rs` (`task_checks` in THREAD_TOOLS)
- Modify: `crates/shadows-mcp/src/run_tools.rs` (`task_checks`; `reason` on `task_done`)
- Modify: `crates/shadows-http/src/runs.rs` (`GET /api/attempts/{id}/checks`; `reason` on the
  done route's body), `crates/shadows-http/src/failure/runs.rs`, `crates/shadows-http/src/openapi.rs`
- Modify: `crates/core-harness/src/prompt.txt` (read checks before judging)
- Modify: contracts of core-runs, core-grants, shadows-http and shadows-mcp; `docs/codebase/README.md`;
  `api/openapi.json`, `web/src/api/schema.d.ts`
- Test: `crates/shadows-core/tests/attempt_checks.rs`, `crates/shadows/tests/runs_routes.rs`,
  `crates/shadows/tests/planner_mcp.rs`

**Interfaces:**
- `Runs::checks_for(&self, grant, workflow, task) -> Result<Vec<CheckResult>, RunsError>`:
  the task's latest committed attempt, after the Planner grant check.
- `mark_done(command, workflow, version, task, done, reason: Option<String>)`; the reason is in
  the fingerprint. `done = true` on a task whose awaiting attempt has any check not `Passed`, with
  no non-empty reason, is refused `ChecksNotPassed { checks }` naming them. The rule lives in
  `mark_done` only, so the route and `task_done` answer the same.
- `TaskRun.done_reason: Option<String>`.

**Steps:**
1. Tests (RED): `done_over_failed_check_needs_reason` and `done_reason_is_recorded` (core);
   `checks_route_answers_results` and `done_route_refuses_without_reason` (routes);
   `task_checks_reads_latest_commit` (MCP). Run the three test targets; the new tests fail.
2. Implement. Planner text: after an attempt ends, read `task_checks` before `task_diff`;
   a check that did not pass is not done unless you know why, and then `task_done` carries the
   reason.
3. GREEN: the same targets, `planner_instructions`, the OpenAPI test, `npm run gen:api`.
4. Commit: `feat(runs): the Planner judges on recorded checks`.

### B-150 · The Planner messages a task's agent

**Owner:** Runs, through Turns' send and notice and Harness's executor session.

**Files:**
- Create: `crates/core-runs/src/message.rs`
- Create: `crates/core-runs/src/store/messages.rs`
- Create: `crates/core-db/migrations/0065_agent_messages.sql`
- Modify: `crates/core-runs/src/lib.rs`, `model.rs` (`AgentMessage`, `AgentReplyNotice`),
  `watch.rs` (pending messages after the attempt's turn), `notify.rs` (the reply notice),
  `recover.rs`, `store/mod.rs`
- Modify: `crates/core-grants/src/tools.rs` (`message_agent` in THREAD_TOOLS)
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `crates/core-harness/src/prompt.txt` (when to message instead of amend)
- Modify: contracts of core-runs, core-grants and shadows-mcp; `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/agent_messages.rs` (new), `crates/shadows/tests/planner_mcp.rs`

**Interfaces:**
- `Runs::message_agent(&self, grant, workflow, task, message: String) -> Result<AgentMessage, RunsError>`:
  the task's latest attempt of the plan's current version; refused `NotFound` when the task has none.
- Table `run_agent_messages (id, attempt_id, message, state, operation_id, reply, created_at, answered_at)`;
  `state`: `waiting | sent | answered | failed`.
- `AgentMessage { id, attempt_id, task, state }`.

**Rules:**
- The attempt is `Running`: the message is saved `waiting`. After the attempt's watcher settles
  it, Runs sends each waiting message in order, so the attempt's own ending and release always
  come first.
- Otherwise it is sent at once: open the executor session as `run_turn` does (it reopens a
  closed or gone one), send one turn (`command_id` `message-<id>`), record its operation, and
  watch it with a watcher that settles no attempt and frees no scope.
- When that turn ends, its last agent message goes to the plan's Planner through
  `Turns::queue_notice` in the executor's name, as "Reply from task agent · T<n>", with an
  `AgentReplyNotice` card; then the session is closed again unless an attempt of the task is
  running in it. A turn that failed sends "No reply from task agent · T<n>" and its reason.
- No attempt row, addition count, plan or mark changes.
- Recovery: `sent` messages whose turn did not end become `failed` and notify; `waiting` ones
  of an attempt no longer running are sent.

**Steps:**
1. Tests (RED): `message_changes_no_attempt`, `message_waits_for_running_attempt`,
   `message_after_done_reopens_session`, `reply_reaches_planner_once` in `agent_messages.rs`
   (the `executor_session.rs` fixture); `message_agent_tool_listed` in `planner_mcp.rs`.
2. Implement.
3. GREEN: the same targets, `executor_session`, `executor_commits`, `planner_instructions`.
4. Commit: `feat(runs): the Planner messages a task's agent`.

### F-103 · Checks and replies in the task page

Masada details it in `docs/frontend/stages.md` from B-111, B-113, B-114 and B-150's wire types:
the task's checks in the brief panel's contract; the Report tab's Checks section (outcome per
check, exit code, HEAD, the output on request); a done mark's reason beside the mark; and the
reply card in the Planner conversation.

### B-151 · Real slice 5 acceptance

**Owner:** integration acceptance through AppCore and the real UI.

**Files:**
- Create: `crates/shadows/tests/checks_live.rs` (`#[ignore]`, signed-in subscription)
- Create: `docs/evidence/2026-11-15-slice-5-checks.md`
- Modify: `docs/backend/stages.md`

**Steps:**
1. Live tests: a Planner writes a task with a passing and a failing check; the executor
   commits; the notice lists both; `task_done` without a reason is refused and with one is
   recorded; `message_agent` reaches a running executor and a done one, and both replies arrive.
2. Browser on a disposable project (`testing4` or new): the same journey, the Report tab's
   checks, two tasks committing together (their checks run one after another in the log),
   and a daemon restart during a long check.
3. Record the evidence with commit, OS, harness, model and what was not tried.

## Slice acceptance and handoff

The slice is done when B-151's browser run and logs are recorded. The full gate runs in CI
after the branch is pushed; one whole-branch review runs before the PR.
