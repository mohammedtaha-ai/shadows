# Slice 6: stop control and full acceptance — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); steps use checkbox (`- [ ]`) syntax. Approved by Mohammed on
> 2026-11-16, after two Codex reviews.

**Goal:** The Planner always knows which attempts can still write and why each one ended, so it
stops the affected ones itself. Work left by a stopped attempt carries on. A done mark rechecks
the plan's earlier work. Project decisions are proposed, accepted by the person, and pinned on
tasks.

**Architecture:** Runs (`crates/core-runs`) owns stops, leftovers and rechecks; Design
(`crates/core-design`) owns decisions; Plans (`crates/core-plans`) owns a task's pins. All are
reached through `AppCore`. Two new tables: `run_rechecks`/`run_recheck_results` (Runs) and
`design_decisions` (Design).

**Tech Stack:** Rust 1.94+, SQLx 0.9 / SQLite, real git through `shadows-process`, the real `/mcp`.

**Spec:** [executors design §12 "When the plan is wrong"](../specs/2026-11-08-executors-design.md)
(Mohammed, 2026-11-14); [task questions design, "Conflicts"](../specs/2026-11-14-task-questions-design.md);
[§17.3 decisions](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md);
[shared model](../../shared/requests/2026-11-09-local-delivery.md), "Design: minimal decisions".
Ordering: [the program](2026-11-09-local-delivery-program.md), slice 6.

**Rulings for this slice (Mohammed, 2026-11-16):**
- **Leftovers continue.** A Start accepts uncommitted files inside the task's writes when the
  task's last attempt ended without a commit, and the new executor is told which files they are.
  This replaces slice 4's "dirty-scope checks continue to govern the next Start" for that case.
- **A done mark rechecks every done task of its plan.** The checks of every other task already
  marked done in the same plan, any version, run again. A failure is reported to the Planner;
  no mark is undone.
- **Decisions are accepted on a card in the conversation.** The Planner proposes; only the
  person accepts, as with map and roadmap proposals. Accepting a new version supersedes the
  older one. Tasks pin accepted revisions; the executor reads them in its brief.

## What is already built, and what is left

Slices 4 and 5 already built most of what the old draft's B-117 asked for:

| Asked | Already built |
|---|---|
| A contract problem stops only its own attempt | `conflict_for` + `watch` (slice 4) |
| The Planner stops an affected attempt | `task_stop` → `Runs::stop_for` (slice 4) |
| A new Draft blocks new Starts on the old version, and stops nothing else | `admit_in` (slice 4) |
| Old work reports and is judged against its own version | `old_work_reports_after_a_corrective_draft` |
| Unfinished checks rerun after a restart | `recover` (slice 5) |

What is left, checked in the code on 2026-11-16:

1. **The Planner cannot list what still runs.** `task_stop` needs an attempt id, and after a
   compaction or a restart the Planner cannot tell which attempts still hold a scope. (B-117)
2. **Leftover files block the next Start.** `Runs::checks` refuses any dirty path inside writes
   with `dirty_scope`, so work a stopped attempt left can never be continued. (B-153)
3. **A restart and the person's Stop are silent.** `recover` sends no notice for an Interrupted
   attempt, and `watch` sends none for a cancelled turn. (B-152)
4. **A producer's later change is never rechecked.** `confirm` runs the checks of the tasks an
   attempt *uses*, but nothing reruns the checks of tasks already done. (B-154)
5. **Decisions do not exist.** `TaskContract` has no `decisions`, and Design has no decision
   record. (B-105)

## Global Constraints

- Start from `AppCore`. Each change stays in its owner: Runs, Design or Plans. Others are called
  only through public methods, and a write that spans owners goes through `*_in(&mut Tx, …)`.
  No SQL outside an owner's `store/`.
- Migrations: `0069_attempt_notice_due.sql` (B-152), `0070_run_rechecks.sql` (B-154) and
  `0071_design_decisions.sql` (B-105). Renumber only if another branch takes the number first.
- **A notice that is owed is written down in the same write that makes it owed** (Codex review,
  2026-11-16). Recovery sends every owed notice not yet sent, never only the ones this start
  created. A crash between the state change and the enqueue loses nothing.
- **Every version of a plan** means every `WorkflowId` that Plans lists for the plan's `PlanId`.
  A `WorkflowId` names one version. Runs gets the list from `Plans::plan` (reads),
  or from `Plans::version_ids_in` (inside a write), and runs no SQL on Plans' tables.
- A notice is resolved once and saved before its first enqueue. Every retry sends the saved
  payload with the same command id: `notify-<attempt>` for an attempt, `recheck-<batch>` for a
  recheck.
- Old fingerprints stay byte-identical. `TaskContract.decisions` and `TaskContext.decisions` are
  serde default and skipped when empty.
- An adapter translates only. Each route or tool calls one method.
- **What the web sees:** B-152 (two notice outcomes), B-154 (a recheck card) and B-105 (the
  decision card, its accept route and the brief's decisions) change it. They stay `blocked`
  until Masada's shared request for them is accepted. B-117 and B-153 change no shape.
- New tests: real git and real folders through the existing `runs_attempts` / `task_conflicts`
  fixtures, as slices 4 and 5 did. The live test runs on the signed-in Claude Code subscription.
- Each task updates the contracts it touches, `docs/codebase/README.md` for a new module, and
  `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`) when a route or
  serialized type changes, all in the same commit. Per-task checks are the ones
  `docs/backend/AGENTS.md` rule 5 names.

## Review Focus

- **A crash between settling and sending.** An owed notice is sent at the next start, exactly
  once: `an_owed_notice_is_sent_on_the_next_start`, `a_persons_stop_notice_survives_a_failed_enqueue`,
  `a_second_restart_sends_no_second_interrupted_notice` (B-152);
  `a_finished_batch_never_loses_its_notice` (B-154).
- **A done mark with no judged attempt.** A task marked done again by a new command stays in the
  recheck: `a_task_marked_done_again_stays_in_the_recheck` (B-154).
- **The Planner's own Stop.** `task_stop` sends the Planner no notice about its own call:
  `the_planners_stop_sends_no_notice` (B-152).
- **The person's own edit is not a leftover.** A dirty file inside writes with no ended attempt
  before it, or after a committed one, is still refused:
  `dirty_files_without_an_ended_attempt_still_refuse_start` (B-153).
- **A replayed done mark.** A replay starts no second recheck:
  `a_replayed_mark_starts_no_second_recheck` (B-154).
- **A superseded decision on a frozen task.** A frozen task's pin keeps its version, and the brief
  shows that version with its state `superseded`:
  `a_frozen_pin_keeps_its_superseded_revision` (B-105).

---

## Task B-117: the Planner lists the attempts that still hold a scope

**Files:**
- Modify: `crates/core-runs/src/model.rs` (new `LiveAttempt`)
- Modify: `crates/core-runs/src/lib.rs` (re-export)
- Modify: `crates/core-runs/src/planner.rs` (`Runs::live_for`)
- Modify: `crates/core-runs/src/store/mod.rs` (`RunsStore::live`)
- Modify: `crates/shadows-core/src/lib.rs` (add `LiveAttempt` to the `core_runs` re-exports)
- Modify: `crates/shadows-mcp/src/run_tools.rs` (`task_runs`)
- Modify: `crates/core-grants/src/tools.rs` (the Planner's list: 34 → 35)
- Modify: `crates/core-harness/src/prompt.txt` (one line)
- Modify: `crates/core-runs/contract.yaml`, `crates/core-grants/contract.yaml`, `crates/shadows-mcp/src/contract.yaml`, `crates/core-harness/contract.yaml`
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/runs_attempts/planner.rs`, `crates/shadows/tests/task_conflict_routes.rs`, `crates/shadows/tests/mcp_server.rs`

**Interfaces:**
- Produces:
  - `pub struct LiveAttempt { attempt_id: AttemptId, workflow_id: WorkflowId, version: i64, task: u32, title: String, number: u32, state: AttemptState, commit: Option<String>, conflict: bool, started_at: String }`, with `Serialize` and `schemars::JsonSchema` like the other MCP answers in `model.rs`.
  - `Runs::live_for(&self, grant: &Grant, workflow: &WorkflowId) -> Result<Vec<LiveAttempt>, RunsError>`
  - `RunsStore::live(&self, workflows: &[WorkflowId]) -> Result<Vec<Attempt>, RunsError>` (private to the crate)
- Consumes: `Runs::planner_plan` (the grant check `start_for` uses); `Plans::plan(&PlanId)` (its `PlanVersions`) for every version's `WorkflowId` (a `WorkflowId` names one version; `Plan.plan_id` is what the versions share); Plans' read of a version's task titles, as `Runs::title` in `notify.rs` does.

- [ ] **Step 1: Write the failing tests** in `runs_attempts/planner.rs`:
  - `task_runs_lists_the_attempts_that_hold_a_scope`: Start T1 and T2 (disjoint writes); T2 reports a commit and its turn ends; T3 is started and stopped. `live_for` returns T1 (`Running`, `commit: None`) and T2 (`Committed`, `commit: Some(sha)`), in `(version, task, number)` order. T3 is absent. `title` is the task's title. `conflict` is false.
  - `task_runs_lists_an_old_versions_attempt_after_a_new_approval`: T1 runs on v1. Draft v2, approve it. `live_for` lists T1 with `version: 1`, and nothing for v2.
  - `task_runs_marks_a_conflicted_attempt`: an accepted conflict on a Running attempt gives `conflict: true` while it still holds its scope.
  - `task_runs_is_refused_to_another_grant`: an executor grant, and a Planner grant of another project, each get `RunsError::NotFound`.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts planner::task_runs`. Expected: FAIL to compile (`live_for` is missing).
- [ ] **Step 3: Implement `RunsStore::live`.** It reads the attempts whose `workflow_id` is one of `workflows` and whose `released_at IS NULL`, `ORDER BY version, task, number`. `conflict` is `conflict_json IS NOT NULL`, read in the same query.
- [ ] **Step 4: Implement `Runs::live_for`.** First `planner_plan(grant, workflow)`; then `Plans::plan(&plan.plan_id)` for the version ids; then `RunsStore::live`. Each attempt maps to a `LiveAttempt` with its version's task title (`T{task}` when it has none, as `Runs::title` does). It only reads, and runs no SQL on Plans' tables.
- [ ] **Step 5: Add the MCP tool `task_runs`** with input `{ workflow_id }`. It answers `Vec<LiveAttempt>`, and `RunsError` maps through the existing run error translation. Add it to the Planner's list in `tools_for` (35 tools; only the Planner holds it).
- [ ] **Step 6: Add one line to `prompt.txt`** next to the `task_stop` line: "`task_runs` lists the attempts of a plan that can still write or still wait for your judgment, any version; read it before you choose which to stop with `task_stop`."
- [ ] **Step 7: Extend the MCP tests.** In `mcp_server.rs`, the Planner's tool list includes `task_runs` and the executor's does not. In `task_conflict_routes.rs`, add `the_planner_lists_live_attempts_over_mcp`: over the real `/mcp`, a Planner grant's `task_runs` answers the running attempt's id, which `task_stop` then stops; a second `task_runs` answers `[]`.
- [ ] **Step 8:** Run `cargo test -p shadows-core --test runs_attempts planner::` and `cargo test -p shadows --test task_conflict_routes --test mcp_server --test planner_instructions`, then the rule 5 checks. Expected: PASS.
- [ ] **Step 9:** Update the contracts: in Runs, `functions.live_for`, `shapes.LiveAttempt` and the obligation with its four tests; the Grants count; the MCP tool line; the Harness prompt line. Set B-117 to done in `stages.md` and commit `feat(core): B-117 the Planner lists the attempts that hold a scope`.

## Task B-153: a new Start continues from the files a stopped attempt left

**Files:**
- Modify: `crates/core-runs/src/start.rs` (`checks`, `start`)
- Modify: `crates/core-runs/src/lib.rs` (`first_prompt`)
- Modify: `crates/core-runs/src/ready.rs` (`start_answer` reads the same `checks`)
- Modify: `crates/core-runs/src/store/mod.rs` (`RunsStore::last_ended`)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/runs_attempts/leftovers.rs` (new `#[path]` module in `runs_attempts.rs`), `crates/shadows-core/tests/task_conflicts.rs`

**Interfaces:**
- Produces:
  - `RunsStore::last_ended(&self, workflows: &[WorkflowId], task: u32) -> Result<Option<Attempt>, RunsError>`: the newest attempt with this task number among `workflows` (`ORDER BY started_at DESC, id DESC`), but only when it is Stopped, Interrupted or Failed, has no commit and has released its scope. Otherwise `None`. `checks` passes every version of the plan, from `Plans::plan(&plan.plan_id)`. Across versions the task is matched by its number; the builder confirms in Plans' draft copy that a copied task keeps its number, and reports it if not.
  - `Runs::checks(&self, workflow, version, task) -> Result<Checked, RunsError>` with private `struct Checked { plan: Plan, content: TaskContent, left: Vec<String>, left_by: Option<u32> }`. `left` holds the dirty paths inside writes that were admitted; `left_by` is the number of the attempt that left them.
  - private `Runs::first_prompt(&self, task: u32, left: &[String], left_by: Option<u32>) -> String`
- Consumes: `Runs::dirty` (unchanged).

- [ ] **Step 1: Write the failing tests** in `runs_attempts/leftovers.rs`:
  - `a_start_after_a_stopped_attempt_continues_from_its_files`: attempt 1 of T1 writes `backend/app/a.php` uncommitted and is stopped. A new Start of T1 is accepted. Its first user entry on the new thread (`Threads::entries`) contains `backend/app/a.php` and `Attempt 1`. The file is unchanged on disk.
  - `a_start_after_an_interrupted_attempt_continues_from_its_files`: the same, with a restart in place of the Stop.
  - `dirty_files_without_an_ended_attempt_still_refuse_start`: a person's edit inside T1's writes, with no attempt before it, is `DirtyScope`. After a Committed attempt that was marked done, a new dirty file is `DirtyScope` too.
  - `the_task_read_allows_a_start_over_leftovers`: `Runs::task` answers `start.allowed == true` in the first test's state (the `start_answer` / `start` agreement).
  - In `task_conflicts.rs`, change `conflict_keeps_uncommitted_files`: the s2 Start is now `Ok`, and its prompt names `lib/wip.rs`.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts leftovers::` and `--test task_conflicts conflict_keeps`. Expected: FAIL (`DirtyScope`).
- [ ] **Step 3: Implement `last_ended` and the new `checks`.** A dirty list that is not empty is admitted only when `last_ended` answers an attempt; otherwise `DirtyScope` as today. The required-input check runs after it, unchanged.
- [ ] **Step 4: The prompt.** When `left` is not empty, `first_prompt` adds this line after the task line: `Attempt {left_by} of this task ended without a commit and left these files uncommitted inside your writes: {left joined by ", "}. Read them first; keep what is right, fix or remove the rest, then commit only this task's work.` A prompt set by `set_first_prompt_for_tests` is used as it is.
- [ ] **Step 5:** Run the Step 2 commands, all of `runs_attempts` (`start_refuses_uncommitted_changes_in_writes` must stay green, since it has no ended attempt), `--test task_conflicts`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 6:** Update the contract: `start`, `start_answer`, the obligation "A folder ... with uncommitted changes inside the task's writes cannot start" (add "unless the task's last attempt ended without a commit; the new executor is told the files"), the rule of `conflict_keeps_uncommitted_files`, and the four new tests. Set B-153 to done and commit `feat(core): B-153 a new Start continues from leftover files`.

## Task B-152: the Planner hears of an interrupted attempt and of the person's Stop

Changes what the web sees:
- two new notice cards in the Planner's conversation, `outcome: interrupted` and `outcome: stopped`;
- a new field, `AttemptNotice.left_files: Option<Vec<String>>` (serde default).

This is as the [shared request](../../shared/requests/2026-11-16-slice-6-stop-control.md)'s
answer says. **Blocked until that request is accepted.**

**Files:**
- Create: `crates/core-db/migrations/0069_attempt_notice_due.sql` (`ALTER TABLE run_attempts ADD COLUMN notice_due INTEGER NOT NULL DEFAULT 0 CHECK (notice_due IN (0,1))`; existing rows stay 0, so no old attempt is notified)
- Modify: `crates/core-runs/src/notify.rs` (`ended_intent`: two new bodies)
- Modify: `crates/core-runs/src/recover.rs`
- Modify: `crates/core-runs/src/store/recover.rs` (`settle_running` marks the notice due; `due_notices`)
- Modify: `crates/core-runs/src/store/mod.rs` (`finish_owed`, beside `finish`)
- Modify: `crates/core-runs/src/stop.rs`
- Modify: `crates/core-runs/src/model.rs` (`AttemptNotice.left_files`)
- Modify: `crates/core-runs/contract.yaml`, `api/openapi.json` (regenerated)
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/runs_attempts/stops.rs` (new; `#[path]` module in `runs_attempts.rs`), `crates/shadows-core/tests/task_conflicts.rs`

**Interfaces:**
- Produces:
  - **The notice is owed durably.** `notice_due = 1` is written in the same write that settles the attempt, so a crash or a failed enqueue never loses it:
    - `RunsStore::settle_running` sets it on every attempt it moves to Interrupted, and still answers the count.
    - `RunsStore::finish_owed(&self, id: &AttemptId) -> Result<bool, RunsError>`: a new method, beside `finish`. In one write it moves a Running attempt with no `conflict_json` to Stopped and sets `notice_due = 1`, and answers whether it moved the attempt. Only the person's Stop calls it. `finish` keeps its signature, so its other callers (`abandon` in `start.rs`, the Planner's Stop, …) do not change.
    - `RunsStore::due_notices(&self) -> Result<Vec<AttemptId>, RunsError>`: attempts with `notice_due = 1` and `notice_sent_at IS NULL`, in `ended_at, id` order.
    - `notify_ended` writes `notice_sent_at` after a successful enqueue (if it does not already for every ended notice), so a sent notice is never due again.
  - private `enum Stopper { Person, Planner }` in `stop.rs`, and private `Runs::stop_by(&self, attempt: &AttemptId, by: Stopper) -> Result<Attempt, RunsError>`. `stop` = `stop_by(Person)`, and `stop_for` calls `stop_by(Planner)`. The agreement "the person's Stop and the Planner's task_stop run one method" still holds through `stop_by`.
- Consumes: `notify` / `notify_ended` (unchanged road; its saved intent and `notify-<attempt>` id make a resend safe), and the `dirty` read Start and report use (the paths changed inside the attempt's writes).

- [ ] **Step 1: Write the failing tests** in `runs_attempts/stops.rs`:
  - `a_restart_tells_the_planner_of_each_interrupted_attempt`: T1 is Running and has written `backend/app/a.php` uncommitted. Restart (the restart pattern of `task_conflicts::restart_replays_pending_conflict_once`). The Planner's thread gets one queued notice, command id `notify-<attempt>`; its card has `outcome: interrupted` and `left_files == ["backend/app/a.php"]`; its body contains `was interrupted` and `backend/app/a.php`.
  - `a_second_restart_sends_no_second_interrupted_notice`: restart twice; still one notice.
  - `an_owed_notice_is_sent_on_the_next_start`: the attempt is settled Interrupted with its notice due, but the start ends before the notice is queued (the fixture drops the daemon after `settle_running`, before the spawned pass sends). The next start queues exactly one notice for it.
  - `a_persons_stop_notice_survives_a_failed_enqueue`: the person's Stop succeeds while the enqueue fails (the Planner thread has no turn settings yet, so `notify` answers `Unavailable`). After the Planner has run a turn, a restart sends exactly one `stopped` notice.
  - `the_persons_stop_tells_the_planner`: `Runs::stop` on a Running attempt queues one notice with `outcome: stopped`, and its body contains `stopped by the person`.
  - `the_planners_stop_sends_no_notice`: `stop_for` on a Running attempt queues none.
  - In `task_conflicts.rs`, `a_stopped_conflict_keeps_its_one_notice`: a conflicted Running attempt stopped by the person has exactly its one conflict notice.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts stops::` and `--test task_conflicts a_stopped_conflict`. Expected: the six `stops::` tests FAIL (no notice); the conflict test may already pass and stays as a guard.
- [ ] **Step 3: The migration, the due mark in `settle_running`, and `finish_owed`.** Both write `notice_due` in the write that settles the attempt. Nothing else in `settle_running`'s write changes.
- [ ] **Step 4: `recover` sends what is owed.** In the spawned pass, after the pending conflicts and before `recover_messages`, call `notify` for each id from `due_notices`. This covers this start's Interrupted attempts and anything an earlier start owed. A failure is logged `runs.notify_due_failed`, stays due, and never stops the pass.
- [ ] **Step 5: `stop_by`.**
  - The Person path calls `finish_owed` first. When that answers `false` (the attempt is not Running, or it has a conflict), it calls `finish` as today. After the release it calls `notify` when `finish_owed` answered `true`; a failure is logged and stays due for the next start.
  - The Planner path calls `finish` as today and keeps today's behavior: only `retry_conflict_notice`.
- [ ] **Step 6: Two bodies in `ended_intent`, and `left_files`.** They replace the `_` arm for these two states only; Failed keeps its text. The card's `left_files` is the same dirty list (`[]` when none, `None` when git failed); it is `None` for every other outcome; `reason` and `report` stay `None` for these two. `{dirty}` is the attempt's changed paths inside its writes, joined by `, `, or `none`; a git failure gives `unknown` and logs.
  - Interrupted: `{head} was interrupted: Shadows stopped while it ran, and nothing was committed.\nUncommitted files in its writes: {dirty}\n{last}Tell the person; start it again only if they ask. A new Start continues from these files.`
  - Stopped: `{head} was stopped by the person.\nUncommitted files in its writes: {dirty}\n{last}Do not start it again unless they ask. A new Start continues from these files.`
- [ ] **Step 7:** Run the Step 2 commands, plus `cargo test -p shadows-core --test runs_attempts` (all of it: `stopping_a_committed_attempt_frees_its_scope` and `recover_interrupts_running_attempts_without_restarting_them` must stay green), `--test recovery`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 8:** Update the contract: `recover`, `stop`, `stop_for`, `notify` and the transitions ("found Running at startup: Interrupted; the Planner is told once", "the person's Stop of a Running attempt: Stopped; the Planner is told", "a notice owed and not sent is sent at the next start"), `notice_due`, and the seven tests. Set B-152 to done and commit `feat(core): B-152 the Planner hears of interrupted and stopped attempts`.

## Task B-154: a done mark rechecks every done task of its plan

Changes what the web sees: a new Shadows notice card, `RecheckNotice`. `RecheckNotice` and
`RecheckFailure` are registered in `components.schemas`, as the
[shared request](../../shared/requests/2026-11-16-slice-6-stop-control.md)'s answer says.
**Blocked until that request is accepted.**

**Files:**
- Create: `crates/core-db/migrations/0070_run_rechecks.sql`
- Create: `crates/core-runs/src/recheck.rs`
- Create: `crates/core-runs/src/store/rechecks.rs`
- Modify: `crates/core-runs/src/checks.rs` (move the per-check run out of `confirm_attempt`)
- Modify: `crates/core-runs/src/lib.rs` (`mark_done` spawns the recheck; module list; re-exports)
- Modify: `crates/core-runs/src/store/marks.rs` (`put_mark` begins the batch in its write)
- Modify: `crates/core-runs/src/model.rs` (`RecheckNotice`, `RecheckFailure`)
- Modify: `crates/core-runs/src/recover.rs`
- Modify: `crates/core-runs/src/store/mod.rs` (module)
- Modify: `crates/core-plans/src/lib.rs`, `crates/core-plans/src/store/plan.rs` (`Plans::version_ids_in`)
- Modify: `crates/shadows-core/src/lib.rs` (re-exports)
- Modify: `crates/shadows-http/src/openapi.rs` (register `RecheckNotice`, `RecheckFailure` beside `AttemptNotice`)
- Modify: `crates/core-runs/contract.yaml`, `crates/core-plans/contract.yaml`, `crates/shadows-http/src/contract.yaml`, `docs/codebase/README.md`, `api/openapi.json` (regenerated)
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/runs_attempts/rechecks.rs` (new `#[path]` module), `crates/shadows/tests/openapi.rs` (the two schemas are named)

**Interfaces:**
- Produces:
  - Tables:
    - `run_rechecks (id TEXT PRIMARY KEY, project_id, plan_id, cause_workflow_id, cause_version, cause_task, cause_command TEXT UNIQUE, created_at, finished_at NULL, notice_json NULL, notice_sent_at NULL)`
    - `run_recheck_results (recheck_id REFERENCES run_rechecks ON DELETE CASCADE, attempt_id REFERENCES run_attempts, workflow_id, version, task, check_id, ordinal, run_json, folder NULL, outcome NULL, exit_code NULL, head NULL, output NULL, truncated, started_at NULL, finished_at NULL, PRIMARY KEY (recheck_id, attempt_id, check_id))`, with the `outcome` CHECKs of `run_checks`.
  - `Plans::version_ids_in(conn, plan: &PlanId) -> Result<Vec<(WorkflowId, i64)>, PlansError>`: every version of the plan, oldest first, in the caller's transaction. It goes in Plans' `shared_in_transaction`; Runs runs no SQL on Plans' tables.
  - `pub struct RecheckFailure { workflow_id: WorkflowId, version: i64, task: u32, title: String, attempt_id: AttemptId, checks: Vec<CheckResult> }` (output left out of the card).
  - `pub struct RecheckNotice { plan_id: PlanId, cause_workflow_id: WorkflowId, cause_version: i64, cause_task: u32, recheck_id: String, failed: Vec<RecheckFailure> }`: the card of a Shadows notice, `Notice { executor: None, card }`.
  - private `Runs::recheck(&self, recheck_id: String)` in `recheck.rs`: runs one batch, then notifies.
  - private `Runs::run_check(...) -> Finished` in `checks.rs`: one check on a quiet project, with today's repetition, overlap and inconclusive rules. `confirm_attempt` and `recheck` both call it.
  - `RunsStore::begin_recheck_in(conn, cause) -> Result<Option<String>, RunsError>` in `store/rechecks.rs`, with `start_recheck`, `finish_recheck`, `finish_batch(id, notice_json: Option<String>)`, `recheck_sent` and `unfinished_rechecks`.
- **Which attempt's checks a done task brings:**
  - The mark's `judged_attempt_id`, when it is set.
  - Otherwise, the task's newest attempt in that version with a commit, whatever its state. `judged_attempt_id` is `NULL` after a mark repeated by a new command once the scope was freed, and on a mark from before the column existed.
  - A done task with no committed attempt brings nothing, and so does an attempt with no saved context.

  The checks are that attempt's own checks, from its saved context.
- Consumes: `put_mark`'s write; `check_specs`; the project check lock in `Runs.inner.checking`; the Planner thread's last-turn settings, read as `ended_intent` does.

- [ ] **Step 1: Write the failing tests** in `runs_attempts/rechecks.rs`. The checks are real commands, as in `checks.rs`:
  - `a_done_mark_reruns_the_checks_of_the_plans_done_tasks`: T1 and T2 are done, each with one check. T3 is marked done. One batch runs T1's and T2's checks (not T3's) at the HEAD of that moment, and records two results.
  - `a_failing_recheck_tells_the_planner_once`: T3's commit broke T2's check. One notice is queued with command id `recheck-<id>` and `executor: None`. The card has `failed[0].task == 2`, and the body contains `T2 c1 failed` and the end of the output. Both done marks stand.
  - `a_passing_recheck_sends_no_notice`
  - `a_replayed_mark_starts_no_second_recheck`
  - `a_restart_finishes_an_unfinished_recheck`: the daemon stops mid-batch. On restart, the unfinished results become `not_run` with output `Shadows restarted during the recheck`, the batch finishes, and its notice is sent once.
  - `a_finished_batch_never_loses_its_notice`: the batch finishes with a failure, and the start ends before the enqueue (the fixture drops the daemon after `finish_batch`). The batch already holds `notice_json`, and the next start sends it once.
  - `an_older_versions_done_task_is_rechecked`: T1 is done on v1; v2 is approved and its T2 marked done. T1 of v1, under v1's `WorkflowId`, is in the batch.
  - `a_task_marked_done_again_stays_in_the_recheck`: T1 is marked done, its scope is freed, and it is marked done again by a new command (`judged_attempt_id` is now `NULL`). T2's later done mark still reruns T1's check, from T1's newest committed attempt.
  - `a_done_task_with_no_commit_is_left_out`: a task marked done with no committed attempt adds no row and logs no failure.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts rechecks::`. Expected: FAIL to compile.
- [ ] **Step 3: `version_ids_in`, the migration and the store.** `begin_recheck_in` runs inside `put_mark`'s write, only for a new `done == true` mark:
  - It reads the plan's versions with `Plans::version_ids_in`, in the same write.
  - It inserts the batch and one pending row per own check of every *other* task with a done mark in any of those versions. The checks come from the attempt chosen above, in `(version, task, ordinal)` order.
  - A batch with no rows is not inserted. A replay returns before it, and `cause_command` UNIQUE guards it.
- [ ] **Step 4: `run_check` and `recheck`.** Move `run_check` out of `confirm_attempt` without changing its behavior (`attempt_checks` stays green). `mark_done` spawns `recheck` after its release when a batch was begun. `recheck` waits for quiet, takes the project lock and runs each row.
  - **The end is one write.** First `recheck` resolves the notice when any outcome is not `passed`: the Planner thread and its settings, the body and the card. Then `finish_batch` writes `finished_at` and that `notice_json` together, so no batch can finish failed without its notice. After that write, the saved notice is queued and `notice_sent_at` is written.
  - Body: `[Shadows] After T{cause_task} of version {cause_version} was marked done, Shadows reran the checks of the plan's done tasks. These no longer pass:`, then one `check_line` per failing check, each followed by the last 4,000 chars of its output, then `Their done marks stand. Read the change with task_diff, then amend or plan a correction.`
  - With no live Planner, the batch finishes with `notice_json` NULL and logs `runs.recheck_no_planner`.
- [ ] **Step 5: Recovery.** In the spawned pass, after the committed attempts:
  - Each unfinished batch finishes its open rows as Step 1 says, then ends through the same single write as Step 4.
  - Each finished batch with `notice_json` and no `notice_sent_at` is sent again, under the same command id.
- [ ] **Step 6:** Run Step 2's command, all of `runs_attempts` (`attempt_checks` included), `--test recovery`, `cargo test -p shadows --test openapi`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 7:** Update the contracts:
  - Runs: `mark_done` (it begins and spawns the recheck), a `rechecks:` section (trigger, set, chosen attempt, storage, the single-write end, notice, recovery), `owns_tables`, `modules` and the nine tests.
  - Plans: `version_ids_in`.
  - Code-map rows for `recheck.rs` and `store/rechecks.rs`.

  Set B-154 to done and commit `feat(core): B-154 a done mark rechecks the plan's done tasks`.

## Task B-105: project decisions, proposed by the Planner and accepted by the person

Changes what the web sees: a `DecisionProposal` thread entry with Accept, two routes, two project
events, and `decisions` in the brief, as the
[shared request](../../shared/requests/2026-11-16-slice-6-stop-control.md)'s answer says.
**Blocked until that request is accepted.**

**Files:**
- Create: `crates/core-db/migrations/0071_design_decisions.sql`
- Create: `crates/core-design/src/decisions.rs`
- Create: `crates/core-design/src/store/decisions.rs`
- Create: `crates/shadows-core/tests/design_decisions.rs`
- Modify: `crates/core-design/src/lib.rs`, `crates/core-design/src/model.rs`, `crates/core-design/src/error.rs`, `crates/core-design/src/store/mod.rs`
- Modify: `crates/core-plans/src/task_contract.rs` (validated `decisions` pins)
- Modify: `crates/core-plans/src/store/edit.rs` (the pin check inside the edit's write)
- Modify: `crates/core-plans/src/task_context.rs`, `crates/core-plans/src/store/task_context.rs` (`TaskContext.decisions`, built in the store)
- Modify: `crates/core-plans/src/model.rs` (`DecisionPin`, the field)
- Create: `crates/shadows-mcp/src/decision_tools.rs` (Design decisions MCP translation: `decision_propose`, `decision_list`)
- Modify: `crates/shadows-mcp/src/server.rs` (its router, under the thread grant policy)
- Modify: `crates/core-grants/src/tools.rs` (the Planner's list: +2)
- Modify: `crates/shadows-http/src/design.rs` (the two handlers), `crates/shadows-http/src/lib.rs` (the route table), `crates/shadows-http/src/openapi.rs` (the paths, `DecisionRecord`, `DecisionInput`, `DecisionState`)
- Modify: `crates/shadows-core/src/error.rs` (`ErrorCode::DecisionStale`, `ErrorCode::InvalidDecision` and the `DesignError` arms)
- Modify: `crates/shadows-http/src/failure/` (the two codes' HTTP status and their mapping cases)
- Modify: `crates/shadows/tests/architecture/table-owners.txt` (`design_decisions` → `design`)
- Modify: `crates/shadows-core/src/lib.rs` (re-exports)
- Modify: `crates/core-threads/src/model.rs` (`ThreadEntryKind::DecisionProposal`)
- Modify: `crates/core-events/src/store.rs` (`DecisionProposed`, `DecisionAccepted` in the project tail)
- Modify: `crates/core-harness/src/prompt.txt`, `crates/core-harness/src/executor.md`
- Modify: `crates/core-design/contract.yaml`, `crates/core-plans/contract.yaml`, `crates/core-threads/contract.yaml`, `crates/core-events/contract.yaml`, `crates/core-grants/contract.yaml`, `crates/shadows-mcp/src/contract.yaml`, `crates/shadows-http/src/contract.yaml`, `crates/core-harness/contract.yaml`, `docs/codebase/README.md`, `api/openapi.json`
- Modify: `docs/core/specs/2026-10-01-task-contracts-and-evidence-design.md` (§17.3, and the §17.11 OPEN for decisions)
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/design_decisions.rs`, `crates/shadows-core/tests/task_contract.rs`, `crates/shadows-core/tests/attempt_briefs.rs`, `crates/shadows/tests/design_routes.rs`, `crates/shadows/tests/mcp_server.rs`

**Interfaces:**
- Produces (Design):
  - Table `design_decisions (project_id, decision_id, version, title, decision, rationale, source, state CHECK IN ('proposed','accepted','superseded'), proposed_by, approved_by_json NULL, created_at, accepted_at NULL, PRIMARY KEY (project_id, decision_id, version))`.
  - `pub struct DecisionInput { id: Option<String>, expected_version: Option<i64>, title: String, decision: String, rationale: String, source: String }`
  - `pub enum DecisionState { Proposed, Accepted, Superseded }`
  - `pub struct DecisionRecord { id: String, version: i64, project_id: ProjectId, title: String, decision: String, rationale: String, source: String, state: DecisionState, approved_by: Option<ApprovedBy>, created_at: String }`
  - `DesignError::InvalidDecision(String)` and `DesignError::DecisionStale(String)`. They reach clients, as every Design refusal does, through `CoreError::Refused` with `ErrorCode::InvalidDecision` (`INVALID_DECISION`, 422) and `ErrorCode::DecisionStale` (`DECISION_STALE`, 409); MCP gets the same through its `Refusal`.
  - `Design::propose_decision(&self, grant: &Grant, command_id: String, input: DecisionInput) -> Result<DecisionRecord, DesignError>`:
    - Needs a Planner grant with a running turn, as `propose_vision` does. The replay is classified first.
    - A new id is `D{n}`, the next number in the project.
    - Giving an `id` needs `expected_version` equal to its latest version; otherwise `DecisionStale`.
    - Trimmed lengths: title 1..200 chars, decision and rationale 1..4,000, source 0..1,000; otherwise `InvalidDecision`.
    - It writes version latest+1 as `proposed`. In the same write it appends a `DecisionProposal` entry on the Planner's thread (through `Threads`' entry write, as `propose_vision_in` writes `VisionProposal`), whose card is the `DecisionRecord` as proposed. It also appends the project event `DecisionProposed { project_id, decision_id, version }`.
  - `Design::decisions(&self, project: &ProjectId) -> Result<Vec<DecisionRecord>, DesignError>`: every revision, ordered by id then version.
  - `Design::accept_decision(&self, command_id: String, project: &ProjectId, id: &str, version: i64) -> Result<DecisionRecord, DesignError>`:
    - The person only, over HTTP. The replay is classified first.
    - The revision must be `proposed` and the latest of its id; otherwise `DecisionStale`.
    - In one write it becomes `accepted` with the person's `ApprovedBy`, and the id's earlier accepted revision becomes `superseded`. The same write appends `DecisionAccepted { project_id, decision_id, version }`; superseding adds no event of its own. `DecisionStale` is HTTP 409 and `InvalidDecision` is 422.
  - `Design::decision_in(conn, project: &ProjectId, id: &str, version: i64) -> Result<Option<DecisionRecord>, DesignError>`
- Produces (Plans):
  - `pub struct DecisionPin { id: String, version: i64 }`, and `TaskContract.decisions: Vec<DecisionPin>` (serde default, skipped when empty).
  - Plan edit validation (task_add, task_update): each pin must name an **accepted** revision of the plan's project, read with `Design::decision_in` inside the edit's write. Otherwise it is a `Problem` whose message is `T{n} pins {id} v{version}, which is not an accepted decision` (`Problem` holds only a message, like every plan rule). A frozen task's pins never change.
  - `TaskContext.decisions: Vec<DecisionRecord>` (serde default, skipped when empty): each pinned revision as Design holds it when the context is built, state included. It is saved with the brief at Start like the rest of the context.
- Adapters:
  - MCP, Planner only: `decision_propose` (input `DecisionInput` plus `command_id`) and `decision_list` (the grant's project).
  - HTTP: `GET /api/projects/{id}/decisions` → `Design::decisions`; `POST /api/projects/{id}/decisions/{decision}/accept` with `{ command_id, version }` → `Design::accept_decision`.
  - The card in the conversation is the `DecisionProposal` entry. Its live state comes from `GET …/decisions`.
- Texts:
  - `prompt.txt`: "When a choice settles a project decision (a library, a data shape, a layout rule), propose it with `decision_propose`. The person accepts it on its card, never you. Pin accepted revisions in a task contract's `decisions`."
  - `executor.md`: "Your brief's `decisions` are the project's accepted decisions: follow them, and report a `task_conflict` if the task cannot."

- [ ] **Step 1: Write the failing tests.** In `design_decisions.rs`:
  - `the_planner_proposes_and_the_person_accepts`: propose → `D1` v1 `proposed`, one `DecisionProposal` entry on the Planner's thread with that card, and one `DecisionProposed` on the project tail; accept → `accepted` with a person `ApprovedBy`, and one `DecisionAccepted`.
  - `a_new_version_supersedes_the_accepted_one`: propose D1 v2 with `expected_version: 1` and accept it; v1 is `superseded`.
  - `a_stale_or_invalid_decision_is_refused`: a wrong `expected_version` is `DecisionStale`; an empty title is `InvalidDecision`; accepting v1 after v2 exists is `DecisionStale`.
  - `only_a_planner_proposes`: an executor grant's `propose_decision` is refused, and no tool in any grant's list accepts.
  - `a_decision_replay_returns_the_original_revision`

  Then:
  - `task_contract.rs`: `a_task_pins_only_accepted_decisions` (a proposed pin is refused with a problem containing `which is not an accepted decision`; an accepted one is saved) and `a_contract_without_decisions_keeps_its_fingerprint`.
  - `attempt_briefs.rs`: `a_frozen_pin_keeps_its_superseded_revision` (D1 v1 is pinned and frozen; v2 is accepted; a new Start's brief shows D1 v1 with state `superseded`).
  - `design_routes.rs`: `decisions_are_listed_and_accepted_over_http`.
  - `mcp_server.rs`: the Planner's list has both tools; the executor's has neither.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test design_decisions --test task_contract --test attempt_briefs`. Expected: FAIL to compile.
- [ ] **Step 3: Migration, store and `decisions.rs`**, with the signatures above.
- [ ] **Step 4: Plans' pins and context**, through `Design::decision_in` inside Plans' own write.
- [ ] **Step 5: Adapters and texts**, then `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
- [ ] **Step 6:** Run Step 2's command, then `cargo test -p shadows --test design_routes --test mcp_server --test planner_instructions --test openapi`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 7:** Update the contracts:
  - Design: owns `design_decisions`, the three methods and `decision_in`.
  - Plans: `DecisionPin`, the pin rule, `TaskContext.decisions`.
  - The Grants count, the MCP and HTTP lines, the Harness texts, and the code-map rows.

  In the spec, §17.3 and the §17.11 OPEN now say: "decisions are accepted and superseded only by the person's card (Mohammed, 2026-11-16); gate actions stay open". Set B-105 to done and commit `feat(core): B-105 project decisions accepted by the person`.

## Task B-118: the full journey, live and in the browser

**Files:**
- Create: `crates/shadows/tests/stop_control_live.rs`
- Create: `docs/evidence/<date>-slice-6-stop-control.md`
- Modify: `crates/core-runs/contract.yaml` (`tests:` line for the live file)
- Modify: `docs/backend/stages.md`

**Interfaces:**
- Consumes: B-117, B-153, B-152, B-154, B-105 and the web tasks; the fixtures `checks_live.rs` uses (`fixtures/app.rs`, `listening.rs`, `live.rs`, `plan.rs`).

- [ ] **Step 1: Write `stop_control_live.rs`**, ignored, on the signed-in subscription, with the same setup as `checks_live.rs`. One test, `live_the_planner_stops_affected_work_and_carries_on_after_a_restart`, in a disposable git folder:
  1. The Planner proposes decision D1 ("plain PHP, no framework"); the test accepts it as the person. The Planner plans three tasks, each with a check, and pins D1 on T1. T2 uses T1's output and T3 is independent. The Planner starts the first group (T1 and T3 run together). Assert: T1's brief holds D1.
  2. T1's executor files a `task_conflict`. Assert: T1 ends Stopped with reason `contract conflict`, and T3 is still Running.
  3. Prompt the Planner to assess the impact. Assert: it called `task_runs` and did not stop T3.
  4. While T3 runs and has written a file, shut the daemon down and start it again on the same database. Assert: T3 is Interrupted, the Planner's conversation has exactly one `interrupted` notice for T3 naming the file, and no task restarted on its own.
  5. Ask the Planner to start T3 again. Assert: the Start is accepted over the leftover file and the prompt names it. T3 commits, its checks pass, and the Planner marks it done.
  6. Mark T1 done after a corrected run. Assert: one recheck batch ran T3's check, and it passed with no recheck notice.
- [ ] **Step 2:** Run `cargo test -p shadows --test stop_control_live -- --ignored --nocapture --test-threads=1`. Expected: PASS. Record the model and run time.
- [ ] **Step 3: Browser journey** on a disposable project with `shadows serve`:
  - Accept a decision on its card.
  - Start two tasks. Stop one from the task page and see the Planner's `stopped` card, then start it again over its leftover files.
  - Restart the daemon during the other and see the `interrupted` card.
  - Break an earlier task's check with a later task, mark the later one done, and see the recheck card.
  - Read every card and the daemon log lines (`runs.*`, `design.*`).
- [ ] **Step 4: Write the evidence file**: date, commit, OS, model, the commands, what passed, and what could not be checked.
- [ ] **Step 5:** Add the `tests:` line, set B-118 to done and commit `test(core): B-118 slice 6 live journey and evidence`.

## Handoff

Order: B-117 → B-153 → B-152 → B-154 → B-105 → web tasks (from F-121) → B-118.

- B-117 and B-153 change no shape and can start as soon as this plan is approved.
- B-152, B-154 and B-105 wait for one shared request from Masada that covers all three: the
  `interrupted` and `stopped` cards, the recheck card, and the decision card with its routes and
  brief field.

The full gate and one whole-branch review run before the PR.
