# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

The settings dialog is delivered by PR #40, merged at `1a2b601`: its tasks F-2 to F-12,
F-14 and F-15 are in this file at `1a2b601`. The architecture plan's F-16 to F-22
tasks below remain. Still open outside any task: the light palette and the real desktop login-item behaviour need their own app
and OS acceptance.

## Tasks

### F-1 · The mode chosen in a new conversation's draft stays after the first send
- Status: done
- Plan: none (bug Mohammed found in the AYLUL desktop on 2026-10-20). In the draft
  (`/projects/:id/new`) he switches the mode from Accept edits to Auto, sends the
  first message, and once the thread opens the mode menu shows Accept edits again;
  the next turns run in Accept edits. Starting a new conversation at Accept edits is
  decided by spec §12 and stays. The fixer finds the root cause; if it lies in the
  backend (`crates/`), it stops and reports where.
- Read: `web/src/app/conversation/contract.yaml`;
  `docs/superpowers/specs/2026-09-24-harness-controls-design.md` §12.4–§12.5
- Touch: `web/src/app/conversation/` (the files the root cause names, their tests,
  its `contract.yaml`)
- Needs: none
- Done when: a test proves that after the draft's first send with Auto the thread's
  composer still shows Auto and the second turn sends `mode: 'auto'`; it and the
  touched tests pass with `npm run typecheck` and `npm run lint`

### F-16 · The provider form's Enter trap
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/app/agents/{provider-form.tsx,provider-models.tsx}, their tests
- Needs: none
- Done when: Enter in "Add a model by id" adds it and does not submit; saving with a typed, unadded id says so; `npm test -- provider` passes

### F-17 · Mutation answers through `writeSaved`; one query-key factory
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
- Read: root `AGENTS.md` "Code structure"; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: web/src/api/ (query keys), web/src/app/agreements/agreements-page.tsx, web/src/app/agents/agent-extensions.tsx, the files holding literal keys
- Needs: none
- Done when: no direct `setQueryData` on a mutation answer and no literal query key outside web/src/api/; web tests pass

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

### F-23 · The Mode menu saves the mode and shows Core's
- Status: blocked
- Plan: none. Owner spec: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.4 ("The mode is remembered per conversation"); request docs/shared/requests/2026-10-22-remembered-mode.md (accepted)
- Read: `web/src/app/conversation/contract.yaml`; `composer-bar.tsx`, `composer.tsx`, `turn-settings.ts`, `use-session.ts`, `web/src/api/client.ts`
- Touch: `web/src/app/conversation/` (the mode choice, its tests, its `contract.yaml`), `web/src/api/` (the new call, regenerated `schema.d.ts` via `npm run gen:api`)
- Needs: B-10
- Done when: choosing Auto calls `PUT /api/threads/{id}/session/mode` at once and the menu shows Core's answer; after navigating away and back, and after a remount with a fresh QueryClient, the menu shows Auto; a refusal keeps Core's mode and shows its message; the draft still starts at the policy default; the tests pass with `npm run typecheck` and `npm run lint`

### F-24 · The `+` menu: Slash commands and Plugins
- Status: todo
- Plan: none. Owner specs: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.11 ("The `+` menu"), docs/superpowers/specs/2026-10-05-slash-menu-design.md; request docs/shared/requests/2026-10-22-image-attachments.md (accepted)
- Read: `web/src/app/conversation/contract.yaml`; `composer-bar.tsx` (the disabled `+`), `composer.tsx` (the `/` menu), the Plugins & Skills settings entry point
- Touch: `web/src/app/conversation/` (the menu, its tests, its `contract.yaml`)
- Needs: none (no backend change)
- Done when: `+` opens a menu; Slash commands opens the existing `/` menu with the same list; Plugins opens the conversation's AYLUL agent's Plugins & Skills settings and is absent in a project conversation; Add photos is shown disabled with "Comes with F-25" until F-25; keyboard (Enter, arrows, Escape) works; the tests pass with `npm run typecheck` and `npm run lint`

### F-25 · Paste, drop and pick images, and send them
- Status: blocked
- Plan: none. Owner specs: §4.2 "Attachment", §12.11; request docs/shared/requests/2026-10-22-image-attachments.md (accepted)
- Read: `web/src/app/conversation/contract.yaml`; `composer.tsx`, `composer-bar.tsx`, `draft.tsx`, `web/src/api/client.ts`
- Touch: `web/src/app/conversation/` (paste, drop, picker, thumbnails, sending, the transcript's images, tests, `contract.yaml`), `web/src/api/` (the calls; `schema.d.ts` via `npm run gen:api`)
- Needs: B-11, F-24
- Done when: Ctrl+V of a screenshot, a drop and Add photos (Ctrl+U) each upload at once and show a thumbnail with remove; ordinary text paste is unchanged; Send waits for uploads and sends the ids in order; an image-only message sends; a failed upload or a refused send keeps the text and images and shows Core's reason; Add photos and paste are disabled with the reason when `capabilities.image` is false; the draft's first send keeps its images; the transcript shows a message's images; the tests pass with `npm run typecheck` and `npm run lint`
