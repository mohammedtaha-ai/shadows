# Slice 6: stop control and full acceptance — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); steps use checkbox (`- [ ]`) syntax. Not approved until Mohammed
> says so; no builder runs before.

**Goal:** The Planner always knows which attempts can still write and why each one ended, so it
stops the affected ones itself and the journey carries on after a restart.

**Architecture:** Everything is in Runs (`crates/core-runs`), reached through `AppCore::runs()`.
One read for the Planner (`task_runs`) and two more notices on the existing `notify` road. No
new table, migration, route or stop machinery.

**Tech Stack:** Rust 1.94+, SQLx 0.9 / SQLite, real git through `shadows-process`, the real `/mcp`.

**Spec:** [executors design §12 "When the plan is wrong"](../specs/2026-11-08-executors-design.md)
(Mohammed, 2026-11-14) and [task questions design, "Conflicts"](../specs/2026-11-14-task-questions-design.md).
Ordering: [the program](2026-11-09-local-delivery-program.md), slice 6.

## What is already built, and what is left

Slices 4 and 5 already built most of what the old draft's B-117 asked for:

| Asked | Already built |
|---|---|
| A contract problem stops only its own attempt | `conflict_for` + `watch` (slice 4) |
| The Planner stops an affected attempt | `task_stop` → `Runs::stop_for` (slice 4) |
| A new Draft blocks new Starts on the old version, and stops nothing else | `admit_in` (slice 4) |
| Old work reports and is judged against its own version | `old_work_reports_after_a_corrective_draft` |
| Pending evidence reruns after a restart | `recover` reruns unfinished checks (slice 5) |

The gaps this slice closes, checked in the code on 2026-11-16:

1. **The Planner cannot list what still runs.** `task_stop` needs an attempt id. The Planner
   only has the ids from `task_start` answers and from notices in its own history. After a
   compaction or a restart, it cannot tell which attempts still hold a scope.
2. **A restart is silent.** `recover` marks each Running attempt Interrupted, but sends no notice
   (`notify_ended` runs only from `watch` and from the committed-attempt pass). The Planner keeps
   waiting for an attempt that ended.
3. **The person's Stop is silent.** `watch` sends nothing for a cancelled turn ("a stopped turn
   is Stop's to record"), and `stop` sends only a pending conflict notice. When the person stops
   a task in the browser, the Planner never hears about it.

**Not in this slice:** pending rechecks after a producer changes, and project decision revisions
(B-105). Both stay in [future](../../core/future.md). Files left behind by an interrupted or
stopped attempt still block the next Start with `dirty_scope`, as slice 4 ruled; the notice
names those files so the Planner can tell the person.

## Global Constraints

- Start from `AppCore`. Runs owns every change here; Plans, Turns and Threads are called only
  through their public methods. No SQL outside `crates/core-runs/src/store/`.
- No new table, column, migration, HTTP route or OpenAPI change. `task_runs` is MCP only.
- A notice is resolved once, saved in `ended_notice_json` before its first enqueue, and sent with
  command id `notify-<attempt>`, exactly as `notify_ended` does today.
- An adapter translates only. `task_runs` in `run_tools.rs` calls one Runs method.
- New tests: real git and real folders through the existing `runs_attempts` / `task_conflicts`
  fixtures, as slices 4 and 5 did. The live test runs on the signed-in Claude Code subscription.
- Each task updates `crates/core-runs/contract.yaml` (and any other contract it touches) in the
  same commit. Per-task checks are the ones `docs/backend/AGENTS.md` rule 5 names.

## Review Focus

- **A daemon killed twice.** Recovery notifies only the attempts it moved to Interrupted in this
  start. A second restart must not notify them again:
  `a_second_restart_sends_no_second_interrupted_notice` (B-152).
- **The Planner's own Stop.** `task_stop` must not send the Planner a notice about its own call:
  `the_planners_stop_sends_no_notice` (B-152).
- **Stopping a committed attempt that waits for judgment.** The existing obligation says this
  "tells the Planner nothing new". The person's-Stop notice covers only Running → Stopped:
  `stopping_a_committed_attempt_frees_its_scope` stays green (B-152).
- **A conflicted attempt stopped by the person.** It already has its one conflict notice, so a
  second (stopped) notice must not go out: `a_stopped_conflict_keeps_its_one_notice` (B-152).
- **An old version's attempt still running after a corrective approval.** `task_runs` lists it
  with its own version, so the Planner can see and stop it:
  `task_runs_lists_an_old_versions_attempt_after_a_new_approval` (B-117).

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
  - `RunsStore::live(&self, workflow: &WorkflowId) -> Result<Vec<Attempt>, RunsError>` (private to the crate)
- Consumes: `Runs::planner_plan` (the grant check `start_for` uses); Plans' read of a version's task titles, as `Runs::title` in `notify.rs` does.

- [ ] **Step 1: Write the failing tests** in `runs_attempts/planner.rs`:
  - `task_runs_lists_the_attempts_that_hold_a_scope`: Start T1 and T2 (disjoint writes); T2 reports a commit and its turn ends; T3 is started and stopped. `live_for` returns T1 (`Running`, `commit: None`) and T2 (`Committed`, `commit: Some(sha)`), in `(version, task, number)` order. T3 is absent. `title` is the task's title. `conflict` is false.
  - `task_runs_lists_an_old_versions_attempt_after_a_new_approval`: T1 runs on v1. Draft v2, approve it. `live_for` lists T1 with `version: 1`, and nothing for v2.
  - `task_runs_marks_a_conflicted_attempt`: an accepted conflict on a Running attempt gives `conflict: true` while it still holds its scope.
  - `task_runs_is_refused_to_another_grant`: an executor grant, and a Planner grant of another project, each get `RunsError::NotFound`.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts planner::task_runs`. Expected: FAIL to compile (`live_for` is missing).
- [ ] **Step 3: Implement `RunsStore::live`.** It reads attempts of the workflow, any version, with `released_at IS NULL`, `ORDER BY version, task, number`. `conflict` is `conflict_json IS NOT NULL`, read in the same query.
- [ ] **Step 4: Implement `Runs::live_for`.** First `planner_plan(grant, workflow)`, then `RunsStore::live`, then map each attempt to a `LiveAttempt` with its task's title (`T{task}` when it has none, as `Runs::title` does). It only reads.
- [ ] **Step 5: Add the MCP tool `task_runs`** with input `{ workflow_id }`. It answers `Vec<LiveAttempt>`, and `RunsError` maps through the existing run error translation. Add it to the Planner's list in `tools_for` (35 tools; only the Planner holds it).
- [ ] **Step 6: Add one line to `prompt.txt`** next to the `task_stop` line: "`task_runs` lists the attempts of a plan that can still write or still wait for your judgment, any version; read it before you choose which to stop with `task_stop`."
- [ ] **Step 7: Extend the MCP tests.** In `mcp_server.rs`, the Planner's tool list includes `task_runs` and the executor's does not. In `task_conflict_routes.rs`, add `the_planner_lists_live_attempts_over_mcp`: over the real `/mcp`, a Planner grant's `task_runs` answers the running attempt's id, which `task_stop` then stops; a second `task_runs` answers `[]`.
- [ ] **Step 8:** Run `cargo test -p shadows-core --test runs_attempts planner::` and `cargo test -p shadows --test task_conflict_routes --test mcp_server --test planner_instructions`, then the rule 5 checks. Expected: PASS.
- [ ] **Step 9:** Update the contracts: in Runs, `functions.live_for`, `shapes.LiveAttempt` and the obligation with its four tests; the Grants count; the MCP tool line; the Harness prompt line. Set B-117 to done in `stages.md` and commit `feat(core): B-117 the Planner lists the attempts that hold a scope`.

## Task B-152: the Planner hears of an interrupted attempt and of the person's Stop

Changes what the web sees: two new notice cards (`outcome: interrupted` and `outcome: stopped`)
in the Planner's conversation. `AttemptNotice` keeps its shape and OpenAPI does not change, but
F-121 must render the two cards. **Blocked until Masada's shared request for F-121 is accepted.**

**Files:**
- Modify: `crates/core-runs/src/notify.rs` (`ended_intent`: two new bodies)
- Modify: `crates/core-runs/src/recover.rs`
- Modify: `crates/core-runs/src/store/recover.rs` (`settle_running` answers the ids)
- Modify: `crates/core-runs/src/stop.rs`
- Modify: `crates/core-runs/src/model.rs` (`RecoverReport.interrupted` stays `u32`; no shape change)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/runs_attempts/stops.rs` (new; `#[path]` module in `runs_attempts.rs`), `crates/shadows-core/tests/task_conflicts.rs`

**Interfaces:**
- Produces:
  - `RunsStore::settle_running(&self) -> Result<Vec<AttemptId>, RunsError>`: the attempts it moved to Interrupted, in `started_at, id` order. `recover` keeps answering `RecoverReport { interrupted: ids.len() }`.
  - private `enum Stopper { Person, Planner }` in `stop.rs`, and private `Runs::stop_by(&self, attempt: &AttemptId, by: Stopper) -> Result<Attempt, RunsError>`. `stop` = `stop_by(Person)`, and `stop_for` calls `stop_by(Planner)`. The agreement "the person's Stop and the Planner's task_stop run one method" still holds through `stop_by`.
- Consumes: `notify` / `notify_ended` (unchanged road), and the `dirty` read Start and report use (the paths changed inside the attempt's writes).

- [ ] **Step 1: Write the failing tests** in `runs_attempts/stops.rs`:
  - `a_restart_tells_the_planner_of_each_interrupted_attempt`: T1 is Running and has written `backend/app/a.php` uncommitted. Restart (the restart pattern of `task_conflicts::restart_replays_pending_conflict_once`). The Planner's thread gets one queued notice, command id `notify-<attempt>`; its card has `outcome: interrupted`; its body contains `was interrupted` and `backend/app/a.php`.
  - `a_second_restart_sends_no_second_interrupted_notice`: restart twice; still one notice.
  - `the_persons_stop_tells_the_planner`: `Runs::stop` on a Running attempt queues one notice with `outcome: stopped`, and its body contains `stopped by the person`.
  - `the_planners_stop_sends_no_notice`: `stop_for` on a Running attempt queues none.
  - In `task_conflicts.rs`, `a_stopped_conflict_keeps_its_one_notice`: a conflicted Running attempt stopped by the person has exactly its one conflict notice.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts stops::` and `--test task_conflicts a_stopped_conflict`. Expected: the four `stops::` tests FAIL (no notice); the conflict test may already pass and stays as a guard.
- [ ] **Step 3: `settle_running` answers the ids.** Same write, same rules; it reads the ids before the UPDATE, as it reads the thread ids now.
- [ ] **Step 4: `recover` notifies them.** In the spawned pass, after the pending conflicts and before `recover_messages`, call `notify` for each id from `settle_running`. A failure is logged `runs.notify_interrupted_failed` and never stops the pass. Only this start's ids are notified, so a second restart finds none.
- [ ] **Step 5: `stop_by`.** After the release, the Person path calls `notify` only when this Stop moved the attempt from Running to Stopped (`finish` answered `true`) and the attempt has no conflict. The Planner path keeps today's behavior: only `retry_conflict_notice`.
- [ ] **Step 6: Two bodies in `ended_intent`.** They replace the `_` arm for these two states only; Failed keeps its text. `{dirty}` is the attempt's changed paths inside its writes, joined by `, `, or `none`; a git failure gives `unknown` and logs.
  - Interrupted: `{head} was interrupted: Shadows stopped while it ran, and nothing was committed.\nUncommitted files in its writes: {dirty}\n{last}Tell the person; start it again only if they ask.`
  - Stopped: `{head} was stopped by the person.\nUncommitted files in its writes: {dirty}\n{last}Do not start it again unless they ask.`
- [ ] **Step 7:** Run the Step 2 commands, plus `cargo test -p shadows-core --test runs_attempts` (all of it: `stopping_a_committed_attempt_frees_its_scope` and `recover_interrupts_running_attempts_without_restarting_them` must stay green), `--test recovery`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 8:** Update the contract: `recover`, `stop`, `stop_for`, `notify` and the transitions ("found Running at startup: Interrupted; the Planner is told once", "the person's Stop of a Running attempt: Stopped; the Planner is told"), with the five tests. Set B-152 to done and commit `feat(core): B-152 the Planner hears of interrupted and stopped attempts`.

## Task B-118: the full journey, live and in the browser

**Files:**
- Create: `crates/shadows/tests/stop_control_live.rs`
- Create: `docs/evidence/<date>-slice-6-stop-control.md`
- Modify: `crates/core-runs/contract.yaml` (`tests:` line for the live file)
- Modify: `docs/backend/stages.md`

**Interfaces:**
- Consumes: B-117 (`task_runs`), B-152 (the notices), F-121; the fixtures `checks_live.rs` uses (`fixtures/app.rs`, `listening.rs`, `live.rs`, `plan.rs`).

- [ ] **Step 1: Write `stop_control_live.rs`**, ignored, on the signed-in subscription, with the same setup as `checks_live.rs`. One test, `live_the_planner_stops_affected_work_and_carries_on_after_a_restart`, in a disposable git folder:
  1. The Planner plans three tasks. T2 uses T1's output and T3 is independent. It starts the first group (T1 and T3 run together).
  2. T1's executor files a `task_conflict`. Assert: T1 ends Stopped with reason `contract conflict`, and T3 is still Running.
  3. Prompt the Planner to assess the impact. Assert: it called `task_runs` and did not stop T3 (T2 never started, so nothing else is affected).
  4. While T3 runs, shut the daemon down and start it again on the same database. Assert: T3 is Interrupted, the Planner's conversation has exactly one `interrupted` notice for T3, and no task restarted on its own.
  5. Ask the Planner to start T3 again. It commits, its checks pass, and the Planner marks it done.
- [ ] **Step 2:** Run `cargo test -p shadows --test stop_control_live -- --ignored --nocapture --test-threads=1`. Expected: PASS. Record the model and run time.
- [ ] **Step 3: Browser journey** on a disposable project with `shadows serve`. Start two tasks. Stop one from the task page and see the Planner's `stopped` card. Restart the daemon during the other and see the `interrupted` card. Read both cards and the daemon log lines (`runs.*`).
- [ ] **Step 4: Write the evidence file**: date, commit, OS, model, the commands, what passed, and what could not be checked.
- [ ] **Step 5:** Add the `tests:` line, set B-118 to done and commit `test(core): B-118 slice 6 live journey and evidence`.

## Handoff

Order: B-117 → B-152 (after the request is accepted) → F-121 → B-118. B-117 needs no web
change and can start as soon as this plan is approved. The full gate and one whole-branch
review run before the PR.
