# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

The character colour per agent is delivered by PR #40, merged at `1a2b601`: U-1 and U-3 are
in this file at `1a2b601`. U-2, done before
that PR, is in this file at `c7e7bac4`. The desktop restructure's D-10 enters here as U-4
(UI part); its Character part is C-7 in `../character/stages.md`, and D-11 waits for both.

Finished work (the text input, the shared dashboard, U-W1 to U-W3, H-2) is in this file at
`8580306`.

## Tasks

### U-4 · D-10 (UI part): companion import boundaries are checked and UI unit tests sit with their owners
- Status: done
- Plan: docs/aylul/native/plans/2026-10-25-desktop-architecture.md#d-10--enforce-ts-entry-boundaries-and-place-unit-tests-with-owners
  and section 6 ("TS modules and entries")
- Read: the plan's section 6 ("TS modules and entries", "Ratchet design"); `apps/desktop/src/platform/contract.yaml`;
  `apps/desktop/src/app/contract.yaml`; `apps/desktop/tests/companion/contracts.test.ts` and
  `production.test.ts`; `apps/desktop/package.json` (`test:companion:*`); `apps/desktop/tsconfig*.json`;
  the `apps/desktop/src` rows of `docs/aylul/codebase/README.md`
- Touch: a new boundary test `apps/desktop/tests/companion/boundaries.test.ts` with its exact
  baseline and adversarial fixtures under `apps/desktop/tests/companion/fixtures/`;
  `apps/desktop/src/platform/` (`companion.ts`, `companion-state.ts`, a co-located
  `companion-state.test.ts` moved from `tests/companion/native-state.test.ts`, `contract.yaml`);
  `features/companion/renderer/palette.ts`'s test moved from `tests/companion/palette.test.ts`
  to `features/companion/renderer/palette.test.ts` (palette is this side's by the 2026-10-21
  ruling) and the `tested_by` paths that name it in `features/companion/contract.yaml`;
  `apps/desktop/package.json` scripts for the paths this task moves;
  `apps/desktop/tsconfig*.json` and `vite.config.*` only to keep co-located tests out of the
  production bundle; `tests/companion/production.test.ts`; `docs/aylul/codebase/README.md` (its
  rows); `apps/desktop/src-tauri/tests/architecture/baseline.txt` and `README.md` only to remove
  the four `command_*_wrapper` entries and their reason lines; this task's `Status:`
- Needs: N-19 (the wrapper inventory reads `src/platform/`)
- Done when: `boundaries.test.ts`, run by `test:companion:unit`, parses `apps/desktop/src` with
  the `typescript` package already present and fails on each D-10 rule: a Tauri API import
  outside `platform/`; renderer or animation importing platform or studio code; production
  companion code (from `app/companion.tsx`) importing studio or a test fixture; an HTTP call
  (`fetch`, `XMLHttpRequest`, `EventSource`) in renderer; studio importing renderer or
  animation internals instead of `renderer/index.ts` and `animation/index.ts`; each rule has a
  fixture that fails for that rule; today's violations sit in an exact baseline that only
  shrinks (new and stale entries fail); the unused `setCompanionVisible` and
  `setCompanionPaused` wrappers and their contract lines are gone and the four
  `command_*_wrapper` entries leave the native baseline; the two moved tests pass where they now live and the
  production test proves no `*.test.ts` or fixture reaches the companion bundle; from
  `apps/desktop/`: `npm run build`, `npm run test:companion:unit`,
  `npm run test:companion:production` and, from `src-tauri/`, `cargo test --test architecture` pass

