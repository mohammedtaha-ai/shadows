# Character (AYLUL) stages

Execution: native, user-approved 2026-10-16. No worktrees.

Finished tasks C-1 to C-5 and the 2026-10-20 dashboard-companion notes are in this file at `8580306`.

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

### C-7 · D-10 (Character part): companion modules have entries and their unit tests sit with them
- Status: todo
- Plan: docs/aylul/native/plans/2026-10-25-desktop-architecture.md#d-10--enforce-ts-entry-boundaries-and-place-unit-tests-with-owners
  and section 6 ("TS modules and entries")
- Read: the plan's section 6 ("TS modules and entries"); `apps/desktop/src/features/companion/contract.yaml`
  and `studio/contract.yaml`; `apps/desktop/tests/companion/boundaries.test.ts` and its baseline
  (from U-4); `apps/desktop/package.json` (`test:companion:unit`); the companion rows of
  `docs/aylul/codebase/README.md`
- Touch: `apps/desktop/src/features/companion/` (new `renderer/index.ts` and `animation/index.ts`
  entries exporting only what studio and the production feature use; `studio/` imports switched
  to them; the tests moved next to their owners: `tests/companion/geometry.test.ts`,
  `messages.test.ts`, `model.test.ts`, `scheduler.test.ts` into `animation/`,
  `composer.test.ts` beside `composer-model.ts`, `desktop-gesture.test.ts` beside
  `desktop-gesture.ts`, and `tests/companion/fakes.ts` beside the tests that use it, named as
  test support; `contract.yaml`, `studio/contract.yaml`); `apps/desktop/tests/companion/` (the
  boundary baseline entries this task removes); `apps/desktop/package.json`
  (`test:companion:unit` paths this task moves); `docs/aylul/codebase/README.md` (its rows);
  this task's `Status:`
- Needs: U-4
- Done when: studio reaches renderer and animation only through their entries; no file outside
  `features/companion/` imports a renderer or animation internal; the six moved unit tests pass
  where they now live and `test:companion:unit` runs them; no shape, motion, expression or
  colour changes (the browser renderer and studio specs pass unchanged); every boundary
  baseline entry in `features/companion/` that the entries fix is gone and none is added; from
  `apps/desktop/`: `npm run build`, `npm run test:companion:unit`,
  `npm run test:companion:production` and `npx playwright test tests/companion/renderer.spec.ts
  tests/companion/studio.spec.ts` pass
