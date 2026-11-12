# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-62 to F-76 and F-68 shipped in PR #49 (`b40e077b`), on `main`: the web half of the AYLUL
agents and panel design, and the AI Studio UI migration that replaced F-68's look. Their
entries are in this file at `b40e077b`. The AYLUL panel plan (`docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md` at `b40e077b`)
and the neon plan (`docs/frontend/plans/2026-11-01-neon-clarity.md` at `b40e077b`) are deleted.
F-77 (the whole web client after the migration) was accepted by Mohammed's visual check on 2026-11-03;
its entry is in this file at `a7b4009c`, and the AI Studio plan (`docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md`) at `a7b4009c`.
The migration's facts are in `docs/evidence/2026-11-02-ai-studio-ui-analysis.md` at `f1459dcc`.

F-78 and F-79 (the agent form starts empty; the background tasks panel) shipped in PR #51
(`2790473b`); their entries are in this file at `16da6b8c`.

F-80 (per-tool permissions in the agent form) shipped in PR #52 (`6b2c66ed`); its entry is in
this file at `9c7c7e5a`.
A click during a save is now queued and built on the saved rules (PR #53, `349606bf`).

F-81 to F-85 (agent groups in the main window) shipped in PR #54 (`9e5b9ab2`), with
B-40 to B-46; their entries are in this file at `9e5b9ab2`. The plan was
`docs/frontend/plans/2026-11-03-agent-groups-ui.md` at `9e5b9ab2`.

Planner context, slice 3 (the inspection view): F-86 to F-89 shipped in PR #57 (`1b413fc4`), with
B-59; their entries are in this file at `1b413fc4`. The plan was
`docs/frontend/plans/2026-11-05-planner-context-view.md` at `1b413fc4`.

Structure and depth, slice 1 (PR #59, `26bf739e`) is backend-only. Its only web change is a test
fixture in `web/src/app/design/create-structure.test.tsx`, with no `F-` task.

Structure and depth, slice 2 (PR #60, `b64242a7`): F-90 and F-91 shipped with B-63 to B-70. Their
entries, and the web fix `fb46cac4` outside their `Touch:` lists, are in this file at `b64242a7`.
The plan is at `a4f89386`, deleted after the merge.

Structure and depth, slices 3 to 5 (PR #61, `a4f89386`): F-92 to F-95 shipped with B-71 to B-82.
Their entries are in this file at `a4f89386`. Mohammed tried them in the browser on 2026-11-07.

Conversation order and Codex: F-96 (the conversation refreshes its order after an assistant reply) shipped in
PR #62 (`3a443ea0`) with B-83; F-97 (Codex in the conversation flow) in PR #63 (`217d7540`) with B-85 to B-87;
F-98 (the task page) in PR #64 (`c71b263d`) with B-89 to B-93. Their entries are in this file at those commits.

Task page review, the Diff tab and the Planner's notices: F-106 to F-113 shipped in PR #65 (`617d806f`) with
B-95 to B-136. Their entries are in this file at `617d806f`. Their plans,
`docs/frontend/plans/2026-11-11-task-page-review-and-diff.md` and `docs/frontend/plans/2026-11-11-shadows-notices.md`,
are at `617d806f`. PR #66 (`5a52a06a`) removed dead web exports and type aliases and has no `F-` task.

Still open outside any task: the light palette and the real desktop login-item behaviour need
their own app and OS acceptance.

Finished work: F-50 to F-61 (branch `feature/web-polish`, PR #48) are in this file at `ce31036d`;
F-45 to F-49 (W-11, with B-14 to B-18, PR #47) are in this file at `4f23c717`;
F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

Two composer fixes in PR #61 have no `F-` task: `1b85b6e6` (the textarea grows to
192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

Structure and planning depth: slices 1 to 5 are on `main`; slice 1 needs no web task. The
[workspace behavior](../backend/specs/2026-11-05-structure-and-depth-design.md#6-what-the-person-sees)
is the review target. Before a new `F-` task is dispatched, its shared
request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first.
Use Roadmap for outcome coverage and Project map for ownership. The old
mockup's `Built` label cannot be used as evidence of completion.

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
- Needs: B-102, F-99; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
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

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 4](plans/2026-11-09-local-delivery-ui.md#task-4-f-102-attempt-brief-and-scoped-question-results).
- Owner: web API and task UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 4; no other source paths.
- Needs: B-109, F-101; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 4 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

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


## Conversation images, 2026-11-12

### F-114 · Image tiles with a corner ✕ in the composer, and a viewer for every conversation image

- Status: done (start after the frontend-fixer's `crossOrigin="anonymous"` image fix is committed)
- Plan: [plans/2026-11-12-conversation-images.md](plans/2026-11-12-conversation-images.md)
- Read: the plan; `web/src/app/conversation/contract.yaml` (`image_attachments`); `web/src/components/ui/dialog.tsx`; code-map rows for `pending-images.tsx`, `entry.tsx`, `waiting-messages.tsx`.
- Touch: `web/src/app/conversation/image-zoom.tsx` (new), `web/src/app/conversation/image-zoom.test.tsx` (new), `web/src/app/conversation/pending-images.tsx`, `web/src/app/conversation/entry.tsx`, `web/src/app/conversation/waiting-messages.tsx`, `web/src/app/conversation/composer-images.test.tsx`, `web/src/app/conversation/waiting-messages.test.tsx`, `web/src/app/conversation/contract.yaml`, `docs/frontend/codebase/README.md`, `docs/frontend/stages.md` (this status only).
- Needs: none (no API change); the crossOrigin fix commit first.
- Done when: from `web/`, `npx vitest run src/app/conversation/image-zoom.test.tsx src/app/conversation/composer-images.test.tsx src/app/conversation/waiting-messages.test.tsx` passes with the test names in the plan's Tests section, and `npm run typecheck`, `npm run lint`, `npm run build`, `npx vitest run src/codemap.test.ts src/architecture` pass.

## Slice 2: the question card and Settings › Code (2026-11-13)

Design: [language servers](../backend/specs/2026-11-13-language-servers-design.md). Plan: [slice 2](../backend/plans/2026-11-13-slice-2-language-servers.md); each entry's Touch, tests and Done when are its task there.

### F-115 · The question card, the Planner's permission cards, and Shadows' notices

- Status: ready after B-143 and B-144.
- Plan: [Task F-115](../backend/plans/2026-11-13-slice-2-language-servers.md#task-f-115-the-question-card-the-planners-permission-cards-and-shadows-notices).

### F-116 · Settings › Code

- Status: ready after B-140 and B-142.
- Plan: [Task F-116](../backend/plans/2026-11-13-slice-2-language-servers.md#task-f-116-settings--code).
