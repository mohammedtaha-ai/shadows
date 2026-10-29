# Backend agents

Rules for **Hamoud** (the backend lead) and the two agents he runs,
`backend-builder` (Sonnet) and `backend-fixer` (Opus). The project rules in
[`CLAUDE.md`](../../CLAUDE.md) still apply; this file adds who does what on the
backend side. The frontend has its own file,
[`docs/frontend/AGENTS.md`](../frontend/AGENTS.md); read it only when a shared
request needs it.

This side is the **Shadows Core backend**. AYLUL/native and independent
code-intelligence product work follow [the documentation map](../README.md),
not this side merely because they also run background code.

## The domain

| Yours | Not yours |
|---|---|
| `crates/`, `api/openapi.json` | `web/` (Masada's side) |
| `docs/backend/` (plans, specs, stages) | `docs/frontend/` |
| `docs/codebase/` (the backend code map) and every `crates/**/contract.yaml` | `docs/superpowers/` (history: read it, add nothing new) |
| answers in `docs/shared/requests/` | another side's files, even to "help" |

## Where things are

0. **`crates/shadows-core/src/app.rs` first, always.** `AppCore` names every
   service, its one job and its contract. Go to the service that owns your
   change and work only there. Reach nothing (a store, live sessions, running
   turns) except through `AppCore` and its owner. If no service owns it, stop
   and report. This is the strict rule in the root `AGENTS.md`; breaking it is
   refused in review.
1. [`docs/codebase/README.md`](../codebase/README.md): what each crate outside
   Core owns.
2. `crates/shadows-core/src/<service>/contract.yaml`: what must stay true of
   that service. Read it before changing the service.
3. `docs/backend/specs/`: decisions for new backend work.
   Older decisions stay in `docs/superpowers/specs/` (the index is its
   `README.md`).
4. `docs/backend/plans/`: one plan per piece of work, broken into tasks.
5. [`docs/backend/stages.md`](./stages.md): where the work is now, and the next
   task. The only place a builder learns its task from.

## The roles

**Hamoud (lead).** Plans with the user, writes specs, plans and `stages.md`,
answers shared requests, and runs the builder and the fixer. He writes no
product code. He reports to the user in Arabic, short.

**backend-builder.** Is started with one line: `Do task B-<n> in
docs/backend/stages.md`. Nothing else is needed; the task says it all.

**backend-fixer.** Is started with one line naming a failed task, a failing
test, or a review finding, and where it is written down.

## A task in `stages.md`

Hamoud writes every task in this shape, so a builder needs no other prompt:

```markdown
### B-3 · <one-line goal>
- Status: todo | doing | done | blocked
- Plan: docs/backend/plans/<file>.md#<section>
- Read: <contract.yaml / spec section / code-map row the task depends on>
- Touch: <the exact files the task may change>
- Done when: <the test(s) that must pass, by name or command>
```

**`stages.md` holds only what is left.** A `done` task stays while its branch is open, so the
reviewer sees it. When the branch's PR merges, `docs-sync` deletes every `done` task that PR
delivered, and a plan whose tasks are all done; its commit names the PR, and Git keeps them
(cite them as "`<path>` at `<commit>`"). No "Done" section is kept.

## Rules for the builder and the fixer

1. Read this file, then the task, then only what the task's `Read:` names.
   Find a symbol with the LSP (or `where_is` when the `shadows` MCP server is
   connected) and open the file at that line. Do not crawl the tree.
2. Change only the files in `Touch:`. If the work needs another file, stop and
   report it: Hamoud updates the task, you do not widen it yourself.
3. A change to a service updates its `contract.yaml` in the same commit.
4. A change to a route or to a type the HTTP API serializes changes what the
   frontend sees. Make it only when an accepted shared request in
   `docs/shared/requests/` covers it, then regenerate the document:
   `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
5. Run the tests the task names, never the whole suite (the full gate in
   `CLAUDE.md` runs once, before the PR). Before the commit, also run what CI
   refuses a commit for:
   - `cargo fmt --all --check`;
   - `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
     and `cargo clippy --workspace -- -D warnings`;
   - `cargo test -p shadows --test architecture --test codemap`;
   - no `.rs` line over 100 characters, since rustfmt leaves a line it cannot break:
     `find crates -name '*.rs' -print0 | LC_ALL=C xargs -0 awk '{ s = $0; sub(/\r$/, "", s);
     gsub(/[\200-\277]/, "", s) } length(s) > 100 { print FILENAME":"FNR }'` prints nothing.
   A deleted or moved file also loses its code-map row and any entry in
   `crates/shadows/tests/architecture/ownership.allowlist`; an entry is only ever removed.
6. Commit with `git add <each file>`, never `git add -A` or `git add .`:
   Masada's builders may be committing to the same branch. Do not switch
   branches, stash, rebase or push. End the message with the `Co-Authored-By`
   line of the model you run on.
7. Set the task's `Status:` in `stages.md` and report: what changed, the test
   result, and anything you found but did not change.

## Shared requests

Masada writes a request in `docs/shared/requests/` when the frontend needs
something from the API. Hamoud answers it in the file's "Backend answer"
section: can do as asked, can do with a change, or cannot (and why). The cycle
is in [`docs/shared/README.md`](../shared/README.md).
