# Slice 5: checks Shadows runs, and messages to a task's agent — Implementation Plan

> **For agentic workers:** work from the named stages entry, one verified task and
> scoped commit at a time. Use superpowers:executing-plans for inline execution, or
> superpowers:subagent-driven-development when the controller selects agents. This
> planning document does not start product implementation.

**Goal:** A task carries the commands that prove it. The executor runs them while it works;
Shadows runs them again on a quiet project and records the results, so the Planner judges on
evidence. The Planner can also ask a task's agent a plain question and hear its reply.

**Architecture:** AppCore composes the existing owners. Plans owns the task contract and its
validation; Runs owns the reported and confirmed checks, the done mark's rule and agent
messages; Turns delivers turns and notices; Harness opens the executor session. No new service.

**Tech Stack:** existing Rust 1.94+, SQLx 0.9/SQLite, Tokio, `shadows-process`, rmcp,
axum/utoipa, schemars. No new third-party package.

**Spec:** [executors design §5.4](../specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15)
(checks, with the 2026-11-15 trial) and [§5.3](../specs/2026-11-08-executors-design.md#53-the-planner-messages-a-tasks-agent-mohammed-2026-11-14)
(`message_agent`).
**Baseline:** `301984cf`, `feature/slice-5-checks`, 2026-11-15.
**Replaces:** `docs/backend/plans/2026-11-09-checks-and-critic.md` at `04125e12`; it was written
for the critic, which §5.1 replaced. Its steps are not executable.
**Order:** B-111 → B-113 → B-114 → B-150 → F-103 → B-151.

## Global Constraints

- Begin at AppCore and the named owner contracts. Stores stay private; cross-owner
  transaction work uses `*_in(&mut core_db::Tx, …)` methods.
- No new service, scheduler, baseline run, second checkout, critic or read-only mode.
- "Busy" is always the executor thread's running turn (Turns' operations), never the attempt's
  state: an attempt is `committed` from its report until its turn ends (about 9 seconds in the
  2026-11-15 trial).
- A check is a program and its arguments, run without a shell. Its folder is inside the
  project folder; `..` and absolute paths are refused when the contract is written.
- Shadows runs one check at a time per project, and only on a quiet project (no executor turn
  running in it). That lock is Runs' live state, like held scopes, and is not persisted. It
  orders Shadows' runs only; executors order theirs by instruction and by waiting on a lock error.
- Mutations use normalized command fingerprints, schema 1 and replay before current-state
  refusals. A notice's command id names its attempt or message, so a repeat is a replay.
- Migrations reserved on this baseline: `0063_run_checks.sql` (B-111),
  `0064_notice_sent.sql` (B-113), `0065_mark_reason.sql` (B-114),
  `0066_agent_messages.sql` (B-150). Renumber only an unshipped file if another branch takes one.
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
   `checks_wait_for_a_quiet_project`, `stopped_attempts_leftover_is_inconclusive`,
   `content_change_during_check_is_inconclusive`, `head_moved_during_check_is_inconclusive`
   and `missing_program_is_not_run`.
2. Two attempts finishing together never run Shadows' checks at once: B-113's
   `checks_of_one_project_run_one_at_a_time`.
3. A restart neither loses nor doubles results or notices, including a stop between the last
   result and the notice: B-113's `restart_reruns_unfinished_checks_and_notifies_once` and
   `restart_sends_an_unsent_notice`; B-150's `restart_delivers_an_undelivered_reply`.
4. A done mark over a check that did not pass, or has no result yet, is never silent, even after
   the scope was freed: B-114's `done_over_failed_check_needs_reason`,
   `done_before_results_needs_reason` and `done_again_after_undo_needs_reason`.
5. A message never creates an attempt, counts as an addition or settles an attempt, and never
   reaches a busy turn: B-150's `message_changes_no_attempt`,
   `message_waits_while_committed_turn_runs`, `messages_go_one_at_a_time` and
   `done_waits_for_reply`.
6. A failed confirmation returns to the executor at most twice and never spends the Planner's
   additions: B-113's `failed_confirmation_returns_to_executor`, `two_returns_then_planner` and
   `returns_are_not_additions`. A return that needs no change ends on the same commit, and only
   the task's latest attempt is ever notified, also after a restart:
   `return_without_change_reconfirms_previous_commit`, `restart_during_return_sends_no_old_notice`
   and `restart_after_return_passed_notifies_once`.

## Tasks

### B-111 · Checks in the task contract, and the executor runs them

**Owner:** Plans (contract and validation), Runs (the report's results), Harness (instruction text).

**Files:**
- Modify: `crates/core-plans/src/task_contract.rs` (the `TaskCheck` type, the `checks` field, its problems)
- Modify: `crates/core-plans/contract.yaml`
- Create: `crates/core-db/migrations/0063_run_checks.sql` (`run_checks`, below; B-111 fills only the reported columns)
- Modify: `crates/core-runs/src/report.rs`, `model.rs`, `error.rs`, `store/mod.rs` (the reported outcomes)
- Modify: `crates/shadows-mcp/src/run_tools.rs` (`checks` on `task_report`)
- Modify: `crates/core-harness/src/prompt.txt` (how the Planner writes checks)
- Modify: `crates/core-harness/src/executor.md` (run them, wait on a lock error, report each)
- Modify: `crates/core-runs/contract.yaml`, `crates/shadows-mcp/src/contract.yaml`
- Modify: `api/openapi.json`, `web/src/api/schema.d.ts` (generated)
- Test: `crates/core-plans/tests/task_contract.rs`, `crates/shadows-core/tests/executor_commits.rs`

**Interfaces:**
- `TaskContract.checks: Vec<TaskCheck>`, `#[serde(default)]`, so existing contracts read unchanged.
- `TaskCheck { id: String, run: Vec<String>, folder: Option<String>, timeout_secs: Option<u32>, proves: Vec<u32> }`:
  `id` unique in the task (`c1`); `run[0]` the program, the rest its arguments;
  `folder` relative to the project folder; `timeout_secs` 1 to 1800, default 600;
  `proves` the task's acceptance items it proves.
- Problems, one each: empty `run` or empty program; duplicate id; `folder` absolute or with `..`;
  timeout out of range; `proves` naming an item the task does not have.
- `task_report` takes `checks: [{ id, outcome, note }]`, `outcome` one of `passed | failed | not_run`.
  A report on a task with checks gives one entry per check id, no more; otherwise it is refused
  (`ReportedChecks`) and the attempt stays running, as other report refusals do.
- Table `run_checks (attempt_id, for_task, check_id, ordinal, reported, reported_note, outcome,
  exit_code, head_before, head_after, state_changed, output, truncated, started_at, finished_at)`,
  key `(attempt_id, for_task, check_id)`, ordered by `ordinal`. `for_task` is the task whose check
  it is: the attempt's own, or a used task's (B-113).

**Steps:**
1. Tests (RED): `check_needs_a_program`, `check_ids_are_unique`, `check_folder_stays_inside`,
   `check_timeout_is_bounded`, `check_proves_existing_items`, `contract_without_checks_reads`;
   `report_needs_each_check`, `report_records_reported_checks`. Run
   `cargo test -p core-plans --test task_contract` and
   `cargo test -p shadows-core --test executor_commits`; the new tests fail.
2. Implement. The `plan_edit` schema picks the field up through schemars.
3. Planner text (`prompt.txt`, after the task_done lines): write one check per acceptance item a
   command can prove, as a program and arguments (`["go","test","./internal/api/..."]`), never a
   shell line. Executor text: run the task's checks before you commit and fix what fails in your
   files; other executors may be checking at once, so a lock, busy or port error is never a
   failure: go on with your code and try again later; when your work is done and they still
   cannot run, commit and report them `not_run`; give each check's outcome in task_report.
   Shadows runs them again once the project is free, and sends you back if they do not pass.
4. GREEN: the same, plus `cargo test -p shadows --test planner_instructions` and
   `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`, then `npm run gen:api` in `web/`.
5. Commit: `feat(plans): checks in the task contract, run and reported by the executor`.

### B-113 · Shadows confirms a task's checks on a quiet project, and returns failures

**Owner:** Runs.

**Files:**
- Create: `crates/core-runs/src/checks.rs` (confirming one attempt's checks)
- Create: `crates/core-runs/src/store/checks.rs`
- Create: `crates/core-db/migrations/0064_notice_sent.sql` (`notice_sent_at` and `origin` on `run_attempts`)
- Modify: `crates/core-runs/src/lib.rs` (the per-project check lock in `Inner`), `model.rs`
  (`CheckOutcome`, `CheckResult`), `watch.rs` (confirmation before the committed notice),
  `notify.rs` (results in the notice; `notice_sent_at`), `amend.rs` (additions count `planner`
  origin only), `report.rs` (a return may report the commit it continues), `recover.rs`,
  `store/mod.rs` (`continue_attempt` takes the origin)
- Modify: `crates/core-runs/contract.yaml`, `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/attempt_checks.rs` (new; the fixture pattern of `executor_commits.rs`)

**Interfaces:**
- `CheckOutcome`: `Passed | Failed | TimedOut | Inconclusive | NotRun`.
- `CheckResult { for_task, id, run, folder, proves, reported, outcome, matches, exit_code,
  head_before, head_after, output, truncated, started_at, finished_at }`. `matches` says whether
  Shadows' outcome agrees with the executor's reported one; `None` for a used task's check.
- `Runs::confirm_checks(&self, attempt: &AttemptId)` (crate-private; watch and recovery call it)
  and `Runs::checks(&self, attempt: &AttemptId) -> Result<Vec<CheckResult>, RunsError>` (public read).
- The checks come from the attempt's saved context, the contract the attempt began on. A used
  task's checks come from its own latest committed attempt's saved context.
- `Attempt.origin`: `start | planner | checks`. A `checks` attempt is a return after a failed
  confirmation; `additions_since_start` counts `planner` ones only.

**Rules:**
- Runs when the watcher finds the attempt `Committed` and its turn ended, before `notify`. A task
  with no checks, and no used task with checks, sends its notice as today.
- Waits for a quiet project: no attempt of the project has an unended turn (Turns' operations of
  each running executor thread). It looks again every second; the wait has no limit, since every
  turn ends or is stopped.
- Takes the project's lock (a `tokio::sync::Mutex` per project id in Runs' live state). Runs the
  attempt's own checks, then those of each task its contract `uses` that has a committed attempt.
  An earlier task's checks are not run for a task that does not use it.
- Before the checks: the uncommitted paths (`git status --porcelain`) inside any task's `writes`
  in the project's approved plans, such as a stopped attempt's leftovers, make every check of this
  run `Inconclusive` without running it; the paths are named in its output. Paths outside every
  task's `writes` are the person's and do not count.
- Per check: read HEAD, the uncommitted paths and each one's content hash (`git hash-object`);
  spawn through `shadows-process` in `folder`; wait with the time limit (`terminate_tree` on
  expiry); keep stdout and stderr together, the last 65536 bytes; read them all again. Outcome:
  spawn error → `NotRun`; time limit → `TimedOut`; HEAD, a path or a hash changed →
  `Inconclusive`; exit 0 → `Passed`; otherwise `Failed`.
- Each row is written when it starts and again when it finishes.
- Every check passed: the Planner's notice lists each result (`c1 passed`, `T3 c1 passed`); its
  card carries them without output. `notice_sent_at` is set once the notice is queued.
- Any check did not pass, and the task has had fewer than two `checks` attempts since its last
  `start` or `planner` attempt: Runs continues the attempt as a new `checks` attempt in the same
  session, through `run_turn`, with the results (output end included) and "The project is free
  now: run the checks and finish, then commit and report as before." No Planner notice is sent
  for the attempt it replaces. The new commit is confirmed the same way.
- A return that needs no change: on a `checks` attempt, `task_report` may name the commit of the
  attempt it continues, with a report saying nothing needed changing. Runs accepts it without
  the `NotNew` and empty-commit refusals (that commit passed every other report check before),
  still refusing an uncommitted change inside the task's writes, and confirms its checks again.
  The return's prompt says so: "If nothing needs to change, report the same commit."
- Only a task's latest attempt is ever notified. An attempt continued by a later one (a return
  or an addition) never sends a notice, at the end of its checks or at recovery.
- After two returns: the notice goes to the Planner with the results, and any mismatch with the
  executor's report (`c2 failed (exit 1); the executor reported passed`).
- Recovery, for each task's latest `Committed` attempt only: one with unfinished rows, or with
  checks and no confirmed rows, clears its unfinished results and confirms again in the
  background; one with complete results and no `notice_sent_at` acts on them as above: a return if one is due and none was started
  (the `checks` attempt's command id names the attempt it continues, so a repeat is a replay),
  otherwise its notice (command id `notify-<attempt>`, a replay if it was sent).

**Steps:**
1. Tests (RED) in `attempt_checks.rs`, real Git folder, `git` as the program:
   `passed_and_failed_checks_are_recorded` (`git --version`, `git rev-parse no-such-ref`),
   `mismatch_with_report_is_marked`, `missing_program_is_not_run`, `check_time_limit_ends_it`,
   `long_output_keeps_its_end`, `checks_wait_for_a_quiet_project`,
   `uncommitted_change_in_writes_is_inconclusive`, `stopped_attempts_leftover_is_inconclusive`
   (a stopped attempt's file makes another task's check pass), `person_files_do_not_count`,
   `content_change_during_check_is_inconclusive`, `head_moved_during_check_is_inconclusive`,
   `checks_of_one_project_run_one_at_a_time`, `used_tasks_checks_run_too`,
   `failed_confirmation_returns_to_executor`, `two_returns_then_planner`,
   `returns_are_not_additions`, `return_without_change_reconfirms_previous_commit`,
   `restart_reruns_unfinished_checks_and_notifies_once`, `restart_sends_an_unsent_notice`,
   `restart_during_return_sends_no_old_notice`, `restart_after_return_passed_notifies_once`. Run `cargo test -p shadows-core --test attempt_checks`.
2. Implement in the listed files.
3. GREEN: the same, plus `--test executor_commits --test recovery --test task_conflicts`, and
   `cargo test -p shadows --test codemap`.
4. Commit: `feat(runs): confirm a task's checks on a quiet project and return failures`.

### B-114 · The Planner reads checks and judges on them

**Owner:** Runs, with its adapters.

**Files:**
- Create: `crates/core-db/migrations/0065_mark_reason.sql` (`reason` on `run_task_marks`)
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
  the fingerprint. `done = true` with no non-empty reason is refused `ChecksNotPassed { checks }`,
  naming them, when the task's latest committed attempt has any check (its own or a used task's)
  that is not `Passed` or has no finished result. That attempt is read directly, not through
  `awaiting`, which ends once the scope is freed, so an undo and a new done mark meet the rule
  again. The rule lives in `mark_done` only, so the route and `task_done` answer the same.
- `TaskRun.done_reason: Option<String>`.

**Steps:**
1. Tests (RED): `done_over_failed_check_needs_reason`, `done_before_results_needs_reason`,
   `done_again_after_undo_needs_reason` and `done_reason_is_recorded` (core);
   `checks_route_answers_results` and `done_route_refuses_without_reason` (routes);
   `task_checks_reads_latest_commit` (MCP). Run the three test targets; the new tests fail.
2. Implement. Planner text: after an attempt ends, read `task_checks` before `task_diff`;
   a check that did not pass, or a mismatch with the executor's report, is not done unless you
   know why, and then `task_done` carries the reason.
3. GREEN: the same targets, `planner_instructions`, the OpenAPI test, `npm run gen:api`.
4. Commit: `feat(runs): the Planner judges on recorded checks`.

### B-150 · The Planner messages a task's agent

**Owner:** Runs, through Turns' send and notice and Harness's executor session.

**Files:**
- Create: `crates/core-runs/src/message.rs`
- Create: `crates/core-runs/src/store/messages.rs`
- Create: `crates/core-db/migrations/0066_agent_messages.sql`
- Modify: `crates/core-runs/src/lib.rs` (`mark_done` leaves an answering session open),
  `model.rs` (`AgentMessage`, `AgentReplyNotice`),
  `watch.rs` (the next waiting message after a turn ends), `amend.rs` (refused while answering),
  `error.rs` (`AgentAnswering`), `notify.rs` (the reply notice), `recover.rs`, `store/mod.rs`
- Modify: `crates/core-grants/src/tools.rs` (`message_agent` in THREAD_TOOLS)
- Modify: `crates/shadows-mcp/src/run_tools.rs`
- Modify: `crates/core-harness/src/prompt.txt` (when to message instead of amend)
- Modify: contracts of core-runs, core-grants and shadows-mcp; `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/agent_messages.rs` (new), `crates/shadows/tests/planner_mcp.rs`

**Interfaces:**
- `Runs::message_agent(&self, grant, workflow, task, message: String) -> Result<AgentMessage, RunsError>`:
  the task's latest attempt of the plan's current version; refused `NotFound` when the task has none.
- Table `run_agent_messages (id, attempt_id, thread_id, ordinal, message, state, operation_id,
  reply, created_at, answered_at, delivered_at)`; `state`: `waiting | sent | replied | delivered |
  failed`; ordered by `ordinal` within the thread.
- `AgentMessage { id, attempt_id, task, state }`.

**Rules:**
- Busy is the executor thread's running turn (Turns' operations), never the attempt's state.
- The same trimmed text to the same attempt while an earlier one is `waiting`, `sent` or `replied`
  answers that message; no second row. After its reply the same text is a new question: MCP
  carries no command id, as for `task_start` (Mohammed, 2026-11-15).
- One message in flight per thread, in order. A message is saved `waiting` and sent when the
  thread has no running turn and no earlier message is `sent`. Whatever ends a turn on that thread
  (the attempt's watcher, after it settles and releases, or a message's watcher) sends the next
  waiting message.
- Sending: open the executor session as `run_turn` does (it reopens a closed or gone one), send one
  turn (`command_id` `message-<id>`), record its operation, and watch it with a watcher that settles
  no attempt and frees no scope.
- When that turn ends, its last agent message is saved (`replied`), then sent to the plan's Planner
  through `Turns::queue_notice` in the executor's name, as "Reply from task agent · T<n>", with an
  `AgentReplyNotice` card (`delivered`). A failed turn sends "No reply from task agent · T<n>" and
  its reason (`failed`). The session is closed again only when its attempt is not running, no turn
  runs on the thread and no message waits.
- `task_amend` while a message of that task is `waiting` or `sent` is refused `AgentAnswering`.
- A done mark records the mark at once, but while a message of the task is `waiting` or `sent`
  it does not close the session or free the scope; the message watcher does both after the
  reply, by the closing rule above.
- No attempt row, addition count, plan or mark changes.
- Recovery: for a `sent` message, read its operation: finished and completed → save the
  thread's last agent message as the reply (`replied`) and deliver it; finished and failed, or
  never finished → `failed`, delivered as no reply. `replied` ones are delivered; `waiting` ones
  are sent in order.

**Steps:**
1. Tests (RED) in `agent_messages.rs` (the `executor_session.rs` fixture):
   `message_changes_no_attempt`, `message_waits_while_committed_turn_runs`,
   `messages_go_one_at_a_time`, `same_text_is_one_message`, `message_after_done_reopens_session`,
   `amend_waits_for_reply`, `done_waits_for_reply`, `reply_reaches_planner_once`,
   `restart_delivers_an_undelivered_reply`, `restart_reads_a_finished_reply_turn`;
   `message_agent_tool_listed` in `planner_mcp.rs`.
2. Implement.
3. GREEN: the same targets, `executor_session`, `executor_commits`, `planner_instructions`.
4. Commit: `feat(runs): the Planner messages a task's agent`.

### F-103 · Checks and replies in the task page

Masada details it in `docs/frontend/stages.md` from B-111, B-113, B-114 and B-150's wire types:
the task's checks in the brief panel's contract; the Report tab's Checks section (each check's
reported and confirmed outcome, a mismatch marked, exit code, HEAD, the output on request);
a done mark's reason beside the mark; and the reply card in the Planner conversation.

### B-151 · Real slice 5 acceptance

**Owner:** integration acceptance through AppCore and the real UI.

**Files:**
- Create: `crates/shadows/tests/checks_live.rs` (`#[ignore]`, signed-in subscription)
- Create: `docs/evidence/2026-11-15-slice-5-checks.md`
- Modify: `docs/backend/stages.md`

**Steps:**
1. Live tests: a Planner writes a task with a passing and a failing check; the executor runs
   them, fixes what it can and reports each; Shadows confirms on a quiet project and the notice
   lists both, with any mismatch; `task_done` without a reason is refused and with one is
   recorded; `message_agent` reaches a running executor and a done one, and both replies arrive.
2. Browser on a disposable project (`testing4` or new): the same journey, the Report tab's
   checks, two tasks committing together (executors check at once; Shadows confirms after both
   turns end, one check at a time in the log), and a daemon restart during a long check.
3. Record the evidence with commit, OS, harness, model and what was not tried.

## Slice acceptance and handoff

The slice is done when B-151's browser run and logs are recorded. The full gate runs in CI
after the branch is pushed; one whole-branch review runs before the PR.
