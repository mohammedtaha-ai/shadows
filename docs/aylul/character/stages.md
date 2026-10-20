# Character (AYLUL) stages

Execution: native, user-approved 2026-10-16. No worktrees.

### C-1 · Extract the renderer with a real Canvas review fixture
- Status: done
- Plan: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md#task-1
- Read: approved companion spec, code map, and this task in the plan; existing contracts before source edits.
- Touch: apps/desktop/src/features/companion/types.ts, apps/desktop/src/features/companion/catalog.ts, apps/desktop/src/features/companion/renderer/character.ts, apps/desktop/src/features/companion/renderer/eyes.ts, apps/desktop/src/features/companion/renderer/crystals.ts, apps/desktop/src/features/companion/renderer/hands.ts, apps/desktop/src/features/companion/renderer/outfits.ts, apps/desktop/src/features/companion/renderer/effects.ts, apps/desktop/src/features/companion/renderer/scene.ts, apps/desktop/src/features/companion/contract.yaml, apps/desktop/tests/companion/fixture.html, apps/desktop/tests/companion/fixture.tsx, apps/desktop/tests/companion/fakes.ts, apps/desktop/tests/companion/renderer.spec.ts, apps/desktop/tests/companion/contracts.test.ts, apps/desktop/playwright.config.ts, apps/desktop/package.json, apps/desktop/package-lock.json, apps/desktop/tsconfig.json, .gitignore, docs/aylul/codebase/README.md, docs/aylul/character/stages.md
- Needs: none
- Done when: task-specific commands and acceptance in the linked plan have observed results; record missing native evidence explicitly.

### C-2 · Add deterministic motion, geometry, scheduling and message expiry
- Status: done
- Plan: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md#task-2
- Read: approved companion spec, code map, and this task in the plan; existing contracts before source edits.
- Touch: apps/desktop/src/features/companion/animation/model.ts, apps/desktop/src/features/companion/animation/geometry.ts, apps/desktop/src/features/companion/animation/scheduler.ts, apps/desktop/src/features/companion/animation/messages.ts, apps/desktop/tests/companion/model.test.ts, apps/desktop/tests/companion/geometry.test.ts, apps/desktop/tests/companion/scheduler.test.ts, apps/desktop/tests/companion/messages.test.ts, apps/desktop/src/features/companion/contract.yaml, apps/desktop/tests/companion/fakes.ts, docs/aylul/character/stages.md
- Needs: Task 1 interfaces
- Done when: task-specific commands and acceptance in the linked plan have observed results; record missing native evidence explicitly.

### C-3 · Build an accessible, lifecycle-safe React presence
- Status: done
- Plan: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md#task-3
- Read: approved companion spec, code map, and this task in the plan; existing contracts before source edits.
- Touch: apps/desktop/src/features/companion/Companion.tsx, apps/desktop/src/features/companion/CompanionCanvas.tsx, apps/desktop/src/features/companion/SpeechBubble.tsx, apps/desktop/src/features/companion/audio.ts, apps/desktop/src/features/companion/companion.css, apps/desktop/tests/companion/presence.spec.ts, apps/desktop/tests/companion/fixture.tsx, apps/desktop/src/features/companion/contract.yaml, docs/aylul/character/stages.md
- Needs: Task 2 interfaces
- Done when: task-specific commands and acceptance in the linked plan have observed results; record missing native evidence explicitly.

### C-4 · Rebuild the development studio on the shared renderer
- Status: done
- Plan: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md#task-4
- Read: approved companion spec, code map, and this task in the plan; existing contracts before source edits.
- Touch: apps/desktop/src/features/companion/studio/CompanionStudio.tsx, apps/desktop/src/features/companion/studio/SceneControls.tsx, apps/desktop/src/features/companion/studio/PreviewGrid.tsx, apps/desktop/src/features/companion/studio/MockWindow.tsx, apps/desktop/src/features/companion/studio/simulation.ts, apps/desktop/src/features/companion/studio/export.ts, apps/desktop/src/features/companion/studio/studio.css, apps/desktop/src/features/companion/studio/contract.yaml, apps/desktop/tests/companion/studio.spec.ts, apps/desktop/tests/companion/fixture.tsx, apps/desktop/src/features/companion/contract.yaml, docs/aylul/codebase/README.md, docs/aylul/character/stages.md
- Needs: Task 3 interfaces
- Done when: task-specific commands and acceptance in the linked plan have observed results; record missing native evidence explicitly.

### C-5 · Agree the boundary and embed the feature in the existing shell
- Status: done
- Plan: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md#task-5
- Read: approved companion spec, code map, and this task in the plan; existing contracts before source edits.
- Touch: docs/aylul/shared/requests/2026-10-16-companion-shell-embedding.md, apps/desktop/tests/companion/shell.spec.ts, apps/desktop/tests/companion/production.test.ts, apps/desktop/src/app/main.tsx, apps/desktop/src/app/contract.yaml, apps/desktop/tests/companion/fixture.tsx, apps/desktop/package.json, docs/aylul/character/stages.md
- Needs: accepted companion-shell shared agreement before shell edits
- Done when: task-specific commands and acceptance in the linked plan have observed results; record missing native evidence explicitly.

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

## Shared dashboard companion · 2026-10-20

Tasks 6–8 of the desktop plan (`docs/aylul/plans/2026-10-20-aylul-desktop-web-companion.md` at
`0187365`)
implement a changeable selected agent, explicit conversation targeting, native
background presence and bounded plain reply previews. Ordinary Send/Queue keeps
its existing command path. Unit and all 41 browser checks passed; the 240px
Arabic preview was visually inspected. Actual-profile readiness now finds the
existing agent ay and provider m. The provider/native journey is reserved for
Mohammed; he waived the final reviewer. Earlier physical DPI/performance gaps
above remain open.
