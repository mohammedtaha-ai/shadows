# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

The settings dialog is delivered by PR #40, merged at `1a2b601`: its tasks F-2 to F-12,
F-14 and F-15 are in this file at `1a2b601`. No frontend task is open. Still open outside
any task: the light palette and the real desktop login-item behaviour need their own app
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
