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
The plan is `docs/backend/plans/2026-11-05-structure-and-depth.md`.

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

Slice 3 is implemented locally as F-92 with B-71 to B-75. The 425 focused web
tests, typecheck, lint and build passed. Mohammed will perform manual
acceptance on the existing project; it is pending. The branch is not merged.

Two composer fixes on this branch have no `F-` task: `1b85b6e6` (the textarea grows to
192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

Structure and planning depth: slices 1 and 2 are on `main`; slice 1 needs no web task. The
[workspace behavior](../backend/specs/2026-11-05-structure-and-depth-design.md#6-what-the-person-sees)
and [slice prerequisites](../backend/plans/2026-11-05-structure-and-depth.md#dispatch-and-progress)
are the review targets. No new `F-` task is executable until its shared
request is accepted and its backend response exists. Before dispatch,
expand the API hooks, routing, event refresh, contracts and code-map paths.
Use Roadmap for outcome coverage and Project map for ownership. The old
mockup's `Built` label cannot be used as evidence of completion.

### F-92 · The map follows the stack, with no Structure step
- Status: done
- Base: `2003992a`
- Plan: `docs/backend/plans/2026-11-05-structure-and-depth.md#task-35-web-f--the-form-the-standards-page-and-the-stage-line`
- Owner: web Design views and the design API client; conversation owns its map-card fixture.
- Read: frontend code map; design/API/conversation contracts; structure spec §6.1; accepted principles-not-layout request.
- Touch: `web/src/app/design/part-editor.tsx`, `map-part-details.tsx`, `parts-view.tsx`, `map-canvas.tsx`, `create-structure.tsx`, `create-structure.test.tsx`, `parts-view.test.tsx`, `standards-view.test.tsx`, `contract.yaml`; `web/src/api/design.ts`, `contract.yaml`; `web/src/test/fake-daemon.ts`; `web/src/app/conversation/map-proposal-card.test.tsx`, `contract.yaml`; `docs/frontend/codebase/README.md`; this stages file.
- Needs: B-71 through B-74 and regenerated API for accepted `2026-11-06-principles-not-layout.md`.
- Input/output: no structure controls, writer call, counts, starter-file field or inspection; layer hint follows the stack; S10 renders from Core. StageChip already renders Core's selected stage without a static step list, so no additional stage machine is introduced.
- Failure cases: old writer survives at Roadmap; new part request still sends files; map approval controls disappear when nested kinds are missing; stale fake schema.
- Done when: typecheck, lint, production build; focused Design/conversation/API tests plus frontend codemap and architecture. Slice live acceptance is tracked separately.


### F-93 · Task phases view
- Status: done
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 5.4.
- Read: structure spec §6.2; workflows, routes and API contracts; frontend code map.
- Touch: exact Task 5.4 Files, workflows contract and frontend code map, this stages file; route search validation in `web/src/router.tsx`.
- Needs: B-80 and accepted request `2026-11-07-task-phases`.
- Verified: workflows/codemap/architecture 127 passed; 7 focused task-view cases include the added loading/error checks. Typecheck, lint and build passed. Exact foreign navigation, Back/Escape focus, empty/missing states and stream refresh are covered. Browser acceptance is recorded after both UI tasks.
- Done when: focused workflows/codemap/architecture tests, typecheck, lint, build; browser checks reported separately.

### F-94 · Expand phases in the plan graph
- Status: done
- Plan: docs/backend/plans/2026-11-05-structure-and-depth.md, Task 5.5.
- Read: structure spec §6.2; workflows contract; frontend code map.
- Touch: exact Task 5.5 Files, workflows contract and frontend code map, this stages file.
- Needs: F-93.
- Verified: 137 focused workflows/codemap/architecture tests passed, including 100 tasks/400 phases, fixed-node fit readiness, mixed interleaving and viewport preservation. Typecheck, lint and build passed. Actual daemon/browser checks on an isolated SQLite backup passed at 1024px and 375px, light/dark, exact foreign navigation/Back, keyboard and empty-task state; no page errors or React Flow warnings. Loading/read errors have component coverage. Daily-project/provider acceptance is not recorded; details are in Task 5.5.
- Done when: focused workflows/codemap/architecture tests, typecheck, lint, build; 100-task/400-phase layout and browser acceptance.
