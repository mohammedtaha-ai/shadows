# Character (AYLUL) agents

Rules for `aylul-lead` (the AYLUL character lead) and the two agents it runs,
`character-builder` (Sonnet) and `character-fixer` (Opus). The repository rules in
[`AGENTS.md`](../../../apps/desktop/AGENTS.md) still apply; this file adds who does what.
The ordinary session in this repository is the **teacher**: it oversees the
leads, rules on disputed requests, and starts new features.

## The domain

Yours:
- `assets/aylul/` (art, states, animations, candidates)
- `apps/desktop/src/features/companion/` (AYLUL presence; created by this side)
- Except, by Mohammed's ruling of 2026-10-21: the colour table `assets/aylul/character-colors.json`
  and the renderer's colours (`features/companion/renderer/palette.ts` and the colour values in
  the other `renderer/` files) belong to the AYLUL UI
  side (U-2, U-3 in `../ui/stages.md`).
- `docs/aylul/character/` (plans, specs, stages), and the `contract.yaml` files and
  code-map rows of the code above

Not yours: `apps/desktop/src/` outside `features/companion/` (desktop UI), `src-tauri/`, `sidecar/`, `hook/` (native); `../shadows-backend` (Hamoud);
`docs/superpowers/` (history: read it, add nothing new). `docs/aylul/vision.md` is changed
only through the teacher.

## Where things are

1. [`docs/aylul/vision.md`](../vision.md): the experience.
2. [`docs/aylul/codebase/README.md`](../codebase/README.md): what each implemented
   module owns. Read it before opening code.
3. `contract.yaml` next to the code
   ([convention](../../../../shadows-backend/docs/contracts/README.md)).
   Read it before changing a module.
4. `docs/aylul/character/specs/` and `docs/aylul/character/plans/`: new work.
5. [`docs/aylul/character/stages.md`](stages.md): where the work is now. The only
   place a builder learns its task from.

## A task in `stages.md`

```markdown
### C-3 · <one-line goal>
- Status: todo | doing | done | blocked
- Plan: docs/aylul/character/plans/<file>.md#<section>
- Read: <contract.yaml / spec section / code-map row the task depends on>
- Touch: <the exact files the task may change>
- Needs: <shared request it depends on, or "none">
- Done when: <the test(s) or command that must pass>
```

## Rules for the builder and the fixer

1. Read this file, then the task, then only what the task's `Read:` names.
   Open a file only at the symbol you need. Do not crawl the tree.
2. Change only the files in `Touch:`. If the work needs another file, stop and
   report it: the lead updates the task, you do not widen it yourself.
3. A changed module updates its `contract.yaml`, and a new or removed module its
   code-map row, in the same commit.
4. Checks: From `apps/desktop/`: `npm run build` and the tests the task names (`node --test <file>`). An asset change names the review it needs in `assets/aylul/README.md`. Never the whole suite while working.
5. Commit with `git add <each file>`, never `git add -A` or `git add .`:
   other sides may be committing to the same branch. Do not switch branches,
   stash, rebase or push.
6. Set the task's `Status:` in `stages.md` and report: what changed, the test
   result, and anything you found but did not change.

## Shared requests

- Between sides of this repository (a Tauri command, a sidecar message, a
  companion hook into the UI): `docs/aylul/shared/requests/`, cycle in
  [`docs/aylul/shared/README.md`](../shared/README.md).
- To the cloud API: `../shadows-backend/docs/shared/requests/`, answered by Hamoud.
