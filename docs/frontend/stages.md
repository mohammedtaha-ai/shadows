# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Work and improvements continue on branch `improvements`. Next: the settings dialog,
F-2 to F-6 (F-7 to F-12 wait on the settings dialog request).

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
- Status: todo
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-2-dialog-shell-sidebar-of-sections-general
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`; `web/src/app/settings/contract.yaml`; `web/src/app/new-project/new-project-dialog.tsx`
- Touch: the Files list of Task F-2 in the plan
- Needs: none
- Done when: `settings-dialog.test.tsx` passes with codemap, `npm run typecheck`, `npm run lint`

### F-3 · Account shows only the actions its state allows
- Status: todo
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-3-account-shows-only-the-actions-its-state-allows
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Account); `web/src/app/desktop/contract.yaml`
- Touch: the Files list of Task F-3 in the plan
- Needs: F-2
- Done when: `account-actions.test.ts` and `account.test.tsx` pass with codemap, typecheck, lint

### F-4 · AYLUL character and Local Core sections
- Status: todo
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-4-aylul-character-and-local-core-sections
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (AYLUL character, Local Core); `web/src/app/desktop/contract.yaml`
- Touch: the Files list of Task F-4 in the plan
- Needs: F-2
- Done when: `companion-binding.test.tsx` and `core.test.tsx` pass with codemap, typecheck, lint

### F-5 · Providers live in the dialog, with search and presets
- Status: todo
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-5-providers-section-with-search-and-presets
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Providers); `web/src/app/agents/contract.yaml`
- Touch: the Files list of Task F-5 in the plan
- Needs: F-2
- Done when: `aylul.test.tsx` and `provider-picker.test.tsx` pass with codemap, typecheck, lint

### F-6 · Agents in the dialog; Agent settings and New agent open it
- Status: todo
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-6-agents-section-agent-settings-and-new-agent-open-it
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Agents, the dialog's rulings); `web/src/app/agents/contract.yaml`;
  `web/src/app/sidebar/contract.yaml`
- Touch: the Files list of Task F-6 in the plan
- Needs: F-2, F-5 (both change `agent-list.tsx` and `home.tsx`)
- Done when: `aylul.test.tsx` and `agent-env.test.tsx` pass with codemap, typecheck, lint

### F-7 · Account: the You and Devices cards
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (Account: the person and their devices) accepted and built
- Done when: to be named with the steps

### F-8 · Providers: label, test connection, model checklist, per-model context
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (Providers: label, test, models) accepted and built
- Done when: to be named with the steps

### F-9 · Agents: character colour per agent
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (Agents: character colour) accepted and built
- Done when: to be named with the steps

### F-10 · Plugins & Skills section
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (Plugins & Skills) accepted and built
- Done when: to be named with the steps

### F-11 · Usage section, by provider and by model
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (Usage) accepted and built
- Done when: to be named with the steps

### F-12 · General: language, theme, start with Windows, tray
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (General) accepted and built
- Done when: to be named with the steps
