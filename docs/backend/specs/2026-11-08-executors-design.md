# Executors: running an approved plan's tasks

- Date: 2026-11-08
- Status: **approved** by Mohammed on 2026-11-08, in conversation, after a brainstorm with the
  controller; `core-runs` (§9) and the controller's calls (§12) approved with it.
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

- An executor that writes outside its `writes`, or a commit that carries another task's files.
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
- **Commits are serialized** by Shadows: one commit at a time per project folder. An executor
  commits through a Shadows tool, not `git commit`. The tool stages only the attempt's changed
  paths inside `writes` and commits them on the folder's current branch, with the task and
  attempt in the message. Changed paths outside `writes` refuse the commit (§7).
- A test failing because of another running or unstarted task's code is reported as pending
  on that task (D4). The critic does not count it as a failure of this task.

## 7. The write boundary

§17.6: the boundary is enforced, not requested. Two layers:

1. **Before writing.** File-edit tool calls outside `writes` are refused through the harness
   permission path (the Planner's sessions already refuse tools this way).
2. **At commit.** The commit tool compares the attempt's changed paths with `writes` and refuses
   on any path outside. The attempt fails with those paths named.

**Ownership (amended 2026-11-08 after review).** Being inside `writes` does not prove a change
is the attempt's: in a shared folder the person, a child process or another task may have
written it, and `git commit --only` takes a file's whole current content. A changed file is the
attempt's only when its current content equals what was recorded after the attempt's own last
permitted write to it, derived from that tool call, never from re-reading the file. A changed
path in `writes` that cannot be proven refuses the commit; nothing is committed on a guess. The
commit records the bytes that were checked, built without git reading the working tree, so a
change made between the check and the commit stays uncommitted. An attempt holds its `writes`
against other Starts until its session can no longer write, not merely until it commits. Paths are checked canonical, with links and junctions resolved.

> **OPEN — shell writes and ownership.** Claude Code's shell tool can write any file, and
> layer 1 cannot see what a shell command or its child processes write. **Trigger:** a probe on
> Claude Code through Shadows (B-89), before the first build task, measuring what the
> permission path sees for editor tools, shell commands, child-process writes, a hand edit
> during a run, and a path through a symlink or junction; and choosing the shell treatment and
> the ownership record from it. It is recorded in `docs/evidence/` and closes this block.

## 8. What the executor and critic receive (D11, D12)

**Executor session**

- Instructions: Shadows' built-in executor instructions (stay in `writes`; build nothing you do
  not own; no mocks for another task's code; a test pending on another task is expected; stop
  and report if the plan is wrong; commit through the tool).
- The task: goal, reads, writes, acceptance, phases, and for each link the other task's number,
  title, state and label.
- The project structure: the approved map's parts and their folders.
- Tools over Shadows MCP, with a grant scoped to this task: `where_is`, `who_uses`, `outline`
  (the code index); `task_commit`; `task_report` (done, or a problem with evidence and the
  tasks it affects).
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
concurrency check, the commit queue, the attempt ladder and the critic dispatch. It is listed
in `AppCore` with its contract. This is the controller's proposal and needs Mohammed's approval
with this spec; CLAUDE.md requires a new owner to be a controller decision.

| Owner | Gains |
|---|---|
| `core-runs` (new) | attempts, the ready rule, concurrency, commit queue, ladder, critic dispatch, task status |
| `core-plans` | the suggested model on `TaskContent`; acceptance marks per attempt are read from Runs, the plan stays immutable |
| `core-harness` | executor and critic sessions: their instructions, their scoped grant, the write refusal |
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

0. **Probe:** the shell-write and ownership OPEN (§7) on Claude Code through Shadows.
1. **One task, by hand:** Runs, the task page (Details, Run), Start/Stop, the executor session
   with its scoped grant, the write refusal, `task_commit` with its ownership check, the Diff
   tab. No critic: the attempt ends at its commit and Mohammed reads it. The `writes` overlap
   check and the commit queue ship here, so two tasks started together are already safe.
2. **The critic:** dispatch after the commit, `task_review`, acceptance marks, the Report tab,
   real plan progress (the two shared requests).
3. **The ladder:** the owner thread, the Planner's failure message and its three tools,
   attempts 2 and 3 in the same session, the stop for Mohammed.
4. **Together:** pending-on-another-task marks, and the acceptance run of several tasks at once
   (the overlap check and commit queue already shipped in slice 1).

## 11. Acceptance

- Mohammed starts T1 of the Laravel plan in the browser; it commits only under `backend/`; the
  Diff tab shows it; the critic marks its three acceptance items.
- An edit outside `writes` is refused; a shell write outside `writes` refuses the commit.
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
3. **Restart.** A running attempt found on startup is matched against git first: a commit whose
   `Shadows-Attempt` trailer names it makes it `Committed` (git is written before SQLite).
   Otherwise it becomes `Interrupted`: its process is gone, its uncommitted changes stay in the
   folder and are listed. An attempt whose session fails to open, or whose turn ends without a
   commit, is `Failed` with that reason. Each attempt is bound to its turn's operation. It does not count toward the cap
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
