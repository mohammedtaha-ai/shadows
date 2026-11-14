# Slice 5: checks Shadows runs, and messages to a task's agent — Implementation Plan

> **For agentic workers:** work from the named stages entry, one verified task and
> scoped commit at a time. Use superpowers:executing-plans for inline execution, or
> superpowers:subagent-driven-development when the controller selects agents. This
> planning document does not start product implementation.

**Goal:** A task carries the commands that prove it. The executor runs them while it works;
Shadows runs them again on a quiet project and records the results, so the Planner judges on
evidence. A check that does not pass goes back to the executor first. The Planner can also ask a
task's agent a plain question and hear its reply.

**Architecture:** AppCore composes the existing owners. Plans owns the task contract and its
validation; Runs owns the reported and confirmed checks, the returns, the done mark's rule and
agent messages; Turns delivers turns and notices; Harness opens the executor session. No new
service, no new crate dependency.

**Tech Stack:** existing Rust 1.94+, SQLx 0.9/SQLite, Tokio, `shadows-process`, rmcp,
axum/utoipa, schemars. No new third-party package.

**Spec:** [executors design §5.4](../specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15)
(checks, with the 2026-11-15 trial) and [§5.3](../specs/2026-11-08-executors-design.md#53-the-planner-messages-a-tasks-agent-mohammed-2026-11-14)
(`message_agent`).
**Baseline:** `2aee91e3`, `feature/slice-5-checks`, 2026-11-15. Every symbol below was read at
this commit; a builder re-checks each with the LSP before changing it.
**Replaces:** `docs/backend/plans/2026-11-09-checks-and-critic.md` at `04125e12`; it was written
for the critic, which §5.1 replaced. Its steps are not executable.
**Order:** B-111 → B-113 → B-114 → B-150 → F-103 → B-151. Each task builds on the one before.

## Global Constraints

- Begin at AppCore and the named owner contracts. Stores stay private; cross-owner
  transaction work uses `*_in(&mut core_db::Tx, …)` methods.
- No new service, scheduler, baseline run, second checkout, critic or read-only mode.
- "Busy" is always the executor thread's running turn, read through `Turns::operations(thread)`
  (an operation with `finished_at == None`), never the attempt's state: an attempt is `committed`
  from its report until its turn ends (about 9 seconds in the 2026-11-15 trial).
- A check is a program and its arguments, run without a shell. Its folder is inside the
  project folder; `..` and absolute paths are refused when the contract is written.
- Shadows runs one check at a time per project, and only on a quiet project. That lock is Runs'
  live state, like held scopes, and is not persisted. It orders Shadows' runs only; executors
  order theirs by instruction and by waiting on a lock error.
- Mutations use normalized command fingerprints, schema 1 and replay before current-state
  refusals. A notice's command id names its attempt or message, so a repeat is a replay.
- Migrations on this baseline (the last is `0062_attempt_conflicts.sql`):
  `0063_run_checks.sql` (B-111), `0064_attempt_origin.sql` (B-113),
  `0065_mark_reason.sql` (B-114), `0066_agent_messages.sql` (B-150). Renumber only an unshipped
  file if another branch takes a number. Style: one comment saying what the table or column
  holds, `CHECK` for every closed set, `json_valid` on JSON columns, as `0062` does.
- A new `RunsError` variant gets its `code()` arm in `crates/core-runs/src/error.rs`; HTTP
  maps every refusal to 409 with that code (`crates/shadows-http/src/failure/runs.rs`), MCP
  through `Refusal::from`. No adapter rule.
- One cache per workspace; no CARGO_TARGET_DIR, worktree build, clean, release/custom
  profile/target or changed RUSTFLAGS. Full gate stays in CI; local tests are focused.
- Public wire types carry serde/utoipa derives and are re-exported through shadows-core.
  Regenerate `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`) and
  `web/src/api/schema.d.ts` (`npm run gen:api` in `web/`) with any route or type change.
- Changed owners update their `contract.yaml` and `docs/codebase/README.md` in the same commit.
- Tests: real temporary Git folders, real SQLite, real commands (`git` itself is the check
  program). A test that needs an executor turn reuses the existing `runs_attempts` fixture
  (`crates/shadows-core/tests/runs_attempts.rs`: `fixture()`, `Fixture::hold`/`release` on the
  `wait-for-release` first prompt). Its `begin`, `put`, `commit` and `report` helpers live in
  `impl Fixture` inside `runs_attempts/report.rs` (lines 173 to 249), private to that module:
  B-111 moves them, unchanged, into `runs_attempts.rs` so the new modules share them. No new
  fake-acp behavior. Live-provider tests are `#[ignore]` and run on the signed-in subscription.

## Review Focus

1. A check never reads as passed when it did not prove the commit: B-113's
   `checks_wait_for_a_quiet_project`, `stopped_attempts_leftover_is_inconclusive`,
   `content_change_during_check_is_inconclusive`, `head_moved_during_check_is_inconclusive`
   and `missing_program_is_not_run`.
2. Two attempts finishing together never run Shadows' checks at once, and a turn that starts
   during a check never lends it a result: B-113's `checks_of_one_project_run_one_at_a_time`,
   `quiet_is_checked_again_under_the_lock` and `turn_during_check_reruns_it`.
3. A restart neither loses nor doubles results or notices, including a stop between the last
   result and the notice: B-113's `restart_reruns_unfinished_checks_and_notifies_once` and
   `restart_sends_an_unsent_notice`; B-150's `restart_delivers_an_undelivered_reply` and
   `restart_reads_a_finished_reply_turn`.
4. A done mark over a check that did not pass, or has no result yet, is never silent, even after
   the scope was freed, and a replay answers before the rule: B-114's
   `done_over_failed_check_needs_reason`, `done_before_results_needs_reason`,
   `done_again_after_undo_needs_reason` and `done_replay_answers_after_checks_change`; B-113's
   `repeated_return_answers_the_same_attempt`.
5. A message never creates an attempt, counts as an addition or settles an attempt, and never
   reaches a busy turn: B-150's `message_changes_no_attempt`,
   `message_waits_while_committed_turn_runs`, `messages_go_one_at_a_time` and
   `done_waits_for_reply`.
6. A failed confirmation returns to the executor at most twice and never spends the Planner's
   additions; a return that needs no change ends on the same commit; only the task's latest
   attempt is ever notified, also after a restart: B-113's
   `failed_confirmation_returns_to_executor`, `two_returns_then_planner`,
   `returns_are_not_additions`, `return_without_change_reconfirms_previous_commit`,
   `restart_during_return_sends_no_old_notice` and `restart_after_return_passed_notifies_once`.

## Tasks

### B-111 · Checks in the task contract, and the executor runs them

**Owner:** Plans (contract and validation), Runs (the report's results), Harness (instruction text).

**What it adds, where:**

1. `crates/core-plans/src/task_contract.rs`
   - On `TaskContract` (line 24, `deny_unknown_fields`), after `forbidden`:
     ```rust
     /// Commands that prove the task, run by its executor and again by Shadows.
     #[serde(default)]
     pub checks: Vec<TaskCheck>,
     ```
   - A new type beside `ProducedOutput`, with the same derive list
     (`Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, JsonSchema`) and
     `#[serde(deny_unknown_fields)]`:
     ```rust
     /// One command that proves acceptance items: a program and its arguments, never a shell line.
     pub struct TaskCheck {
         /// Chosen by the writer, unique within the task: `c1`.
         pub id: String,
         /// The program, then its arguments: `["go", "test", "./internal/api/..."]`.
         pub run: Vec<String>,
         /// Where it runs, relative to the project's folder; the folder itself when left out.
         #[serde(default)]
         pub folder: Option<String>,
         /// Seconds before it is stopped: 1 to 1800, 600 when left out.
         #[serde(default)]
         pub timeout_secs: Option<u32>,
         /// The task's acceptance items it proves.
         #[serde(default)]
         pub proves: Vec<u32>,
     }
     ```
     and `impl TaskCheck { pub const DEFAULT_TIMEOUT_SECS: u32 = 600; pub const MAX_TIMEOUT_SECS: u32 = 1800; pub fn timeout(&self) -> std::time::Duration }`.
   - In `ContractRules::task` (line 258), after the `uses` loop, in this order, through the
     same `say` closure:
     - a missing or repeated id: ``missing or repeated check id `{id}` `` (the pattern the
       output ids use);
     - `run` empty or `run[0].trim()` empty: ``the check `{id}` names no program``;
     - `folder` that fails `Self::exact` (line 408) and is not `"."`:
       ``the check `{id}` runs in `{folder}`, which is not a folder inside the project``;
     - `timeout_secs` of 0 or over 1800: ``the check `{id}` has a time limit outside 1 to 1800 seconds``;
     - each `proves` item not in `acceptance`:
       ``the check `{id}` proves acceptance item {k}, which T{n} does not have``.
   - Re-export `TaskCheck` wherever `ProducedOutput` is re-exported (`core-plans/src/lib.rs`,
     then `shadows-core`).
2. `crates/core-db/migrations/0063_run_checks.sql`:
   ```sql
   -- One check of a task, per attempt: what its executor reported and, once Shadows ran it,
   -- what Shadows found. `for_task` is the task whose check it is: the attempt's own, or a task
   -- whose output the attempt's contract uses. Rows without `outcome` were only reported.
   CREATE TABLE run_checks (
       attempt_id    TEXT NOT NULL REFERENCES run_attempts(id) ON DELETE CASCADE,
       for_task      INTEGER NOT NULL,
       check_id      TEXT NOT NULL,
       ordinal       INTEGER NOT NULL,
       run_json      TEXT NOT NULL CHECK (json_valid(run_json)),
       folder        TEXT NULL,
       reported      TEXT NULL CHECK (reported IS NULL OR reported IN ('passed','failed','not_run')),
       reported_note TEXT NULL,
       outcome       TEXT NULL
           CHECK (outcome IS NULL
                  OR outcome IN ('passed','failed','timed_out','inconclusive','not_run')),
       exit_code     INTEGER NULL,
       head_before   TEXT NULL,
       head_after    TEXT NULL,
       output        TEXT NULL,
       truncated     INTEGER NOT NULL DEFAULT 0 CHECK (truncated IN (0, 1)),
       started_at    TEXT NULL,
       finished_at   TEXT NULL,
       PRIMARY KEY (attempt_id, for_task, check_id),
       CHECK ((outcome IS NULL) = (finished_at IS NULL))
   );
   ```
3. `crates/core-runs/src/model.rs`: `ReportedOutcome { Passed, Failed, NotRun }`
   (`serde(rename_all = "snake_case")`, `ToSchema`, `JsonSchema`) and
   `ReportedCheck { id: String, outcome: ReportedOutcome, #[serde(default)] note: String }`.
4. `crates/core-runs/src/error.rs`: `ReportedChecks(String)`, message
   `"the report's checks do not match the task's: {0}"`, code `reported_checks`.
5. `crates/core-runs/src/report.rs`: `Runs::report` takes `checks: Vec<ReportedCheck>` after
   `report`. Before `verify`, read the attempt's saved context
   (`self.inner.store.context(&attempt.id)`, `store/brief.rs:48`) and its
   `context.task.contract.checks`. Refuse `ReportedChecks` when an id is repeated, unknown, or
   missing (a task with no checks takes an empty list). An attempt with no saved context takes
   any list and records nothing. `RunsStore::record_report` (`store/mod.rs:457`) gains
   `checks: &[(u32, usize, &TaskCheck, &ReportedCheck)]` and inserts one `run_checks` row per
   check (`for_task` = the attempt's task, `ordinal` its position in the contract) inside the
   same write, only when the `UPDATE` moved one row.
6. `crates/shadows-mcp/src/run_tools.rs`: `TaskReportArgs` (line 16) gains
   `/// One entry per check in your task's contract: its id, passed, failed or not_run, and a short note.`
   `#[serde(default)] checks: Vec<ReportedCheck>`; `task_report` passes it on.
7. `crates/core-harness/src/prompt.txt`, after the `task_done` lines (34 to 47): one rule —
   "In a task's contract, add a check for each acceptance item a command can prove: `run` is
   the program and its arguments (`[\"go\",\"test\",\"./internal/api/...\"]`), never a shell
   line; `proves` names the items. A check shared with other tasks' code is fine: Shadows runs
   a project's checks one at a time."
8. `crates/core-harness/src/executor.md`: one paragraph — "Run your task's checks before you
   commit, and fix what fails in your own files. Other executors may be checking at once: a
   lock, busy or port error is not a failure. Go on with your code and try again later. When
   your work is done and a check still cannot run, commit and report it `not_run`. In
   task_report give every check's outcome. Shadows runs the checks again once the project is
   free, and sends you back if they do not pass."
9. Contracts: `crates/core-plans/contract.yaml` (the contract's checks and their problems),
   `crates/core-runs/contract.yaml` (`report` takes and records reported checks),
   `crates/shadows-mcp/src/contract.yaml` (`task_report`'s `checks`).

**Tests (RED first):**
- `crates/core-plans/tests/task_contract.rs`, using `find_customer()` and `with()`:
  `check_needs_a_program`, `check_ids_are_unique`, `check_folder_stays_inside`,
  `check_timeout_is_bounded`, `check_proves_existing_items`,
  `contract_without_checks_reads` (a stored contract JSON with no `checks` key deserializes).
- `crates/shadows-core/tests/runs_attempts/report.rs`, existing fixture, a task whose contract
  has checks `c1`, `c2`: `report_needs_each_check` (missing `c2`, unknown `c9`, `c1` twice:
  each `reported_checks`, attempt still running) and `report_records_reported_checks`.

**Run:** `cargo test -p core-plans --test task_contract`,
`cargo test -p shadows-core --test runs_attempts report`,
`cargo test -p shadows --test planner_instructions`, the OpenAPI update, `npm run gen:api`.
**Commit:** `feat(plans): checks in the task contract, run and reported by the executor`.

### B-113 · Shadows confirms a task's checks on a quiet project, and returns failures

**Owner:** Runs.

**What it adds, where:**

1. `crates/core-db/migrations/0064_attempt_origin.sql`:
   ```sql
   -- What began an attempt: Start, the Planner's addition, or Shadows returning a commit whose
   -- checks did not pass. Attempts before this are 'start' when they carry no addition.
   ALTER TABLE run_attempts ADD COLUMN origin TEXT NOT NULL DEFAULT 'start'
       CHECK (origin IN ('start','planner','checks'));
   UPDATE run_attempts SET origin = 'planner' WHERE addition IS NOT NULL;
   -- When the attempt's Planner notice was queued; NULL until then, and for an attempt that
   -- a later attempt continued, which is never notified.
   ALTER TABLE run_attempts ADD COLUMN notice_sent_at TEXT NULL;
   ```
2. `crates/core-runs/src/model.rs`
   - `AttemptOrigin { Start, Planner, Checks }` (snake_case, `ToSchema`); `Attempt` (line 59)
     gains `pub origin: AttemptOrigin` after `addition`; `AttemptRow`/`into_attempt`
     (`store/mod.rs:336`) and every `SELECT … FROM run_attempts` that builds an `Attempt` read it.
   - `CheckOutcome { Passed, Failed, TimedOut, Inconclusive, NotRun }` (snake_case, `ToSchema`).
   - `CheckResult { for_task: u32, id: String, run: Vec<String>, folder: Option<String>,
     reported: Option<ReportedOutcome>, reported_note: Option<String>,
     outcome: Option<CheckOutcome>, matches: Option<bool>, exit_code: Option<i32>,
     head_before: Option<String>, head_after: Option<String>, output: Option<String>,
     truncated: bool, started_at: Option<String>, finished_at: Option<String> }`
     (`Serialize`, `ToSchema`). `matches` is `Some(outcome == reported)` for the attempt's own
     checks once both exist (`passed`↔`Passed`, `failed`↔`Failed`/`TimedOut`,
     `not_run`↔`NotRun`; `Inconclusive` matches nothing), `None` otherwise.
   - `AttemptNotice` (line 182) gains
     `#[serde(skip_serializing_if = "Vec::is_empty", default)] pub checks: Vec<CheckResult>`
     with `output` set to `None` in the card.
3. `crates/core-runs/src/store/checks.rs` (new; `mod checks;` in `store/mod.rs`), on `RunsStore`:
   - `check_results(&self, attempt) -> Result<Vec<CheckResult>, RunsError>`, ordered by `for_task`
     (own task first), then `ordinal`;
   - `start_check(&self, attempt, for_task, &TaskCheck, ordinal, head) -> Result<(), RunsError>`:
     inserts or updates the row with `started_at`, `head_before`, clearing outcome fields;
   - `finish_check(&self, attempt, for_task, id, Finished) -> Result<(), RunsError>`;
   - `clear_unfinished(&self, attempt)`: deletes rows of `for_task != own` and resets own rows'
     Shadows columns where `finished_at IS NULL`;
   - `quiet_threads(&self, project) -> Result<Vec<ThreadId>, RunsError>`: `thread_id` of every
     attempt of the project with `released_at IS NULL` (B-150 adds threads with a `sent` message);
   - `attempt_writes(&self, project) -> Result<Vec<Vec<String>>, RunsError>`: `writes_json` of
     every attempt of the project, any state, distinct;
   - `latest_committed(&self, workflow, version, task) -> Result<Option<AttemptId>, RunsError>`:
     the task's newest attempt by `number` when it is `committed`, released or not;
   - `returns_since(&self, attempt) -> Result<u32, RunsError>`: counting from `attempt` down by
     `number`, the attempts with `origin = 'checks'` before the first `start` or `planner` one;
   - `continued(&self, attempt) -> Result<bool, RunsError>`: a later attempt of the same task
     and version exists;
   - `mark_notice_sent(&self, attempt)`; `unsent_committed(&self) -> Vec<AttemptId>`: each
     task's newest attempt when it is `committed` with `notice_sent_at IS NULL`.
4. `crates/core-runs/src/store/mod.rs`
   - `Continued` (line 75) gains `origin: AttemptOrigin`; `continue_attempt` (line 557) writes it
     and takes its `command_id` from the caller: `amend-{new id}` for `Planner` as today, and
     `checks-{previous id}` for `Checks`. A return is a replay first: its first statement in the
     write looks up the row whose `command_id` is `checks-{previous id}` and, when one exists,
     answers it (`Self::load`) before the `state`/`released_at` check that would refuse a
     released `previous` with `NotAwaiting`. A `Planner` addition keeps today's order.
   - `additions_since_start` (line 534) counts `origin = 'planner'` attempts, from the newest
     down, skipping `checks` ones, until the first `start` one.
5. `crates/core-runs/src/git.rs`: `Git::hash(&self, paths: &[String]) -> Result<Vec<(String, String)>, String>`
   running `git hash-object --no-filters -- <paths>` (a deleted path hashes to `"deleted"`).
6. `crates/core-runs/src/checks.rs` (new; `mod checks;` in `lib.rs`), `impl Runs`:
   - `pub async fn check_results(&self, attempt: &AttemptId) -> Result<Vec<CheckResult>, RunsError>`
     (`Runs::checks` already names Start's admission checks, `start.rs:148`; the store's own
     `RunsStore::check_results` reads the rows).
   - `pub(crate) async fn confirm(&self, attempt: &Attempt)`: the whole job, logged, never
     failing the attempt:
     1. Gather: own checks from `store.context(&attempt.id)`'s `task.contract.checks`; for each
        `ContractSource::Task { target: Local { task: t }, .. }` in its `uses`, `t`'s
        `latest_committed` attempt's saved context's checks. None at all → `self.after_checks`.
     2. Wait for quiet: loop `store.quiet_threads(project)`, excluding this attempt's own
        thread, and `turns.operations(thread)` for each; any `finished_at == None` → sleep 1 s
        and look again.
     3. Lock: `Inner.checking: tokio::sync::Mutex<HashMap<ProjectId, Arc<tokio::sync::Mutex<()>>>>`
        (new field, built in `Runs::new`); take the project's mutex for the whole run, then look
        for quiet again (step 2) while holding it, so a turn that began while this one waited
        for the lock is waited for too.
     4. Leftovers: `git.changed(prefix)` (`git.rs:222`) filtered to paths that any
        `attempt_writes` pattern admits (`Scope::new(...).admits`). Non-empty → every check
        finishes `Inconclusive` with output `uncommitted changes inside task writes: <paths>`,
        none is spawned.
     5. Each check in order: `head = git.head()`, `dirty = git.changed(prefix)`,
        `hashes = git.hash(&dirty)`; `start_check`; spawn through `shadows_process::spawn` with
        `ProcessSpec { executable: run[0], args: run[1..], cwd: folder joined to the project
        folder, env, capture_stdout: true, pipe_stdin: true }`, dropping stdin at once; read
        stdout and stderr together into a buffer that keeps the last 65 536 bytes (`truncated`
        when more came); `tokio::time::timeout(check.timeout(), handle.wait())`, on expiry
        `handle.terminate_tree()`; then read HEAD, `changed` and hashes again, and the project's
        executor operations: Start and messages are not blocked during a check, so a turn that
        began or was still running at any point of it (an operation of a `quiet_threads` thread
        whose `started_at` is after the check's start, or whose `finished_at` is `None`) is
        found here. Such a check is not recorded: Runs waits for quiet again and runs it once
        more, three times at most; the third such run is `Inconclusive` with output
        `another executor's turn ran during the check`. `env`: `PATH`,
        `HOME`, and every daemon variable whose name starts with `GO`, `CARGO_`, `RUSTUP_`,
        `JAVA_`, `NODE_`, `NPM_`, `PYTHON`, `VIRTUAL_ENV`, `COMPOSER_` or `PHP`, never one whose
        name contains `KEY`, `TOKEN`, `SECRET` or `PASSWORD` (the launcher adds the Windows
        essentials itself, `shadows-process/src/launch.rs:80`). Outcome: spawn error →
        `NotRun` (output: the error); time limit → `TimedOut`; HEAD, dirty paths or a hash
        changed → `Inconclusive`; exit 0 → `Passed`; else `Failed`. `finish_check`.
     6. Release the lock, then `self.after_checks(attempt)`.
   - `async fn after_checks(&self, attempt: &Attempt)`:
     1. `store.continued(&attempt.id)` → return (only the latest attempt acts).
     2. Every own and used check `Passed` (or none) → `self.notify(attempt)`, then
        `mark_notice_sent`.
     3. Otherwise, `returns_since(attempt) < 2` → `self.return_to_executor(attempt)`.
     4. Otherwise → `self.notify(attempt)`, `mark_notice_sent`.
   - `async fn return_to_executor(&self, attempt: &Attempt)`: `continue_attempt(&attempt.id,
     Continued { addition: <text>, start_head: git.head(), origin: Checks })`, then
     `self.run_turn(next, prompt)` (`start.rs:67`). A refusal from `continue_attempt` (the
     version may no longer start work) → `notify` as in step 4. `addition` text and prompt:
     ```text
     Shadows ran T{n}'s checks on commit {sha} once the project was free, and they did not all pass:
     {one line per check: "c1 failed (exit 1); you reported passed", then its output end, up to 4,000 chars}
     The project is free now: run the checks and finish inside the task's writes, then commit and
     report as before. If nothing needs to change, report the same commit {sha}.
     {crate::COMMIT}
     ```
7. `crates/core-runs/src/watch.rs` (line 95, `AttemptState::Committed`): replace
   `if told { self.notify_logged(&now).await; }` with
   `if told { let runs = self.clone(); tokio::spawn(async move { runs.confirm(&now).await; }); }`,
   so the watcher's `hold` drops as before and confirmation runs on its own task.
8. `crates/core-runs/src/report.rs`: in `verify` (line 71), when the attempt's `origin` is
   `Checks` and `sha` equals the commit of the attempt it continues (the task's attempt with
   `number - 1`), skip the `NotNew` and `EmptyCommit` checks; every other check stays,
   `DirtyScope` included.
9. `crates/core-runs/src/notify.rs`: `notify_ended` (line 228), committed branch, puts
   `self.check_results(&attempt.id)` (output cut) in `card.checks` and adds to the prompt, before
   "Read its change", a `Checks:` block, one line each:
   `c1 passed` · `c2 failed (exit 1); the executor reported passed` · `T3 c1 passed`, and, when
   any did not pass, "Shadows returned it to the executor twice; read task_checks for the
   output." (B-114 adds `task_checks`; until then the line says "read the output in the task's
   Report tab").
10. `crates/core-runs/src/recover.rs`: after `settle_running` and the conflict pass, a third
    pass in the same spawned task: for each `store.unsent_committed()` attempt that is not
    `continued`: unfinished rows or checks without Shadows rows → `clear_unfinished`, then
    `confirm`; otherwise `after_checks`.
11. `crates/core-runs/src/amend.rs`: unchanged code; `additions_since_start` now ignores
    `checks` attempts (item 4).
12. Contracts: `crates/core-runs/contract.yaml` (confirmation, returns, origin, recovery);
    `docs/codebase/README.md` (row for `checks.rs` and `store/checks.rs`).

**Tests (RED first)**, new module `crates/shadows-core/tests/runs_attempts/checks.rs`
(`#[path = "runs_attempts/checks.rs"] mod checks;` in `runs_attempts.rs`), the existing fixture
with a task whose contract has checks, `git` as the program:
`passed_and_failed_checks_are_recorded` (`["git","--version"]`, `["git","rev-parse","no-such-ref"]`),
`mismatch_with_report_is_marked`, `missing_program_is_not_run`,
`check_time_limit_ends_it` (`["git","hash-object","--stdin"]` with stdin held open, 1 s),
`long_output_keeps_its_end` (`["git","log","--all","--format=%H%n%H%n%H"]` on a repository of
enough commits, or a check folder holding a large file read by `["git","hash-object","-w","--stdin"]`;
the builder picks one that prints over 65 536 bytes), `checks_wait_for_a_quiet_project`
(a second attempt held running; the first one's rows stay unstarted until it is released),
`uncommitted_change_in_writes_is_inconclusive`, `stopped_attempts_leftover_is_inconclusive`,
`person_files_do_not_count`, `content_change_during_check_is_inconclusive`,
`head_moved_during_check_is_inconclusive`, `quiet_is_checked_again_under_the_lock`,
`turn_during_check_reruns_it`, `checks_of_one_project_run_one_at_a_time`,
`used_tasks_checks_run_too`, `failed_confirmation_returns_to_executor`,
`two_returns_then_planner`, `returns_are_not_additions`,
`return_without_change_reconfirms_previous_commit`, `repeated_return_answers_the_same_attempt`
(the return is started twice for one attempt: one new attempt, the second call answers it),
`restart_reruns_unfinished_checks_and_notifies_once`, `restart_sends_an_unsent_notice`,
`restart_during_return_sends_no_old_notice`, `restart_after_return_passed_notifies_once`.

**Run:** `cargo test -p shadows-core --test runs_attempts`, `--test recovery`,
`--test task_conflicts`; `cargo test -p shadows --test codemap`.
**Commit:** `feat(runs): confirm a task's checks on a quiet project and return failures`.

### B-114 · The Planner reads checks and judges on them

**Owner:** Runs, with its adapters.

**What it adds, where:**

1. `crates/core-db/migrations/0065_mark_reason.sql`:
   ```sql
   -- Why a task was marked done over a check that did not pass; NULL otherwise.
   ALTER TABLE run_task_marks ADD COLUMN reason TEXT NULL;
   ```
2. `crates/core-runs/src/error.rs`: `ChecksNotPassed { checks: Vec<String> }`, message
   `"checks did not pass: {}; mark it done only with a reason"` (ids joined, `T3 c1` for a
   used task's), code `checks_not_passed`.
3. `crates/core-runs/src/lib.rs`, `mark_done` (line 225): new last parameter
   `reason: Option<String>` (trimmed; empty is `None`), added to the fingerprint JSON and
   passed to `put_mark`. The release that follows stays as is (B-150 changes it).
4. `crates/core-runs/src/store/marks.rs`: `put_mark` (line 61) takes and writes `reason`, and
   holds the rule, so a replay answers before it and the rule reads the same snapshot as the
   write: after `CommandContext::classify` returns `None` (not a replay), when `done` and
   `reason` is `None`, it reads in the same `conn` the task's newest attempt by `number` when it
   is `committed` (released or not) and that attempt's `run_checks` rows and contract checks
   (its `run_attempt_context`); any check whose `outcome` is not `'passed'` (own or used,
   finished or not, a contract check with no row included) → `ChecksNotPassed`. `mark_in`
   (line 28) reads `reason`.
5. `crates/core-runs/src/model.rs`: `TaskRun` (line 95) gains
   `#[schema(required)] pub done_reason: Option<String>`.
6. `crates/core-runs/src/planner.rs`:
   - `checks_for(&self, grant, workflow, task) -> Result<Vec<CheckResult>, RunsError>`:
     `planner_plan`, then the task's `latest_committed` attempt's `check_results` (`NotFound` when none);
   - `mark_done_for` (line 109) takes `reason: Option<String>` and passes it on.
7. `crates/core-grants/src/tools.rs`: `THREAD_TOOLS` becomes `[&str; 35]` with `"task_checks"`
   after `"task_file"`.
8. `crates/shadows-mcp/src/run_tools.rs`:
   - `TaskChecksArgs { workflow_id, task_number }` (as `TaskDoneArgs`) and tool `task_checks`,
     description: "The checks of a task's latest committed attempt: for each, what the executor
     reported, what Shadows found when it ran it on a quiet project (passed, failed, timed_out,
     inconclusive or not_run), whether the two match, its exit code and the end of its output.
     Read it before you judge the commit." Answer: the `Vec<CheckResult>` as JSON.
   - `TaskDoneArgs` (line 37) gains `/// Why the task is done although a check did not pass; leave it out otherwise.`
     `#[serde(default)] reason: Option<String>`.
9. `crates/shadows-http/src/runs.rs`:
   - `MarkDone` (line 34) gains `#[serde(default)] reason: Option<String>`; `mark_done` passes it.
   - New route `GET /api/attempts/{id}/checks` → `Vec<CheckResult>`, written like
     `/api/attempts/{id}/conflict` (line 271), registered beside it and in
     `crates/shadows-http/src/openapi.rs`.
10. `crates/core-harness/src/prompt.txt`: the line at 43 changes from "first read what it
    changed with task_diff" to "first read task_checks, then what it changed with task_diff";
    one new line: "A check that did not pass, or whose result differs from the executor's
    report, is not done unless you know why; then task_done carries that reason."
11. Contracts: core-runs (`mark_done`'s rule, `checks_for`), core-grants (`task_checks`),
    shadows-http (route, `reason`), shadows-mcp (`task_checks`, `reason`).

**Tests (RED first):** in `runs_attempts/checks.rs`: `done_over_failed_check_needs_reason`,
`done_before_results_needs_reason`, `done_again_after_undo_needs_reason`,
`done_reason_is_recorded`, `done_replay_answers_after_checks_change` (a done mark with a reason,
then a check result changes, then the same command again: the first answer, no refusal); in `crates/shadows/tests/runs_routes.rs`:
`checks_route_answers_results`, `done_route_refuses_without_reason`; in
`crates/shadows/tests/planner_mcp.rs`: `task_checks_reads_latest_commit`.
**Run:** those three targets, `--test planner_instructions`, the OpenAPI update, `npm run gen:api`.
**Commit:** `feat(runs): the Planner judges on recorded checks`.

### B-150 · The Planner messages a task's agent

**Owner:** Runs, through Turns' send and notice and Harness's executor session.

**What it adds, where:**

1. `crates/core-db/migrations/0066_agent_messages.sql`:
   ```sql
   -- The Planner's plain messages to a task's executor, one in flight per thread, in order.
   CREATE TABLE run_agent_messages (
       id           TEXT PRIMARY KEY,
       attempt_id   TEXT NOT NULL REFERENCES run_attempts(id) ON DELETE CASCADE,
       thread_id    TEXT NOT NULL,
       ordinal      INTEGER NOT NULL,
       message      TEXT NOT NULL,
       state        TEXT NOT NULL
           CHECK (state IN ('waiting','sent','replied','delivered','failed')),
       operation_id TEXT NULL,
       reply        TEXT NULL,
       reason       TEXT NULL,
       created_at   TEXT NOT NULL,
       answered_at  TEXT NULL,
       delivered_at TEXT NULL,
       UNIQUE (thread_id, ordinal)
   );
   ```
2. `crates/core-runs/src/model.rs`: `AgentMessageId` (as `AttemptId`); `AgentMessageState`
   (the five states, snake_case); `AgentMessage { id, attempt_id, task, state }` (`Serialize`,
   `ToSchema`); `AgentReplyNotice { workflow_id, version, task, title, attempt_id, message,
   reply: Option<String>, reason: Option<String> }` (`Serialize`).
3. `crates/core-runs/src/error.rs`: `EmptyMessage` (`"a message says something; it cannot be
   empty"`, `empty_message`) and `AgentAnswering { task: u32 }` (`"T{task}'s agent is answering
   a message; send the addition after its reply"`, `agent_answering`).
4. `crates/core-runs/src/store/messages.rs` (new): `add(&self, attempt, thread, text) ->
   Result<(AgentMessage, bool), _>` (one write: an earlier `waiting`/`sent`/`replied` row of the
   same attempt with the same trimmed text answers it with `false`; otherwise inserts at
   `MAX(ordinal)+1`, `true`); `next_waiting(thread)`; `in_flight(thread) -> bool`
   (a `sent` row); `set_sent(id, operation)`; `set_replied(id, reply)`; `set_failed(id, reason)`;
   `set_delivered(id)`; `pending_for_task(workflow, version, task) -> bool` (a `waiting` or `sent`
   row on any attempt of the task); `unfinished() -> Vec<rows>` for recovery. `quiet_threads`
   (B-113) also returns threads holding a `sent` row.
5. `crates/core-runs/src/notify.rs`: `failed_message` (line 335) becomes
   `last_message(&self, thread, operation) -> Option<String>` (same 4,000-char cut), used by
   both. New `notify_reply(&self, attempt, message)`: `recipient`, a `SendTurn` with
   `command_id` `reply-{message id}`, prompt
   `[Shadows] Reply from task agent · T{n} "{title}":\n{reply}` (or
   `[Shadows] No reply from task agent · T{n}: {reason}`), card `AgentReplyNotice`, through
   `turns.queue_notice` in the executor's name; then `set_delivered`.
6. `crates/core-runs/src/message.rs` (new), `impl Runs`:
   - `pub async fn message_agent(&self, grant, workflow, task, message: String) ->
     Result<AgentMessage, RunsError>`: `planner_plan`; trimmed empty → `EmptyMessage`; the
     task's newest attempt of `plan.version` (`self.attempts(...)` first) or `NotFound`;
     `store.add`; when inserted, `self.pump(&thread)`.
   - `async fn pump(&self, thread)`: return when `in_flight(thread)` or the thread has an
     operation with `finished_at == None`; otherwise take `next_waiting(thread)` and send it.
   - `async fn send_message(&self, message)`: `harness.open_executor(thread, TaskScope { the
     attempt's workflow, version, task }, attempt.model)` (it reopens a closed or gone session;
     `run_turn` is not used, since it refuses an attempt that is not running);
     `turns.send_with_default_effort(thread, SendTurn { command_id: format!("message-{id}"),
     prompt: <text>, mode: MODE, model: attempt.model, .. })` (`MODE` in `start.rs:15` becomes `pub(crate)`); `set_sent`; spawn
     `watch_message`. Prompt: "The Planner asks you, about task T{n}, without changing it:
     {message}\nAnswer in plain words. Change no file and make no commit for this."
     An open or send error → `set_failed`, `notify_reply`.
   - `async fn watch_message(&self, message, operation)`: poll `turns.operations(thread)` every
     second as `watch.rs` does until `finished_at`; `Completed` → `last_message` →
     `set_replied` → `notify_reply`; else `set_failed` (`failure_reason`) → `notify_reply`.
     Then `self.close_if_idle(&attempt)` and `self.pump(&thread)`.
   - `async fn close_if_idle(&self, attempt)`: when the attempt is not `Running`, no
     operation of the thread is unfinished and no message waits: `harness.release_executor`,
     and `store.release(&attempt.id)` when the task has a done mark or the attempt is not
     `Committed`.
7. `crates/core-runs/src/watch.rs`: at the end of `observe`, after the state's arm, call
   `self.pump(&thread)`.
8. `crates/core-runs/src/amend.rs` (line 26): before `awaiting`, `pending_for_task` →
   `AgentAnswering { task }`.
9. `crates/core-runs/src/lib.rs`, `mark_done` (line 250): when `pending_for_task`, skip
   `release_logged`; `close_if_idle` after the reply releases instead.
10. `crates/core-runs/src/recover.rs`: a fourth pass: each `unfinished()` row — `sent`: read
    `turns.operations(thread)`; the operation finished `Completed` → `last_message`,
    `set_replied`, deliver; finished otherwise or not found → `set_failed`, deliver; `replied`
    → deliver; `waiting` → `pump(thread)`.
11. `crates/core-grants/src/tools.rs`: `THREAD_TOOLS` `[&str; 36]` with `"message_agent"`.
12. `crates/shadows-mcp/src/run_tools.rs`: `MessageAgentArgs { workflow_id, task_number,
    message }` and tool `message_agent`, description: "Ask a task's executor a plain question
    (why it chose something, what it ran) without changing the task: no attempt, no addition.
    A busy agent answers when its turn ends. The reply reaches you as \"Reply from task agent ·
    T<n>\"."
13. `crates/core-harness/src/prompt.txt`, after the `task_amend` lines: "To ask an executor a
    question, use message_agent, never task_amend: an addition is for changes to the task's
    files."
14. Contracts: core-runs, core-grants, shadows-mcp; `docs/codebase/README.md`.

**Tests (RED first)**, new module `crates/shadows-core/tests/runs_attempts/messages.rs`, the
existing fixture: `message_changes_no_attempt`, `message_waits_while_committed_turn_runs`
(held turn, report accepted, message stays `waiting` until `release`),
`messages_go_one_at_a_time`, `same_text_is_one_message`, `message_after_done_reopens_session`,
`amend_waits_for_reply`, `done_waits_for_reply`, `reply_reaches_planner_once`,
`restart_delivers_an_undelivered_reply`, `restart_reads_a_finished_reply_turn`;
in `crates/shadows/tests/planner_mcp.rs`: `message_agent_tool_listed`.
**Run:** `cargo test -p shadows-core --test runs_attempts`, `--test executor_session`;
`cargo test -p shadows --test planner_mcp --test planner_instructions`.
**Commit:** `feat(runs): the Planner messages a task's agent`.

### F-103 · Checks and replies in the task page

Masada details it in `docs/frontend/stages.md` from the wire types above
(`TaskCheck`, `CheckResult`, `AttemptOrigin`, `TaskRun.done_reason`, `AgentReplyNotice`,
`GET /api/attempts/{id}/checks`): the task's checks in the brief panel's contract; the Report
tab's Checks section (each check's reported and confirmed outcome, a mismatch marked, exit code,
HEAD, the output on request); an attempt that Shadows returned shown as "Returned by checks";
a done mark's reason beside the mark; and the reply card in the Planner conversation.

### B-151 · Real slice 5 acceptance

**Owner:** integration acceptance through AppCore and the real UI.

**Files:**
- Create: `crates/shadows/tests/checks_live.rs` (`#[ignore]`, signed-in subscription, the
  pattern of `crates/shadows/tests/task_questions_live.rs`)
- Create: `docs/evidence/2026-11-15-slice-5-checks.md`
- Modify: `docs/backend/stages.md`

**Steps:**
1. Live tests: a Planner writes a task with a passing and a failing check; the executor runs
   them, fixes what it can and reports each; Shadows confirms on a quiet project; a failing one
   returns to the executor, and the notice lists the results with any mismatch; `task_done`
   without a reason is refused and with one is recorded; `message_agent` reaches a running
   executor and a done one, and both replies arrive.
2. Browser on a disposable project (`testing4` or new): the same journey, the Report tab's
   checks, two tasks committing together (executors check at once; Shadows confirms after both
   turns end, one check at a time in the log), and a daemon restart during a long check.
3. Record the evidence with commit, OS, harness, model and what was not tried.

## Slice acceptance and handoff

The slice is done when B-151's browser run and logs are recorded. The full gate runs in CI
after the branch is pushed; one whole-branch review runs before the PR.
