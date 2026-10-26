# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Left: F-28 waits on its Core request; F-18 to F-22 are the remaining work of the
[web architecture plan](plans/2026-10-25-web-architecture.md), which enters them as its tasks
W-9 to W-13. Still open outside any task: the light palette and the real desktop login-item
behaviour need their own app and OS acceptance.

Finished work: F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) are in this file
at `ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

### F-33 · W-9a: session-setting changes leave the composer
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-9--split-conversation-behavior-by-job
  (W-9 enters in four packets: a, session settings; b, attachments; c, send, queue and stop;
  d, `draft.tsx` and `carried-send.ts`)
- Read: the plan's W-9 and section 4 (the `conversation/composer.tsx` row);
  `web/src/app/conversation/contract.yaml`; `web/src/app/conversation/composer.tsx`;
  `turn-settings.ts`; the `conversation` rows of `docs/frontend/codebase/README.md`
- Touch: `web/src/app/conversation/composer.tsx`; a new
  `web/src/app/conversation/use-session-settings.ts` and its test
  `use-session-settings.test.tsx`; `composer-bar.test.tsx` only to move out the tests that
  exercise settings switching alone; `web/src/app/conversation/contract.yaml`;
  `docs/frontend/codebase/README.md` (the new file's row); `web/src/architecture.baseline.txt`
  only if a moved test carries an existing entry with it (one-to-one); this task's `Status:`
- Needs: F-17, F-31 (done)
- Done when: `use-session-settings.ts` owns the whole job of changing the session's settings:
  the held turn settings and their note (today's `useTurnSettings`), the mode switch with its
  replay attempt, the model switch with its refusal, the effort switch, and one `switching`
  flag the composer reads for `ready`. `composer.tsx` keeps no `changeMode`, `changeModel` or
  `changeEffort` call and no settings state of its own. Behaviour is unchanged: the same
  requests in the same order, a model refusal still shows "The session kept …", a lost mode
  answer still replays with the same command id, the model and effort are not switched while a
  turn runs (a mode choice goes to Core at once, as before), and
  Send still waits for the switch. The tests that pinned this move with it, assertions
  unchanged, and the rest of `composer-bar.test.tsx` passes unchanged. From `web/`:
  `npx vitest run src/app/conversation src/architecture.test.ts src/codemap.test.ts`,
  `npm run typecheck`, `npm run lint` and `npm run build` pass

### F-34 · W-9b: the attachment lifecycle leaves the composer
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-9--split-conversation-behavior-by-job
  (packet b of four; F-33 was packet a)
- Read: the plan's W-9 and section 4 (the `conversation/composer.tsx` and `carried-send.ts`
  rows); `web/src/app/conversation/contract.yaml`; `composer.tsx`; `carried-send.ts`;
  `use-session-settings.ts` (the shape packet a gave its hook); the `conversation` rows of
  `docs/frontend/codebase/README.md`
- Touch: `web/src/app/conversation/composer.tsx`; a new `use-images.ts` (the selected images:
  list, Core's image reason, add, pick, retry, remove, and dropping those a send took); a new
  `pending-images.tsx` (the image list with its previews,
  status lines and Retry/Remove buttons, today's `ImagePreview` included);
  `web/src/app/conversation/contract.yaml`; `docs/frontend/codebase/README.md` (the new files'
  rows); this task's `Status:`
- Needs: F-33 (done)
- Done when: `composer.tsx` keeps no image state, no `uploadImages`/`retryImageUpload` call
  outside the send and queue mutations, no `imageReason` computation and no `ImagePreview`; the
  paste, drop, Ctrl+U and file-input handlers in the composer only hand files to the hook.
  `carried-send.ts` is unchanged (packet d). Behaviour is unchanged: uploads start on selection
  in a thread and on Send in a draft, a selection change still drops the pending send attempt,
  Core's image refusal still shows its reason on the error line, removing an uploading image
  still keeps its late answer out of the send, switching conversations still revokes local
  previews, and Send/queue still wait for uploads and send ids in selection order. The attachment
  tests stay in `composer-bar.test.tsx` unchanged (they share its upload stub with the send and
  queue tests) and pass through the composer. From `web/`: `npx vitest run src/app/conversation src/architecture.test.ts
  src/codemap.test.ts`, `npm run typecheck`, `npm run lint` and `npm run build` pass

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
- Replaced by: the web plan's W-7 (stream refresh, unblocked), W-10 (proposal card, confirm pattern, plan wording, unblocked) and W-11 (`turn-state`, blocked by domain). Not dispatched in this form; this entry goes when those packets enter.
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/stream/, web/src/app/workflows/plan-frames.ts, the three proposal cards, the four confirm patterns, web/src/app/conversation/turn-state.ts
- Needs: a shared request if `turn-state` needs a field Core does not send
- Done when: one plan-refresh owner, one ProposalCard, one confirm pattern; web tests pass

### F-20 · Split by job
- Status: blocked
- Replaced by: the web plan's W-8 (`api/client.ts`, and `stream/frames.ts` by parser), W-9 (`conversation/composer.tsx`) and W-12 (`app/test-app.tsx`), each with its destinations, callers, contracts, map and baseline. Not dispatched in this form.
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/api/client.ts, web/src/stream/frames.ts, web/src/app/conversation/composer.tsx, web/src/app/test-app.tsx
- Needs: F-17 and W-7 (W-8 needs F-17; W-9 needs F-17 and W-7; W-12 needs F-26)
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
