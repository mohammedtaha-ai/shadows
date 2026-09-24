# Task 5 Report: Local-directory Project with external command idempotency

## Status: DONE

## What was implemented

- `src/command/mod.rs` — `CommandContext` and `fingerprint(kind, params)`, using
  SHA-256 over a canonicalised (key-sorted) JSON representation, with the
  command kind mixed into the hash so identical params under a different
  command kind never collide.
- `src/project/mod.rs` — `Project { id, slug, name, created_at }`, a plain
  serializable domain type (no persistence imports).
- `src/storage/sqlite/project.rs` —
  - `pub(super) classify(conn, ctx, scope_kind, scope_key)`: looks up
    `command_record` by `(principal_kind, principal_id, command_scope_kind,
    command_scope_key, command_id)`; returns `Ok(None)` if unseen, `Ok(Some(outcome_ref))`
    if the same `command_kind` + `request_fingerprint` was already recorded,
    `Err(CommandConflict)` if the id was reused with a different kind or
    fingerprint.
  - `pub(super) record_command(...)`: inserts the `command_record` row that
    marks a command as durably completed.
  - `Storage::create_project(ctx, slug, name) -> Result<Project, StorageError>`:
    inside one `write_txn`, classifies the command, and either returns the
    existing project (replay) or inserts `project`, appends a `ProjectCreated`
    `DurableEvent`, and records the command — all in the same transaction.
  - `Storage::list_projects() -> Result<Vec<Project>, StorageError>`: reads
    ordered by `created_at, id` (explicit ordering, no `rowid`/insertion order).
- `src/storage/sqlite/mod.rs`: added `pub(super) fn now() -> String` (moved out
  of `runtime.rs`) so `project.rs` and `runtime.rs` share one clock
  implementation; declared `mod project;`.
- `src/storage/sqlite/runtime.rs`: removed its private `now()`, now imports
  `super::now`.
- `src/lib.rs`: added `pub mod command;` and `pub mod project;`.
- `Cargo.toml`: added `sha2 = "0.10"`.
- `tests/project_contract.rs` (new file, per the dispatch override — not
  `tests/storage_contract.rs`): the three tests specified in the brief,
  verbatim, with a module doc comment stating the file's one job ("the
  project capability's durable contract").

No migration file was touched — `project` and `command_record` already
existed in `migrations/0001_milestone0.sql` from Task 1-3.

## Binding-constraints check

- Read `CLAUDE.md` `## Rules (project-specific)` and documentation rules: no
  new ADR/design doc needed (this is an implementation detail governed by an
  existing owner file); accretion-point rule for `storage/mod.rs` /
  `storage/sqlite/` respected by adding one sibling file
  (`storage/sqlite/project.rs`) rather than growing `mod.rs`'s logic.
- Read plan lines 19-51 (`## Global Constraints`): only `storage/` imports
  `sqlx` (confirmed — `command/` and `project/` have none); ids are UUID-v4
  (`uuid::Uuid::new_v4()`); timestamps are RFC3339 UTC TEXT (`now()`);
  ordering is explicit (`ORDER BY created_at, id`, no `rowid`); durable state
  mutation + durable event are atomic in the same `write_txn`; external
  mutation adds a `CommandRecord` in that same transaction; no public raw
  `append_event` (still `pub(in crate::storage)`, called only from inside
  `create_project`); `project/` and `command/` are both listed as modules
  this milestone builds. No contradiction found between brief and either
  binding file.

## TDD evidence

**RED** — before implementation, ran (target changed per dispatch override
to `project_contract` instead of `storage_contract`):

```
cargo test --test project_contract
```

Failing output (file existed with only the brief's test bodies, before
`command`/`project` modules or `Storage::create_project` existed):

```
error[E0432]: unresolved import `shadows::command`
 --> tests\project_contract.rs:6:14
  |
6 | use shadows::command::{fingerprint, CommandContext};
  |              ^^^^^^^ could not find `command` in `shadows`

error[E0599]: no method named `create_project` found for struct `shadows::storage::Storage` in the current scope
  --> tests\project_contract.rs:33:25
...
error: could not compile `shadows` (test "project_contract") due to 5 previous errors
```

This is the expected failure: the brief's Step 2 predicts exactly "`shadows::command`
and `create_project` do not exist" (the target test binary just did not
compile).

**GREEN** — after implementing `src/command/mod.rs`, `src/project/mod.rs`,
`src/storage/sqlite/project.rs`, and wiring `src/lib.rs` /
`src/storage/sqlite/mod.rs`:

```
cargo test --test project_contract
```

```
running 3 tests
test the_fingerprint_ignores_key_order_but_not_command_kind ... ok
test replaying_an_identical_command_returns_the_stored_outcome ... ok
test the_same_command_id_with_a_different_request_is_a_conflict ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

(The brief's Step 6 says "all five tests" — that count assumed the tests
were appended to `tests/storage_contract.rs` alongside its existing tests.
Under the dispatch override, the new file's own three tests all pass; the
pre-existing `storage_contract.rs` suite is unaffected — see full-suite run
below.)

## Full verification before commit

```
cargo fmt --check         # clean after running `cargo fmt` (see below)
cargo clippy --all-targets -- -D warnings   # clean, 0 warnings
cargo test                # all suites pass
```

`cargo fmt --check` initially reported diffs in `src/command/mod.rs` and
`src/storage/sqlite/project.rs` (line-wrapping of chained `.bind()` calls and
one multi-line `Ok(Some(...))`), matching the brief's code blocks copied
verbatim but not matching this repo's `rustfmt` settings. Ran `cargo fmt`
to apply the project's formatting (no logic changed), then re-ran
`cargo fmt --check` clean.

Full `cargo test` output (final run):

```
Running unittests src\lib.rs ... 0 tests, ok
Running unittests src\main.rs ... 0 tests, ok
Running tests\project_contract.rs
running 3 tests
test the_fingerprint_ignores_key_order_but_not_command_kind ... ok
test the_same_command_id_with_a_different_request_is_a_conflict ... ok
test replaying_an_identical_command_returns_the_stored_outcome ... ok
test result: ok. 3 passed; 0 failed

Running tests\recovery.rs
running 4 tests ... test result: ok. 4 passed; 0 failed

Running tests\serve_smoke.rs
running 1 test ... test result: ok. 1 passed; 0 failed

Running tests\storage_contract.rs
running 6 tests ... test result: ok. 6 passed; 0 failed

Doc-tests shadows: 0 tests, ok
```

All 14 tests across the whole suite pass; nothing in the existing suite
regressed.

## Files changed

- `Cargo.toml` (added `sha2 = "0.10"`)
- `Cargo.lock` (updated for `sha2` and transitive deps)
- `src/lib.rs` (added `pub mod command;`, `pub mod project;`)
- `src/command/mod.rs` (new)
- `src/project/mod.rs` (new)
- `src/storage/sqlite/mod.rs` (added shared `now()`, declared `mod project;`)
- `src/storage/sqlite/runtime.rs` (removed local `now()`, imports `super::now`)
- `src/storage/sqlite/project.rs` (new)
- `tests/project_contract.rs` (new — brief's tests, per dispatch override)

`tests/storage_contract.rs` was left untouched at 361 lines, per the
dispatch's override instruction.

## Self-review

- **Completeness:** every interface named in the brief's "Produces" line
  exists with the exact signature: `CommandContext` fields, `fingerprint`,
  `Project` fields, `Storage::create_project`, `Storage::list_projects`,
  `pub(super) classify`, `pub(super) record_command`.
- **Naming:** matches the brief exactly (no renaming for personal taste),
  since Tasks 6/9/10/11 depend on these signatures verbatim.
- **YAGNI:** no extra fields, no extra public API beyond what four later
  tasks are stated to need. `ctx_kind` helper in the test file is used by
  `ctx`, not dead code.
- **No `allow` attributes:** confirmed via `grep -rn "allow(" src/command
  src/project src/storage/sqlite/project.rs tests/project_contract.rs` — no
  matches.
- **Test quality — the specific question asked:**
  - `replaying_an_identical_command_returns_the_stored_outcome` replays with
    the *same* `command_id` ("cmd-1") and the *same* fingerprint-producing
    params, exactly as a real client retry would look (client resubmits an
    identical request after e.g. a dropped response). Asserts same entity
    id, exactly one `project` row, exactly one `durable_event` row — a real
    double-insert or a second event append would fail this.
  - `the_same_command_id_with_a_different_request_is_a_conflict` keeps the
    same `command_id` ("cmd-1") and changes only the request params
    (`slug`/`name` from "demo"/"Demo" to "other"/"Other") — the one field
    that is allowed to vary for this to be a meaningful conflict test is the
    *fingerprint-derived* content, not the command id. This is a precise,
    narrow test: a weaker check (e.g. only comparing `command_kind`, or only
    comparing `slug`) would still fail this test the same way it's currently
    passing, so I traced the implementation path — `classify` compares both
    `kind == ctx.command_kind` and `fp == ctx.request_fingerprint`; changing
    either field alone would already trigger `CommandConflict`. The test's
    params change both `slug` and `name` together (a natural "different
    request" shape) rather than isolating one field, but since the
    fingerprint is a hash over the whole canonicalised object, any single
    differing key already changes the fingerprint — so this is not a weaker
    test than isolating one field would be.
  - `the_fingerprint_ignores_key_order_but_not_command_kind` is a pure unit
    test of `fingerprint`, independent of storage.

## Concerns

- None blocking. One documentation note: the brief's Step 6 text ("all five
  tests") was written assuming the tests were appended to
  `storage_contract.rs`; under this dispatch's override they live alone in
  `project_contract.rs` (3 tests) and `storage_contract.rs` keeps its
  original 6 unaffected. Total test count across both files is unchanged
  from what the brief intended; only the file location differs, as directed.
- The brief's own commit command referenced `tests/storage_contract.rs`;
  I committed `tests/project_contract.rs` instead, per the dispatch's
  explicit override, and adjusted the commit message body accordingly.

---

# Addendum: Fix for canonicalisation collision finding (2026-09-22)

## Finding

`canonical()` in `src/command/mod.rs` built each object entry as
`format!("{}:{}", key, canonical(value))` and joined entries with `,`,
without escaping the key text. Because the separator characters (`:` and
`,`) are not excluded from key text, two structurally different objects
could canonicalise to the identical string, e.g.:

- `{"a": 1, "bc": 2}` -> `{a:1,bc:2}`
- `{"a:1,bc": 2}`      -> `{a:1,bc:2}`

Both produce the same SHA-256 fingerprint under the same `command_kind`,
so the second request would be silently treated as a replay of the first
instead of executing — not reachable today (the only real caller,
`project.create`, has fixed Rust-literal keys `slug`/`name`), but reachable
the first time any command's params carry a user-derived key name. Tasks 6,
9, 10, 11 reuse `fingerprint()` verbatim, so the flaw would propagate.

## What changed

`src/command/mod.rs`, inside `canonical()`'s `Object` arm: each key is now
encoded with `serde_json::to_string(*k)` (which JSON-quotes and escapes the
string) instead of interpolated as raw text, before being joined with the
canonicalised value:

```rust
let inner: Vec<String> = keys
    .iter()
    .map(|k| {
        let key_json = serde_json::to_string(*k).expect("string keys always serialise");
        format!("{}:{}", key_json, canonical(&map[*k]))
    })
    .collect();
```

This makes key encoding injective: a JSON string literal is unambiguously
delimited by its own quotes and escapes, so no key text can produce the
byte sequence `",",` or fake a `:`/`,` structural separator. Two distinct
`serde_json::Value::Object`s can no longer canonicalise to the same string.

**Ordering is untouched.** The existing `keys.sort()` (sorting `&String`
key references lexicographically) is kept exactly as-is — only *how* each
key is textually encoded changed, not what determines their order. This
does not depend on `serde_json::Map`'s default `BTreeMap` behavior at all:
`keys` is collected into a plain `Vec<&String>` and sorted explicitly by
this function, independent of the `Map`'s own iteration order and
independent of whether `preserve_order` is ever enabled. Only
`serde_json::to_string` is used, and only on an owned/borrowed `String`
key (never on the `Map` itself), so enabling `preserve_order` changes
nothing about this code path's behavior or safety.

Arrays and nested objects recurse into the same `canonical()` function, so
the injectivity guarantee is inherited recursively without further changes.

`command_kind` mixing (hashed with a `\0` separator before the canonical
payload) was not touched.

## Covering tests

Added to `tests/project_contract.rs`, alongside
`the_fingerprint_ignores_key_order_but_not_command_kind`:

1. `the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure`
   — the exact collision from the finding, as a regression test:
   `fingerprint("k", &json!({"a": 1, "bc": 2}))` must differ from
   `fingerprint("k", &json!({"a:1,bc": 2}))`.
2. `the_fingerprint_changes_when_a_value_changes` — a single value change
   with all keys and command kind held constant must change the
   fingerprint: `fingerprint("k", &json!({"a": 1}))` !=
   `fingerprint("k", &json!({"a": 2}))`.

Both existing tests (`the_fingerprint_ignores_key_order_but_not_command_kind`,
and the two `project_contract.rs` storage-level idempotency tests) were left
in place and still pass.

## TDD evidence

**RED** — ran the new collision test alone against the pre-fix code
(`canonical()` still using `format!("{}:{}", k, ...)` with raw key text):

```
cargo test --test project_contract the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure -- --nocapture
```

```
running 1 test
thread 'the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure' panicked at tests\project_contract.rs:111:5:
assertion `left != right` failed
  left: "b8102b0b804ee1994725225682425f3cca266cb7281f399c5b8bc3e562edaa54"
 right: "b8102b0b804ee1994725225682425f3cca266cb7281f399c5b8bc3e562edaa54"
test the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```

This confirms the collision is real: both fingerprints hash to the
identical hex digest before the fix.

**GREEN** — after changing `canonical()`'s key encoding to
`serde_json::to_string`:

```
cargo test --test project_contract
```

```
running 5 tests
test the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure ... ok
test the_fingerprint_changes_when_a_value_changes ... ok
test the_fingerprint_ignores_key_order_but_not_command_kind ... ok
test the_same_command_id_with_a_different_request_is_a_conflict ... ok
test replaying_an_identical_command_returns_the_stored_outcome ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

## Full verification before commit

```
cargo fmt --check
```
Initially reported one diff (line-length of the new `let key_json = ...`
line); ran `cargo fmt` (no logic change) and re-ran `cargo fmt --check`
clean.

```
cargo clippy --all-targets -- -D warnings
```
```
Checking shadows v0.1.0 (E:\Globalprojects\shadows)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.53s
```
Clean, zero warnings.

```
cargo test
```
```
Running unittests src\lib.rs ... 0 tests, ok
Running unittests src\main.rs ... 0 tests, ok
Running tests\project_contract.rs
running 5 tests
test the_fingerprint_does_not_let_a_key_impersonate_the_separator_structure ... ok
test the_fingerprint_changes_when_a_value_changes ... ok
test the_fingerprint_ignores_key_order_but_not_command_kind ... ok
test the_same_command_id_with_a_different_request_is_a_conflict ... ok
test replaying_an_identical_command_returns_the_stored_outcome ... ok
test result: ok. 5 passed; 0 failed

Running tests\recovery.rs
running 4 tests ... test result: ok. 4 passed; 0 failed

Running tests\serve_smoke.rs
running 1 test ... test result: ok. 1 passed; 0 failed

Running tests\storage_contract.rs
running 6 tests ... test result: ok. 6 passed; 0 failed

Doc-tests shadows: 0 tests, ok
```

All 16 tests pass (14 pre-existing + 2 new); nothing regressed.

## Files changed (this addendum)

- `src/command/mod.rs` — injective key encoding in `canonical()`'s `Object`
  arm (the only change; `classify`, `record_command`, `create_project`,
  and everything under `src/storage/` untouched).
- `tests/project_contract.rs` — two new tests, appended after
  `the_fingerprint_ignores_key_order_but_not_command_kind`; no existing
  test bodies changed.

## Concerns

None. No `allow` attributes were added (`grep -rn "allow(" src/command`
has no matches). `tests/storage_contract.rs` was not touched.
