# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-137 is open on `feature/after-75`. The project's state view, its ER diagram and screen flow,
and `code_where` in the web (F-130 to F-136) are on `main` in PR #75 (`4d69977d`).

### F-137 · The person rechecks the project from the plan page
- Status: todo
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
  `recheck-notice.test.tsx`, `web/src/app/conversation/contract.yaml`, the code map row.
- Needs: B-206.
- Done when: `npx vitest run src/app/workflows/recheck-control.test.tsx
  src/app/conversation/recheck-notice.test.tsx` passes (idle with last result, no result,
  running state, a click posts once with a command id, the all-passed card), then the task
  checks in "Now".

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
- The plan page's progress and the task page are not refreshed by `TaskProgressChanged`; the state read asks only for the state and the elements. Using the event there is a later choice (`docs/frontend/plans/2026-11-20-project-state-views.md` at `4d69977d`, "Not in these tasks").

### Before a new task

Its shared request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first. The old mockup's `Built` label is not
evidence of completion.
