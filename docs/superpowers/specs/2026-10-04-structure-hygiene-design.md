# Structure hygiene — what the gate misses, and what duplication cost

**Date:** 2026-10-04
**Status:** Draft for Mohammed's review. Nothing here is implemented.
**Branch:** `chore/gate-dedup-cleanup`, derived from
`codex/shared-contracts-planning`.

This document owns four changes to the Shadows repository that change no
behaviour: one gate gap, one duplicated algorithm, one dead export, and six
finished plan documents. It is a spec because the CI gate is a decision
others depend on, not because any code shape needed designing — §1's finding
is a measurement, and §2's is a reading of code that already exists.

Terms used here follow the ordinary sense in
[`../codebase/README.md`](../../codebase/README.md): a **store** is a service's
SQLite queries, an **adapter** is `shadows-http`, `shadows-mcp` or the binary,
and a **service** is one of `AppCore`'s members.

---

## 1. The format gate does not run on 14 files

### 1.1 What was measured

`cargo fmt --all --check` exits 0 on this repository. `rustfmt` nevertheless
leaves 86 lines across 14 files exactly as a human wrote them, including
`crates/shadows-core/src/design/store/vision.rs:90`, which is 221 characters
long and which no `rustfmt` configuration would produce.

The cause is `rustfmt`'s documented behaviour, not a defect in the repository's
configuration: when a block contains a line that cannot be brought within
`max_width`, `rustfmt` emits that whole block verbatim instead of formatting
it. The block here is `edit_design`'s body — `write_txn`'s closure, then
`Box::pin(async move { … })`, then a `match` whose arm at line 90 is a
221-character or-pattern. Every statement in that body is therefore
unformatted, and `--check` has nothing to report.

This was confirmed by injecting a deliberately misformatted statement into
that body (`let mut changed   =   BTreeSet :: new ();`) and observing
`rustfmt --check` still exit 0, while the same injection outside that body was
detected. Rewriting the over-long arm so that every line fits restored
detection.

The 14 files and their unformattable line counts:

| File | Lines | Unformattable |
|---|---|---|
| `crates/shadows-core/src/lib.rs` | 61 | 19 |
| `crates/fake-acp/src/main.rs` | 359 | 12 |
| `crates/shadows-core/src/turns/mod.rs` | 474 | 9 |
| `crates/shadows-core/src/turns/turn.rs` | 300 | 8 |
| `crates/shadows-core/src/design/store/mod.rs` | 7 | 7 |
| `crates/shadows-core/src/design/mod.rs` | 52 | 7 |
| `crates/shadows-core/src/design/store/vision.rs` | 136 | 5 |
| `crates/shadows-http/src/design.rs` | 109 | 5 |
| `crates/shadows-http/src/lib.rs` | 209 | 5 |
| `crates/shadows-core/src/events/mod.rs` | 98 | 3 |
| `crates/shadows-core/src/events/subscription.rs` | 292 | 3 |
| `crates/shadows-core/src/design/store/part_edit.rs` | 130 | 1 |
| `crates/shadows-core/src/design/store/parts.rs` | 117 | 1 |
| `crates/shadows-core/src/turns/shutdown.rs` | 163 | 1 |

Widening the measurement to every line over 100 characters, not only those
`rustfmt` gives up on, finds **266**: 130 in product code and 136 in tests.
Most are macro invocations — `json!`, `sqlx::query`, `tracing::warn!` — which
`rustfmt` will not reflow, so each needs a manual line break.

### 1.2 What is decided

**A line-length check joins the gate.** `.github/workflows/ci.yml` gains a
step after `Format`, in both the `windows` and `linux` jobs, that fails when
any `crates/**/*.rs` line exceeds 100 characters and names the file, line and
width. This catches the *cause*, which is what makes `cargo fmt --check`
silently skip; the existing step keeps its job of catching ordinary drift.

The check is a shell step rather than a `rustfmt.toml` setting because
`error_on_line_overflow` is an unstable option: on stable it prints
`Warning: can't set error_on_line_overflow = true` and continues. Verified on
the toolchain this repository develops against. `rustfmt.toml` is not added.

**The 266 lines are broken before the check is added.** Adding the check first
would fail CI on the first commit for a reason unrelated to any change under
review. The order is: break the lines, commit, add the check, commit.

**No behaviour changes.** Breaking a line is a formatting change; every edit
must leave `cargo clippy --workspace --all-targets --features
fake-acp/test-support -- -D warnings` and `cargo test --workspace` passing.

**No new toolchain.** The check is `awk`, available on both runners.

### 1.3 What is not decided

Whether the check should carry an exception list for lines that cannot be
broken, such as a generated file or a `#[utoipa::path(...)]` attribute. At the
time of writing, all 266 are in hand-written code, so the question has not
arisen. **OPEN — trigger:** the first commit that must add an exception to
`ci.yml`'s new step.

---

## 2. `part-editor.tsx` and `outcome-editor.tsx` are one algorithm

### 2.1 What was read

`part-editor.tsx` (116 lines) and `outcome-editor.tsx` (121 lines) hold, line
for line:

- the same `base` / `draft` / `linked` / `moving` / `order` state;
- the same `createRevision` pinned on first sight so a refetch cannot advance a
  retry's basis (`part-editor.tsx:29`, `outcome-editor.tsx:31`);
- the same `setState`-during-render adoption of a newer revision when the
  editor is not dirty (`part-editor.tsx:33-35`, `outcome-editor.tsx:35-37`);
- the same three-way `newer` condition (`part-editor.tsx:36`, `outcome-editor.tsx:38-39`);
- the same `onSuccess` ordering, in which `attempt.current = null` runs *after*
  the follow-up read resolves and *before* any state is advanced
  (`part-editor.tsx:44-46`, `outcome-editor.tsx:46-48`);
- the same `REVISION_CONFLICT`-only `onError` guard;
- the same two-branch `reload` with `fetchQuery({ …, staleTime: 0 })`.

The differences are the type names, the query names, two `parts` lines that
only `outcome-editor.tsx` has, and one comment.

That ordering at `outcome-editor.tsx:46-48` is load-bearing and is pinned by
`roadmap-view.test.tsx:63-100`: if the follow-up read fails, the attempt must
not be cleared, so a retry reuses the same `command_id` and the same
`expected_revision` and the daemon's idempotency replays the committed edit
rather than making a second one.

### 2.2 What is decided

**One hook, `useDesignEditor`, owns the algorithm.** `part-editor.tsx` and
`outcome-editor.tsx` become thin components over it. It is parameterised by:

| Parameter | Why it differs |
|---|---|
| `entityKind: 'Part' \| 'Outcome'` | the op kinds and the plan-link anchor |
| `idOf`, `contentOf`, `empty` | `.part` against `.outcome` |
| `detailQuery`, `listQuery` | `partQuery` against `outcomeQuery` |
| `relations` | `Outcome` carries an extra `OutcomePartPut`/`OutcomePartRemove` pair |

The hook owns the three behaviours above in this order, unchanged: the
`attemptFor` fingerprint over `{expected_revision, ops}`, clearing
`attempt.current` after the follow-up read and before any `setState`, and the
`REVISION_CONFLICT`-only `onError` guard.

**The hook has no create/update branch beyond the one both files already
share.** Both already have it at `part-editor.tsx:42` and
`outcome-editor.tsx:44`; the hook carries it as-is rather than adding a mode.

**`vision-editor.tsx` is not folded in.** It is a different algorithm: no
create mode, `setQueryData` with a narrow query key instead of a re-read, a
per-field `===` dirty comparison instead of `JSON.stringify`, and a bespoke
mutation variable type. Forcing it in would put two code paths behind one
parameter, which is what "No layer before its first user" in `CLAUDE.md`
warns against.

**`use-design-pages.ts` is not folded in.** Its revision test is an equality
on pagination coherence (`use-design-pages.ts:12,19`), while the editors test
an ordering on concurrent writes. It has no `base`/`draft` pair, no
`command_id`, and no `REVISION_CONFLICT`. Its three-line wrappers
`use-part-pages.ts` and `use-outcome-pages.ts` are already the correct
de-duplication over the one generic.

### 2.3 Testing

`roadmap-view.test.tsx` and `parts-view.test.tsx` must pass unchanged. They
are the pins on the behaviour above, and a refactor that keeps them green has
kept it. No new test is written for the hook: the two editors' existing
integration tests exercise it through the components that use it, which is how
every other behaviour in this area is tested.

---

## 3. `api/client.ts` has a dead export, and `api/design.ts` a second client

### 3.1 What was read

`api/design.ts:32` and `api/client.ts:55` construct `openapi-fetch` clients
that are byte-for-byte identical:

```ts
const client = createClient<paths>({ baseUrl: DAEMON_URL, fetch: (request) => fetch(request) })
```

`client.ts:1-4` states that it is the only door to the daemon; `design.ts`
repeats it. `design.ts` uses its own `client` at lines 20, 23, 27 and 30,
which are above its declaration at line 32 — safe only because each use is
inside a function body that runs after module evaluation.

`api/client.ts:25` exports `type LimitWindow = Schemas['LimitWindow']`. No
module imports it. `stream/frames.ts:48` declares a second `LimitWindow` with
`resetsAt`, and `context-ring.tsx:15` imports that one.

### 3.2 What is decided

**`design.ts` imports `client` from `client.ts`.** `client.ts:55` gains
`export`; `design.ts` loses its `createClient` import, its `client`
declaration, and its now-unused `paths` and `DAEMON_URL` imports. Nothing
outside these two files changes: every consumer imports a named function or
type, never the client object. Both clients have the same `baseUrl`, the same
late-bound `fetch` and the same un-narrowed `paths` generic, so this is not a
behaviour change. `error.test.ts:9` keeps its own client — it injects a fixed
answer function and is not duplication.

**The dead export at `client.ts:25` is deleted.** `toLimits` at
`stream/frames.ts:205` stays: it converts the wire's `resets_at` to the UI's
`resetsAt`, and both spellings are load-bearing. What is removed is only a
type alias nobody reads.

**This is smaller than first reported.** The two `LimitWindow` declarations
are not the same thing under two names — one is the wire shape and one is the
UI shape, and the conversion between them is the point. The confusion was in
a dead export sharing a name with a live one, not in the conversion.

---

## 4. Six finished plans

### 4.1 What was measured

`docs/` is 20,986 lines against 20,892 lines of product Rust — a ratio of
1.00:1. `docs/superpowers/plans/` holds 10,173 of those lines in 8 files. Six
of the eight describe work that is on `main`:

| Plan | Merged |
|---|---|
| `2026-09-21-milestone-0-browser-planner.md` | PR #1–#3 |
| `2026-09-24-milestone-1-harness-controls.md` | PR #4 |
| `2026-09-25-milestone-2-plan-workflow.md` | PR #6 |
| `2026-09-26-milestone-2-5-app-core.md` | PR #7 |
| `2026-09-30-milestone-3-code-index.md` | PR #9 |
| `2026-10-01-project-plans-1a.md` | PR #14 |

Two remain: `2026-10-03-planning-workspace-stage-1.md`, describing work in
flight on this branch, and `2026-10-03-project-planning-roadmap.md`, describing
work not yet started. Both stay.

`CLAUDE.md` already applies this rule to code — "Spike code is deleted once its
evidence file is written" — and it was applied to the execution ledgers, which
were removed before their merges. It was not applied to the plans themselves.

### 4.2 What is decided

**The six merged plans are deleted.** They describe a state six weeks old, and
a reader who finds `milestone-2-plan-workflow.md` may reasonably take it for
current. Their content is not lost: the specs indexed by
[`README.md`](./README.md) own every
decision they made, and `docs/evidence/` holds their measurements.

**The five links in `docs/status.md` are amended in place.** `status.md`
points at four of the six by path, at lines 15, 102, 114, 124 and 153. Each
becomes a reference to the owning spec section, which is where the decision
actually lives — `§11`, `§13`, `§14`, `§15`, `§16` respectively. This follows
"Every decision lives in exactly one owner file. Another file refers to its
section number instead of copying or summarising it."

**Nothing else in `docs/` is touched.** The specs (7,505 lines) are the
architecture. The evidence files are dated measurements. `CLAUDE.md`,
`docs/codebase/README.md` and `docs/vision.md` are tooling and intent.

### 4.3 What is not decided

**No cap on documentation volume enters the gate.** A ratio check would make
CI block on something that does not depend on the code, and would be argued
with rather than obeyed. This document records the 1.00:1 measurement once; if
it is worth revisiting, the trigger is the moment `docs/` passes 1.5:1 against
product Rust, which is well beyond anything recorded here. **OPEN — trigger:**
that ratio.

---

## 5. Order of work

Each item is independently reviewable and none depends on another's result.

1. **§4** — delete six plans, amend `status.md`. Nothing else reads them.
2. **§3** — two edits in `web/src/api/`. Then `npm run typecheck`.
3. **§1, line-breaking** — 266 lines across the workspace. Then the full Rust
   gate.
4. **§1, the check** — one step in `ci.yml`. Run locally against the tree
   §1's line-breaking left, then commit.
5. **§2** — the hook, then `npm test`.

§2 is last because it is the only item that can change behaviour if it is got
wrong. If it goes wrong, the four before it are still worth having.

Every commit runs the project's gate from the repository root: `cargo fmt
--all --check`, both clippy modes, `cargo test --workspace`, the `test-support`
isolation check, and `git diff --exit-code api/`. Web commits additionally run
`npm run typecheck`, `npm run lint` and `npm test`.

---

## 6. What this document does not do

It does not change a service's contract, a route, a type in
`api/openapi.json`, or the schema. It does not add a dependency. It does not
touch `crates/shadows-core/src/db/`, the stores, the runtime, the harness, the
agent or the index.

The 62% of `web/src` files without a sibling test is a real gap — including
`api/queries.ts` and `workflows/plan-frames.ts`, which hold logic — and it is
not addressed here. It deserves its own decision, after this branch is merged.
**OPEN — trigger:** the next change to `web/src/api/queries.ts` or
`web/src/app/workflows/plan-frames.ts` should first add the test that change
would otherwise go without.