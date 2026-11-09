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

### F-96 · Refresh conversation order after an assistant reply

- Status: done (automated checks; Mohammed owns manual browser acceptance)
- Plan: Direct user request, 2026-11-07; owner spec §13.11 in `docs/core/specs/2026-09-25-planner-workflow-design.md`.
- Read: `web/src/stream/contract.yaml`, `web/src/app/sidebar/contract.yaml`.
- Touch: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/stream/contract.yaml`, `web/src/app/sidebar/sidebar.test.tsx`, `web/src/app/sidebar/contract.yaml`, `web/src/api/threads.ts`, `web/src/api/contract.yaml`, this stages file.
- Needs: B-83; `docs/shared/requests/2026-11-07-conversation-reply-order.md` (accepted).
- Done when: affected sidebar/stream tests, typecheck, lint, build, codemap and architecture checks pass. Manual browser acceptance belongs to Mohammed.
- Verified: 164 tests in sidebar, stream, codemap and architecture; typecheck, lint and production build. Vite retains its existing chunk-size warning.

### F-98 · The task page

- Status: done
- Plan: [F-98](../backend/plans/2026-11-08-executors.md#f-98-the-task-page); design [executors](../backend/specs/2026-11-08-executors-design.md) §4 (mockup A, tabs).
- Touch: the plan's F-98 file list.
- Needs: B-93's routes and generated types; `docs/shared/requests/2026-11-08-task-run-page.md` (accepted).
- Done when: the plan's F-98 tests, typecheck, lint, build, codemap and architecture pass.
