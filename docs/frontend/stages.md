# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Work and improvements continue on branch `improvements`. The settings dialog,
F-2 to F-9, is implemented and accepted by Mohammed in the app. F-10 and F-11 now have
steps against the built API, waiting for the controller's review. F-12 and F-14 retain
their recorded blockers.

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
- Status: blocked
- Blocker: Canonical OpenAPI gives both DELETE /api/aylul/agents/{slug}/env/{name} and DELETE /api/aylul/extensions/{kind}/{id} the operationId remove; generated operations["remove"] resolves to the environment route's { slug, name }, so the typed extension DELETE fails typecheck (TS2353). The backend must publish unique operation IDs and regenerate OpenAPI/types before F-10 can pass; do not hand-edit schema.d.ts or cast around the contract.
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
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#tasks-that-wait-on-the-request (its steps are written when the request is answered)
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md`
- Touch: to be named with the steps
- Needs: `docs/shared/requests/2026-10-20-settings-dialog.md` (General) accepted; B-5 built; the native
  switches through an AYLUL shared request to the native side
- Done when: to be named with the steps

### F-14 · Saving the companion agent repaints AYLUL at once
- Status: blocked
- Plan: docs/frontend/plans/2026-10-21-settings-dialog.md#task-f-14-saving-the-companion-agent-repaints-aylul-at-once
- Read: `docs/frontend/specs/2026-10-20-settings-dialog-design.md` (Character colour, how it reaches the companion); `web/src/desktop/contract.yaml`
- Touch: the Files list of Task F-14 in the plan
- Needs: F-9; the native `refresh_companion_appearance` command (AYLUL shared request, not yet filed) built
- Done when: `aylul.test.tsx` passes with codemap, typecheck, lint
