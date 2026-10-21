# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

The character colour per agent is delivered by PR #40, merged at `1a2b601`: U-1 and U-3 are
in this file at `1a2b601`. U-2 was done before PR #40 and stays below. No UI task is open.

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
