# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

PR #76 (`e7b9332`) is on `main`: the project recheck on the plan page (F-137, F-138) and the screens
that follow the stream instead of polling (F-139 to F-144). Open: the Planner's project switches (F-145 to F-148, below). The project's state view, its ER diagram and screen flow,
and `code_where` in the web (F-130 to F-136) are on `main` in PR #75 (`4d69977d`).

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
| #76 | The project recheck on the plan page and its live terminal panel; the screens follow the project stream instead of polling (B-208 to B-210 in `docs/backend/stages.md` at `e7b9332`) | F-137 to F-144 | `e7b9332` |

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

## The Planner's project switches

Plan: `docs/frontend/plans/2026-11-25-planner-project-switches.md`. Shared request `docs/shared/requests/2026-11-25-planner-project-switches.md`,
`accepted` (Hamoud's answer at `77ed3cd`). Backend: B-234 in `docs/backend/stages.md` (the
routes and `Project.planner_reads_claude_md`). Order: F-145, then F-146 once B-234 is `done`,
then F-147, then F-148.

### F-145 · The agent form's switch becomes a shared switch
- Status: done
- Plan: docs/frontend/plans/2026-11-25-planner-project-switches.md#f-145--the-agent-forms-switch-becomes-a-shared-switch
- Read: `web/src/app/agents/contract.yaml` (`AgentExtensions`); `docs/frontend/codebase/README.md`
  (the architecture table's `shared` row); `web/src/app/agents/agent-extensions.tsx`
  (`ExtensionSwitch`, `AgentExtensions`)
- Touch: `web/src/app/saving-switch.tsx` (new), `web/src/app/agents/agent-extensions.tsx`,
  `web/src/app/agents/contract.yaml`, `docs/frontend/codebase/README.md`
- Needs: none (the request is `accepted`; no backend)
- Done when: `npx vitest run src/app/agents/agent-extensions.test.tsx src/app/agents/tool-rules.test.tsx
  src/app/agents/agent-form.test.tsx` passes with no test file changed, then the four checks

### F-146 · The project's switch calls in `web/src/api/`
- Status: done
- Plan: docs/frontend/plans/2026-11-25-planner-project-switches.md#f-146--the-projects-switch-calls-in-websrcapi
- Read: `web/src/api/contract.yaml` (`extensions`, `extension_cache`, the projects functions);
  the plan's "Core's answers, exactly"; `web/src/api/extensions.ts` (`extensionKeys`,
  `saveAgentExtensionAnswer`, `writeSaved`); `web/src/api/projects.ts` (`saveProjectAnswer`)
- Touch: `web/src/api/schema.d.ts` (through `npm run gen:api` only, run first),
  `web/src/api/extensions.ts`, `web/src/api/projects.ts`, `web/src/api/extensions.test.ts`,
  `web/src/api/requests.test.ts`, `web/src/api/contract.yaml`, `web/src/test/contract-fixtures.ts`
- Needs: `docs/shared/requests/2026-11-25-planner-project-switches.md` (accepted); B-234 `done`
- Done when: `npx vitest run src/api/extensions.test.ts src/api/requests.test.ts
  src/app/agents/agent-extensions.test.tsx` passes, with the four tests the plan names, then
  the four checks

### F-147 · The section "What the Planner may use"
- Status: done
- Plan: docs/frontend/plans/2026-11-25-planner-project-switches.md#f-147--the-section-what-the-planner-may-use
- Read: `web/src/app/project-settings/contract.yaml`; `docs/frontend/codebase/README.md`
  (the `project-settings` row); the plan's "Core's answers, exactly" and decisions 5, 7, 8;
  `web/src/app/project-settings/project-settings.tsx` (the `ProjectModes` block);
  `web/src/app/agents/agent-extensions.test.tsx` (`switchOf`, `a failed switch goes back`)
- Touch: `web/src/app/project-settings/planner-switches.tsx` (new),
  `web/src/app/project-settings/planner-switches.test.tsx` (new),
  `web/src/app/project-settings/project-settings.tsx`,
  `web/src/app/project-settings/contract.yaml`, `web/src/test/fake-daemon.ts`,
  `docs/frontend/codebase/README.md`
- Needs: F-145, F-146 (so B-234 `done`)
- Done when: `npx vitest run src/app/project-settings` passes, with the nine tests the plan
  names, then the four checks

### F-148 · A library change reaches the project's switches
- Status: done
- Plan: docs/frontend/plans/2026-11-25-planner-project-switches.md#f-148--a-library-change-reaches-the-projects-switches
- Read: `web/src/app/settings/contract.yaml` (the library notes, `extension_refresh_owner`);
  the three `onSuccess` blocks the plan names in `mcp-form.tsx`, `extension-import.tsx`,
  `extension-list.tsx`
- Touch: `web/src/app/settings/mcp-form.tsx`, `web/src/app/settings/extension-import.tsx`,
  `web/src/app/settings/extension-list.tsx`, `web/src/app/settings/contract.yaml`,
  `web/src/app/project-settings/planner-switches.test.tsx`
- Needs: F-146, F-147
- Done when: `npx vitest run src/app/settings src/app/agents/agent-extensions.test.tsx
  src/app/project-settings/planner-switches.test.tsx` passes, with
  `a library removal refreshes the project's switches`, then the four checks
