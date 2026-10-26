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

### F-35 · W-9c: delivering a message and stopping a turn leave the composer
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-9--split-conversation-behavior-by-job
  (packet c of four; F-33 and F-34 were a and b)
- Read: the plan's W-9 and section 4 (the `conversation/composer.tsx` row);
  `web/src/app/conversation/contract.yaml`; `composer.tsx`; `use-session-settings.ts` and
  `use-images.ts` (the shape packets a and b gave their hooks); `carried-send.ts` (read only);
  the `conversation` rows of `docs/frontend/codebase/README.md`
- Touch: `web/src/app/conversation/composer.tsx`; a new `use-send.ts` (delivering the message:
  the pending command attempt, the send mutation for a thread or a draft, the queue mutation
  while a turn runs, and the choice between them); a new `use-stop.ts` (stopping the running
  turn: its mutation and the Stop / Stopping… / Stop again label);
  `web/src/app/conversation/contract.yaml`; `docs/frontend/codebase/README.md` (the new files'
  rows); this task's `Status:`
- Needs: F-34 (done)
- Done when: `composer.tsx` keeps no `useMutation`, no `startTurn`, `queueMessage`, `stopTurn`,
  `uploadImages` or `noteCompanionSend` call, and no `pending` attempt ref of its own; it keeps
  the prompt text, the slash menu, `act`, `ready` and the markup, and calls the send hook to
  deliver. `carried-send.ts` and `draft.tsx` are unchanged (packet d). Behaviour is unchanged:
  the same requests in the same order; a retried Send reuses the same command id and a changed
  prompt, setting, image or focus makes a new one; a `COMMAND_CONFLICT` drops the attempt; a
  draft's Send still goes through `to.draft`; Enter while a turn runs queues and a queued
  success refreshes the queue; a success clears the prompt, drops the sent images, clears the
  pointed task and the continued plan; Stop reads Stopping… until `running` clears and offers
  Stop again after a failed stop. `composer-bar.test.tsx` and the baseline are unchanged. From
  `web/`: `npx vitest run src/app/conversation src/architecture.test.ts src/codemap.test.ts`,
  `npm run typecheck`, `npm run lint` and `npm run build` pass

### F-36 · W-9d: the draft's first send and image uploads get their own owners
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-9--split-conversation-behavior-by-job
  (packet d, the last; F-33 to F-35 were a to c)
- Read: the plan's W-9 and section 4 (the `draft.tsx` and `carried-send.ts` rows);
  `web/src/app/conversation/contract.yaml`; `draft.tsx`; `carried-send.ts`; their callers
  `composer.tsx`, `conversation.tsx`, `use-send.ts`, `use-images.ts`, `pending-images.tsx`;
  the `conversation` rows of `docs/frontend/codebase/README.md`
- Touch: `web/src/app/conversation/carried-send.ts`; a new `upload-images.ts` (the
  `PendingImage` type, `uploadImages` and `retryImageUpload`, moved as they are);
  `draft.tsx`; a new `first-send.ts` (today's `useFirstSend`: create the thread, open its
  session, start the first turn, and hand a failed turn to the thread page); a new
  `draft-session.ts` (today's `draftSession` and `UNOPENED`: the choices a draft offers before
  a session exists); the import lines only of `composer.tsx`, `conversation.tsx`, `use-send.ts`,
  `use-images.ts` and `pending-images.tsx`; `web/src/app/conversation/contract.yaml`;
  `docs/frontend/codebase/README.md` (the new files' rows and the changed jobs of
  `draft.tsx` and `carried-send.ts`); this task's `Status:`
- Needs: F-35 (done)
- Done when: `carried-send.ts` holds only the failed first-send handoff (`CarriedSend`,
  `carrySend`, `useCarriedSend`) and imports no API module; `upload-images.ts` owns uploading;
  `draft.tsx` holds the draft routes and the draft page and makes no API call itself
  (`createThread`, `openSession`, `changeModel`, `startTurn`, `uploadImages` move to
  `first-send.ts`). Behaviour is unchanged: a failed create stays in the draft and retries with
  the same command id; a turn that fails after the thread exists goes to the thread with its
  text, images, error, plan and a replayable attempt (a `COMMAND_CONFLICT` gives a fresh one);
  a person who left the draft while it sent is not pulled back; an upload keeps its command id
  across the handoff and is never uploaded twice. Every test file and
  `architecture.baseline.txt` are unchanged. From `web/`: `npx vitest run src/app/conversation
  src/architecture.test.ts src/codemap.test.ts`, `npm run typecheck`, `npm run lint` and
  `npm run build` pass

### F-37 · W-10a: the project plan list words a frozen plan as the plan page does
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-10--consolidate-repeated-presentation-behavior
  (W-10 enters in four packets: a, plan-state wording; b, one proposal approval lifecycle for the
  vision, map and roadmap cards; c, one confirmation component for the four delete/remove
  confirms; d, desktop capabilities)
- Read: the plan's W-10 and its "Plan state wording" row; `web/src/app/design/workspace-plans.tsx`;
  `web/src/app/workflows/plan-state.ts`; the agreement and the defect naming `stateLabel` in
  `web/src/app/design/contract.yaml`; `web/src/app/workflows/contract.yaml` (`feature_entry`)
- Touch: `web/src/app/design/workspace-plans.tsx`; one test for it, either added to an existing
  `web/src/app/design/*.test.tsx` that already renders the plan list, or a new
  `workspace-plans.test.tsx` that adds no `architecture.baseline.txt` entry;
  `web/src/app/design/contract.yaml`; this task's `Status:`
- Needs: W-6 (done)
- Done when: `WorkspacePlans` shows each version through `stateLabel` from
  `@/app/workflows/plan-state` ("Approved v1" for a frozen version, "Draft v2" otherwise) and
  keeps " · Archived" for an archived plan; a test proves a frozen plan reads "Approved v1" and
  never "Frozen"; the design contract's `WorkspacePlans`/`stateLabel` agreement holds (true, with
  that test) and its defect line is gone. `plan-state.ts` is unchanged. From `web/`:
  `npx vitest run src/app/design src/app/workflows src/architecture.test.ts src/codemap.test.ts`,
  `npm run typecheck`, `npm run lint` and `npm run build` pass

### F-38 · W-10b: the three proposal cards share one approval lifecycle
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-10--consolidate-repeated-presentation-behavior
  (packet b of four; F-37 was a)
- Read: the plan's W-10 and its "Proposal status/approval lifecycle" row;
  `web/src/app/conversation/vision-proposal-card.tsx`, `map-proposal-card.tsx`,
  `roadmap-proposal-card.tsx`, `approved-by.tsx`; their tests; the proposal-card sections,
  agreements and checks of `web/src/app/conversation/contract.yaml`
- Touch: the three card files; a new `web/src/app/conversation/use-proposal-approval.ts` (the
  live proposal read, the approval mutation with its command attempt and cache invalidations,
  and the status text) and a new `proposal-approval.tsx` (the status line, Approved by, the
  Approve button and the two error lines); `web/src/app/conversation/contract.yaml`;
  `docs/frontend/codebase/README.md` (the new files' rows); this task's `Status:`
- Needs: W-4 and F-37 (done)
- Done when: no card holds `useMutation`, `attemptFor`, a proposals `useQuery` or the status
  ladder; each card keeps only its snapshot parser, its title and its content (vision field and
  old text; map changed fields, part tree and waivers; roadmap current outcome, move line and
  outcome tree) and renders the shared approval part with its own button label. Behaviour and
  output are unchanged: the same approve request with the same command attempt (reused on retry,
  dropped on success), success invalidates `designKeys.all(projectId)`, error invalidates that
  card's proposals query, the status texts and their order (Approved, vision's "Approved · item",
  Superseded, Outdated…, Pending, Loading status…, Status unavailable), no Approve while loading,
  outdated or on a read error, and each card's text classes. Every test file and
  `architecture.baseline.txt` are unchanged. From `web/`: `npx vitest run src/app/conversation
  src/architecture.test.ts src/codemap.test.ts`, `npm run typecheck`, `npm run lint` and
  `npm run build` pass

### F-39 · W-10c: the four delete and remove confirmations share one confirmation panel
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-10--consolidate-repeated-presentation-behavior
  (packet c of four; F-37 and F-38 were a and b). The plan's rule holds: shared confirmation UI
  never centralizes command identities or pending state.
- Read: the plan's W-10 and its risks line on shared confirmation UI;
  `web/src/app/agents/delete-zone.tsx`, `web/src/app/sidebar/delete-thread-dialog.tsx`,
  `web/src/app/project-settings/remove-project.tsx`, `web/src/app/settings/extension-list.tsx`
  and their tests; `web/src/components/ui/dialog.tsx`; the `shared` module row and the
  `error-line.tsx` row of `docs/frontend/codebase/README.md`
- Touch: a new `web/src/app/confirm-panel.tsx` (rendering a confirmation: its question, what it
  will do, the error line, Cancel and the destructive confirm button with its pending label; as
  a dialog body or as an inline group); the four files above; their module contracts
  (`agents`, `sidebar`, `project-settings`, `settings`); `docs/frontend/codebase/README.md` (the
  `shared` module row and the new file's row); this task's `Status:`
- Needs: F-38 (done)
- Done when: all four confirmations render through `ConfirmPanel`; it holds no mutation, no
  command attempt, no query and no open state; each caller keeps its own mutation, attempt,
  invalidations, navigation, open state and lock (extension list's `locked` ref and per-item
  attempts, the thread dialog's close guard while pending, remove-project's attempt kept outside
  the unmounting dialog). The rendered text, roles, labels, button variants, disabled rules and
  pending labels ("Deleting...", "Deleting…", "Removing…") of each confirmation are unchanged.
  Every test file and `architecture.baseline.txt` are unchanged. From `web/`:
  `npx vitest run src/app/agents src/app/sidebar src/app/project-settings src/app/settings
  src/architecture.test.ts src/codemap.test.ts`, `npm run typecheck`, `npm run lint` and
  `npm run build` pass

### F-40 · W-10d: screens ask for a desktop capability, never for the platform
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-10--consolidate-repeated-presentation-behavior
  ("Desktop capabilities", first half). Registering the desktop-only settings sections and
  sidebar entries from `app/desktop/` in one place is F-41, not this task.
- Read: the plan's "Desktop capabilities" paragraph; `web/src/desktop/runtime.ts`,
  `web/src/desktop/index.ts`, `web/src/desktop/startup.ts`, `web/src/desktop/contract.yaml`;
  every file outside `web/src/desktop/` that calls `isDesktop()`; how
  `app/agents/aylul.test.tsx` and `app/settings/settings-dialog.test.tsx` mock
  `@/desktop/runtime`; `web/src/architecture.test.ts` and `architecture.fixtures.ts`
- Touch: a new `web/src/desktop/capabilities.ts` (what this build of the app can do natively);
  `web/src/desktop/index.ts`, `web/src/desktop/startup.ts`, `web/src/desktop/contract.yaml`;
  the files outside `desktop/` that call `isDesktop()` today: `main.tsx`, `router.tsx`,
  `app/settings/sections.ts`, `app/settings/sections/{general,account,agents}.tsx`,
  `app/sidebar/sidebar.tsx`, `app/agents/agent-settings.tsx`, `app/desktop/activity.tsx`, and
  the contracts of their modules; `web/src/architecture.test.ts` and
  `web/src/architecture.fixtures.ts` (one rule with a failing fixture);
  `docs/frontend/codebase/README.md` (the new file's row); this task's `Status:`
- Needs: F-39 (done)
- Rulings: the capabilities have one field per native feature: `localCore`, `companion`,
  `tray` (the tray menu's navigation and hiding the window to it), `accountLogin`,
  `windowPreferences` (starting with the system and closing to the tray, set in General), and
  `hashRouting` (the native shell loads the app from a custom scheme, so the router needs hash
  history). The browser build gets every field `false`.
  They are read from `isDesktop()` in `desktop/runtime.ts` at call time, not frozen in a module
  constant, because the platform never changes while the app runs and the existing tests switch
  the mocked `isDesktop` between cases. Each caller asks for the field that names what it shows;
  where a screen shows a desktop-only thing, the contract names which capability gates it.
- Done when: no file outside `web/src/desktop/` imports or calls `isDesktop`; the architecture
  rule refuses it, proven by a fixture that fails without the rule, and needs no baseline entry;
  `isDesktop` is exported from `desktop/runtime.ts` only (the tests mock it there), not from
  `desktop/index.ts` or `desktop/startup.ts`. Browser and desktop show exactly what they show today. Every
  test file other than `architecture.test.ts` (which gains the rule and keeps its existing
  checks) and `architecture.baseline.txt` are unchanged. From `web/`:
  `npx vitest run src/desktop src/app/desktop src/app/settings src/app/sidebar src/app/agents
  src/architecture.test.ts src/codemap.test.ts`, `npm run typecheck`, `npm run lint` and
  `npm run build` pass

### F-41 · W-10e: desktop-only settings sections and sidebar actions register from `app/desktop/`
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-10--consolidate-repeated-presentation-behavior
  ("Desktop capabilities", second half; F-40 was the first).
- Read: the plan's "Desktop capabilities" paragraph; `web/src/app/settings/sections.ts`,
  `settings-dialog.tsx`, `sections/character.tsx`, `sections/core.tsx`;
  `web/src/app/sidebar/sidebar.tsx`; `web/src/app/desktop/index.ts` and its contract;
  `web/src/desktop/capabilities.ts`; the `settings`, `sidebar` and `desktop-ui` module rows of
  `docs/frontend/codebase/README.md`; `app/desktop/companion-binding.test.tsx`,
  `app/settings/core.test.tsx`, `app/settings/settings-dialog.test.tsx`
- Touch: a new `web/src/app/desktop/settings-sections.tsx` (the desktop-only settings sections:
  their ids, labels, group, the capability that shows each, and their content, which moves here
  from `settings/sections/character.tsx` and `settings/sections/core.tsx`, both deleted); a new
  `web/src/app/desktop/sidebar-actions.tsx` (the sidebar's "Local activity" and "Hide to tray"
  row and its error line, moved from `sidebar.tsx`); `web/src/app/desktop/index.ts`;
  `web/src/app/settings/sections.ts`, `settings-dialog.tsx`; `web/src/app/sidebar/sidebar.tsx`;
  the import line only of `app/desktop/companion-binding.test.tsx` and `app/settings/core.test.tsx`;
  `web/src/architecture.baseline.txt` (removing the entries the move fixes, adding none); the
  `settings`, `sidebar` and `desktop-ui` contracts; `docs/frontend/codebase/README.md` (the new
  files' rows, the deleted files' rows, and `desktop-ui` added to the `sidebar` row's
  dependencies); this task's `Status:`
- Needs: F-40 (done)
- Done when: `settings` lists only its own sections and appends the ones `app/desktop/` registers;
  the sidebar renders one component from `app/desktop/` for its desktop actions; no file in
  `settings/` or `sidebar/` reads a capability for a desktop-only section or action. The section
  ids, the router's settings search, the nav order ("This machine", "AYLUL character", "Local
  Core") and every rendered DOM in browser and desktop are unchanged. No test file changes except
  the two import lines named above; the baseline only shrinks. From `web/`:
  `npx vitest run src/desktop src/app/desktop src/app/settings src/app/sidebar
  src/architecture.test.ts src/codemap.test.ts`, `npm run typecheck`, `npm run lint` and
  `npm run build` pass

### F-42 · W-12b: the composer's image tests leave `composer-bar.test.tsx`
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-12--put-test-infrastructure-under-its-owner
  ("oversized composer test file"; the harness move is a separate packet).
- Read: `web/src/app/conversation/composer-bar.test.tsx` (678 lines); `use-images.ts`,
  `upload-images.ts`, `pending-images.tsx`; `web/src/architecture.test.ts` (`compare` and the
  relocation test); `web/src/architecture.baseline.txt`
- Touch: `web/src/app/conversation/composer-bar.test.tsx`; a new
  `web/src/app/conversation/composer-images.test.tsx` (every test of attaching, uploading,
  removing, sending and queueing images, with the helpers only they use); a new
  `web/src/test/composer.ts` only if both files need the same helper (`start`, `ready`,
  `textarea`, `turnStarts`), never a copy of it; `web/src/architecture.baseline.txt` (the
  `dependency :: ... router.tsx` and `server_access :: ... global:fetch` entries move one-for-one
  to the file that now holds them, none added); `web/src/app/conversation/contract.yaml`
  (`tested_by` paths); `docs/frontend/codebase/README.md` if a new file needs a row; this task's
  `Status:`
- Needs: F-36 (done)
- Done when: `composer-bar.test.tsx` holds the composer bar's menus, settings, send and command-id
  tests; `composer-images.test.tsx` holds the image tests; every test keeps its name, body and
  assertions, and the two files together run the same 33 test cases (29 declarations, one
  `it.each` with four rows) as before; no other test file changes; the baseline's entry count is unchanged. From `web/`:
  `npx vitest run src/app/conversation src/architecture.test.ts src/codemap.test.ts`,
  `npm run typecheck` and `npm run lint` pass

### F-43 · W-12a: the test harness and the fake event source live in `test/`
- Status: done
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-12--put-test-infrastructure-under-its-owner
  (the harness move; F-42 split the composer test file).
- Read: `web/src/app/test-app.tsx`, `web/src/stream/fake-event-source.ts`, the `web/src/test/`
  files; the `test-support` module row and the "Test-only code never reaches the product" lines
  of `docs/frontend/codebase/README.md`; the FakeSource note in `web/src/stream/contract.yaml`;
  `web/src/architecture.test.ts` (`compare`, the fixture-import and production-test checks);
  `web/src/architecture.baseline.txt`
- Touch: `web/src/app/test-app.tsx` → `web/src/test/test-app.tsx`;
  `web/src/stream/fake-event-source.ts` → `web/src/test/fake-event-source.ts`; the import line of
  every file that imports either (tests, `web/src/test/*`); `web/src/architecture.baseline.txt`
  (the `public_entry :: stream/fake-event-source.ts :: ...` entry moves one-for-one to the new
  path; none added); `docs/frontend/codebase/README.md` (the `test-support` row's paths and
  entries, the two file rows, the "Test-only code" lines); `web/src/stream/contract.yaml` and
  any contract that names either old path; this task's `Status:`
- Needs: F-42 (done)
- Done when: no file under `app/` or `stream/` is a test harness or test double; both files are
  moved with `git mv` and their contents change only in import paths; every other file changes
  only in import lines; the production-reachability check stays clean; the baseline's entry
  count is unchanged. From `web/`: `npx vitest run`, `npm run typecheck`, `npm run lint` and
  `npm run build` pass

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

### F-21 · No `§` and no history in the conversation's comments
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
  (first of two packets; F-44 takes every other folder)
- Read: root `CLAUDE.md` "Code structure" rule 5; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: hand-written `.ts`/`.tsx` files under `web/src/app/conversation/` (comments only); this
  task's `Status:`
- Needs: none
- Done when: no comment under `web/src/app/conversation/` cites `§`, a spec section or history
  ("used to", "no longer", "before", "was moved", a task id); each such comment is reworded to
  say why, or deleted when it says nothing the code does not; no code, test name, string or
  assertion changes. From `web/`: `npx vitest run src/app/conversation`, `npm run typecheck`
  and `npm run lint` pass

### F-44 · No `§` and no history in the other folders' comments
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#web-tasks-masadas-side-docsfrontendstagesmd
  (second of two packets; F-21 takes `app/conversation/`)
- Read: root `CLAUDE.md` "Code structure" rule 5; docs/frontend/AGENTS.md "Where things are" rule 0
- Touch: hand-written `.ts`/`.tsx` files under `web/src/` outside `web/src/app/conversation/`
  (comments only), including the header of `web/src/test/fake-event-source.ts`, which says
  nothing outside a test imports it although `test/test-app.tsx` does; this task's `Status:`
- Needs: none
- Done when: no comment under `web/src/` outside `app/conversation/` cites `§`, a spec section or
  history; each such comment is reworded to say why, or deleted when it says nothing the code
  does not; no code, test name, string or assertion changes. From `web/`: `npx vitest run`,
  `npm run typecheck` and `npm run lint` pass

### F-22 · Keep the summary ring after breakdown removal
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-7b--remove-the-breakdown-route-after-agreement
- Read: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.8; web/src/app/conversation/contract.yaml; web/src/api/contract.yaml; docs/shared/README.md
- Touch: web/src/app/conversation/context-ring.tsx, web/src/api/client.ts, web/src/api/schema.d.ts (generated), both owning contracts, docs/frontend/codebase/README.md; controller enumerates the actual remaining callers and tests before dispatch
- Needs: A-7b on the same delivery branch (its regenerated OpenAPI); the accepted request
  `docs/shared/requests/2026-10-27-summary-only-context-ring.md`
- Done when: `npm run gen:api` regenerates the schema; focused tests prove the ring makes no hidden breakdown request, summary usage remains visible and explicit `/context` still sends; typecheck and lint pass; both sides land on the same delivery branch
