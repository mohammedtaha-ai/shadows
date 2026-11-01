# Character (AYLUL) stages

Execution: native, user-approved 2026-10-16. No worktrees.

C-7 (the Character part of the desktop restructure) is in this file at `ca618f89`. Finished tasks C-1 to C-5 and the 2026-10-20 dashboard-companion notes are at `8580306`.

### C-6 · Close visual and Windows acceptance with truthful evidence
- Status: blocked
- Plan: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md#task-6
- Read: approved companion spec, code map, and this task in the plan; existing contracts before source edits.
- Touch: apps/desktop/tests/companion/visual.spec.ts, docs/evidence/nylra/2026-10-16-aylul-in-window-acceptance.md at `0187365`, apps/desktop/tests/windows-shell.ps1, apps/desktop/README.md, assets/aylul/README.md, docs/aylul/codebase/README.md, apps/desktop/src/features/companion/contract.yaml, apps/desktop/src/features/companion/studio/contract.yaml, apps/desktop/src/app/contract.yaml, docs/aylul/character/stages.md, docs/aylul/character/stages.md, companion feature/test files from Tasks 1–5 (formatting and still-direction regression), apps/desktop/playwright.config.ts
- Needs: Task 5 interfaces
- Done when: task-specific commands and acceptance in the linked plan have observed results; record missing native evidence explicitly.

C-6 implementation and automated/native-shell checks passed; physical WebView
suspension/performance, real DPI transitions, native studio download/audio and user visual
acceptance remain open. See docs/evidence/nylra/2026-10-16-aylul-in-window-acceptance.md at
`0187365`.

### C-8 · Studio export of outfit previews for the web form, and the manifest test
- Status: done
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#c-8
- Read: design §7.3; `apps/desktop/src/features/companion/studio/contract.yaml`; `apps/desktop/tests/companion/renderer.spec.ts` test "export colour review stills" (the opt-in precedent)
- Touch: apps/desktop/src/features/companion/studio/export.ts, apps/desktop/src/features/companion/studio/contract.yaml, apps/desktop/tests/companion/outfit-export.spec.ts (new), apps/desktop/tests/companion/outfit-manifest.test.ts (new), apps/desktop/package.json (adds the manifest test to `test:companion:unit`), web/src/assets/outfits/manifest.json (new, generated), web/src/assets/outfits/*.png (new, generated, one per outfit), assets/aylul/README.md (preview visual review), docs/aylul/codebase/README.md, docs/aylul/character/stages.md
- Needs: none
- Done when: from `apps/desktop/`, `EXPORT_OUTFITS=1 npx playwright test tests/companion/outfit-export.spec.ts` writes ten 160 px PNGs and the manifest; `node --test tests/companion/outfit-manifest.test.ts` passes, and fails when an id, a label, the order or an image differs from `OUTFITS` (shown by a fixture case); `npx playwright test tests/companion/studio.spec.ts`, `npm run test:companion:unit`, `npm run build` and `npm run typecheck:tests` pass. The ten images are attached to the report.

### C-9 · The character wears the bound agent's outfit
- Status: blocked (waits for U-5)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#c-9
- Read: design §7.7; `apps/desktop/src/features/companion/contract.yaml`
- Touch: apps/desktop/src/features/companion/DesktopCompanion.tsx, apps/desktop/src/features/companion/Companion.tsx, apps/desktop/src/features/companion/catalog.ts, apps/desktop/src/features/companion/contract.yaml, apps/desktop/tests/companion/desktop-window.spec.ts, docs/aylul/character/stages.md
- Needs: U-5 done
- Done when: from `apps/desktop/`, `npx playwright test tests/companion/desktop-window.spec.ts tests/companion/renderer.spec.ts` passes with a new test "the companion window draws the state's outfit" (a known outfit draws it; an unknown id draws `default`; the purple frames are unchanged), and `npm run test:companion:unit`, `npm run test:companion:production`, `npm run build` and `npm run typecheck:tests` pass.
