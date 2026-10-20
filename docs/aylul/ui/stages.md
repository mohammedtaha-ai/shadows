# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

Companion text input: all five tasks of the
text-input plan (`docs/aylul/plans/2026-10-20-aylul-companion-text-input.md` at `0187365`) are
implemented.
Ready opens the compact editor; native keyboard/Escape, same-thread real-agent
reply with hidden main, edge fitting and regression gates passed. Shell/tray
cleanup details and remaining hardware limits are in the
dated evidence (`docs/evidence/aylul/2026-10-20-companion-text-input.md` at `0187365`).

2026-10-20: the desktop main now packages the shared Shadows `web/` dashboard.
Native account, activity and companion settings live in that main. The retired
U-W1–U-W4 main UI and its conversation-source selector have been removed.
Tasks 1–8 of the desktop plan (`docs/aylul/plans/2026-10-20-aylul-desktop-web-companion.md` at
`0187365`)
are committed. Task 9 prepared the actual program on its existing database.
Full automated gates and actual readiness passed. Mohammed performs the real
assistant/native acceptance; he explicitly waived the final reviewer.
See evidence (`docs/evidence/aylul/2026-10-20-desktop-web-companion.md` at `0187365`).

2026-10-21: the character colour per agent (web F-9). By Mohammed's ruling the colour table and
the renderer's palette are this side's: U-2 (table) first, since F-9 needs it; then U-1 and U-3.

## Tasks

### U-2 · One colour table for AYLUL's character, and the palette function
- Status: todo
- Plan: docs/aylul/ui/plans/2026-10-21-character-colour.md#task-u-2-the-colour-table-and-the-palette-function
- Read: `apps/desktop/src/features/companion/contract.yaml`; `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Character colour)
- Touch: the Files list of Task U-2 in the plan, `docs/aylul/ui/stages.md`
- Needs: none
- Done when: `node --test tests/companion/palette.test.ts` and `npm run build` pass from `apps/desktop/`

### U-1 · The companion state carries the character colour
- Status: blocked
- Plan: docs/aylul/ui/plans/2026-10-21-character-colour.md#task-u-1-the-companion-state-carries-the-character-colour
- Read: `apps/desktop/src/platform/contract.yaml`; `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Character colour)
- Touch: `apps/desktop/src/platform/companion-state.ts`, `apps/desktop/src/platform/contract.yaml`,
  `apps/desktop/tests/companion/native-state.test.ts`, `docs/aylul/ui/stages.md`
- Needs: the native side's answer naming the `appearance` field of `companion-state` (AYLUL shared
  request, not yet filed); it may be built before the native side, since a missing field draws purple
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

## Done

| Task | Result |
| --- | --- |
| U-B1 | Workspace design and mockup approved (now superseded) |
| U-W1, U-W1b, U-W1c | Workspace shell, mockup look, icons |
| U-W2 | Live conversations shared with the sidebar |
| U-W3 | Settings, bots list, assistant inspector |
| H-2 | One account-backend conversation; Mohammed got a reply in the desktop window on 2026-10-19 (evidence: `docs/evidence/aylul/2026-10-19-normal-chat.md` at `0187365`) |

U-W4 (batch gate and hands-on journey) is stopped: its automated gate passed
(evidence: `docs/evidence/aylul/2026-10-19-agent-workspace.md` at `0187365`); the journey
will not run on a UI that is being replaced.
