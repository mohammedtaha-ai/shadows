# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Open: F-62 to F-67, the web half of the AYLUL agents and panel design
([plan](plans/2026-11-01-aylul-agents-panel-frontend.md)), on `feature/aylul-agents-panel`.
Runnable now: F-67. The desktop half is U-5 to U-8 in `docs/aylul/ui/stages.md`, C-8 and C-9 in
`docs/aylul/character/stages.md`, N-22 to N-25 in `docs/aylul/native/stages.md`. Still open outside any task: the light palette and the real desktop
login-item behaviour need their own app and OS acceptance.

Finished work: F-50 to F-61 (branch `feature/web-polish`, PR #48) are in this file at `ce31036d`;
F-45 to F-49 (W-11, with B-14 to B-18, PR #47) are in this file at `4f23c717`;
F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

### F-62 · Outfit picker drawn from the studio's exported previews
- Status: blocked (waits for C-8, which commits `web/src/assets/outfits/`)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-62
- Read: design §7.3; `web/src/app/agents/contract.yaml`; `character-colors.ts` and `color-swatches.tsx` (the pattern to follow)
- Touch: web/src/app/agents/outfits.ts (new), web/src/app/agents/outfit-picker.tsx (new), web/src/app/agents/outfit-picker.test.tsx (new), web/src/app/agents/index.ts, web/src/app/agents/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: C-8 done
- Done when: From `web/`: `npx vitest run src/app/agents/outfit-picker.test.tsx` passes (every manifest outfit is a radio with its label and image; an unknown id selects `default`), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot of the picker attached to the report.

### F-63 · Agent form: outfit, description and instructions, in the design's order
- Status: blocked (backend shape A not built)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-63
- Read: design §7.1, §7.3, §7.5; plan "Shapes we assume" A; `web/src/app/agents/contract.yaml`; `web/src/api/contract.yaml`; `web/src/app/sidebar/contract.yaml`
- Touch: web/src/api/schema.d.ts (only through `npm run gen:api`), web/src/api/aylul-profiles.ts, web/src/api/aylul-profiles.test.ts, web/src/api/contract.yaml, web/src/app/agents/agent-settings.tsx, web/src/app/agents/agent-form.test.tsx (new), web/src/app/agents/contract.yaml, web/src/app/sidebar/agent-list.tsx, web/src/app/sidebar/sidebar.test.tsx, web/src/app/sidebar/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: backend shape A in `api/openapi.json`; F-62 done
- Done when: From `web/`: `npm run gen:api` first, then `npx vitest run src/api/aylul-profiles.test.ts src/app/agents/agent-form.test.tsx src/app/sidebar/sidebar.test.tsx` pass (new agent prefilled with the template; existing agent loads its instructions; untouched instructions are not sent; outfit and description saved; Core's 422 shown; the sidebar shows the description), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot of the form attached.

### F-64 · MCP tools in the library, with secret fields
- Status: blocked (backend shape B not built)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-64
- Read: design §7.2, §8 (stdio OPEN); plan "Shapes we assume" B; `web/src/app/settings/contract.yaml`; `web/src/api/contract.yaml`
- Touch: web/src/api/schema.d.ts (only through `npm run gen:api`), web/src/api/extensions.ts, web/src/api/extensions.test.ts (new), web/src/api/contract.yaml, web/src/app/settings/mcp-form.tsx (new), web/src/app/settings/mcp-form.test.tsx (new), web/src/app/settings/sections/plugins.tsx, web/src/app/settings/extension-list.tsx, web/src/app/settings/plugins.test.tsx, web/src/app/settings/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: backend shape B in `api/openapi.json`
- Done when: From `web/`: `npm run gen:api` first, then `npx vitest run src/api/extensions.test.ts src/app/settings/mcp-form.test.tsx src/app/settings/plugins.test.tsx` pass (add http and stdio items; values typed in password fields and never shown back; edit keeps the id and sends only changed values; Core's 422 shown; delete), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass.

### F-65 · Capabilities in the agent form: skills, plugins and MCP tools
- Status: blocked (waits for F-63 and F-64)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-65
- Read: design §7.5 item 5; `web/src/app/agents/contract.yaml`
- Touch: web/src/api/extensions.ts, web/src/app/agents/agent-extensions.tsx, web/src/app/agents/agent-extensions.test.tsx, web/src/app/agents/agent-settings.tsx, web/src/app/agents/contract.yaml, docs/frontend/stages.md
- Needs: F-63 and F-64 done
- Done when: From `web/`: `npx vitest run src/app/agents/agent-extensions.test.tsx src/app/agents/agent-form.test.tsx` pass (three kinds with switches; an MCP switch saves through the `mcp` kind; "Add to library" opens Settings → Plugins & Skills), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot attached.

### F-66 · Permission card in the conversation
- Status: blocked (backend shape C not built)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-66
- Read: design §7.8; plan "Shapes we assume" C; `web/src/app/conversation/contract.yaml`; `web/src/stream/contract.yaml`; `web/src/api/contract.yaml`
- Touch: web/src/api/schema.d.ts (only through `npm run gen:api`), web/src/api/permissions.ts (new), web/src/api/permissions.test.ts (new), web/src/api/contract.yaml, web/src/stream/refresh.ts, web/src/stream/refresh.test.tsx, web/src/stream/contract.yaml, web/src/app/conversation/permission-card.tsx (new), web/src/app/conversation/permission-card.test.tsx (new), web/src/app/conversation/entry.tsx, web/src/app/conversation/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: backend shape C in `api/openapi.json`
- Done when: From `web/`: `npm run gen:api` first, then `npx vitest run src/api/permissions.test.ts src/stream/refresh.test.tsx src/app/conversation/permission-card.test.tsx src/app/conversation/conversation.test.tsx` pass (buttons only while the request is in the waiting list; a click answers with a command id and a retry reuses it; 404/409 refresh and show Core's message; an answered entry shows the label and "by you"; `PermissionsChanged` refreshes the list), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass. Screenshot attached.

### F-67 · Main-window destinations the panel opens
- Status: todo
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#f-67
- Read: `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (the `main-navigation` row); `web/src/desktop/contract.yaml`
- Touch: web/src/desktop/navigation.ts, web/src/desktop/native-parity.test.ts, web/src/desktop/contract.yaml, web/src/main.tsx, docs/frontend/stages.md
- Needs: none (must land before N-23)
- Done when: From `web/`: `npx vitest run src/desktop/native-parity.test.ts` passes (`agentSettings` with a valid slug, `newAgent` and `settings` parse; a bad slug and unknown kinds are refused; each maps to its route), then `npm run typecheck`, `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass.
