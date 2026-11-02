# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-62 to F-76 and F-68 shipped in PR #49 (`b40e077b`), on `main`: the web half of the AYLUL
agents and panel design, and the AI Studio UI migration that replaced F-68's look. Their
entries are in this file at `b40e077b`. The AYLUL panel plan (`docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md` at `b40e077b`)
and the neon plan (`docs/frontend/plans/2026-11-01-neon-clarity.md` at `b40e077b`) are deleted.
The AI Studio plan, `docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md`, stays until F-77 is closed.
The migration's facts are in `docs/evidence/2026-11-02-ai-studio-ui-analysis.md` at `f1459dcc`.

F-78 and F-79 (the agent form starts empty; the background tasks panel) shipped in PR #51
(`2790473b`); their entries are in this file at `16da6b8c`.

Open: F-77, the whole web client check after the migration. Its Done when still asks for
screenshots of home, the sidebar, the conversation in both modes, the plan page, the agents
home and settings, each in both themes.

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

### F-77 · The whole web client, once, after the migration
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-77)
- Read: the plan; the rules in `docs/frontend/AGENTS.md`
- Touch: docs/frontend/stages.md (status only; any fix goes to frontend-fixer as its own task)
- Needs: F-69 to F-76 done, in this file at `b40e077b`
- Done when: From `web/`: `npm test`, `npm run typecheck`, `npm run lint` and `npm run build` pass. A grep of the changed `.tsx` files for hex values, `rgba`, raw palette classes and `text-[1` finds nothing. Screenshots are attached for home, the sidebar, the conversation (both modes), the plan page, the agents home and settings, each in both themes.
