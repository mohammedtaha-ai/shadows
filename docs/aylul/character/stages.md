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
