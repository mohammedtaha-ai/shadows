# Desktop UI agents

Rules for **Masada** (the desktop UI lead) and the two agents she runs,
`frontend-builder` (Sonnet) and `frontend-fixer` (Opus). Since 2026-10-19 (the
human's ruling) Masada leads this side as well as the Shadows web client
([`docs/frontend/AGENTS.md`](../../frontend/AGENTS.md)); the former
`desktop-ui-lead`/`-builder`/`-fixer` roles are retired. The repository rules in
[`AGENTS.md`](../../../apps/desktop/AGENTS.md) still apply; this file adds who does what.
The ordinary session in this repository is the **teacher**: it oversees the
leads, rules on disputed requests, and starts new features.

## The domain

Yours:
- `apps/desktop/src/` except `features/companion/`
- `apps/desktop/tests/`, `apps/desktop/index.html`
- `docs/aylul/ui/` (plans, specs, stages), and the `contract.yaml` files and
  code-map rows of the code above
- the character colour (Mohammed's ruling, 2026-10-21): the colour table
  `assets/aylul/character-colors.json`, `features/companion/renderer/palette.ts`, and the passes
  in `features/companion/` that make the renderer draw from that palette (no shape, motion,
  expression or outfit change)

Not yours: `assets/aylul/` and `features/companion/` (character), `src-tauri/`, `sidecar/`, `hook/` (native); `../shadows-backend` (Hamoud);
`docs/superpowers/` (history: read it, add nothing new). `docs/aylul/vision.md` is changed
only through the teacher.

## Where things are

1. [`docs/aylul/vision.md`](../vision.md): the experience.
2. [`docs/aylul/codebase/README.md`](../codebase/README.md): what each implemented
   module owns. Read it before opening code.
3. `contract.yaml` next to the code
   ([convention](../../../../shadows-backend/docs/contracts/README.md)).
   Read it before changing a module.
4. `docs/aylul/ui/specs/` and `docs/aylul/ui/plans/`: new work.
5. [`docs/aylul/ui/stages.md`](stages.md): where the work is now. The only
   place a builder learns its task from.

## A task in `stages.md`

```markdown
### U-3 · <one-line goal>
- Status: todo | doing | done | blocked
- Plan: docs/aylul/ui/plans/<file>.md#<section>
- Read: <contract.yaml / spec section / code-map row the task depends on>
- Touch: <the exact files the task may change>
- Needs: <shared request it depends on, or "none">
- Done when: <the test(s) or command that must pass>
```

**`stages.md` holds only what is left.** A `done` task stays while its branch is open, so the
reviewer sees it. When the branch's PR merges, `docs-sync` deletes every `done` task that PR
delivered, and a plan whose tasks are all done; its commit names the PR, and Git keeps them
(cite them as "`<path>` at `<commit>`"). No "Done" section is kept.

## Rules for the builder and the fixer

1. Read this file, then the task, then only what the task's `Read:` names.
   Open a file only at the symbol you need. Do not crawl the tree.
2. Change only the files in `Touch:`. If the work needs another file, stop and
   report it: the lead updates the task, you do not widen it yourself.
3. A changed module updates its `contract.yaml`, and a new or removed module its
   code-map row, in the same commit.
4. Checks: From `apps/desktop/`: `npm run build` (typecheck + bundle), `npm run typecheck:tests` and the tests the task names (`node --test <file>`). The desktop CI gate runs `npm test`, `npm run test:companion:unit` and `npm run test:companion:production`; a separate browser job runs `npm run test:companion:browser`. `RECORD_PURPLE`, `EXPORT_COLORS` and native acceptance scripts stay opt-in. Never the whole suite while working.
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
