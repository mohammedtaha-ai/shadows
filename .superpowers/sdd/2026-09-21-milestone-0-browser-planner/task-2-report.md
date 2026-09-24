# Task 2 Report: SQLite open, connection policy, and the seven-table migration

## What was implemented

Exactly per the brief (`task-2-brief.md`), Steps 1–7:

- `tests/storage_contract.rs` — the contract test verifying WAL journal mode,
  `foreign_keys = ON`, and that the fresh database contains precisely the
  seven milestone tables.
- `migrations/0001_milestone0.sql` — the seven tables (`project`,
  `planning_thread`, `thread_entry`, `runtime_instance`, `operation`,
  `durable_event`, `command_record`) transcribed verbatim from the brief,
  including all CHECK constraints and the three named indexes
  (`idx_thread_by_project`, `idx_operation_by_thread`,
  `idx_operation_non_terminal`, `idx_event_project`, `idx_event_thread`,
  `idx_event_operation`).
- `src/storage/sqlite/mod.rs` — `Storage::open`, `Storage::reader`, and
  `StorageError`, using the measured connection policy: WAL, `synchronous =
  NORMAL`, `foreign_keys = ON`, `busy_timeout = 5000ms`, an 8-connection read
  pool, and a single serialized write connection held behind a `tokio::sync::Mutex`
  (unused by this task; reserved for Task 3's `write_txn`).
- `src/storage/mod.rs` — facade re-exporting `Storage` and `StorageError`
  only; `sqlite` submodule stays private.
- `src/lib.rs` — added `pub mod storage;` (the only change to this file).

## Deviation from the brief's literal code (and why)

The brief's code block for `Storage` holds a `write: Mutex<SqliteConnection>`
field that Task 2 never reads (Task 3 owns `write_txn`). Under strict
`cargo clippy --all-targets -- -D warnings` this produces a `dead_code`
error, which would fail the CI gate this task is required to leave clean. I
added `#[allow(dead_code)]` on that one field with a one-line comment
pointing at Task 3, and ran `cargo fmt` (which only reformatted the
`Ok(Self { ... })` construction onto multiple lines). No other line differs
from the brief's given code.

## TDD evidence

**RED** — before `src/storage` existed:

```
$ cargo test --test storage_contract
error[E0432]: unresolved import `shadows::storage`
 --> tests\storage_contract.rs:1:14
  |
1 | use shadows::storage::Storage;
  |              ^^^^^^^ could not find `storage` in `shadows`
```

Expected: `shadows::storage` did not exist yet (Task 1 only declared `cli`,
`config`, `error`, `tracing`), so any reference to it fails to compile.

**GREEN** — after implementing Steps 3–5:

```
$ cargo test --test storage_contract
running 1 test
test fresh_database_migrates_and_applies_the_connection_policy ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

(One intermediate run surfaced a `dead_code` warning on the `write` field
before the `#[allow(dead_code)]` was added; the test itself passed
throughout — the warning only affected the clippy gate, not this test.)

## Full verification (CI gate, run before commit)

```
$ cargo fmt --check
(clean, exit 0)

$ cargo clippy --all-targets -- -D warnings
    Checking shadows v0.1.0 (E:\Globalprojects\shadows)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.16s
(no warnings, exit 0)

$ cargo test
running 0 tests   (unittests src\lib.rs)
running 0 tests   (unittests src\main.rs)
running 1 test    (tests\serve_smoke.rs) ... ok
running 1 test    (tests\storage_contract.rs) ... ok
running 0 tests   (Doc-tests shadows)
```

All green, no warnings.

## Files changed

- `migrations/0001_milestone0.sql` (new)
- `src/storage/mod.rs` (new)
- `src/storage/sqlite/mod.rs` (new)
- `src/lib.rs` (added `pub mod storage;`)
- `tests/storage_contract.rs` (new)

## Self-review

- **Completeness against the brief:** all five files match the brief's
  interfaces (`Storage::open(&Path) -> Result<Storage, StorageError>`,
  `Storage::reader(&self) -> &SqlitePool`, `StorageError`). Migration has
  exactly seven `CREATE TABLE` statements — verified with
  `grep -c "CREATE TABLE" migrations/0001_milestone0.sql` → `7`.
- **Naming:** table, column, and index names transcribed character-for-character
  from the brief; no renames.
- **YAGNI:** no extra column, index, table, or helper added. Did not create
  `runtime/`, `project/`, `thread/`, `command/`, `operation/`, or any
  per-entity file under `src/storage/sqlite/` — those are Tasks 3–9's. Did
  not implement `write_txn` or `append_event` — Task 3's, per the brief's
  explicit deferral.
- **Module boundary:** `grep -rn "sqlx::" src --include=*.rs | grep -v
  "^src/storage"` returns nothing — no `sqlx` leakage outside `storage/`.
  `src/storage/mod.rs` only re-exports `Storage` and `StorageError`, no
  `sqlx` types in its public surface.
- **Test quality:** the test opens a real temp-file SQLite database (not
  `:memory:`), runs the real migration, and asserts real PRAGMA values and
  the real table list via `sqlite_master` — not mocked. Output is warning-free.

## Concerns

- The `#[allow(dead_code)]` on the `write` field is a necessary but
  intentional deviation from the brief's literal code, explained above. If
  the reviewer prefers a different way to silence this (e.g. a `#[cfg(test)]`-only
  accessor, or simply accepting the warning until Task 3), flag it and I will
  adjust.
- No other concerns.

---

## Fix round 1 (coordinator review)

Two Important findings, both confirmed defects in the brief's given code
(not in the transcription), addressed as directed.

### Finding 1 — remove `.synchronous(SqliteSynchronous::Normal)`

Spec §6.23 lists `synchronous` among settings not to lock without evidence
and says to stay conservative by default; `NORMAL` is less durable than
SQLite's own default, so setting it was the opposite of conservative.
`WAL_VALIDATION.md` only carries `NORMAL` as the fixed backdrop of an
experiment that was not measuring this setting. Removed the
`.synchronous(...)` call from `SqliteConnectOptions` in
`src/storage/sqlite/mod.rs`. `journal_mode`, `foreign_keys`, and
`busy_timeout` are unchanged. The `SqliteSynchronous` import was removed
since it became unused (the `sqlx::sqlite::{...}` import list now reads
`SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions`).

### Finding 2 — remove the unused `write` field

Task 2's Produces contract is only `Storage::open` and `Storage::reader`;
nothing needs a write connection yet. Removed:
- the `write: Mutex<SqliteConnection>` field from `Storage`,
- its construction (`let write = sqlx::ConnectOptions::connect(&opts).await?;`
  and the `write: Mutex::new(write)` in `Ok(Self { ... })`),
- the `#[allow(dead_code)]` attribute on it,
- the now-unused `use sqlx::{SqliteConnection, SqlitePool};` split — replaced
  with `use sqlx::SqlitePool;` — and `use tokio::sync::Mutex;`.

`Storage` is now `pub struct Storage { read: SqlitePool }`, with a doc
comment noting Task 3 adds the write connection alongside `write_txn`. Since
`opts` is no longer cloned for a second connection, `connect_with(opts)` now
takes it by value directly (no `.clone()`).

The third finding (sqlx import in `tests/storage_contract.rs`) was ruled
against by the coordinator — spec §2.9 was amended to permit this
explicitly. No change made to that file.

### Covering tests

```
$ cargo test --test storage_contract
running 1 test
test fresh_database_migrates_and_applies_the_connection_policy ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

### Full CI gates (all clean)

```
$ cargo fmt --check
(clean, exit 0 after applying `cargo fmt`)

$ cargo clippy --all-targets -- -D warnings
    Checking shadows v0.1.0 (E:\Globalprojects\shadows)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.32s
(no warnings, exit 0)

$ cargo test
running 0 tests   (unittests src\lib.rs)
running 0 tests   (unittests src\main.rs)
running 1 test    (tests\serve_smoke.rs) ... ok
running 1 test    (tests\storage_contract.rs) ... ok
running 0 tests   (Doc-tests shadows)
```

No unused-import or dead-code warnings from either removal.

### Files changed

- `src/storage/sqlite/mod.rs` (`synchronous` call and `write` field/imports
  removed; 7 insertions, 21 deletions)

### Commit

`93a84ac` fix(storage): drop synchronous=NORMAL override and the unused write field

### Concerns

None. Both findings were clear-cut removals with no ripple into other files
(`src/storage/mod.rs`'s re-exports of `Storage`/`StorageError` are unaffected
since neither type's public shape changed at the module boundary — only
`Storage`'s private field went away).
