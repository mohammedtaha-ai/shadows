# AYLUL documentation scope

Read [README.md](README.md) first, then the [brain design](specs/2026-10-19-aylul-claude-code-brain-design.md).
The source map is [codebase/README.md](codebase/README.md); desktop source lives
in `apps/desktop/`, not in Core crates or `web/`.

`character/`, `native/` and `ui/` retain separate design, plan and stage owners.
`shared/` holds cross-side requests. `specs/` holds the AYLUL designs that span
more than one side.
Historical acceptance is under `docs/evidence/` at `0187365`; do not rewrite dated facts.
Amend a decision in its owner rather than copying it into another document.
Follow [desktop instructions](../../apps/desktop/AGENTS.md) for implementation.

## Who does what (shared rules for the three sides)

Each side's `AGENTS.md` adds its own domain and checks. These rules apply to all of them.

Roles: `aylul-lead` (character), `native-lead` (native), `masada` (desktop UI). Each runs
a builder (Sonnet) and a fixer (Opus). The ordinary session is the **teacher**: it
oversees the leads, rules on disputed requests, starts new features, and is the only
one who changes `docs/aylul/vision.md` or `docs/aylul/future.md`.

### Where things are

1. [`docs/aylul/vision.md`](vision.md): the experience.
2. [`docs/aylul/codebase/README.md`](codebase/README.md): what each implemented module owns.
   Read it before opening code.
3. `contract.yaml` next to the code. Read it before changing a module.
4. The side's `specs/` and `plans/`: new work.
5. The side's `stages.md`: where the work is now. The only place a builder learns its task from.

### A task in `stages.md`

```markdown
### <ID> · <one-line goal>
- Status: todo | doing | done | blocked
- Plan: docs/aylul/<side>/plans/<file>.md#<section>
- Read: <contract.yaml / spec section / code-map row the task depends on>
- Touch: <the exact files the task may change>
- Needs: <shared request it depends on, or "none">
- Done when: <the test(s) or command that must pass>
```

`<ID>` is the side's letter and a number: `C-` character, `N-` native, `U-` desktop UI.

**`stages.md` holds only what is left.** A `done` task stays while its branch is open, so the
reviewer sees it. When the branch's PR merges, `docs-sync` deletes every `done` task that PR
delivered, and a plan whose tasks are all done; its commit names the PR, and Git keeps them
(cite them as "`<path>` at `<commit>`"). No "Done" section is kept.

### Rules for the builder and the fixer

1. Read the side's `AGENTS.md`, then the task, then only what the task's `Read:` names.
   Open a file only at the symbol you need. Do not crawl the tree.
2. Change only the files in `Touch:`. If the work needs another file, stop and
   report it: the lead updates the task, you do not widen it yourself.
3. A changed module updates its `contract.yaml`, and a new or removed module its
   code-map row, in the same commit.
4. Checks: the side's own checks in its `AGENTS.md`. Never the whole suite while working.
5. Commit with `git add <each file>`, never `git add -A` or `git add .`:
   other sides may be committing to the same branch. Do not switch branches,
   stash, rebase or push.
6. Set the task's `Status:` in `stages.md` and report: what changed, the test
   result, and anything you found but did not change.

### Shared requests

- Between sides of this repository (a Tauri command, a sidecar message, a
  companion hook into the UI): `docs/aylul/shared/requests/`, cycle in
  [`docs/aylul/shared/README.md`](shared/README.md).
- A delivered request is deleted. Its rules move into the owner spec or the owner's
  `contract.yaml` before deletion, and references to it become "`<path>` at `<commit>`".
- To the cloud API: `../shadows-backend/docs/shared/requests/`, answered by Hamoud.
  This applies to later work (remote alerts, cloud relay), not to the local desktop.
