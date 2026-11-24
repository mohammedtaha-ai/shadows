# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-137 and F-138 are done and F-139 to F-144 open on `feature/after-75`. The project's state view, its ER diagram and screen flow,
and `code_where` in the web (F-130 to F-136) are on `main` in PR #75 (`4d69977d`).

### F-137 · The person rechecks the project from the plan page
- Status: done (`41926e85`; `notice-item.tsx` passes the card's body, added to Touch after the build)
- Plan: none. Mockup agreed with Mohammed on 2026-11-24: in the plan page header, the button
  row reads "Show phases · Versions · Continue this plan · Recheck project · Archive"; a line
  between the progress bar and the button row reads "Last project recheck HH:MM" with passed,
  failed and not-run counts (failed in danger color, shown only when above 0), or nothing when
  there was none; while a batch has unfinished rows the button's place reads "Rechecking…"
  with no button, and the page reads again every 5 s until it finishes. In the Planner's
  conversation the recheck card shows an all-passed notice (empty failed list) as a success
  card titled "Project recheck passed" with its body text.
- Read: `docs/backend/stages.md` B-200 and B-206; `POST /api/workflows/{workflow}/recheck` and
  `GET /api/workflows/{workflow}/recheck` in `api/openapi.json`; `web/src/app/workflows/plan-page.tsx`;
  `web/src/app/conversation/recheck-notice.tsx`; `web/src/api/runs.ts`.
- Touch: `web/src/api/schema.d.ts` (generated), `web/src/api/runs.ts`, `web/src/api/contract.yaml`,
  `web/src/app/workflows/plan-page.tsx`, a new `web/src/app/workflows/recheck-control.tsx` and its
  test, `web/src/app/workflows/contract.yaml`, `web/src/app/conversation/recheck-notice.tsx`,
  `recheck-notice.test.tsx`, `web/src/app/conversation/notice-item.tsx`,
  `web/src/app/conversation/contract.yaml`, the code map row.
- Needs: B-206.
- Done when: `npx vitest run src/app/workflows/recheck-control.test.tsx
  src/app/conversation/recheck-notice.test.tsx` passes (idle with last result, no result,
  running state, a click posts once with a command id, the all-passed card), then the task
  checks in "Now".

### F-138 · The recheck's terminal panel on the plan page
- Status: done
- Plan: none. Mockup agreed with Mohammed on 2026-11-24: a panel at the bottom of the plan
  page, over the plan graph (which gets shorter). Its header reads "Project recheck", who began
  it and when, the passed, failed and left counts, and a chevron that closes it. Its left column
  lists every check of the batch (task and command) with a state icon (passed, failed, running,
  waiting, not run); its right side shows the selected check's output in the mono font, line by
  line as it arrives, kept scrolled to the end while it runs. The running check is selected
  until the person picks another. Pressing "Recheck project" opens the panel. A terminal icon
  button beside "Last project recheck" opens and closes it at any time, showing the newest
  batch (its stored output once finished). While a recheck runs the button's place reads
  "Rechecking… N of M".
- Read: `docs/backend/stages.md` B-207; the two `/api/rechecks/{recheck}` routes in
  `api/openapi.json`; `web/src/app/workflows/recheck-control.tsx`; `plan-page.tsx`.
- Touch: `web/src/api/schema.d.ts` (generated), `web/src/api/runs.ts`, `web/src/api/contract.yaml`,
  `web/src/app/workflows/plan-page.tsx`, `recheck-control.tsx` and its test, a new
  `web/src/app/workflows/recheck-panel.tsx` and its test, `web/src/app/workflows/contract.yaml`, a new `web/src/stream/recheck-output.ts` (the
  `EventSource` reader and its hook; only `stream/` opens one) and its test, its export in
  `web/src/stream/index.ts`, `web/src/stream/contract.yaml`, the code map rows.
- Needs: B-207.
- Done when: `npx vitest run src/app/workflows/recheck-panel.test.tsx
  src/app/workflows/recheck-control.test.tsx` passes (opens on Recheck, the icon opens and
  closes it, live chunks append to the running check, selecting a finished check shows its
  stored output, the counts), then the task checks in "Now".

### Polling gives way to the event stream (F-139 to F-144)

Agreed with Hamoud and approved by Mohammed on 2026-11-24. The client polls where an event
already reaches it. F-139 to F-141 need no backend. F-142 to F-144 each drop a poll once
Hamoud's matching task in `docs/backend/stages.md` journals the change: B-208 (thread list changes
on the project tail), B-209 (`TaskProgressChanged` also when a check finishes or a conflict's delivery
changes) and B-210 (RecheckStarted and RecheckFinished on the project tail). The facts this rests on:
- `shell.tsx` holds the open project's stream at all times, and
  `TaskProgressChanged {project_id, workflow_id, version, task, attempt_id}` reaches it. Core
  writes it in the same write that records an attempt as running, before it answers Start.
- `holdThreadRefresh` (`stream/refresh.ts`) already refreshes a thread's entries and operations.
- External connections change only by the person's own issue or revoke, which already
  invalidates the list.

### F-139 · The recheck panel and external connections stop polling
- Status: done
- Plan: none (the section above). While the recheck panel holds a batch's output stream,
  `lastRecheckQuery` is not read on an interval; with the panel closed it keeps today's 5 s read
  while rows are unfinished (until F-144). A `finished` or `end` frame invalidates only that
  workflow's `runKeys.recheck` and that batch's `runKeys.recheckRows`, not every run key.
  `connectionsQuery` loses its 10 s interval.
- Read: `web/src/stream/recheck-output.ts`, `web/src/app/workflows/recheck-panel.tsx`,
  `recheck-control.tsx`, `lastRecheckQuery` and `recheckRowsQuery` in `web/src/api/runs.ts`,
  `web/src/api/external-connections.ts`, `web/src/stream/contract.yaml`, `web/src/api/contract.yaml`.
- Touch: `web/src/stream/recheck-output.ts` and its test, `web/src/api/runs.ts`, `web/src/api/runs.test.ts`,
  `web/src/app/workflows/recheck-panel.tsx` and its test, `web/src/app/workflows/recheck-control.tsx`,
  `web/src/api/external-connections.ts`, `web/src/api/contract.yaml`, `web/src/stream/contract.yaml`,
  `web/src/app/workflows/contract.yaml`.
- Needs: none.
- Done when: `npx vitest run src/stream/recheck-output.test.tsx src/app/workflows/recheck-panel.test.tsx
  src/app/workflows/recheck-control.test.tsx src/api/runs.test.ts` passes, with new cases: a
  `finished` frame invalidates the batch's recheck and rows keys and leaves another task's run
  key fresh; `lastRecheckQuery` has no interval while the panel streams a batch.

### F-140 · Task progress follows the stream, not a 2 s read
- Status: done
- Plan: none (the section above). `classifyRefresh` maps `TaskProgressChanged` to that task's run
  key, its attempts key and the plan's progress key (`workflow_id`, `version`, `task` from the
  payload), through invalidate functions in `api/runs.ts` and `api/workflows.ts`. Then
  `taskRunQuery`, `attemptsQuery` and `planProgressQuery` lose their 2 s intervals, and
  `use-task-run.ts` drops `STARTING_READ_MS`. `run-tab.tsx` holds the executor's thread with
  `holdThreadRefresh` on `attempt.thread_id` instead of reading entries and operations every 2 s.
  Remove the "Known and left" line about `TaskProgressChanged` below.
- Read: `web/src/stream/refresh.ts`, `web/src/stream/contract.yaml`, `taskRunQuery` and `attemptsQuery`
  in `web/src/api/runs.ts`, `planProgressQuery` in `web/src/api/workflows.ts`,
  `web/src/app/tasks/use-task-run.ts`, `web/src/app/tasks/run-tab.tsx`.
- Touch: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/api/runs.ts`,
  `web/src/api/runs.test.ts`, `web/src/api/workflows.ts`, `web/src/api/workflows.test.ts`,
  `web/src/app/tasks/use-task-run.ts`, `web/src/app/tasks/run-tab.tsx`, `web/src/app/tasks/task-page.test.tsx`,
  `web/src/api/contract.yaml`, `web/src/stream/contract.yaml`, `web/src/app/tasks/contract.yaml`,
  this file.
- Needs: none.
- Done when: `npx vitest run src/stream/refresh.test.tsx src/api/runs.test.ts src/api/workflows.test.ts
  src/app/tasks/task-page.test.tsx` passes, with new cases: `TaskProgressChanged` refreshes that
  task's run, attempts and plan progress and no other task's; none of the three queries has an interval.

### F-141 · The connection indicator reads the stream; code servers poll only while starting
- Status: done
- Plan: none (the section above). With a project open, `ConnectionIndicator` shows the project
  stream's connection (`live` is Connected, `connecting` and `reconnecting` are Connecting,
  `failed` is Not reachable) and the project list is not polled. With no project open, the
  project list is read every 30 s, as today's reachability check. Never both. `stream/` gives the
  connection through a hook beside `useProjectEvents`, from the subscriber's `onState`.
  `codeServersQuery` is read every 5 s only while a server's state is `starting`.
- Read: `web/src/app/daemon-status.tsx`, `projectsQuery` in `web/src/api/projects.ts`,
  `web/src/stream/use-project-events.ts`, `web/src/stream/thread-registry.ts`,
  `web/src/app/sidebar/sidebar.tsx`, `codeServersQuery` in `web/src/api/code.ts`, `ServerState`
  in `web/src/api/schema.d.ts`.
- Touch: `web/src/app/daemon-status.tsx`, a new `web/src/app/daemon-status.test.tsx`,
  `web/src/app/sidebar/sidebar.tsx`, `web/src/api/projects.ts`, `web/src/api/projects.test.ts`,
  `web/src/stream/use-project-events.ts` and its test, `web/src/stream/index.ts`, `web/src/api/code.ts`,
  `web/src/api/code.test.ts`, `web/src/api/contract.yaml`, `web/src/stream/contract.yaml`,
  the `contract.yaml` that covers `daemon-status.tsx`, the code map row.
- Needs: none.
- Done when: `npx vitest run src/app/daemon-status.test.tsx src/stream/use-project-events.test.tsx
  src/api/projects.test.ts src/api/code.test.ts` passes, with cases: each stream connection shows
  its label; with a project open the project list has no interval, without one it has 30 s;
  code servers have an interval only while one is `starting`.

### F-142 · Conversation lists follow the project stream
- Status: done
- Plan: none (the section above). After B-208, `threadsQuery` and `agentThreadsQuery` lose their
  10 s intervals, and `ThreadRemoved` and a turn's end (the `Operation…` kinds) refresh `thread-lists` in
  `classifyRefresh`; the filter in `dispatch` that drops `thread-lists` from a project scope goes.
  `AgentList` holds the AYLUL project's stream itself with `useProjectEvents(agents.data?.project_id)`;
  today it is only held because `GroupList` is drawn beside it.
- Read: B-208 in `docs/backend/stages.md`, `web/src/stream/refresh.ts`, `web/src/api/threads.ts`,
  `agentThreadsQuery` in `web/src/api/aylul-profiles.ts`, `web/src/app/sidebar/agent-list.tsx`.
- Touch: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/api/threads.ts`,
  `web/src/api/threads.test.ts`, `web/src/api/aylul-profiles.ts`, `web/src/api/aylul-profiles.test.ts`,
  `web/src/app/sidebar/agent-list.tsx`, `web/src/app/sidebar/sidebar.test.tsx`, `web/src/api/contract.yaml`,
  `web/src/stream/contract.yaml`, `web/src/app/sidebar/contract.yaml`.
- Needs: B-208 (`e3195821`). On the project tail an event has no thread scope: `classifyRefresh` is called
  with `threadId: null` there, so `ThreadRemoved` and the `Operation…` kinds map to `thread-lists` whatever the scope.
- Done when: `npx vitest run src/stream/refresh.test.tsx src/api/threads.test.ts src/api/aylul-profiles.test.ts
  src/app/sidebar/sidebar.test.tsx` passes, with cases: on a project stream `ThreadRetitled`, `ThreadRemoved` and
  `OperationCompleted` refresh the thread lists; neither list query has an interval; `AgentList` holds the
  AYLUL project's stream.

### F-143 · Checks and a conflict's delivery follow the stream
- Status: todo
- Plan: none (the section above). After B-209, `TaskProgressChanged` also refreshes `runKeys.checks`
  and `runKeys.conflict` of its `attempt_id`, and `attemptChecksQuery` and `attemptConflictQuery` lose their 2 s intervals.
- Read: B-209 in `docs/backend/stages.md`, `web/src/stream/refresh.ts`, `web/src/api/runs.ts`.
- Touch: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/api/runs.ts`,
  `web/src/api/runs.test.ts`, `web/src/api/contract.yaml`, `web/src/stream/contract.yaml`.
- Needs: B-209 (`691af7f2`): `TaskProgressChanged` is also written when an attempt's check takes its result,
  when a conflict is recorded and when its delivery leaves `pending`.
- Done when: `npx vitest run src/stream/refresh.test.tsx src/api/runs.test.ts` passes, with cases:
  `TaskProgressChanged` refreshes the checks and conflict of its `attempt_id` and no other attempt's;
  neither query has an interval.

### F-144 · The last recheck follows the stream
- Status: todo
- Plan: none (the section above). After B-210, `lastRecheckQuery` has no interval at all;
  RecheckStarted and RecheckFinished refresh it.
- Read: B-210 in `docs/backend/stages.md`, `web/src/stream/refresh.ts`, `lastRecheckQuery` in `web/src/api/runs.ts`.
- Touch: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/api/runs.ts`,
  `web/src/api/runs.test.ts`, `web/src/stream/recheck-output.ts` and its test (the stream hold from F-139 goes),
  `web/src/api/contract.yaml`, `web/src/stream/contract.yaml`.
- Needs: B-210 (`37b1c186`): `RecheckStarted` and `RecheckFinished {project_id, workflow_id, recheck_id}` on the
  project tail; `workflow_id` is the plan whose done mark or request began the batch.
- Done when: `npx vitest run src/stream/refresh.test.tsx src/api/runs.test.ts src/stream/recheck-output.test.tsx`
  passes, with cases: `RecheckStarted` and `RecheckFinished` refresh that workflow's recheck read and that batch's
  rows; `lastRecheckQuery` has no interval.

Every task's checks run from `web/`: the tests it names, then `npm run typecheck`, `npm run lint`,
`npm run build` and `npx vitest run src/codemap.test.ts src/architecture`.

### Shipped

A shipped task's entry, and its plan once deleted, are in this file in Git at the commit named.

| PR | Work | Tasks | Entries at |
|---|---|---|---|
| #40 | The settings dialog | F-2 to F-12, F-14, F-15 | `1a2b601` |
| #41 | Architecture restructuring, web side | F-1, F-23 to F-25 | `c7e7bac4` |
| #43 | W-1 to W-8 | F-16, F-17, F-26, F-27, F-29 to F-32 | `ca618f89` |
| #44 | W-9, W-10, W-12, W-13 | F-20, F-21, F-33 to F-44 | `4d5c022d` |
| #45 | The context ring shows the summary only | F-22 | `29ddfd88` |
| #46 | Each settings form saves only what changed | F-28 | `e95b79c3` |
| #47 | W-11, with B-14 to B-18 | F-45 to F-49 | `4f23c717` |
| #48 | Web polish | F-50 to F-61 | `ce31036d` |
| #49 | AYLUL agents and panel, and the AI Studio UI migration | F-62 to F-76, F-77 (accepted 2026-11-03) | `b40e077b`, `a7b4009c` |
| #51 | The agent form starts empty; the background tasks panel | F-78, F-79 | `16da6b8c` |
| #52, #53 | Per-tool permissions in the agent form | F-80 | `9c7c7e5a` |
| #54 | Agent groups in the main window | F-81 to F-85 | `9e5b9ab2` |
| #57 | Planner context, the inspection view | F-86 to F-89 | `1b413fc4` |
| #60, #61 | Structure and depth, slices 2 to 5 (tried in the browser 2026-11-07) | F-90 to F-95 | `b64242a7`, `a4f89386` |
| #62 to #64 | Conversation order, Codex, the task page | F-96, F-97, F-98 | `3a443ea0`, `217d7540`, `c71b263d` |
| #65 | Task page review, the Diff tab, the Planner's notices | F-106 to F-113 | `617d806f` |
| #67 | Conversation images, the question card, Settings › Code | F-114 to F-116 | `db760593` |
| #68 | Attempt briefs and contract conflicts on the task page and in the Planner's notices | F-102, F-117 to F-120 (skeleton loading and task-page follow-ups) | `d73bb646` |
| #69 | Checks on the task's Report tab, the done reason under its title, and replies in the conversation | F-103 | `4031168f` |
| #70 | Interrupted and stopped attempt cards, the recheck card, decisions with Accept and in the attempt brief (shared request `docs/shared/requests/2026-11-16-slice-6-stop-control.md` at `e78e07dc`) | F-121 to F-124 | `449d4f95` |
| #71 | Executor choice on a task and in a new conversation | F-125 | `0e281585` |
| #72 | Settings › Code as one collapsible row, the plan page's real progress (its read is in `docs/backend/stages.md` at `753b26a4`), the visual acceptances recorded | F-126 to F-128 | `753b26a4` |
| #73 | The Report tab shows the `contract` check as Shadows' own (F-129; plan `docs/backend/plans/2026-11-19-code-record.md` at `7b7abec`) | F-129 | `7b7abec` |
| #75 | The project's state view, its ER diagram and screen flow, pinned elements in plan review and in the attempt brief, `code_where` and its contract inputs and return (the shared requests `docs/shared/requests/2026-11-19-project-state-and-diagrams.md` and `docs/shared/requests/2026-11-19-code-where.md`) | F-130 to F-136 | `4d69977d` |

Dropped without being built: F-99 and F-101 (covered by F-102's brief panel and F-116's server list; the rest is in [future](../core/future.md)), F-100 (with B-102), F-104 and F-105 (with the repair plan; slice 6 replanned them as F-121 to F-124). Their entries and plan are at `d997393a`.

PRs #59 and #66 have no `F-` task. Two composer fixes in PR #61 have none either: `1b85b6e6`
(the textarea grows to 192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

### Known and left

- The light palette and F-114's composer tiles are recorded in `docs/evidence/2026-11-18-visual-acceptance.md`.
  Left to Mohammed: the tiles inside a sent message and the desktop login item (start with Windows).

### Before a new task

Its shared request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first. The old mockup's `Built` label is not
evidence of completion.
