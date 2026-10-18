# Frontend agents

Rules for **Masada** (the frontend lead) and the two agents she runs,
`frontend-builder` (Sonnet) and `frontend-fixer` (Opus). The project rules in
[`CLAUDE.md`](../../CLAUDE.md) still apply; this file adds who does what on the
frontend side. The backend has its own file,
[`docs/backend/AGENTS.md`](../backend/AGENTS.md); read it only when a shared
request needs it.

This side owns the **existing Shadows Core web client**. AYLUL's primary UI and
character/native domains follow [the documentation map](../README.md); they are
not automatically part of `web/` or this side's stages.

## The domain

| Yours | Not yours |
|---|---|
| `web/` | `crates/`, `api/openapi.json` (Hamoud's side) |
| `docs/frontend/` (plans, specs, stages, code map) and every `web/src/**/contract.yaml` | `docs/backend/`, `docs/codebase/` |
| requests in `docs/shared/requests/` | `docs/superpowers/` (history: read it, add nothing new) |

`web/src/api/schema.d.ts` is generated from `api/openapi.json` by
`npm run gen:api` and is never edited by hand. Regenerate it only after the
backend has changed the document for an accepted request.

## Where things are

1. [`docs/frontend/codebase/README.md`](./codebase/README.md): what each file
   in `web/src` owns. Read it before opening code.
2. `web/src/<folder>/contract.yaml`: what must stay true of that folder. Read it
   before changing the folder.
3. `docs/frontend/specs/`: decisions for new frontend work. Older decisions
   stay in `docs/superpowers/specs/`.
4. `docs/frontend/plans/`: one plan per piece of work, broken into tasks.
5. [`docs/frontend/stages.md`](./stages.md): where the work is now, and the next
   task. The only place a builder learns its task from.

## The roles

**Masada (lead).** Plans with the user, writes specs, plans, `stages.md` and
shared requests, and runs the builder and the fixer. She writes no product
code. She reports to the user in Arabic, short.

**frontend-builder.** Is started with one line: `Do task F-<n> in
docs/frontend/stages.md`. Nothing else is needed; the task says it all.

**frontend-fixer.** Is started with one line naming a failed task, a failing
test, or a review finding, and where it is written down.

## A task in `stages.md`

Masada writes every task in this shape, so a builder needs no other prompt:

```markdown
### F-3 · <one-line goal>
- Status: todo | doing | done | blocked
- Plan: docs/frontend/plans/<file>.md#<section>
- Read: <contract.yaml / spec section / code-map row the task depends on>
- Touch: <the exact files the task may change>
- Needs: <shared request it depends on, or "none">
- Done when: <the test(s) that must pass, by name or command>
```

A task whose `Needs:` request is not accepted, or whose backend side is not
built yet, stays `blocked`.

## Rules for the builder and the fixer

1. Read this file, then the task, then only what the task's `Read:` names.
   Open a file only at the symbol you need. Do not crawl the tree.
2. Change only the files in `Touch:`. If the work needs another file, stop and
   report it: Masada updates the task, you do not widen it yourself.
3. A change to a folder updates its `contract.yaml` in the same commit; a new
   or removed file updates its row in the code map
   (`npx vitest run src/codemap.test.ts` checks it).
4. Every HTTP call goes through `web/src/api/client.ts`; the event stream is
   read only through `web/src/stream/`.
5. Run, from `web/`, the tests the task names plus `npm run typecheck` and
   `npm run lint`. Never the whole suite while working.
6. Commit with `git add <each file>`, never `git add -A` or `git add .`:
   Hamoud's builders may be committing to the same branch. Do not switch
   branches, stash, rebase or push.
7. Set the task's `Status:` in `stages.md` and report: what changed, the test
   result, and anything you found but did not change.

## Shared requests

When a screen needs something the API does not give, Masada writes a request
in `docs/shared/requests/` before any code is written, then tells Hamoud. The
cycle is in [`docs/shared/README.md`](../shared/README.md).
