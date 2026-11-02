# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-62 to F-67 are implemented: the web half of the AYLUL agents and panel design
([plan](plans/2026-11-01-aylul-agents-panel-frontend.md)), on `feature/aylul-agents-panel`.
Runnable next: U-5, N-23 and N-24 on the AYLUL side. Web focused gates and synthetic browser checks passed; real desktop/provider acceptance remains. Shapes: `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md`
§3, accepted in `docs/shared/requests/2026-11-01-aylul-agents-panel.md`. The desktop half is U-5 to U-8 in `docs/aylul/ui/stages.md`, C-8 and C-9 in
`docs/aylul/character/stages.md`, N-22 to N-25 in `docs/aylul/native/stages.md`. Still open outside any task: the light palette and the real desktop
login-item behaviour need their own app and OS acceptance.

Next on this branch: F-69 to F-77, the AI Studio UI migration
([plan](plans/2026-11-02-ai-studio-ui-migration.md); facts in
`docs/evidence/2026-11-02-ai-studio-ui-analysis.md`). Masada's step M-0 (amend the
neon-clarity spec) comes first. These tasks touch no `apps/` file and never touch
`web/src/test/native-presence.json`, so U-8 and N-25 can run before them or alongside them.
F-69 replaces F-68's look. F-68's unsaved indicator and its test stay.

Finished work: F-50 to F-61 (branch `feature/web-polish`, PR #48) are in this file at `ce31036d`;
F-45 to F-49 (W-11, with B-14 to B-18, PR #47) are in this file at `4f23c717`;
F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

### F-68 · Neon clarity across existing shared controls
- Status: done (20 focused tests; typecheck, lint, build, 82 codemap/architecture checks; dark/light/reduced-motion browser review)
- Plan: docs/frontend/plans/2026-11-01-neon-clarity.md
- Read: docs/frontend/specs/2026-11-01-neon-clarity.md; existing feature contracts
- Touch: exactly the files listed in that plan
- Needs: neon direction approved by Mohammed on 2026-11-01
- Done when: focused identity/outfit/permission tests, typecheck, lint, build, codemap and architecture pass; browser review covers dark, light and reduced motion.

### F-62 · Outfit picker drawn from the studio's exported previews
- Status: done (picker tests 3/3; typecheck, lint, build, codemap/architecture passed; browser preview captured)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-62
- Read: design §7.3; `web/src/app/agents/contract.yaml`; `character-colors.ts` and `color-swatches.tsx` (the pattern to follow)
- Touch: web/src/app/agents/outfits.ts (new), web/src/app/agents/outfit-picker.tsx (new), web/src/app/agents/outfit-picker.test.tsx (new), web/src/app/agents/index.ts, web/src/app/agents/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: C-8 done
- Done when: From `web/`: `npx vitest run src/app/agents/outfit-picker.test.tsx` passes (every manifest outfit is a radio with its label and image; an unknown id selects `default`), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot of the picker attached to the report.

### F-63 · Agent form: outfit, description and instructions, in the design's order
- Status: done (focused tests 55/55; typecheck, lint, build, codemap/architecture passed; browser form and save verified with synthetic API responses)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-63
- Read: design §7.1, §7.3, §7.5; `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3.1; `web/src/app/agents/contract.yaml`; `web/src/api/contract.yaml`; `web/src/app/sidebar/contract.yaml`
- Touch: web/src/api/schema.d.ts (only through `npm run gen:api`), web/src/api/aylul-profiles.ts, web/src/api/aylul-profiles.test.ts, web/src/api/contract.yaml, web/src/app/agents/agent-settings.tsx, web/src/app/agents/agent-form.test.tsx (new), web/src/app/agents/contract.yaml, web/src/app/sidebar/agent-list.tsx, web/src/app/sidebar/sidebar.test.tsx, web/src/app/sidebar/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: docs/shared/requests/2026-11-01-aylul-agents-panel.md (accepted); backend §3.1 built and in `api/openapi.json`; F-62 done
- Additional Touch: web/src/test/fake-daemon.ts (shared test daemon supplies the new immutable template route); web/src/app/agents/aylul.test.tsx (existing colour save expects the new fields and waits for template readiness before saving).
- Done when: From `web/`: `npm run gen:api` first, then `npx vitest run src/api/aylul-profiles.test.ts src/app/agents/agent-form.test.tsx src/app/sidebar/sidebar.test.tsx` pass (new agent prefilled from `GET /api/aylul/agent-template`; an existing agent shows `Agent.instructions`, or the template when it is `""`; untouched instructions are not sent; outfit and description saved; Core's 400 `PROFILE_INVALID` message shown; the sidebar shows the description), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot of the form attached.

### F-64 · MCP tools in the library, with secret fields
- Status: done (17 focused tests; typecheck, lint, build and 82 codemap/architecture checks passed)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-64
- Read: design §7.2, §8 (stdio OPEN); `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3.2; `web/src/app/settings/contract.yaml`; `web/src/api/contract.yaml`
- Touch: web/src/api/schema.d.ts (only through `npm run gen:api`), web/src/api/extensions.ts, web/src/api/extensions.test.ts (new), web/src/api/contract.yaml, web/src/app/settings/mcp-form.tsx (new), web/src/app/settings/mcp-form.test.tsx (new), web/src/app/settings/sections/plugins.tsx, web/src/app/settings/extension-list.tsx, web/src/app/settings/plugins.test.tsx, web/src/app/settings/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: docs/shared/requests/2026-11-01-aylul-agents-panel.md (accepted); backend §3.2 built and in `api/openapi.json`
- Done when: From `web/`: `npm run gen:api` first, then `npx vitest run src/api/extensions.test.ts src/app/settings/mcp-form.test.tsx src/app/settings/plugins.test.tsx` pass (add `http` and `stdio` items with `{ name, value }` pairs typed in password fields; values never shown back, a pair with `set` shows "Set"; an edit keeps the id and sends a pair without `value` to keep it; Core's 400/409/503 messages shown; a `bundled` item is marked; delete), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass.

### F-65 · Capabilities in the agent form: skills, plugins and MCP tools
- Status: done (18 focused tests; typecheck, lint, build and 82 codemap/architecture checks passed; browser screenshot captured)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-65
- Read: design §7.5 item 5; `web/src/app/agents/contract.yaml`
- Touch: web/src/api/extensions.ts, web/src/app/agents/agent-extensions.tsx, web/src/app/agents/agent-extensions.test.tsx, web/src/app/agents/agent-settings.tsx, web/src/app/agents/contract.yaml, docs/frontend/stages.md
- Needs: F-63 and F-64 done
- Additional Touch: web/src/api/contract.yaml (records the third switch-cache kind in the same commit as its owner).
- Done when: From `web/`: `npx vitest run src/app/agents/agent-extensions.test.tsx src/app/agents/agent-form.test.tsx` pass (three kinds with switches; an MCP switch saves through the `mcp` kind; "Add to library" opens Settings → Plugins & Skills), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot attached.

### F-66 · Permission card in the conversation
- Status: done (23 focused tests; typecheck, lint, build and 82 codemap/architecture checks passed; browser permission click verified with synthetic API responses)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-66
- Read: design §7.8; `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3.3; `web/src/app/conversation/contract.yaml`; `web/src/stream/contract.yaml`; `web/src/api/contract.yaml`
- Touch: web/src/api/schema.d.ts (only through `npm run gen:api`), web/src/api/permissions.ts (new), web/src/api/permissions.test.ts (new), web/src/api/contract.yaml, web/src/stream/refresh.ts, web/src/stream/refresh.test.tsx, web/src/stream/contract.yaml, web/src/app/conversation/permission-card.tsx (new), web/src/app/conversation/permission-card.test.tsx (new), web/src/app/conversation/entry.tsx, web/src/app/conversation/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: docs/shared/requests/2026-11-01-aylul-agents-panel.md (accepted); backend §3.3 built and in `api/openapi.json`
- Done when: From `web/`: `npm run gen:api` first, then `npx vitest run src/api/permissions.test.ts src/stream/refresh.test.tsx src/app/conversation/permission-card.test.tsx src/app/conversation/conversation.test.tsx` pass (option buttons, labelled with each option's `name`, only while the request is in `GET /api/aylul/permissions`; a click posts `{ command_id, option_id }` to `POST /api/threads/{id}/session/permissions/{request_id}` and a retry reuses the command id; `PERMISSION_NOT_FOUND` and `PERMISSION_SETTLED` refetch the list and show Core's message; there is no typed answer; a `PermissionAnswered` entry shows its body and "by you"; a `PermissionRefused` with a card shows its reason; a `PermissionsChanged` frame on the project stream refetches the list), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot attached.

### F-67 · Main-window destinations the panel opens
- Status: done
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-67
- Read: `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (the `main-navigation` row); `web/src/desktop/contract.yaml`
- Touch: web/src/desktop/navigation.ts, web/src/desktop/native-parity.test.ts, web/src/desktop/contract.yaml, web/src/main.tsx, docs/frontend/stages.md
- Needs: none (must land before N-23)
- Done when: From `web/`: `npx vitest run src/desktop/native-parity.test.ts` passes (`agentSettings` with a valid slug, `newAgent` and `settings` parse; a bad slug and unknown kinds are refused; each maps to its route), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass.

### F-69 · Palette, type and surfaces from the AI Studio design
- Status: done
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-69)
- Read: the export's `src/index.css`; ours `web/src/index.css`; evidence Report D "242984d8" and "Token and class gap"
- Touch: web/src/index.css, web/package.json, web/package-lock.json, web/src/app/conversation/permission-card.tsx, web/src/app/agents/agent-extensions.tsx, web/src/app/agents/outfit-picker.tsx, web/src/app/agents/agent-settings.tsx, web/src/app/agents/contract.yaml, web/src/app/conversation/contract.yaml, docs/frontend/stages.md
- Needs: M-0 done; Mohammed approves `@fontsource/plus-jakarta-sans` and `@fontsource/jetbrains-mono`
- Done when: From `web/`: `npx vitest run src/app/agents/agent-form.test.tsx src/app/agents/outfit-picker.test.tsx src/app/agents/agent-extensions.test.tsx src/app/conversation/permission-card.test.tsx` passes, then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. The home and agent form screenshots, in dark and light, are compared with the export.

### F-70 · Shared primitives: `Panel` and the `Planned` marker
- Status: done
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-70)
- Read: `docs/frontend/codebase/README.md` (the `components/ui` invariant); `web/src/components/ui/button.tsx` (the `cva` pattern); evidence Report C (c) 3
- Touch: web/src/components/ui/panel.tsx (new), web/src/components/ui/planned.tsx (new), web/src/components/ui/planned.test.tsx (new), docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-69 done
- Done when: From `web/`: `npx vitest run src/components/ui/planned.test.tsx` passes (it shows the badge and the reason; a button inside cannot be focused or clicked; the region has an accessible name), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass.

### F-71 · Conversation cards and entries
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-71)
- Read: `web/src/app/conversation/contract.yaml`; the export's files of the same names; evidence Report D "f747f296", Report B item 10 and "Acceptable"
- Touch: web/src/app/conversation/entry.tsx, web/src/app/conversation/messages.tsx, web/src/app/conversation/plan-card.tsx, web/src/app/conversation/plan-side-panel.tsx, web/src/app/conversation/map-proposal-card.tsx, web/src/app/conversation/roadmap-proposal-card.tsx, web/src/app/conversation/vision-proposal-card.tsx, web/src/app/conversation/status-badge.tsx, web/src/app/conversation/subagent-card.tsx, web/src/app/conversation/subagent-panel.tsx, web/src/app/conversation/todo-card.tsx, web/src/app/conversation/tool-diff.tsx, web/src/app/conversation/tool-group.tsx, web/src/app/conversation/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-70 done
- Done when: From `web/`: `npx vitest run src/app/conversation/conversation.test.tsx src/app/conversation/permission-card.test.tsx src/app/conversation/waiting-messages.test.tsx src/app/conversation/todo-card.test.tsx src/app/conversation/tool-group.test.tsx src/app/conversation/tool-diff.test.tsx src/app/conversation/subagent-card.test.tsx src/app/conversation/map-proposal-card.test.tsx src/app/conversation/roadmap-proposal-card.test.tsx src/app/conversation/plan-in-conversation.test.tsx src/stream/refresh.test.tsx` passes, then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshots compared.

### F-72 · Composer, draft and context ring
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-72)
- Read: `web/src/app/conversation/contract.yaml` (Composer, ContextRing); the export's files of the same names; evidence Report B item 14
- Touch: web/src/app/conversation/composer.tsx, web/src/app/conversation/composer-bar.tsx, web/src/app/conversation/draft.tsx, web/src/app/conversation/context-ring.tsx, web/src/app/conversation/context-ring.test.tsx, web/src/app/conversation/contract.yaml, docs/frontend/stages.md
- Needs: F-70 done
- Done when: From `web/`: `npx vitest run src/app/conversation/composer-bar.test.tsx src/app/conversation/composer-focus.test.tsx src/app/conversation/draft.test.tsx src/app/conversation/context-ring.test.tsx` passes (a new case: the popover shows only the figures Core gave), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshots compared.

### F-73 · Sidebar, with its search filter
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-73)
- Read: `web/src/app/sidebar/contract.yaml`; the export's sidebar files; evidence Report B item 12
- Touch: web/src/app/sidebar/sidebar.tsx, web/src/app/sidebar/agent-list.tsx, web/src/app/sidebar/project-list.tsx, web/src/app/sidebar/thread-list.tsx, web/src/app/sidebar/workspace-switcher.tsx, web/src/app/sidebar/sidebar.test.tsx, web/src/app/sidebar/project-list.test.tsx, web/src/app/sidebar/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-70 done
- Done when: From `web/`: `npx vitest run src/app/sidebar/sidebar.test.tsx src/app/sidebar/project-list.test.tsx src/app/sidebar/workflow-list.test.tsx` passes. New cases: the filter narrows projects by name and directory, and agents by name and description; no match shows the empty line; Escape closes the search and clears it. Then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshots compared.

### F-74 · Home pages, design header, settings dialog, status lines
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-74)
- Read: the export's `app/home.tsx`, `app/agents/home.tsx`, `app/design/workspace-page.tsx`, `app/desktop/activity.tsx`, `app/daemon-status.tsx`, `app/settings/settings-dialog.tsx`; their folders' `contract.yaml`; evidence Report A features 4, 5 and 7, and Report B items 5, 6 and 13
- Touch: web/src/app/home.tsx, web/src/app/agents/home.tsx, web/src/app/design/workspace-page.tsx, web/src/app/desktop/activity.tsx, web/src/app/daemon-status.tsx, web/src/app/settings/settings-dialog.tsx, web/src/app/settings/sections/plugins.tsx, web/src/app/settings/extension-list.tsx, web/src/app/settings/mcp-form.tsx, web/src/app/agents/aylul.test.tsx, web/src/app/agents/contract.yaml, web/src/app/settings/contract.yaml, web/src/app/design/contract.yaml, web/src/app/desktop/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-70 done
- Done when: From `web/`: `npx vitest run src/app/agents/aylul.test.tsx src/app/settings/settings-dialog.test.tsx src/app/settings/plugins.test.tsx src/app/settings/mcp-form.test.tsx src/app/desktop/native-sections.test.tsx` passes (a new case: the agents home lists each agent with its provider and model; the planned blocks are marked and inert), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshots compared.

### F-75 · Plan map: staggered columns, nodes, edges, page, inspect panel
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-75)
- Read: `web/src/app/workflows/contract.yaml` (the "box dagre spaced" trap); the export's `layout.ts`, `task-node.tsx`, `plan-edge.tsx`, `plan-page.tsx`, `inspect-panel.tsx`; evidence Report A features 1 and 2, Report B items 2, 3, 7 and 11, and "Masada's own view"
- Touch: web/src/app/workflows/layout.ts, web/src/app/workflows/layout.test.ts, web/src/app/workflows/task-node.tsx, web/src/app/workflows/plan-edge.tsx, web/src/app/workflows/plan-page.tsx, web/src/app/workflows/plan-page.test.tsx, web/src/app/workflows/inspect-panel.tsx, web/src/app/workflows/contract.yaml, docs/frontend/stages.md
- Needs: F-70 done
- Done when: From `web/`: `npx vitest run src/app/workflows/layout.test.ts src/app/workflows/plan-page.test.tsx src/app/workflows/plan-graph.test.tsx src/app/workflows/plan-map.test.tsx src/app/workflows/plan-frames.test.tsx` passes. New cases: single-task columns alternate above and below; a two-task column and the start do not move; no boxes overlap; every edge ends on its drawn box; the progress block is marked planned, shows the real count and no percent. Then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. A plan of five or more tasks is screenshot in both themes and compared.

### F-76 · Agent Mode: the execution board beside the chat
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-76)
- Read: `web/src/app/conversation/contract.yaml` (Messages); `activity-rows.ts`, `usage.ts`, `turn-state.ts`; the export's `conversation.tsx` lines 100-1037; evidence Report A feature 3, Report B item 4, and the tsc errors in Report C
- Touch: web/src/app/conversation/conversation.tsx, web/src/app/conversation/agent-board.tsx (new), web/src/app/conversation/agent-console.tsx (new), web/src/app/conversation/agent-board.test.tsx (new), web/src/app/conversation/activity-rows.ts, web/src/app/conversation/activity-rows.test.ts, web/src/app/conversation/messages.tsx, web/src/app/conversation/index.ts, web/src/app/conversation/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-71 and F-72 done
- Done when: From `web/`: `npx vitest run src/app/conversation/agent-board.test.tsx src/app/conversation/activity-rows.test.ts src/app/conversation/conversation.test.tsx` passes. Cases: the switch shows the board and switches back; the board lists the real steps and todos; with no steps it shows an empty state and no percent; the stepper is marked planned and inert; Messages draws the same rows as before. Then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. A running turn and a finished turn are screenshot in Agent Mode, in both themes, and compared.

### F-77 · The whole web client, once, after the migration
- Status: todo
- Plan: docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md (F-77)
- Read: the plan; the rules in `docs/frontend/AGENTS.md`
- Touch: docs/frontend/stages.md (status only; any fix goes to frontend-fixer as its own task)
- Needs: F-69 to F-76 done
- Done when: From `web/`: `npm test`, `npm run typecheck`, `npm run lint` and `npm run build` pass. A grep of the changed `.tsx` files for hex values, `rgba`, raw palette classes and `text-[1` finds nothing. Screenshots are attached for home, the sidebar, the conversation (both modes), the plan page, the agents home and settings, each in both themes.
