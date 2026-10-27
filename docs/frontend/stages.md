# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Left: F-22 is ready to build on a delivery branch with the backend's A-7b
(`docs/shared/requests/2026-10-27-summary-only-context-ring.md`). F-28 waits on its Core
request; F-18 and F-19's remaining `turn-state` part are the
[web architecture plan](plans/2026-10-25-web-architecture.md)'s W-11, blocked by domain until
Core supplies the facts. Still open outside any task: the light palette and the real desktop
login-item behaviour need their own app and OS acceptance.

Finished work: F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

### F-28 · Each settings form saves only the fields it changed
- Status: blocked
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#decisions (3)
- Read: `docs/shared/requests/2026-10-26-partial-person-update.md`; `web/src/api/person.ts`;
  `web/src/app/settings/contract.yaml`
- Touch: `web/src/api/person.ts` and its tests; `web/src/app/settings/you-card.tsx`,
  `general-preferences.tsx`, their tests, `contract.yaml`; `web/src/api/schema.d.ts` through
  `npm run gen:api` only
- Needs: the partial person update request accepted and built in Core; F-17
- Done when: the You card sends only names, General preferences only language and theme;
  a test saves both forms close together and both changes survive; `npm test -- settings`,
  the architecture and codemap tests, `npm run typecheck` and `npm run lint` pass

### F-18 · Core owns the rules the client copies
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/app/agents/, web/src/app/settings/active-projects.tsx, web/src/app/design/create-structure.tsx, web/src/app/mode-policy.ts
- Needs: a shared request for the values Core must return; replaced by the web plan's W-11 per-domain packets as they enter, and not dispatched in this form
- Done when: model-picks.ts and the copied limits are gone; forms show Core's answers; web tests pass

### F-19 · One owner per behaviour
- Status: blocked
- Replaced by: the web plan's W-7 and W-10 (shipped: one stream-refresh owner, one proposal approval lifecycle, one confirmation panel) and W-11 (`turn-state`, blocked by domain). Not dispatched in this form; this entry goes when W-11 enters.
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/stream/, web/src/app/workflows/plan-frames.ts, the three proposal cards, the four confirm patterns, web/src/app/conversation/turn-state.ts
- Needs: a shared request if `turn-state` needs a field Core does not send
- Done when: one plan-refresh owner, one ProposalCard, one confirm pattern; web tests pass

### F-22 · Keep the summary ring after breakdown removal
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-7b--remove-the-breakdown-route-after-agreement
- Read: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.8; web/src/app/conversation/contract.yaml; web/src/api/contract.yaml; docs/shared/README.md
- Touch: web/src/app/conversation/context-ring.tsx, web/src/api/client.ts, web/src/api/schema.d.ts (generated), both owning contracts, docs/frontend/codebase/README.md; controller enumerates the actual remaining callers and tests before dispatch
- Needs: A-7b on the same delivery branch (its regenerated OpenAPI); the accepted request
  `docs/shared/requests/2026-10-27-summary-only-context-ring.md`
- Done when: `npm run gen:api` regenerates the schema; focused tests prove the ring makes no hidden breakdown request, summary usage remains visible and explicit `/context` still sends; typecheck and lint pass; both sides land on the same delivery branch
