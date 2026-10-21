# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Work and improvements continue on branch `improvements`. The settings dialog,
F-2 to F-9, is implemented and accepted by Mohammed in the app. F-10 and F-11 are
implemented with focused automated checks, as is F-14. F-12's steps await the controller's review.

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

### F-2 · Settings opens as a dialog with a sidebar of sections, General inside
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-2-dialog-shell-sidebar-of-sections-general
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`; `web/src/app/settings/contract.yaml`; `web/src/app/new-project/new-project-dialog.tsx`
- Touch: the Files list of Task F-2 in the plan
- Needs: none
- Done when: `settings-dialog.test.tsx` passes with codemap, `npm run typecheck`, `npm run lint`

### F-3 · Account shows only the actions its state allows
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-3-account-shows-only-the-actions-its-state-allows
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Account); `web/src/app/desktop/contract.yaml`
- Touch: the Files list of Task F-3 in the plan
- Needs: F-2
- Done when: `account-actions.test.ts` and `account.test.tsx` pass with codemap, typecheck, lint

### F-4 · AYLUL character and Local Core sections
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-4-aylul-character-and-local-core-sections
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (AYLUL character, Local Core); `web/src/app/desktop/contract.yaml`
- Touch: the Files list of Task F-4 in the plan
- Needs: F-2
- Done when: `companion-binding.test.tsx` and `core.test.tsx` pass with codemap, typecheck, lint

### F-5 · Providers live in the dialog, with search and presets
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-5-providers-section-with-search-and-presets
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Providers); `web/src/app/agents/contract.yaml`
- Touch: the Files list of Task F-5 in the plan
- Needs: F-2
- Done when: `aylul.test.tsx` and `provider-picker.test.tsx` pass with codemap, typecheck, lint

### F-6 · Agents in the dialog; Agent settings and New agent open it
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-6-agents-section-agent-settings-and-new-agent-open-it
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Agents, the dialog's rulings); `web/src/app/agents/contract.yaml`;
  `web/src/app/sidebar/contract.yaml`
- Touch: the Files list of Task F-6 in the plan
- Needs: F-2, F-5 (both change `agent-list.tsx` and `home.tsx`)
- Done when: `aylul.test.tsx` and `agent-env.test.tsx` pass with codemap, typecheck, lint

### F-7 · Account: the You card
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-7-account-the-you-card
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Account); `web/src/app/settings/contract.yaml`
- Touch: the Files list of Task F-7 in the plan
- Needs: B-5 (built). Devices left this plan: the account server and device management are a
  separate piece of work
- Done when: `you-card.test.tsx`, `settings-dialog.test.tsx` and `account.test.tsx` pass with codemap,
  `npm run typecheck`, `npm run lint`

### F-8 · Providers: label, presets from Core, test connection, model checklist
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-8-providers-label-presets-from-core-test-connection-model-checklist
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Providers); `web/src/app/agents/contract.yaml`;
  the request's Providers answer
- Touch: the Files list of Task F-8 in the plan (`web/src/api/schema.d.ts` is already regenerated: commit it, do not revert it)
- Needs: B-1, B-2, B-3 (built)
- Done when: step 1's `npm run typecheck` has 0 errors; `provider-setup.test.tsx`, `provider-picker.test.tsx`,
  `aylul.test.tsx`, `agent-env.test.tsx`, `companion-send.test.tsx`, `draft.test.tsx` pass with codemap,
  `npm run typecheck`, `npm run lint`

### F-9 · Agents: character colour per agent
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-9-agents-character-colour-per-agent
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Agents, Character colour);
  `web/src/app/agents/contract.yaml`; `web/src/app/desktop/contract.yaml`; `web/src/app/sidebar/contract.yaml`
- Touch: the Files list of Task F-9 in the plan
- Needs: F-6; `docs/shared/requests/2026-10-20-settings-dialog.md` (Agents: character colour)
  accepted; B-4 built and shown in `api/openapi.json`; U-2 in `docs/aylul/ui/stages.md` (the colour
  table) done
- Done when: `character-colors.test.ts`, `aylul.test.tsx` and `companion-binding.test.tsx` pass with
  codemap, `npm run typecheck`, `npm run lint`

### F-10 · Plugins & Skills: the shared library and each agent's picker
- Status: done
- Reviewed: steps released by the controller, 2026-10-21
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-10-plugins--skills-shared-library-and-immediate-agent-switches
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Plugins & Skills);
  `docs/shared/requests/2026-10-20-settings-dialog.md` (Plugins & Skills answer);
  `docs/backend/plans/2026-10-21-settings-dialog-backend.md` (B-9);
  `web/src/api/contract.yaml`, `web/src/app/settings/contract.yaml`, `web/src/app/agents/contract.yaml`
- Touch: the Files list of Task F-10 in the plan
- Needs: F-2, F-6, F-9 (done); accepted request and B-9 at `5a34995` (built);
  controller approval of these steps before implementation
- Done when: `plugins.test.tsx`, `agent-extensions.test.tsx`, `settings-dialog.test.tsx`,
  `aylul.test.tsx`, `agent-env.test.tsx`, codemap, `npm run typecheck` and `npm run lint` pass;
  library changes refresh all agent views, switches save independently, contracts/map and task
  commit are complete (full assertions and commands in the plan)

### F-11 · Usage section, by provider and by model
- Status: done
- Reviewed: steps released by the controller, 2026-10-21
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-11-usage-periods-provider-comparison-model-totals
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Usage);
  `docs/shared/requests/2026-10-20-settings-dialog.md` (Usage answer);
  `docs/backend/plans/2026-10-21-settings-dialog-backend.md` (B-7);
  `web/src/api/contract.yaml`, `web/src/app/settings/contract.yaml`
- Touch: the Files list of Task F-11 in the plan
- Needs: F-2 (done); accepted request and B-7 at `58b4138` (built);
  controller approval of these steps before implementation
- Done when: `usage.test.tsx`, `usage-period.test.ts`, `settings-dialog.test.tsx`, codemap,
  `npm run typecheck` and `npm run lint` pass; real period queries, snapshot titles, token bars
  and unknown-cost handling match the plan; contracts/map and task commit are complete

### F-12 · General: language and theme in Core with the person; start with Windows and tray native
- Status: todo
- Reviewed: steps released by the controller, 2026-10-22
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-12-general-person-preferences-and-desktop-behaviour
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (General);
  `docs/aylul/shared/requests/2026-10-21-settings-dialog-native.md` (accepted);
  `docs/aylul/native/plans/2026-10-21-settings-native.md` (N-7);
  `web/src/api/person.ts`, `web/src/api/saved-answer.ts`, settings/desktop contracts and frontend code map
- Touch: the Files list of Task F-12 in the plan
- Needs: B-5, N-7 at `cd7fd94`, F-7 and F-15 (built); controller approval of these steps
- Done when: `general-preferences.test.tsx`, `person-theme.test.tsx`, `desktop-preferences.test.tsx`,
  `settings-dialog.test.tsx`, `you-card.test.tsx`, `account.test.tsx`, codemap, typecheck and lint pass;
  complete person writes and actual native state match the plan, with current contracts/map and a task commit

### F-14 · Saving the companion agent repaints AYLUL at once
- Status: done
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-14-saving-the-companion-agent-repaints-aylul-at-once
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Character colour, how it reaches the companion); `web/src/desktop/contract.yaml`
- Touch: the Files list of Task F-14 in the plan
- Needs: F-9 (done); accepted `docs/aylul/shared/requests/2026-10-21-settings-dialog-native.md`;
  N-6 at `0c4d8c6` (built), refresh returns `{ ok: true }`
- Done when: `aylul.test.tsx` passes with codemap, typecheck, lint

### F-15 · A read sent before a save cannot overwrite the saved answer
- Status: done
- Plan: none; the problem and the fix are here. Found and fixed once in `you-card.tsx` (`79aa898`):
  a query read sent before a mutation answers after it, and its old data overwrites the answer
  the mutation just wrote with `setQueryData`. The screen then shows the old value although Core
  saved the new one. `you-card.test.tsx` ("a read sent before the save cannot bring the old name
  back") reproduces it by holding the second GET across the save.
- Fix: one function, `writeSaved(client, queryKey, updater)` in a new
  `web/src/api/saved-answer.ts`, that awaits `client.cancelQueries({ queryKey })` and then calls
  `client.setQueryData`. Every place below that writes a mutation's answer into the cache uses it
  (and `you-card.tsx` moves to it too). A place that is not writing a server answer after a
  mutation (a seeded cache, a list read) is left as it is and named in the report with the reason.
- Read: `web/src/api/contract.yaml`; `docs/frontend/codebase/README.md`; `web/src/app/settings/you-card.tsx`
  and its test
- Touch: `web/src/api/saved-answer.ts` (new), `web/src/api/saved-answer.test.ts` (new),
  `web/src/api/contract.yaml`, `docs/frontend/codebase/README.md`, `docs/frontend/stages.md`, and
  only these callers: `web/src/app/settings/you-card.tsx`, `web/src/app/settings/active-projects.tsx`,
  `web/src/app/agreements/agreements-page.tsx`, `web/src/app/agreements/binding-editor.tsx`,
  `web/src/app/conversation/cli-picker.tsx`, `web/src/app/conversation/composer.tsx`,
  `web/src/app/conversation/draft.tsx`, `web/src/app/conversation/use-commands.ts`,
  `web/src/app/conversation/use-session.ts`, `web/src/app/design/standards-view.tsx`,
  `web/src/app/project-settings/code-index.tsx`, `web/src/app/project-settings/external-agents.tsx`,
  `web/src/app/project-settings/instructions-editor.tsx`, `web/src/app/project-settings/project-modes.tsx`,
  `web/src/app/project-settings/remove-project.tsx`, `web/src/app/sidebar/delete-thread-dialog.tsx`,
  and the `contract.yaml` of each folder whose rule changes
- Needs: none
- Done when: `saved-answer.test.ts` shows a held read cannot overwrite a written answer (and fails
  without the cancel); `you-card.test.tsx` and the existing tests of every changed caller pass;
  `npx vitest run src/codemap.test.ts`, `npm run typecheck`, `npm run lint` pass. Not the whole suite.
