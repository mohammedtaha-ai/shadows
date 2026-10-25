# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

The settings dialog is delivered by PR #40, merged at `1a2b601`: its tasks F-2 to F-12,
F-14 and F-15 are in this file at `1a2b601`. The architecture plan's F-16 to F-22
tasks below remain. F-1, F-23, F-24 and F-25 shipped in PR #41 (`c7e7bac4`) and are in this
file at `c7e7bac4`. The [web architecture plan](plans/2026-10-25-web-architecture.md) absorbs
F-16 to F-22 into its W tasks as they enter. Still open outside any task: the light palette and the real desktop login-item behaviour need their own app
and OS acceptance.

## Tasks

### F-27 · W-2: every contract parses and the code map tells the truth
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-2--make-contracts-parseable-and-map-startup-ownership-truthfully
- Read: the plan's section 1 ("Important map discrepancies"); `web/src/architecture.test.ts`
  (the `contract` rule); `docs/frontend/codebase/README.md`
- Touch: `web/src/desktop/contract.yaml`; `web/src/app/conversation/contract.yaml`;
  `docs/frontend/codebase/README.md`; `web/src/architecture.baseline.txt` (the two `contract`
  entries removed); this task's `Status:`
- Needs: F-26
- Done when: both contracts parse as YAML with their meaning unchanged (quote or block the
  scalars, do not reword them); the code map names `app-root.tsx` as the QueryClient owner
  and `app/mode-policy.ts` at its real path; the two `contract` baseline entries are gone;
  `npx vitest run src/architecture.test.ts src/codemap.test.ts`, `npm run typecheck` and
  `npm run lint` pass

### F-26 · W-1: a shrinking architecture ratchet for `web/src`
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#decisions and #enforcing-check
  (rules 1 to 7) and #w-1--establish-the-frontend-architecture-ratchet
- Read: the plan's sections 2, 3, 5 and 6; `web/src/codemap.test.ts`;
  `docs/frontend/codebase/README.md`; `apps/desktop/src-tauri/tests/architecture/README.md` for
  the baseline rules (exact entries, new and stale fail, one-for-one moves)
- Touch: `web/src/architecture.test.ts` and its baseline and fixture files (new, next to it);
  `web/src/codemap.test.ts` only to share its file walk; `docs/frontend/codebase/README.md`
  (its row, and the module table the plan's section 6 names: owner, entry, contract, allowed
  dependencies); `web/package.json` and `web/package-lock.json` only for a YAML parser as a
  dev-dependency; this task's `Status:`
- Needs: none
- Done when: `npx vitest run src/architecture.test.ts` passes on today's tree with every current
  violation listed exactly in the baseline (the plan's 34 cross-feature edges, the upward
  `desktop/` imports, server access outside its owners, query-key literals and cache writes
  outside `api/`, test infrastructure reachable from `main.tsx`, the invalid
  `desktop/contract.yaml`); a new violation fails; a stale entry fails; in-memory fixtures prove
  alias (`@/`), relative, type-only, re-export and literal dynamic imports, and that the
  injected `fetch` in `rule-preview.ts` is not flagged; no product source changes;
  `npm run typecheck`, `npm run lint` and `npx vitest run src/codemap.test.ts` pass

### F-16 · The provider form's Enter trap
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/app/agents/{provider-form.tsx,provider-models.tsx}, their tests, web/src/app/agents/contract.yaml
- Needs: none
- Done when: Enter in "Add a model by id" adds it and does not submit; saving with a typed, unadded id says so; `npm test -- provider` passes

### F-17 · W-4: query identities and cache writes belong to `api/`
- Status: todo
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-4--establish-api-owned-query-identities-and-safe-cache-writes
  and #decisions (1)
- Read: the plan's section 3 ("Current owners", "Highest-priority ownership problems" 2 and 3);
  `web/src/api/contract.yaml`; `web/src/api/saved-answer.ts`; `web/src/architecture.test.ts`
  (`query_key`, `cache_write` rules); `web/src/architecture.baseline.txt`
- Touch: `web/src/api/` (one key factory and the cache-update methods per server domain, next
  to that domain's queries; `contract.yaml`); every product file the `query_key` and
  `cache_write` baseline entries name (agreements, agents, conversation including
  `use-session.ts`, `use-commands.ts` and composer callers, design, project-settings, sidebar,
  workflows, stream, desktop) and their tests and `contract.yaml`;
  `web/src/architecture.baseline.txt`; `docs/frontend/codebase/README.md`; this task's `Status:`
- Needs: F-26
- Done when: no product file outside `web/src/api/` builds a query key or writes the cache: every
  production `query_key` and `cache_write` baseline entry is gone (test files may keep theirs
  only where the test seeds a cache on purpose, each named in the commit); key values are
  unchanged (same arrays), so SSE refresh still matches; mutation answers go through owner
  methods backed by `writeSaved`, including `agreements-page.tsx` and `agent-extensions.tsx`;
  session and command frames update through their owner; a regression test per fixed
  mutation shows a stale in-flight read cannot overwrite the saved answer; no other baseline
  count goes up; `npm test` for each touched area, the architecture and codemap tests,
  `npm run typecheck` and `npm run lint` pass. One commit per server domain.

### F-18 · Core owns the rules the client copies
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/app/agents/, web/src/app/settings/active-projects.tsx, web/src/app/design/create-structure.tsx, web/src/app/conversation/mode-policy.ts
- Needs: a shared request for the values Core must return
- Done when: model-picks.ts and the copied limits are gone; forms show Core's answers; web tests pass

### F-19 · One owner per behaviour
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/stream/, web/src/app/workflows/plan-frames.ts, the three proposal cards, the four confirm patterns, web/src/app/conversation/turn-state.ts
- Needs: a shared request if `turn-state` needs a field Core does not send
- Done when: one plan-refresh owner, one ProposalCard, one confirm pattern; web tests pass

### F-20 · Split by job
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/api/client.ts, web/src/stream/frames.ts, web/src/app/conversation/composer.tsx, web/src/app/test-app.tsx
- Needs: none
- Done when: each new file's job is named without "and"; typecheck, lint, tests and build pass

### F-21 · No `§` and no history in comments
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src (hand-written files)
- Needs: none
- Done when: no hand-written comment cites `§` or history; lint and tests pass

### F-22 · Keep the summary ring after breakdown removal
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-7b--remove-the-breakdown-route-after-agreement
- Read: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.8; web/src/app/conversation/contract.yaml; web/src/api/contract.yaml; docs/shared/README.md
- Touch: web/src/app/conversation/context-ring.tsx, web/src/api/client.ts, web/src/api/schema.d.ts (generated), both owning contracts, docs/frontend/codebase/README.md; controller enumerates the actual remaining callers and tests before dispatch
- Needs: accepted shared request naming GET /api/threads/{id}/context removal and the summary-only ring; A-7b's generated OpenAPI; A-2's explicit slash-command path
- Done when: `npm run gen:api` regenerates the schema; focused tests prove the ring makes no hidden breakdown request, summary usage remains visible and explicit `/context` still sends; typecheck and lint pass; both sides land on the same delivery branch
