# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

Open: U-5 to U-8, the desktop UI half of the AYLUL agents and panel design
([plan](../../frontend/plans/2026-11-01-aylul-agents-panel-frontend.md)), on
`feature/aylul-agents-panel`. They consume the native IPC of
[`docs/aylul/shared/requests/2026-11-01-companion-panel.md`](../shared/requests/2026-11-01-companion-panel.md), accepted on 2026-11-01.
The panel is UI-owned (design §7.8, Mohammed 2026-11-01): U-7 and U-8 build
`features/companion/panel/` and the click gesture; the character side owns only the drawing.

U-4 (the UI part of the desktop restructure) is in this file at `ca618f89`. U-1 and U-3 (the
character colour per agent, PR #40) are at `1a2b601`, and U-2 is at `c7e7bac4`. The text
input, the shared dashboard, U-W1 to U-W3 and H-2 are at `8580306`.

## Tasks

### U-5 · Read the bound agent's outfit from the companion state
- Status: todo (N-22 completed in `c79475c2`)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#u-5
- Read: `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`appearance.outfit`); `apps/desktop/src/platform/contract.yaml`
- Touch: apps/desktop/src/platform/companion-state.ts, apps/desktop/src/platform/companion-state.test.ts, apps/desktop/src/platform/contract.yaml, docs/aylul/ui/stages.md
- Needs: docs/aylul/shared/requests/2026-11-01-companion-panel.md (accepted); N-22 done
- Done when: From `apps/desktop/`: `node --test src/platform/companion-state.test.ts` passes (outfit copied opaque; absent, empty or over 32 characters reads `default`; colour behaviour unchanged), then `npm run build` and `npm run typecheck:tests` pass.

### U-6 · Panel state and panel actions in the platform layer
- Status: blocked (waits for N-23 and N-24)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#u-6
- Read: `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`panel`, `companion_panel`, `select_companion_agent`, `open_main_from_companion`); `apps/desktop/src/platform/contract.yaml`
- Touch: apps/desktop/src/platform/companion-state.ts, apps/desktop/src/platform/companion-state.test.ts, apps/desktop/src/platform/companion.ts, apps/desktop/src/platform/contract.yaml, docs/aylul/codebase/README.md, docs/aylul/ui/stages.md
- Needs: N-23 and N-24 done
- Done when: From `apps/desktop/`: `node --test src/platform/companion-state.test.ts` passes (panel agents parsed with their bounds; over-long or malformed entries refuse the state; an absent panel reads closed and empty) and `npm run test:companion:unit` passes, then `npm run build` and `npm run typecheck:tests` pass.

### U-7 · The panel: top bar, card, agent grid
- Status: blocked (waits for U-6 and C-9)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#u-7
- Read: design §4, §7.6; `apps/desktop/src/features/companion/contract.yaml`; `docs/aylul/native/specs/2026-10-26-desktop-architecture-design.md` "Companion TypeScript entries"; `apps/desktop/tests/companion/boundaries.test.ts` (how the import ratchet reads)
- Touch: apps/desktop/src/features/companion/panel/CompanionPanel.tsx (new), apps/desktop/src/features/companion/panel/AgentTile.tsx (new), apps/desktop/src/features/companion/panel/panel.css (new), apps/desktop/src/features/companion/DesktopCompanion.tsx, apps/desktop/src/features/companion/desktop-gesture.ts, apps/desktop/src/features/companion/desktop-gesture.test.ts, apps/desktop/src/features/companion/contract.yaml, apps/desktop/tests/companion/fakes.ts, apps/desktop/tests/companion/panel.spec.ts (new), apps/desktop/tests/companion/desktop-window.spec.ts, docs/aylul/codebase/README.md, docs/aylul/ui/stages.md
- Needs: U-6 and C-9 done (ownership ruled: design §7.8)
- Done when: From `apps/desktop/`: `node --test src/features/companion/desktop-gesture.test.ts`, `npm run test:companion:unit`, `npm run test:companion:production` and `npx playwright test tests/companion/panel.spec.ts tests/companion/desktop-window.spec.ts` pass (click and Enter open the panel; Escape closes it; the grid shows every agent in its outfit and colour with the bound one marked; a tile click calls select; nine agents scroll inside the grid; Talk and Chat open the composer; Settings, New agent and Open call `open_main_from_companion` with their targets; Sound toggles the tones), then `npm run build` and `npm run typecheck:tests` pass. A screenshot of the panel is attached to the report.

### U-8 · Status pill and the card's answer to a waiting request
- Status: blocked (waits for N-25 and U-7)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#u-8
- Read: design §7.6 "Status pill" and "The card answers its own agent", §7.8; `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`waiting`, `answer_companion_permission`)
- Touch: apps/desktop/src/platform/companion-state.ts, apps/desktop/src/platform/companion-state.test.ts, apps/desktop/src/platform/companion.ts, apps/desktop/src/platform/contract.yaml, apps/desktop/src/features/companion/panel/CompanionPanel.tsx, apps/desktop/src/features/companion/panel/PermissionPrompt.tsx (new), apps/desktop/src/features/companion/contract.yaml, apps/desktop/tests/companion/fakes.ts, apps/desktop/tests/companion/panel.spec.ts, docs/aylul/codebase/README.md, docs/aylul/ui/stages.md
- Needs: N-25 and U-7 done
- Done when: From `apps/desktop/`: `node --test src/platform/companion-state.test.ts` and `npx playwright test tests/companion/panel.spec.ts` pass (the pill shows "Ready" at zero and the count otherwise; a pill click asks for `oldestWaiting`; the card shows the bound request's title and one button per option, labelled with its `name`, and no text answer; a click answers with a command id, a retry reuses it; `permission_settled`, `permission_not_found` and `permission_option_not_offered` each show their line), then `npm run build` and `npm run typecheck:tests` pass. A screenshot is attached.
