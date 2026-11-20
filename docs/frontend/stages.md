# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Branch `ccr-9e1b02d8-b0ylm1`: the project's state, its ER diagram and screen flow, and `code_where`
in the web. Plan: [plans/2026-11-20-project-state-views.md](plans/2026-11-20-project-state-views.md).
Shared requests, accepted 2026-11-20: `docs/shared/requests/2026-11-19-project-state-and-diagrams.md`
(F-130 to F-134) and `docs/shared/requests/2026-11-19-code-where.md` (F-135). The controller
dispatches the builders (Mohammed, 2026-11-19); Masada prepares and checks. Every task is blocked
until its backend tasks are built and `api/openapi.json` shows them. The plan's "Rulings for every
task" bind each one.

Every task's checks run from `web/`: the tests it names, then `npm run typecheck`, `npm run lint`,
`npm run build` and `npx vitest run src/codemap.test.ts src/architecture`.

### F-130 · The API domain and the stream refresh for the project's state

- Status: blocked (on B-177, B-178, B-179, B-180 and B-187)
- Plan: [plans/2026-11-20-project-state-views.md#f-130--the-api-domain-and-the-stream-refresh](plans/2026-11-20-project-state-views.md#f-130--the-api-domain-and-the-stream-refresh)
- Read:
  - `web/src/api/contract.yaml`, `web/src/stream/contract.yaml` (`refresh_policy`);
  - the shared request's Fields and "Refresh";
  - `web/src/stream/refresh.ts`.
- Touch:
  - `web/src/api/schema.d.ts` (only by `npm run gen:api`);
  - `web/src/api/error.ts`, `web/src/api/runs.ts` (`retryLost` moves, nothing else);
  - `web/src/api/project-state.ts` (new), `web/src/api/project-state.test.ts` (new);
  - `web/src/api/contract.yaml`;
  - `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/stream/contract.yaml`;
  - `docs/frontend/codebase/README.md` (two rows).
- Not: any file under `web/src/app/`; `web/src/stream/use-project-events.ts`;
  `web/src/stream/thread-registry.ts`.
- Needs: the shared request (accepted); B-177 to B-180 and B-187 built.
- Done when, run with `npx vitest run src/api/project-state.test.ts src/stream/refresh.test.tsx src/api/groups.test.ts src/stream/use-project-events.test.tsx`:
  - `project_state_events_refetch_the_state_and_the_elements` (all 12 kinds);
  - `a_design_element_change_refetches_only_that_elements_revisions`;
  - `task_progress_and_record_changes_name_only_the_project_state`;
  - `replayed_state_events_reconcile_the_state_once_at_caught_up`;
  - `the_state_read_sends_part_id_only_when_asked`;
  - `the_reads_retry_only_a_lost_call`;
  - every earlier test in those files still passes;
  - the four standard checks pass.

### F-131 · The state page and the state view

- Status: blocked (on F-130)
- Plan: [plans/2026-11-20-project-state-views.md#f-131--the-state-page-and-the-state-view](plans/2026-11-20-project-state-views.md#f-131--the-state-page-and-the-state-view)
- Read:
  - the plan's "Where the screens live" and "What is reused";
  - `web/src/app/design/workspace-page.tsx`;
  - `web/src/router.tsx` (`workspaceRoute`, `agreementsRoute`);
  - `web/src/stream/use-project-events.test.tsx` (the integration pattern);
  - `docs/frontend/codebase/README.md` ("Architecture modules").
- Touch:
  - `web/src/router.tsx`;
  - `web/src/app/design/workspace-page.tsx` (one `Link`);
  - `web/src/app/project-state/index.ts`, `state-page.tsx`, `state-badge.tsx`, `state-view.tsx`,
    `state-view.test.tsx`, `contract.yaml` (all new);
  - `docs/frontend/codebase/README.md` (rows, the `project-state` module row, `composition`'s
    dependencies, the Contracts list).
- Not: `web/src/api/`, `web/src/stream/`, any other file in `web/src/app/design/`.
- Needs: F-130.
- Done when, run with `npx vitest run src/app/project-state/state-view.test.tsx src/app/design`:
  - the nine tests the plan names for F-131 pass;
  - the design folder's tests still pass;
  - the four standard checks pass.

### F-132 · The element panel and the ER diagram

- Status: blocked (on F-131)
- Plan: [plans/2026-11-20-project-state-views.md#f-132--the-element-panel-and-the-er-diagram](plans/2026-11-20-project-state-views.md#f-132--the-element-panel-and-the-er-diagram)
- Read:
  - `web/src/app/design/map-canvas.tsx`;
  - `web/src/app/workflows/layout.ts` (`placeGraph`, `handles`);
  - `web/src/app/workflows/flow-theme.ts`;
  - `web/src/app/project-state/contract.yaml`;
  - the shared request's "Reading the shapes".
- Touch:
  - `web/src/app/element-content.tsx` (new, shared);
  - `web/src/app/project-state/diagram-layout.ts`, `diagram-layout.test.ts`, `element-canvas.tsx`,
    `element-panel.tsx`, `tables-view.tsx`, `tables-view.test.tsx` (all new);
  - `web/src/app/project-state/state-page.tsx`, `web/src/app/project-state/contract.yaml`;
  - `docs/frontend/codebase/README.md` (rows; `app/element-content.tsx` in the `shared` row).
- Not: `web/src/app/design/`, `web/src/app/workflows/`, `web/src/api/`, `web/src/stream/`,
  `web/src/router.tsx`.
- Needs: F-131.
- Done when, run with `npx vitest run src/app/project-state`:
  - the four layout tests and the seven tables-view tests the plan names pass;
  - F-131's tests still pass;
  - the four standard checks pass.

### F-133 · The screen flow

- Status: blocked (on F-132)
- Plan: [plans/2026-11-20-project-state-views.md#f-133--the-screen-flow](plans/2026-11-20-project-state-views.md#f-133--the-screen-flow)
- Read: `web/src/app/project-state/tables-view.tsx`, `web/src/app/project-state/contract.yaml`.
- Touch:
  - `web/src/app/project-state/screens-view.tsx`, `screens-view.test.tsx` (new);
  - `web/src/app/project-state/state-page.tsx`, `web/src/app/project-state/contract.yaml`;
  - `docs/frontend/codebase/README.md` (rows);
  - only as the plan allows: `web/src/app/project-state/elements-view.tsx` (new) and
    `tables-view.tsx`.
- Not: anything outside `web/src/app/project-state/` but the code map.
- Needs: F-132.
- Done when, run with `npx vitest run src/app/project-state`:
  - the four screens-view tests pass;
  - every earlier test in the folder still passes;
  - the four standard checks pass.

### F-134 · Pinned elements in plan review and in the attempt brief

- Status: blocked (on F-132, and B-177)
- Plan: [plans/2026-11-20-project-state-views.md#f-134--pinned-elements-in-plan-review-and-in-the-brief](plans/2026-11-20-project-state-views.md#f-134--pinned-elements-in-plan-review-and-in-the-brief)
- Read:
  - `web/src/app/workflows/contract.yaml`, `web/src/app/tasks/contract.yaml`;
  - `web/src/app/workflows/inspect-panel.tsx`, `web/src/app/tasks/brief-panel.tsx`;
  - the shared request's "Plan review and the task brief".
- Touch:
  - `web/src/app/workflows/inspect-panel.tsx`, `web/src/app/workflows/plan-page.test.tsx`,
    `web/src/app/workflows/contract.yaml`;
  - `web/src/app/tasks/brief-panel.tsx`, `web/src/app/tasks/task-page.test.tsx`,
    `web/src/app/tasks/contract.yaml`.
- Not: `web/src/api/`, `web/src/app/element-content.tsx` (used as F-132 left it), any other file.
- Needs: F-132; B-177 built (its fields are in the schema F-130 generated).
- Done when, run with `npx vitest run src/app/workflows/plan-page.test.tsx src/app/tasks/task-page.test.tsx`:
  - `the_inspect_panel_lists_pinned_elements_by_name`;
  - `a_pin_whose_element_is_not_read_shows_its_id`;
  - `the_brief_lists_its_pinned_revisions_in_full`;
  - `a_brief_without_elements_has_no_pinned_section`;
  - every earlier test in both files still passes;
  - the four standard checks pass.

### F-135 · `code_where` in the web

- Status: blocked (on B-182, B-183, B-185, and F-132)
- Plan: [plans/2026-11-20-project-state-views.md#f-135--code_where-in-the-web](plans/2026-11-20-project-state-views.md#f-135--code_where-in-the-web)
- Read:
  - `docs/shared/requests/2026-11-19-code-where.md` (Fields, "Who uses a symbol", Errors);
  - `web/src/api/contract.yaml`, `web/src/app/project-state/contract.yaml`.
- Touch:
  - `web/src/api/schema.d.ts` (only by `npm run gen:api`);
  - `web/src/api/code-record.ts`, `web/src/api/code-record.test.ts` (new);
  - `web/src/api/contract.yaml`;
  - `web/src/app/project-state/where-view.tsx`, `where-view.test.tsx` (new);
  - `web/src/app/project-state/state-page.tsx`, `web/src/app/project-state/contract.yaml`;
  - `docs/frontend/codebase/README.md` (rows).
- Not: `web/src/stream/` (a question is never asked again by an event); `web/src/router.tsx`
  (F-131 defined `topic`); `web/src/api/code.ts`.
- Needs: the shared request (accepted); B-182, B-183 and B-185 built; F-132.
- Done when, run with `npx vitest run src/api/code-record.test.ts src/app/project-state`:
  - the two API tests and the eleven where-view tests the plan names pass;
  - every earlier test in the folder still passes;
  - the four standard checks pass.

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
