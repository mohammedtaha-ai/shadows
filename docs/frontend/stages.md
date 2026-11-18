# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-126, F-127 and F-128 below. F-125 (choose an executor on a task or in a new conversation) is on `main` at `0e281585` (PR #71); its live acceptance is Mohammed's.

### F-127 · The plan page shows the plan's real progress

- Status: done (B-163 is built: `TaskProgress { task, done, done_at, state }`, state one of `AttemptState` or null)
- Plan: none. The plan page's "Plan progress" block sits inside `Planned` with "— done", an
  empty bar and disabled task toggles. With B-163, read
  `GET /api/workflows/{workflow}/versions/{version}/progress` (a new `planProgressQuery` in
  `web/src/api/workflows.ts`, keyed under the plan version, refetched when a task's run changes
  on the event stream as the task page's run is) and draw: "N tasks · M done" with M the count of
  `done`, a bar whose width is M / N, and one small mark per task (`T1`, `T2`, ...) showing done,
  running (latest state live), or not started; each mark links to that task's page. The marks
  are read-only: marking done stays on the task page. Remove the `Planned` wrapper and its
  reason. While the read is pending or fails, show the existing "— done" text and the error line.
- Read: `web/src/app/workflows/contract.yaml`; `web/src/api/contract.yaml` (`workflows.ts`);
  `web/src/app/workflows/plan-page.tsx` (the `Planned` block); B-163's `TaskProgress`.
- Touch: `web/src/api/workflows.ts`, `web/src/api/contract.yaml`, `web/src/app/workflows/plan-page.tsx`,
  `web/src/app/workflows/plan-page.test.tsx`, `web/src/app/workflows/contract.yaml`,
  `web/src/test/fake-daemon.ts`, `web/src/test/contract-fixtures.ts`, `docs/core/future.md` (drop
  its "Plan progress on the plan page" row), this entry's status.
- Needs: none (B-163 is a backend task on this branch).
- Done when: `plan-page.test.tsx` shows "3 tasks · 1 done", a bar at a third, a running mark and
  a link per task, and no `Planned` block; `npx --prefix web vitest run src/app/workflows`,
  `npm --prefix web run typecheck` and `npm --prefix web run lint` pass.

### F-128 · Record the visual acceptances left open

- Status: done (evidence `docs/evidence/2026-11-18-visual-acceptance.md`; the sent-message tiles and the login item are left to Mohammed)
- Plan: none. "Known and left" below lists three looks never recorded. In the browser
  (`npx --prefix web vite web` against a running daemon): the light palette on the main screens
  (sidebar, a conversation, the plan page, settings), and the image tiles of F-114 (attach two
  images in the composer, send, and see them in the conversation). Take a screenshot of each and
  note any defect. The desktop login item (start with Windows) needs the AYLUL desktop app and
  stays Mohammed's: record it as not run here, with that reason.
  Record everything in `docs/evidence/2026-11-18-visual-acceptance.md`; a defect found is listed
  there, not fixed in this task.
- Read: `web/src/app/settings/person-theme.tsx`; F-114 in Git (`docs/frontend/stages.md` at `d997393a`).
- Touch: the evidence file, this file's "Known and left" entries.
- Done when: the evidence file has each look's observed result, or "not run" with its reason.

### F-126 · Language servers become one collapsible row in Settings › Code

- Status: done
- Plan: none; the shape is Mohammed's ruling of 2026-11-18, written here in full.
  Settings › Code becomes a list of setting rows like Claude Code's own settings page: each row
  has a title, a one-line description under it, and its control on the right; rows are divided
  by a hairline. Today the section's whole page is the server list; after this, the list is one
  row, "Language servers", with the description "Help the Planner read your code. It installs
  what's missing when it needs it.", a summary on the right ("N running", or "None running")
  and a chevron. The row starts collapsed; clicking it (a button with `aria-expanded`) opens the
  list under it, indented. Open, the list shows only the servers whose `wanted` is true for the
  open project, one line each: the language name (from a small map of the catalog ids, e.g.
  `gopls` → Go, `typescript` → TypeScript; an added server or an unmapped id
  shows its id), a coloured dot and one word for the state (Running, Starting, Idle,
  Not installed, Failed, Off), and the existing on/off switch with its existing confirm panel.
  A failed server keeps its error line. The servers whose `wanted` is false sit behind one
  more line, "N more languages", which opens them in the same shape. "Add a server" stays, as
  a small link at the end of the open list that opens the existing add form. Removed from the
  rows: the extension list, the install command and its copy button (the Planner installs),
  and the raw server id as the title. The switch text under it says the switch acts in every
  project ("Off in every project"), since `PUT /api/code/servers/{server}/off` is global.
  No backend change: `GET /api/projects/{id}/code/servers` already answers `wanted` and `state`.
- Read: `web/src/app/settings/contract.yaml` (`CodeSection`), `web/src/api/code.ts`
  (`ServerStatus`, `codeServersQuery`), `web/src/app/settings/sections/code.tsx`.
- Touch: `web/src/app/settings/sections/code.tsx`, `web/src/app/settings/sections/code.test.tsx`,
  `web/src/app/settings/settings-dialog.test.tsx` (its `opening_code_shows_its_servers` opens the row first),
  `web/src/app/settings/contract.yaml`, this entry's status.
- Needs: none.
- Done when: `code.test.tsx` covers: the row starts collapsed with the running count; opening it
  lists only wanted servers by language name with their state word; "N more languages" reveals
  the rest; switching off still confirms; no install command is shown; "Add a server" opens the
  form. `npx --prefix web vitest run src/app/settings`, `npx --prefix web tsc -b` and
  `npx --prefix web eslint src/app/settings` pass.

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
