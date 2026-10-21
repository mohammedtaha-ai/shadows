# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

2026-10-21: the character colour per agent (web F-9). By Mohammed's ruling the colour table and
the renderer's palette are this side's: U-2 (table) first, since F-9 needs it; then U-1 and U-3.

Finished work (the text input, the shared dashboard, U-W1 to U-W3, H-2) is in this file at
`8580306`.

## Tasks

### U-2 · One colour table for AYLUL's character, and the palette function
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-21-character-colour.md#task-u-2-the-colour-table-and-the-palette-function
- Read: `apps/desktop/src/features/companion/contract.yaml`; `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Character colour)
- Touch: the Files list of Task U-2 in the plan, `docs/aylul/ui/stages.md`
- Needs: none
- Done when: `node --test tests/companion/palette.test.ts` and `npm run build` pass from `apps/desktop/`

### U-1 · The companion state carries the character colour
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-21-character-colour.md#task-u-1-the-companion-state-carries-the-character-colour
- Read: `apps/desktop/src/platform/contract.yaml`; `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Character colour)
- Touch: `apps/desktop/src/platform/companion-state.ts`, `apps/desktop/src/platform/contract.yaml`,
  `apps/desktop/tests/companion/native-state.test.ts`, `docs/aylul/ui/stages.md`
- Needs: accepted `docs/aylul/shared/requests/2026-10-21-settings-dialog-native.md`;
  N-6 at `0c4d8c6` supplies optional `appearance.color` (built)
- Done when: `node --test tests/companion/native-state.test.ts` and `npm run build` pass from `apps/desktop/`

### U-3 · The renderer draws in the palette; purple is unchanged
- Status: todo
- Plan: docs/aylul/ui/plans/2026-10-21-character-colour.md#task-u-3-the-renderer-draws-in-the-palette-purple-is-unchanged
- Read: `apps/desktop/src/features/companion/contract.yaml`; the U-2 table in the plan
- Touch: the Files list of Task U-3 in the plan, `docs/aylul/ui/stages.md`
- Needs: U-2 and U-1 (it reads `state.color`)
- Done when: `npx playwright test tests/companion/renderer.spec.ts tests/companion/desktop-window.spec.ts`,
  `npm run test:companion:unit` and `npm run build` pass from `apps/desktop/`, purple frames equal
  the recorded hashes, and the report holds the stills of every colour

