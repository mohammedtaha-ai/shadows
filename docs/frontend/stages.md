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

F-81 to F-85 are built on `docs/agent-messaging-design`. The live acceptance is tracked in the
[group UI plan](plans/2026-11-03-agent-groups-ui.md#acceptance).

### F-81 · Groups API module
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-f-81-groups-api-module
- Read: api/openapi.json (`Group`, `Message`); web/src/api/contract.yaml
- Touch: web/src/api/groups.ts, groups.test.ts, contract.yaml; docs/frontend/codebase/README.md
- Needs: B-45, B-46
- Done when: `npx vitest run src/api/groups.test.ts` passes (the caller's command id; messages
  after a sequence).

### F-82 · The stream refreshes a group
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-f-82-the-stream-refreshes-a-group
- Read: web/src/stream/contract.yaml
- Touch: web/src/stream/refresh.ts, refresh.test.tsx, index.ts (exports `holdThreadRefresh`
  for F-84), contract.yaml
- Needs: F-81
- Done when: `npx vitest run src/stream/refresh.test.tsx` passes (a `MessageAdded` refreshes
  its group, its log and the list; replay batches until caught-up).

### F-83 · Groups in the AYLUL sidebar, and the new-group dialog
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-f-83-groups-in-the-aylul-sidebar-and-the-new-group-dialog
- Read: web/src/app/sidebar/contract.yaml
- Touch: web/src/app/sidebar/group-list.tsx, sidebar.tsx, contract.yaml;
  web/src/app/groups/new-group-dialog.tsx, new-group-dialog.test.tsx, contract.yaml, index.ts;
  docs/frontend/codebase/README.md
- Needs: F-81, F-82
- Done when: `npx vitest run src/app/groups/new-group-dialog.test.tsx` passes (the seventh agent
  cannot be picked; creating opens the group and lists it at once; a retry after a lost
  answer reuses the command id).

### F-84 · The group page
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-f-84-the-group-page
- Read: web/src/app/conversation/contract.yaml; web/src/stream/contract.yaml
- Touch: web/src/app/groups/group-page.tsx, group-log.tsx, group-composer.tsx,
  mention-menu.tsx, group-page.test.tsx; web/src/router.tsx
- Widened (recorded): the composer shares the conversation's frame, so
  web/src/app/conversation/composer-surface.tsx, composer.tsx, index.ts and contract.yaml
  changed; the log reuses `ReplyText` and the thinking mark reuses `turn-state` through
  web/src/app/conversation/index.ts.
- Needs: F-81, F-82
- Done when: `npx vitest run src/app/groups/group-page.test.tsx` passes (seq order and
  speaker; a notice is not a bubble; `@` without deleted agents; no jump when scrolled up;
  thinking appears and clears without a `MessageAdded`; a resend reuses the command id).

### F-85 · A member's permission request in the group
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-f-85-a-members-permission-request-in-the-group
- Read: web/src/api/permissions.ts; web/src/app/conversation/contract.yaml
- Touch: web/src/app/conversation/permission-answer.tsx, permission-answer.test.tsx,
  permission-card.tsx, contract.yaml; web/src/app/groups/group-page.tsx, group-page.test.tsx,
  contract.yaml
- Needs: F-84
- Done when: `npx vitest run src/app/conversation/permission-answer.test.tsx
  src/app/conversation/permission-card.test.tsx src/app/groups/group-page.test.tsx` passes,
  with `permission answer answers a waiting request` and `shows the member and request title
  before answering in place`; `permission-card.test.tsx` is unchanged.
