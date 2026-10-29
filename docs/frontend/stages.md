# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Left: W-11 of the [web architecture plan](plans/2026-10-25-web-architecture.md), split on
2026-10-29 into five packets, F-45 to F-49. Each waits on its shared request and its Core
task (B-14 to B-17); F-49 waits on the plan's OPEN block. F-18 and F-19 are replaced by
these packets. Still open outside any task: the light palette and the real desktop
login-item behaviour need their own app and OS acceptance.

Finished work: F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

### F-45 · The provider form shows Core's refusal
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 1)
- Read: docs/shared/requests/2026-10-29-provider-refusals.md; `web/src/app/agents/contract.yaml`
- Touch: delete `web/src/app/agents/model-picks.ts`; `provider-form.tsx`, `provider-models.tsx`,
  their tests and `contract.yaml`; `web/src/api/transport/index.ts` (the problem carries
  `field`), `transport/error.test.ts` and `web/src/api/contract.yaml`; `web/src/api/schema.d.ts`
  through `npm run gen:api` only; `web/src/test/fake-daemon.ts` (refuses create and edit as
  Core does); the `model-picks.ts` row in `docs/frontend/codebase/README.md`
- Needs: B-14 on the same branch
- Done when: no web file holds the model limits, the 64-character label limit or the 200000
  window default; a refused save shows Core's message under its
  field and keeps the input; `npx vitest run src/app/agents`, the architecture and codemap tests,
  `npm run typecheck` and `npm run lint` pass

### F-46 · Active projects reads its bounds from Core
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 2)
- Read: docs/shared/requests/2026-10-29-active-limit-bounds.md; `web/src/app/settings/contract.yaml`
- Touch: `web/src/app/settings/active-projects.tsx` and `contract.yaml`; the settings fixtures in
  `web/src/app/settings/{settings-dialog,desktop-preferences}.test.tsx` and
  `web/src/api/code.test.ts`; `web/src/api/schema.d.ts` through `npm run gen:api` only;
  `web/src/test/fake-daemon.ts`
- Needs: B-15 on the same branch
- Done when: `MIN` and `MAX` are gone and the input uses the settings' bounds; the settings
  tests, `npm run typecheck` and `npm run lint` pass

### F-47 · Mode names and policy come from the harness list
- Status: todo
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 3)
- Read: docs/shared/requests/2026-10-29-harness-mode-policy.md; the `shared` rows of
  `docs/frontend/codebase/README.md`
- Touch: delete `web/src/app/mode-policy.ts`; its callers `conversation/composer-bar.tsx`,
  `draft-session.ts`, `entry.tsx`, `messages.tsx`, `project-settings/project-modes.tsx`, their
  tests and contracts; `web/src/api/` harness reading; `web/src/api/schema.d.ts` through
  `npm run gen:api` only; `web/src/test/fake-daemon.ts`; the `mode-policy.ts` rows in
  `docs/frontend/codebase/README.md` and the architecture baseline it feeds
- Needs: B-16 on the same branch
- Done when: no web file names `acceptEdits` or `auto` outside tests and fixtures; the composer,
  draft and project-modes tests pass with the policy from `GET /api/harnesses`;
  `npm run typecheck` and `npm run lint` pass

### F-48 · Create structure shows Core's file count
- Status: todo
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 4)
- Read: docs/shared/requests/2026-10-29-structure-file-count.md; `web/src/app/design/contract.yaml`
- Touch: `web/src/app/design/create-structure.tsx`, its test and `contract.yaml`;
  `web/src/api/design.ts` (the create sends `map_revision`); `web/src/api/schema.d.ts` through
  `npm run gen:api` only; `web/src/test/fake-daemon.ts`
- Needs: B-17 on the same branch
- Done when: `fileCount` and its part walk are gone; the confirm shows the stage's count and
  sends its revision; a moved map re-reads the stage and asks again; the design tests,
  `npm run typecheck` and `npm run lint` pass

### F-49 · Turn state follows Core's ordering
- Status: blocked
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 5)
- Read: `web/src/app/conversation/turn-state.ts` and its contract
- Touch: decided with the packet's OPEN block
- Needs: the OPEN block in W-11 closed; a request if Core must send more
- Done when: written when the OPEN block closes; stale-event reconciliation stays correct
