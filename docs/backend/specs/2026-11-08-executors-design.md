# Executors: running an approved plan's tasks

- Date: 2026-11-08
- Status: **approved** by Mohammed on 2026-11-08, in conversation, after a brainstorm with the
  controller; `core-runs` (§9) and the controller's calls (§12) approved with it. Amended
  2026-11-09 by Mohammed: the executor makes its own commit and reports it (§7).
- Builds on, and does not replace:
  - Core vision §2 (bounded executors who ask Shadows), §3 (a failing test that belongs to
    another task), §5 (the escalation ladder), §6 (executors and critics) — `docs/core/vision.md`;
  - §17 task contracts and evidence (`docs/core/specs/2026-10-01-task-contracts-and-evidence-design.md`);
  - §19 profiles and the critic (`docs/core/specs/2026-10-03-agent-profiles-and-extensions-design.md`);
  - the agent messaging spec, slice 2 (`docs/backend/specs/2026-11-03-agent-messaging-design.md`);
  - the planner context spec, slice 4 `Plans::owner_thread`
    (`docs/backend/specs/2026-11-04-planner-context-design.md`).
- Roadmap: Stage 4, "Executors and verification" (`docs/core/roadmap.md`).

## 1. Why

Planning is built: an idea becomes a vision, a map, a roadmap and approved plans whose tasks
say what they do, where they write and how they are accepted. Nothing runs those tasks yet.
Mohammed opens an approved plan, looks at T1 "Scaffold the Laravel application in backend/",
and has no button that does it.

This spec gives that button. Mohammed presses Start on a task; an executor agent receives that
task and only that task, does the work inside the task's write scope, commits it and reports. A
critic checks the commit against the task's acceptance items. A failure goes to the Planner,
which retries, investigates or stops, and Mohammed hears only when the work cannot go on.

## 2. Decisions

Mohammed's, 2026-11-08, in conversation unless marked as the controller's call.

| # | Decision |
|---|---|
| D1 | A task whose `needs` links are not complete waits; its Start is disabled with the waiting task named. |
| D2 | Independent tasks run **at the same time**, from the first release. This replaces the roadmap's "serial first, concurrency at Stage 8" and vision §6's "initial execution remains serial". |
| D3 | Concurrent tasks run **in the same project folder**, not in separate worktrees. Their declared `writes` keep them apart. |
| D4 | A test that fails because another task's code is missing is not the executor's fault. It is pending on that task (vision §3). |
| D5 | The task opens on **its own page** with tabs: Details, Run, Diff, Report (mockup "A", chosen 2026-11-08). |
| D6 | Mohammed chooses the model for the executor and for the critic, per task. The Planner writes a **suggested model** on each task; the page starts on it. This replaces vision §4's "Shadows chooses the model per task". |
| D7 | No manager in the first release. A failure reaches **the Planner**: the conversation that wrote the plan. |
| D8 | The Planner retries on its own. The ladder is §5 below: two attempts, then the Planner investigates, then one directed attempt, then Mohammed. |
| D9 | A retry or a directed fix is sent **into the same executor session**, so the executor keeps what it did and what it was told. |
| D10 | **Each executor commits its own work.** The critic reviews that commit. |
| D11 | One built-in executor profile and one built-in critic profile, with no Dashboard to edit them yet. |
| D12 | The profiles live **inside Shadows**, as the Planner does: Shadows sends their instructions and gives them their tools over MCP with a grant scoped to the task. Nothing is written into the person's project folder except the task's own code. |

## 3. Goals, non-goals, anti-goals

**Goals**

1. Start any ready task of a frozen (Approved) plan version from its task page.
2. Run independent ready tasks concurrently in the project folder, never two whose `writes` intersect.
3. The executor receives the task, not the plan: goal, reads, writes, acceptance, the links it
   waits on, and the project's structure. It asks Shadows "where is X?" instead of crawling.
4. Each attempt ends in a commit containing only paths inside the task's `writes`.
5. A critic reviews that commit and marks each acceptance item met, not met, or pending on another task.
6. Failures climb the ladder in §5 without Mohammed until it reaches him.
7. Everything survives a daemon restart: no lost attempt, no duplicate run, no duplicate commit.

**Non-goals (later work)**

- The manager (messaging slice 3) and its authority.
- Profiles edited in the Dashboard, specialists (library reviewer, researcher), per-role extensions (§19).
- §17's richer contract fields (`target_symbols`, `interface_contract`, `invariants`, ...).
  The first release runs on today's `TaskContent`: goal, reads, writes, acceptance, phases, links.
- Codex as an executor harness. It joins when the Codex harness lands (B-85 to B-88).
- Worktrees, merges, pushes, branches per task.

**Anti-goals: a failure even if it works**

- A commit that carries a path outside its task's `writes`, or another task's files.
- An executor that builds another task's part or writes mocks to make its tests pass.
- A critic PASS on changes it did not see, or a pass inferred from the executor saying "done".
- A retry loop with no end, or the Planner approving its own plan change.
- Anything written into the person's project that is not the task's code (`.claude/`, prompts, profiles).

## 4. What the person sees

### The task page (D5)

A new route for one task of one plan version. The current plan page's task panel links to it.

- **Header, always visible:** plan › task number and title; the task's status; the tasks it
  waits on, each with its state (`needs T1 ✓`); Start, or Stop while it runs.
- **Details:** goal; writes and reads; executor and critic, each with a model selector that
  starts on the task's suggested model (D6); acceptance items with their marks.
- **Run:** the live executor and critic sessions, streamed with the conversation view's existing
  entry rendering.
- **Diff:** the task's commits, newest first, with each file's diff.
- **Report:** each attempt's executor report and critic verdict; the Planner's decision on a
  failure; the reason when the task stopped for Mohammed.

### The plan page

The header's progress ("N tasks · M done") and the per-task marks become real: they read
Core's task status. This answers the two shared requests waiting since 2026-11-02:
`plan-task-status.md` and `acceptance-ticks.md` (§9).

## 5. One task's life

```text
Start (person)  ─►  attempt 1: executor works, commits
                       └─► critic reviews the commit
                             ├─ PASS ─► task done; dependants become ready
                             └─ FAIL ─► Planner notified
                                         └─► attempt 2 in the same session, with the critic's findings
                                               └─► critic ─ PASS ─► done
                                                         └─ FAIL ─► Planner investigates the code
                                                                     ├─ found ─► attempt 3, same session,
                                                                     │           with a precise instruction
                                                                     │             └─► critic ─ PASS ─► done
                                                                     │                       └─ FAIL ─► stop, Mohammed
                                                                     └─ not found ─► stop, Mohammed
```

At every FAIL the Planner may decide instead that **the plan is wrong**. It then edits only the
affected tasks in a Draft of the next version, with its reason; every task of the plan stops
(§12, "When the plan is wrong"); approving that Draft stays Mohammed's (§24 person authority).
The Planner never approves a plan.

**Attempts.** Each attempt is its own record (§17.4: a retry is a new attempt, never a reset).
Attempts 2 and 3 continue the same executor session (D9). The cap is three attempts per task
start; a fourth needs Mohammed's Start. This is the controller's reading of D8; Mohammed
confirms it with this spec.

**Stop.** Mohammed may Stop at any time. Stop cancels the running turn and confirms it ended,
as the Planner's Stop does today. An attempt stopped before its commit has no commit; its
working-tree changes stay in the folder and are named in the Report.

## 6. Running together in one folder (D2, D3)

- A task is **ready** when its plan version is Frozen, its `needs` links are complete, and no
  running task's `writes` intersects its own. Intersection is decided on the glob patterns,
  conservatively: when Shadows cannot prove two patterns disjoint, they intersect.
- Ready tasks may run together. Mohammed starts each one; nothing starts on its own in the
  first release, except the retries in §5.
- **Each executor commits its own files** with `git commit`, on the folder's current branch, and
  reports that commit to Shadows, which checks it and records it (§7, amended 2026-11-09).
  Disjoint `writes` keep two executors' commits apart; Shadows writes no commit and keeps no
  commit queue.
- A test failing because of another running or unstarted task's code is reported as pending
  on that task (D4). The critic does not count it as a failure of this task.

## 7. The write boundary

**Amended 2026-11-08 by Mohammed, after the probe**
([evidence](../../evidence/2026-11-08-executor-write-boundary-probe.md)). The executor is
built as the Planner is: it has only the tools and permissions its task gives it.

- **Its permissions are its task.** The session gets a fixed tool list and Claude Code
  permission rules built from the task, passed in the session's options: `Read`, `Grep`, `Glob`
  and `TodoWrite` anywhere; `Edit`, `Write` and `MultiEdit` on each `writes` pattern; `Bash`
  for everything (tests, git, generators, containers). What these rules allow runs with no
  question. Any other request reaches Shadows and is refused, as a Planner session's are
  (`PermissionPolicy::Refuse`), and the refusal is a thread entry. The session runs in mode
  `default`, set explicitly: in `auto` and `acceptEdits` no request reaches Shadows at all.
- Shell commands are not policed. A command may still write outside `writes`; the check on the
  reported commit handles that.
- Containers and other processes a task starts keep running after its turn and after Stop;
  stopping them is the project's business, not the attempt's.

**The commit: amended 2026-11-09 by Mohammed, after the first browser run.** Evidence: the
executor ran `python` (a Windows store stub), which installed a Python runtime into
`backend/Python/`; `task_commit` then committed 2354 files, everything changed inside `writes`.
The agent removed the runtime and ran `git commit --amend` from its shell, which dropped the
`Shadows-Attempt` trailer and left Shadows pointing at an orphaned commit. The rule now: the
agent knows what it did, so **the agent makes its own commit and then reports to Shadows
exactly what it did; Shadows checks and records.** This replaces `task_commit`, its temporary
index, its commit queue and its trailer.

- **The executor commits.** It removes its strays first, stages only its own files inside
  `writes`, by name, never a binary or an installer, and runs `git commit` with its own message
  on the branch that is checked out. Every other change in the folder, the person's included,
  stays where it is.
- **It reports.** It calls `task_report { commit, report }`: the commit's SHA and an honest
  account of what it did, what it ran or tested, and what passed or failed. The executor's
  instructions and its first prompt say this plainly.
- **Runs checks the reported commit** and refuses, with a reason the agent can act on, when the
  name is not a SHA of a commit in the repository; the commit is not reachable from HEAD; it
  was already reachable from the HEAD recorded when the attempt started; any path it changes
  against its first parent lies outside `writes` (all listed); or it adds or modifies a file
  git counts as binary (all listed). A refusal leaves the attempt `Running`, so the agent fixes
  its commit and reports again.
- **On acceptance** the attempt becomes `Committed` with the SHA and the report text, which
  the Report tab shows. One accepted report per attempt; another is `AlreadyCommitted`.
- **Accepted:** a hand edit inside a running task's `writes` enters the commit only if the
  executor stages it. Two tasks never share `writes` (§6), so no task's commit passes the check
  with another's files.

## 8. What the executor and critic receive (D11, D12)

**Executor session**

- Instructions: Shadows' built-in executor instructions (stay in `writes`; build nothing you do
  not own; no mocks for another task's code; a test pending on another task is expected; stop
  and report if the plan is wrong; commit your own files with git, then report the commit).
- The task: goal, reads, writes, acceptance, phases, and for each link the other task's number,
  title, state and label.
- The project structure: the approved map's parts and their folders.
- Tools over Shadows MCP, with a grant scoped to this task: `where_is`, `who_uses`, `outline`
  (the code index); `task_report` (the commit the executor made and its account of the work,
  §7). Reporting a problem with evidence and the tasks it affects joins it with the ladder.
- Model: the one chosen on the task page.

**Critic session**

- Instructions: Shadows' built-in critic instructions (read only; review this commit against
  this task; mark each acceptance item with its evidence).
- The task as above, the attempt's commit and its diff, and the executor's report. Not the
  executor's conversation (§19.3).
- Tools: the code index and running the task's checks; `task_review` (the §17.4 outcome and one
  mark per acceptance item: met, not met, or pending on task N).
- Read only: its file-edit tools are refused.

**The Planner on a failure.** The plan's own conversation (`Plans::owner_thread`, planner context
slice 4, brought forward from the manager slice) receives a message naming the task, attempt,
critic findings and evidence. It answers through tools: `task_retry` (with its instruction, into
the executor's session), `task_stop` (with its reason for Mohammed), or a plan Draft. It may read
the code and the diff to investigate.

## 9. Ownership

A new service, **Runs** (`core-runs`), owns attempts: their records, the ready rule, the
concurrency check, the check of a reported commit, the attempt ladder and the critic dispatch. It is listed
in `AppCore` with its contract. This is the controller's proposal and needs Mohammed's approval
with this spec; CLAUDE.md requires a new owner to be a controller decision.

| Owner | Gains |
|---|---|
| `core-runs` (new) | attempts, the ready rule, concurrency, reported commits, ladder, critic dispatch, task status |
| `core-plans` | the suggested model on `TaskContent`; acceptance marks per attempt are read from Runs, the plan stays immutable |
| `core-harness` | executor and critic sessions: their instructions, their scoped grant, their mode |
| `core-grants` | a task-scoped grant |
| `core-threads` / `core-turns` | the Planner's failure message in the plan's owner thread |
| `shadows-mcp` | the task tools of §8 |
| `shadows-http` | start, stop, read attempts, the task page's data |
| web | the task page; the plan page's real progress |

Task status and acceptance marks belong to an attempt, not to the frozen plan, so the two
waiting shared requests are answered by Runs: a task's status is its latest attempt's state;
an item's mark is its latest review's mark. Marks do not carry over to a new plan version.

## 10. Order of building

Each slice runs end to end in the browser before the next.

0. **Probe:** done 2026-11-08; §7 holds its outcome.
1. **One task, by hand:** Runs, the task page (Details, Run), Start/Stop, the executor session
   with its scoped grant and its task's permission rules, `task_report` with its check of the
   executor's own commit (§7, amended 2026-11-09), the Diff tab. No critic: the attempt ends at
   its commit and Mohammed reads it. The `writes` overlap check ships here, so two tasks started
   together are already safe.
2. **The critic:** dispatch after the commit, `task_review`, acceptance marks, the Report tab,
   real plan progress (the two shared requests). Its plan decides whether a task carries
   `checks` (commands) that Runs runs at Start, before the first write, and again after the
   commit, judging each by the earlier project's baseline rule (`shadow-verify/src/baseline.rs`
   in `E:\Globalprojects\shadow`: pass, regression, pre-existing, fixed). A check failing before
   and after is a candidate for "pending on another task" (D4) on evidence, not on the critic's
   word. A baseline at the base commit is not used: it needs a second checkout, which D3 rules out.
3. **The ladder:** the owner thread, the Planner's failure message and its three tools,
   attempts 2 and 3 in the same session, the stop for Mohammed.
4. **Together:** pending-on-another-task marks, and the acceptance run of several tasks at once
   (the overlap check already shipped in slice 1).

## 11. Acceptance

- Mohammed starts T1 of the Laravel plan in the browser; it commits only under `backend/`; the
  Diff tab shows it; the critic marks its three acceptance items.
- A change outside `writes` is left uncommitted and named in the Report.
- A forced critic failure reaches the Planner, which retries in the same session; a second
  failure leads to an investigation and a directed third attempt; a third failure stops and
  shows Mohammed the reason.
- Two tasks with disjoint `writes` run together and produce two separate commits; two with
  intersecting `writes` cannot both start.
- A test waiting on an unstarted task is marked pending on it, not failed.
- A daemon restart mid-attempt neither loses the attempt nor starts it twice.

## 12. Controller's calls

Settled by the controller after the builder check of 2026-11-08. Mohammed reviews them with
this spec; each is marked as the controller's, not his.

1. **Sessions are Threads.** An executor or critic session is a Thread of a new kind (`executor`,
   `critic`), owned by Threads and opened by Harness like the Planner's. The Run tab reuses the
   conversation's entries and events. Runs holds the attempt's thread ids.
2. **Git belongs to Runs**, run through `shadows-process`; no new owner. Start refuses when the
   project folder is not a git repository, or when it already has uncommitted changes inside the
   task's `writes`. Changes elsewhere in the folder are other tasks' business and do not block.
   Commits go to whatever branch the folder has checked out; Shadows never switches it.
3. **Restart.** A running attempt found on startup becomes `Interrupted`: with no accepted
   report Shadows recorded no commit, and git is not searched (amended 2026-11-09, §7). Its
   process is gone; its uncommitted changes stay in the folder. An attempt whose session fails
   to open, or whose turn ends without an accepted report, is `Failed` with that reason. Each attempt is bound to its turn's operation. It does not count toward the cap
   and does not climb the ladder. The Report tab shows it; Start begins a new attempt in a new
   session. Runs reconciles this on startup through Operations.
4. **The ladder lives in Runs, not in prompts.** After a critic FAIL, Runs posts the failure into
   the plan's owner thread and starts a Planner turn with no person present, on the thread's
   remembered model. If that thread is busy, the message waits for the turn to end. Runs refuses
   a fourth attempt without Mohammed's Start, and refuses a third whose `task_retry` carries no
   instruction text.
5. **"Complete" for a `needs` link** is a critic PASS. In slice 1, before the critic exists,
   Mohammed marks a task done by hand on its page. A PASS with items pending on another task is
   complete for its dependants. When the blocking task passes, Runs dispatches the critic again
   for the pending items only; their marks then close.
6. **The suggested model** is an optional `suggested_model` on `TaskContent`, written by the
   Planner in a Draft. Existing Frozen versions keep their content and read it as absent; the
   page then starts on the Planner thread's model. Mohammed's choice is recorded on each attempt
   and remembered per task for the next Start; it never changes the plan version.

**When the plan is wrong (Mohammed, 2026-11-08).** The Planner edits only the task or tasks the
problem touches; it does not rewrite the plan. The edit is still the next version (v2 = v1 with
those tasks changed), so history and approval work as today, and Mohammed sees only the changed
tasks marked. Because one task's change may touch others, **every task of that plan stops** the
moment the Planner starts the Draft: running attempts are stopped and confirmed ended, and no task
of the plan may start. Committed work stays. After Mohammed approves the new version, the plan's
tasks are ready again under it and he starts them; nothing restarts on its own.

## 13. Documents this changed on approval

- `docs/core/vision.md` §4 (model choice: the person, with the Planner's suggestion) and §6
  (concurrency from the first release, in one folder).
- `docs/core/roadmap.md` Stage 4 (concurrent tasks, no manager) and Stage 8 (what remains: worktrees).
- §17.1 and §17.4 (overlapping-folder and serial-first exclusions), pointing here.
- The messaging spec §8 (slice 2's reports go to the Planner, not the person).
- The planner context spec §6 slice 4 (built with this spec's slice 3).
- `docs/core/future.md` (Executors row, Critic row).
