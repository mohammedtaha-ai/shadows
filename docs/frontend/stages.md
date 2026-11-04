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

Planner context, slice 3: the inspection view ([plan](plans/2026-11-05-planner-context-view.md)).

### F-86 · Planning context API module
- Status: todo
- Plan: docs/frontend/plans/2026-11-05-planner-context-view.md#task-f-86-planning-context-api-module
- Read: api/openapi.json (`AnchorResponse`, `SetAnchor`, `ThreadPlanningAnchor`, the context record); web/src/api/contract.yaml
- Touch: web/src/api/planning-context.ts, planning-context.test.ts, contract.yaml; web/src/test/fake-daemon.ts;
  docs/frontend/codebase/README.md
- Done when: `npx vitest run src/api/planning-context.test.ts` passes (the caller's command id; a missing
  record is null)

### F-87 · The anchor chip in the conversation header
- Status: todo
- Plan: docs/frontend/plans/2026-11-05-planner-context-view.md#task-f-87-the-anchor-chip-in-the-conversation-header
- Read: web/src/app/conversation/stage-chip.tsx, conversation.tsx; web/src/api/design.ts
- Touch: web/src/app/conversation/anchor-chip.tsx, anchor-chip.test.tsx, anchor-picker.tsx, conversation.tsx,
  contract.yaml; docs/frontend/codebase/README.md
- Needs: F-86
- Done when: `npx vitest run src/app/conversation/anchor-chip.test.tsx` passes, with the seven tests the task names

### F-88 · Start a conversation anchored at a part or an outcome
- Status: todo
- Plan: docs/frontend/plans/2026-11-05-planner-context-view.md#task-f-88-start-a-conversation-anchored-at-a-part-or-an-outcome
- Read: web/src/router.tsx (draft route); web/src/app/conversation/first-send.ts, draft.tsx
- Touch: web/src/router.tsx; web/src/app/conversation/draft.tsx, first-send.ts, draft.test.tsx, carried-send.ts,
  use-send.ts, composer.tsx, contract.yaml;
  web/src/app/design/parts-view.tsx, roadmap-view.tsx, contract.yaml
- Needs: F-86
- Done when: `npx vitest run src/app/conversation/draft.test.tsx` passes, with the five tests the task names

### F-89 · Each turn's planning context, and its record
- Status: blocked
- Plan: docs/frontend/plans/2026-11-05-planner-context-view.md#task-f-89-each-turns-planning-context-and-its-record
- Read: web/src/app/conversation/messages.tsx, conversation.tsx, plan-side-panel.tsx
- Touch: web/src/app/conversation/context-mark.tsx, context-record-panel.tsx, context-record-panel.test.tsx,
  messages.tsx, conversation.tsx, deleted-conversation.tsx, contract.yaml; docs/frontend/codebase/README.md
- Needs: B-59, F-86
- Done when: `npx vitest run src/app/conversation/context-record-panel.test.tsx` passes, with the six tests the
  task names
