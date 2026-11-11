# Executors: running an approved plan's tasks

- Date: 2026-11-08
- Status: **approved** by Mohammed on 2026-11-08, in conversation, after a brainstorm with the
  controller; `core-runs` (§9) and the controller's calls (§12) approved with it. Amended
  2026-11-09 by Mohammed: the executor makes its own commit and reports it (§7). Amended
  2026-11-11 by Mohammed: the executor runs in `auto` from one file (§7); the Planner starts,
  judges and continues tasks in place of the critic and the ladder (§5.1), with three
  additions per task, a lessons record and one review path; setup comes first, as a plan the
  Planner runs (§5.2).
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

### 5.1 The Planner judges (Mohammed, 2026-11-11)

Evidence: the first run of slice 1 on a real Go project. T1 committed and said its Compose file
starts PostgreSQL and Redis, but only ran `docker-compose config -q`. T2 committed with a report
saying its pool test skipped, so two of its three acceptance items were never checked. Nobody
read either report and acted. The folder was not a git repository, so Start was refused until
Mohammed ran `git init` by hand.

This replaces the critic (§8 "Critic session", §10 slice 2's critic) and the ladder (D8, the
diagram above, §8 "The Planner on a failure", §12.4, §10 slice 3). D7 and D9 stand.

- **The Planner starts a task** with `task_start`, which calls the method the Start button
  calls; only the Planner of the plan's project may. It starts a task when the person asks it
  to run one.
- **After a done mark the Planner moves on by itself** (Mohammed, 2026-11-11). Marking a task
  done answers `next`: the lowest-numbered task of that version that has never run and that
  Start allows now (its needs links done, no overlap, a clean folder). The Planner starts it at
  once without waiting for the person. A task that already ran, done or failed, is never
  offered; a failure goes to the person.
- **The Planner hears how each attempt ended.** When an attempt's turn ends with an accepted
  commit, or fails without one, Runs puts a message in the plan's own conversation
  (`written_by`'s thread) naming the task, the commit, the files it changed and the executor's
  report, or the failure. It is a waiting message: a busy Planner reads it when its turn ends,
  and a restart does not lose it. It arrives in the executor's name (Mohammed, 2026-11-11): its
  entry stays the turn's `UserMessage`, written by `Executor` and the attempt's executor thread
  with a card of its facts (task, attempt, model, outcome, commit, the full report), so the page
  draws it inside the Planner's activity, never as the person's message; the model reads the
  same text as before. The turn it starts runs under the executor's principal, never the
  person's, and Grants refuses any approval (vision, map, roadmap, plan) in a turn that holds no
  message from the person. A waiting notice cannot be removed from the queue
  (`QUEUED_NOTICE_KEPT`); the person's own waiting messages can.
- **A committed attempt waits for the judgment.** Its session stays open and its `writes` stay
  held until the task is marked done or the attempt is stopped. This replaces "a committed
  attempt holds its scope until its turn has ended".
- **The Planner judges** the commit against the task's acceptance items by reading what it
  changed, with its own tools (Mohammed, 2026-11-11): `task_diff`, the latest committed
  attempt's files with their patches, and `task_file`, any file inside the task's `writes` as
  that commit holds it. They call Runs' `diff` and `file`, the methods the task page's Diff tab
  reads. The Planner never asks the executor for file contents, and the executor's report
  carries what the commit cannot show: the commands it ran and their results. Evidence: the
  slice 1b trial on `testing3` spent two of T2's three additions asking for the committed
  files.
- **The Planner reads the code before it plans** (Mohammed, 2026-11-11), on every harness:
  before a task names a file, a function, a class or a layer, it reads the manifest, the
  composition root, the folders of the layers the task works in and every file the task
  modifies. It stays an instruction: Shadows sees reads only as tool calls (a Codex read is a
  shell command line), so a check would be a guess that refuses honest edits.
- **An executor's commit changes something** (Mohammed, 2026-11-11). `report` refuses a
  commit that changes no file (`empty_commit`); the attempt stays Running. An executor with
  nothing to change commits nothing and ends; the attempt fails "ended without a commit" and
  the Planner's notice carries the executor's last message. An addition asks for a change,
  never a question. `project_git_init`'s empty first commit is not a report. It marks the task done (`task_done`), which closes the session and frees the scope;
  or it sends an **addition** (`task_amend`): what is missing or must be extended. The Planner
  may also take a done mark back.
- **An addition continues the executor in its session.** It is a new attempt on the same
  executor thread, carrying the addition's text; the attempt before it keeps its commit and
  report. The addition is recorded on the task through its attempts. There is no new plan
  version and no approval: the frozen version does not change. Runs owns it, as it owns the
  done marks of a frozen version.
- **The Planner can make the folder a git repository** (`project_git_init`): `git init` and a
  first commit made in Shadows' name, refused when the folder is already inside one. The first
  commit takes the files the folder already holds, leaving out what git ignores (an empty
  folder gets an empty commit; Mohammed, 2026-11-11) and what Code's inventory classes as
  secret or dependency (Mohammed, 2026-11-12): those stay in the folder untracked, listed in
  `.git/info/exclude`. Runs reads the inventory through Code. It writes no file; starter files stay a
  task's work. Git belongs to Runs (§12.2).

- **Three additions per task start** (Mohammed, 2026-11-11). A fourth is refused: Runs stops
  the waiting attempt (its commit stays; its session closes and its scope is freed) and the
  task goes back to the person. A new Start begins a new count.
- **A task stopped on the cap leaves a lessons record** (Mohammed, 2026-11-11), a JSON record
  Runs keeps to improve Shadows: the plan version and task, each attempt of the chain with its
  addition (what was missing), commit and report, the shell commands the executor ran (their
  titles, status and error, from the executor thread's tool-call entries; their output is not
  stored), and two texts the Planner writes with `task_lessons`: what stopped the work, and
  what the plan should have said so the executor need not have come back.

**One review path** (Mohammed, 2026-11-11). An attempt's end reaches a judgment by one road:
Runs' watcher sees the turn end → `Runs::notify` hands the attempt on → the Planner judges →
`task_done` (`mark_done`, which the person's done mark also calls) or `task_amend` (`amend`).
Nothing else marks a task done or continues it. **A reviewer plugs in at `notify`:** a critic
(or any third agent) would be started there on the attempt's commit, and its findings would
join the same notice, so the Planner still decides and nothing after `notify` changes. No
hook, trait or field exists for it until that reviewer is built.

> **OPEN: does the critic come back?** The Planner's judgment replaces it. Closed when a real
> run shows the Planner marking done work that does not hold, which is the critic's case; it
> then plugs in at `notify` as above.

### 5.2 Setup before detailed planning (Mohammed, 2026-11-11)

Setup belongs to the Planner and comes first: once the roadmap is approved, the Planner makes
the folder a git repository if it is not one, then writes and runs a **setup plan** that
brings the environment (database and service containers) and the initial structure (the
framework's scaffold, in each runnable part's folder as the map places it). Only when it is
done does the Planner refresh the file listing and plan the detailed work on the files that
appeared. Executors then build features.

The setup is an ordinary plan run through §5.1: its tasks are executor attempts the Planner
starts and judges, and **each lands as a reported commit**, so no file appears in the folder
without a commit as its source. The Planner itself writes no file. This replaces the separate
preparation worker the local delivery draft planned (its B-100 to B-102).

**Binary files in setup only** (Mohammed, 2026-11-11). A scaffold may bring binary files (an
icon). A task of a setup plan may commit them; every other task's commit with a binary file is
refused, as §7 says. A task is a setup task when its plan version's title is `Setup` (trimmed,
any case), the title the Planner is told to give it: no new field marks it.

> **OPEN: is "setup first" enforced?** Only the Planner's instructions order it; nothing
> refuses a detailed plan while the setup plan is not done. Closed when a real run shows the
> Planner planning in detail before its setup is done.

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

**The mode and the file: amended 2026-11-11 by Mohammed, after the first run of a task with a
contract.** Evidence: in two attempts of two, the executor wrote through the shell, which
dropped the backslashes of PHP namespaces, and the `Edit` or `Write` it then asked for on a
path inside `writes` was refused; neither attempt committed. The attempts that committed
before had never called `Edit` or `Write`. This replaces the first bullet above:

- **The executor runs in `auto`.** Nothing it does is asked about and nothing is refused
  while it works. The shell could already write anywhere, so the permission rules guarded
  nothing the commit check does not; they are gone.
- **The reported commit is the one write boundary.** `task_report` refuses a commit that
  changes a path outside `writes`, as before. It also refuses one while `writes` still hold
  uncommitted changes (`dirty_scope`, the same read Start makes; planned as B-136,
  2026-11-12): what an attempt leaves inside its task's paths is either committed or removed,
  so the next Start is not refused for it. A person's edit inside a running task's `writes`
  blocks its report the same way until it is committed or removed (accepted, Mohammed,
  2026-11-12).
- **The executor is one file.** Its instructions and its tool list live in
  `crates/core-harness/src/executor.md`: the front matter names the tools, the body is the
  instructions. It is compiled in; a project does not change it. The instructions tell the
  executor to change files with its file tools, never through the shell, and never to use
  Python.
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
