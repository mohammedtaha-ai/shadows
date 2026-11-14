# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

### F-117 · Content-shaped skeleton loading

- Status: done. Initial loading, success, failure and background-refresh checks passed. The user reports trying the interface in the browser.
- Design: approved in the current conversation; implemented directly by the controller.
- Read: frontend code map and the touched feature contracts.
- Touch: `docs/frontend/codebase/README.md`, `docs/frontend/stages.md`, `web/src/app/agents/agent-env.tsx`, `web/src/app/agents/agent-extensions.tsx`, `web/src/app/agents/agent-settings.tsx`, `web/src/app/agents/contract.yaml`, `web/src/app/agents/home.tsx`, `web/src/app/agents/tool-rules.tsx`, `web/src/app/agreements/agreements-page.tsx`, `web/src/app/agreements/contract.yaml`, `web/src/app/conversation/context-record-panel.tsx`, `web/src/app/conversation/contract.yaml`, `web/src/app/conversation/conversation.tsx`, `web/src/app/conversation/draft.tsx`, `web/src/app/conversation/plan-card.tsx`, `web/src/app/conversation/plan-side-panel.tsx`, `web/src/app/design/contract.yaml`, `web/src/app/design/parts-view.tsx`, `web/src/app/design/roadmap-view.tsx`, `web/src/app/design/standards-view.tsx`, `web/src/app/design/vision-editor.tsx`, `web/src/app/design/workspace-plans.tsx`, `web/src/app/groups/contract.yaml`, `web/src/app/groups/group-page.tsx`, `web/src/app/home.tsx`, `web/src/app/page-pending.tsx`, `web/src/app/project-settings/contract.yaml`, `web/src/app/project-settings/instructions-editor.tsx`, `web/src/app/settings/contract.yaml`, `web/src/app/settings/general-preferences.tsx`, `web/src/app/settings/general-preferences.test.tsx`, `web/src/app/settings/sections/agents.tsx`, `web/src/app/settings/sections/code.tsx`, `web/src/app/settings/sections/plugins.tsx`, `web/src/app/settings/sections/providers.tsx`, `web/src/app/settings/sections/usage.tsx`, `web/src/app/settings/you-card.tsx`, `web/src/app/sidebar/agent-list.tsx`, `web/src/app/sidebar/contract.yaml`, `web/src/app/sidebar/group-list.tsx`, `web/src/app/sidebar/project-list.test.tsx`, `web/src/app/sidebar/project-list.tsx`, `web/src/app/sidebar/thread-list.tsx`, `web/src/app/sidebar/workflow-list.tsx`, `web/src/app/tasks/brief-panel.tsx`, `web/src/app/tasks/contract.yaml`, `web/src/app/tasks/diff-tab.tsx`, `web/src/app/tasks/run-tab.tsx`, `web/src/app/tasks/task-page.tsx`, `web/src/app/workflows/contract.yaml`, `web/src/app/workflows/plan-map.tsx`, `web/src/app/workflows/plan-page.tsx`, `web/src/components/ui/skeleton.tsx`.
- Needs: none; existing API reads only.
- Done when: initial loading renders accessible skeletons, success and errors replace them, background refresh keeps existing content, reduced motion disables the pulse; affected interaction tests, typecheck, lint, build, codemap and architecture pass; browser acceptance is reported by the user in the current conversation.


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

PRs #59 and #66 have no `F-` task. Two composer fixes in PR #61 have none either: `1b85b6e6`
(the textarea grows to 192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

### Known and left

- The light palette and the real desktop login-item behaviour need their own app and OS acceptance.
- The image tiles' look in the browser (F-114) is not recorded.

### Before a new task

Its shared request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first. The old mockup's `Built` label is not
evidence of completion.

## Approved local delivery UI, 2026-11-09

These tasks are blocked until their named backend API exists; no component implements Core readiness policy.

### F-99 · Files view and server status

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 1](plans/2026-11-09-local-delivery-ui.md#task-1-f-99-files-view-and-server-status).
- Owner: web API and design UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-99); accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-100 · Preparation status from the Planner's project

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-local-delivery-ui.md#task-2-f-100-preparation-status-from-the-planners-project).
- Owner: web API and design UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: F-99 (B-102 was dropped 2026-11-11: the setup plan is read through the plan and task routes); accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-101 · Readable task contract and decision pins

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 3](plans/2026-11-09-local-delivery-ui.md#task-3-f-101-readable-task-contract-and-decision-pins).
- Owner: web API, task and workflow UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-106), F-99; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-102 · Attempt brief and scoped question results

- Status: done. Added to Touch by controller ruling: web/src/stream/frames.ts, frames.test.ts and web/src/stream/contract.yaml, so `readAttemptNotice` carries the notice's `conflict`.
- Plan: [F-102](../backend/plans/2026-11-14-slice-4-task-questions.md#task-f-102-saved-brief-and-conflict-presentation).
- Owner: web API, tasks and conversation presentation.
- Read: [approved design](../backend/specs/2026-11-14-task-questions-design.md), [accepted agreement](../shared/requests/2026-11-14-task-questions.md), frontend code map and the task's owner contracts/Interfaces.
- Touch: exact Files of F-102 in the detailed slice 4 plan; no other sources.
- Needs: B-107, B-147 (including B-109), accepted agreement and generated OpenAPI. No dependency on the superseded F-102 draft or a new F-101 implementation.
- Done when: F-102's five steps, focused tests, typecheck/lint/build and codemap/architecture pass; generated schema and current contracts/maps updated; scoped commit recorded. Browser acceptance is B-149.

### F-103 · Check evidence, critic model and acceptance

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 5](plans/2026-11-09-local-delivery-ui.md#task-5-f-103-check-evidence-critic-model-and-acceptance).
- Owner: web API and task UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 5; no other source paths.
- Needs: B-114, F-102; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 5 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-104 · Repair state and replan controls

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 6](plans/2026-11-09-local-delivery-ui.md#task-6-f-104-repair-state-and-replan-controls).
- Owner: web API and task UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 6; no other source paths.
- Needs: B-117, F-103; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 6 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-105 · Browser acceptance of the complete local journey

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 7](plans/2026-11-09-local-delivery-ui.md#task-7-f-105-browser-acceptance-of-the-complete-local-journey).
- Owner: Frontend/controller.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 7; no other source paths.
- Needs: B-118, F-104; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 7 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.
