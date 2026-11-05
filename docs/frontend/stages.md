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

Structure and planning depth: slice 1 is on `main` and needs no web task. The
[workspace behavior](../backend/specs/2026-11-05-structure-and-depth-design.md#6-what-the-person-sees)
and [slice prerequisites](../backend/plans/2026-11-05-structure-and-depth.md#dispatch-and-progress)
are the review targets. No new `F-` task is executable until its shared
request is accepted and its backend response exists. Before dispatch,
expand the API hooks, routing, event refresh, contracts and code-map paths.
Use Roadmap for outcome coverage and Project map for ownership. The old
mockup's `Built` label cannot be used as evidence of completion.
