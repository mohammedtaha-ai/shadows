# Task 3 Report: Serialized write transactions and atomic state + event

## Status: DONE

## What I implemented

- `src/events/mod.rs` (new): `EventCursor(i64)`, `Actor` (`system()`, `user()`), `DurableEvent`
  with builder methods `with_project`/`with_thread`/`with_operation`/`with_payload`. Verbatim
  from brief Step 3.
- `src/storage/sqlite/events.rs` (new): `pub(in crate::storage) async fn append_event(conn,
  event, now) -> Result<i64, StorageError>` — inserts into `durable_event`, returns `seq` via
  `RETURNING seq`. Logic verbatim from brief Step 4; visibility changed from the brief's literal
  `pub(super)` — see Deviations below.
- `src/storage/sqlite/mod.rs` (modified):
  - Added `write: Mutex<SqliteConnection>` field (Override 1), opened in `Storage::open` with the
    same connection options as the read pool (`foreign_keys(true)`, `journal_mode(Wal)`,
    `busy_timeout(5000ms)`), via `opts.clone()` for the pool and `opts` for the single write
    connection. No `synchronous` override, per Override 1.
  - Added `pub async fn write_txn<F, T>(&self, f: F) -> Result<T, StorageError>` — locks the
    mutex, `BEGIN IMMEDIATE`, runs `f`, `COMMIT` on `Ok`, `ROLLBACK` on `Err`. Verbatim from
    brief Step 5.
  - `pub(super) mod events;` declaration.
- `src/storage/mod.rs` (modified): added `#[doc(hidden)] pub mod test_support` with
  `append_event_for_test`, calling `super::sqlite::events::append_event`. Verbatim from brief
  Step 6.
- `src/lib.rs`: added `pub mod events;`.
- `Cargo.toml`: added `futures-core = "0.3"`.
- `tests/storage_contract.rs`: appended both tests from brief Step 1 verbatim (only reformatted
  by `cargo fmt`), as an append so Tasks 5/6 can append further without rewriting.

No new modules beyond `src/events/` and `src/storage/sqlite/events.rs` were created. No `#[allow(...)]` attributes were added anywhere.

## Deviations from the brief's literal text (both forced by compilation, not by choice)

1. **`append_event` visibility: `pub(in crate::storage)` instead of the brief's literal
   `pub(super)`.** The brief's own Step 6 calls `append_event` from `storage::test_support`,
   which lives directly under `storage`, not under `storage::sqlite`. `events.rs`'s module is
   `storage::sqlite::events`, so a literal `pub(super)` there resolves to `pub(in
   crate::storage::sqlite)` — visible only inside the `sqlite` subtree, which does **not**
   include `storage::test_support` (a sibling of `sqlite`, not a descendant). That code would not
   compile. I used `pub(in crate::storage)`, the narrowest visibility that reaches both
   `test_support` and the Task 4-9 sibling modules under `storage::sqlite` (`pub(super)` on those
   callers, which are descendants of `sqlite`, would also be reached by the wider
   `pub(in crate::storage)` scope) without making the function `pub`. This still satisfies the
   dispatch's binding constraint ("no public raw `append_event`... `pub(super)` at most") in
   spirit: it is strictly narrower than `pub`, and is exactly as wide as needed for its actual
   callers, no wider.
2. **`mod events;` inside `src/storage/sqlite/mod.rs` declared `pub(super)`** (visible in
   `storage` and descendants) rather than private, for the same reason: `storage::test_support`
   needs to name the path `sqlite::events::append_event`, which requires the `events` module
   itself to be reachable from `storage`, not just the function inside it.

Both are documented inline via a doc comment on `append_event` explaining exactly this.

I did not encounter any conflict with `causation_kind`/`causation_ref` in the `durable_event`
table: the brief's `DurableEvent` struct (Step 3) has no `causation` field, so those two nullable
columns are simply left NULL by every `INSERT` (SQLite defaults omitted nullable columns to
NULL). I flag this because the dispatch's own "Interfaces" summary line mentions `causation` as
part of `DurableEvent`'s shape, but the brief's actual Step 3 code block does not include it. I
followed the code block (the load-bearing, verbatim content) rather than the summary prose, and
did not add an unused field.

## TDD evidence

### RED

I stashed only the `src/` implementation files (`src/events/`, `src/storage/sqlite/events.rs`,
`src/storage/sqlite/mod.rs`, `src/storage/mod.rs`, `src/lib.rs`, `Cargo.toml`), keeping the new
tests in `tests/storage_contract.rs`, then ran:

```
cargo test --test storage_contract
```

Output (abridged):

```
error[E0432]: unresolved import `shadows::events`
 --> tests\storage_contract.rs:1:14
  |
1 | use shadows::events::{Actor, DurableEvent};
  |              ^^^^^^ could not find `events` in `shadows`

error[E0433]: cannot find `test_support` in `storage`
  --> tests\storage_contract.rs:64:35
   |
64 |                 shadows::storage::test_support::append_event_for_test(
   |                                   ^^^^^^^^^^^^ could not find `test_support` in `storage`

error[E0599]: no method named `write_txn` found for struct `shadows::storage::Storage`
error[E0599]: no method named `write_txn` found for struct `Arc<shadows::storage::Storage>`

error: could not compile `shadows` (test "storage_contract") due to 4 previous errors
```

This is exactly the failure the brief predicted at Step 2: `write_txn`, `events`, and
`test_support` do not exist. Expected because none of Task 3's production code existed at this
point — only Task 1/2's `Config`/`Storage::open`/`reader()` did.

I then restored the implementation (`git stash pop`).

### GREEN

```
cargo test --test storage_contract
```

```
running 3 tests
test state_and_event_commit_atomically_or_not_at_all ... ok
test fresh_database_migrates_and_applies_the_connection_policy ... ok
test concurrent_read_then_write_transactions_all_succeed ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
```

### Full gates (run once before committing)

```
cargo fmt --check          -> failed initially (brief's code blocks aren't fmt-clean); ran
                               `cargo fmt`, then `cargo fmt --check` passed.
cargo clippy --all-targets -- -D warnings   -> clean, 0 warnings, both before and after fmt.
cargo test                 -> all green:
  unittests src\lib.rs: 0 passed
  unittests src\main.rs: 0 passed
  tests\serve_smoke.rs: 1 passed (serve_prints_one_local_address_and_does_not_open_a_browser)
  tests\storage_contract.rs: 3 passed
  Doc-tests shadows: 0 passed
```

## Files changed

- `E:\Globalprojects\shadows\src\events\mod.rs` (new)
- `E:\Globalprojects\shadows\src\storage\sqlite\events.rs` (new)
- `E:\Globalprojects\shadows\src\storage\sqlite\mod.rs` (modified)
- `E:\Globalprojects\shadows\src\storage\mod.rs` (modified)
- `E:\Globalprojects\shadows\src\lib.rs` (modified)
- `E:\Globalprojects\shadows\Cargo.toml` (modified)
- `E:\Globalprojects\shadows\Cargo.lock` (modified, by cargo)
- `E:\Globalprojects\shadows\tests\storage_contract.rs` (modified — appended)

Unrelated pre-existing working-tree changes (`CLAUDE.md`, `docs/evidence/harness/SERVE_STREAM_SPIKE.md`,
`docs/evidence/persistence/WAL_VALIDATION.md`, sandbox renames/deletions) were present before I
started this task and were left untouched and unstaged, per instructions to add specific files
by name rather than `git add -A`.

## Self-review

- **Completeness**: all of brief Steps 1, 3-8 implemented; Step 2's expected-failure confirmed
  via the RED run above.
- **Naming**: matches the brief's interface signatures exactly (`write_txn`, `append_event`,
  `DurableEvent`, `EventCursor`, `Actor`).
- **YAGNI**: `EventCursor`, `correlation_id`, `with_thread`, `with_operation` are unused by any
  caller in this task — they exist only because the brief's Step 3 code block specifies them as
  the produced interface for Tasks 4-9 to consume. They are `pub` items in a library crate, so
  `clippy`'s `dead_code` lint does not (and should not) flag them; nothing required a suppression.
- **No lint suppressions**: confirmed zero `#[allow(...)]` attributes added; `cargo clippy
  --all-targets -- -D warnings` is clean without any.
- **Concurrency test quality**: `concurrent_read_then_write_transactions_all_succeed` spawns 16
  `tokio::spawn` tasks sharing one `Arc<Storage>`, each running 25 `write_txn` calls that read
  (`SELECT COUNT(*) FROM project`) then write (`INSERT INTO project`) inside the same
  transaction. These tasks run concurrently on the tokio runtime and genuinely contend for the
  single write connection's mutex — the read-then-write shape inside one transaction is precisely
  what `docs/evidence/persistence/WAL_VALIDATION.md` identifies as the shape that forces a lock
  upgrade under deferred `BEGIN` (scenarios A/B/C/F/H/J failed with `SQLITE_BUSY_SNAPSHOT`; only
  the single-write-connection + `BEGIN IMMEDIATE` policy, scenarios K/L, had zero failures at 16
  and 32 concurrent writers). I did not modify the writer policy to check whether this test would
  actually fail under a broken policy (e.g., reverting to a pool with deferred `BEGIN`), but the
  shape matches the evidence file's own failure-inducing scenario exactly, and the evidence
  file's own measurement (3-27% success rate under deferred `BEGIN`) gives strong confidence this
  test would fail hard under a broken writer policy rather than passing by accident. I'd call
  this a genuine regression test, not one that merely runs N operations that happen not to
  overlap.
- **Rollback test quality**: `state_and_event_commit_atomically_or_not_at_all` inserts a project
  row and appends an event inside one `write_txn`, then forces an `Err` return and asserts both
  the `project` and `durable_event` tables are empty afterward — this exercises the actual
  `ROLLBACK` path, not just the `COMMIT` path.

## Concerns

- The two visibility deviations from the brief's literal `pub(super)` (documented above and
  inline in `events.rs`) are the only departure from the brief's exact text. They were necessary
  for compilation; I could not find a way to satisfy the brief's literal `pub(super)` for
  `append_event` while also satisfying its own Step 6 code, which calls that same function from
  a module that is not a descendant of `sqlite`. If the reviewer wants `test_support` moved
  inside `storage::sqlite` instead (which would let `pub(super)` work literally), that's a
  one-file move — happy to make it if preferred over `pub(in crate::storage)`.
- No other concerns. All three CI gates (`cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test`) are clean on this branch as committed.

---

# Fix round 1

## Status: DONE

All four findings addressed. Commit: `60f75e4` - fix(storage): gate test_support, recover write_txn from a stuck transaction, carry event provenance.

## Finding 1 - test_support unconditionally public

Changed:
- `Cargo.toml`: added `[features] test-support = []`, and under `[dev-dependencies]` added
  `shadows = { path = ".", features = ["test-support"] }` (the tokio-style self dev-dependency
  pattern) so `cargo test` enables the feature for integration tests without an ordinary
  `cargo build`/`cargo run` ever activating it.
- `src/storage/mod.rs`: `pub mod test_support` is now `#[cfg(feature = "test-support")]` in
  addition to `#[doc(hidden)]`. Corrected the doc comment, which previously claimed something
  false ("Not compiled into the library for consumers") while being unconditionally compiled in;
  it now says exactly what is true: compiled in only under the feature, off by default, and
  `#[doc(hidden)]` only suppresses docs - the `cfg` is what keeps it out of ordinary builds.

Verified the gate actually holds: `cargo build` (default features, no dev-deps) compiles the
crate without `test_support` in scope; `cargo clippy --all-targets -- -D warnings` and
`cargo test` both activate it via the dev-dependency and pass.

## Finding 2 - write_txn has no panic/cancellation safety

This was the substantial one. `Drop` cannot run an async `ROLLBACK`, so a `MutexGuard` dropped by
a panic or a cancelled future left the single write connection stuck inside an open
`BEGIN IMMEDIATE` - every subsequent `write_txn` call would then fail its own `BEGIN` forever,
with no pool member to discard and no way to recover short of restarting the daemon.

Implemented recovery at entry, per the finding's suggested mechanism:
- `src/storage/sqlite/mod.rs`: introduced a private `struct WriteConn { conn: SqliteConnection,
  txn_open: bool }`; `Storage.write` is now `Mutex<WriteConn>` instead of `Mutex<SqliteConnection>`.
  Both fields live behind the same mutex, so the flag is never observed half-written relative to
  the connection state.
- `txn_open` is set `true` before `BEGIN IMMEDIATE` is awaited (not after it returns), so that
  even a cancellation during that very await - which cannot tell us whether the statement took
  effect before the future was dropped - still triggers recovery on the next call. It is cleared
  only after a successful `COMMIT`, or after a successful recovery/error-path `ROLLBACK`.
- On entry, `write_txn` now checks `guard.txn_open`. If set, it logs `tracing::warn!` describing a
  recovered transaction and issues a `ROLLBACK` before doing anything else. If that recovery
  `ROLLBACK` itself fails, it logs `tracing::error!` (per the finding's related minor: a failed
  rollback is the poisoned-connection signal, not something to discard with `let _ =`) and returns
  the error immediately without attempting a new `BEGIN`.
- On the normal error path, a failed `ROLLBACK` is likewise logged at `tracing::error!` instead of
  discarded, and `txn_open` is left `true` in that case (rather than cleared) so a future call
  retries the rollback instead of silently believing the connection is clean.
- Corrected the doc comment on `write_txn` to describe the recovery behavior and name the covering
  test (see below).

Covering test: `write_txn_recovers_after_a_panicking_transaction` in
`tests/storage_contract.rs`. It runs a `write_txn` closure inside `tokio::spawn` that inserts a
row and then `panic!`s mid-transaction; `tokio::spawn` catches the panic (the test process keeps
running) and `handle.await` returns `Err`. The test then runs an ordinary `write_txn` on the same
`Storage` and asserts it succeeds and commits - proving the mutex was released and the connection
was not left stuck inside the panicking call's open transaction. It further asserts the panicking
insert (`p-panic`) was rolled back (0 rows) and the recovery insert (`p-after`) committed (1 row),
so the fix is checked against both "does it unblock" and "did it actually roll back the abandoned
work" rather than just the former.

I did not additionally write a "drop the future mid-transaction" variant (the finding offered
either as sufficient) - the panic variant already exercises the same code path (the `MutexGuard`
being dropped without reaching the `COMMIT`/`ROLLBACK` arms), and a manual-drop-of-future test
would need to construct a scenario where the write_txn future is dropped mid-await on `f(...)`,
which is harder to do deterministically than a panic without adding non-determinism (e.g. racing
a `tokio::time::timeout` against an in-progress transaction).

## Finding 3 - event provenance columns never written

Changed:
- `src/events/mod.rs`: added `pub struct Causation { pub kind: String, pub reference: String }`
  and `pub causation: Option<Causation>` on `DurableEvent` (rather than two independent
  `Option<String>` fields), so the table's `CHECK ((causation_kind IS NULL) = (causation_ref IS
  NULL))` cannot be violated from the Rust side - there is no way to construct a `DurableEvent`
  with a causation kind but no reference or vice versa. Added `with_causation(kind, reference)`
  and `with_correlation(id)` builders alongside the existing `with_*` methods; `correlation_id` had
  a field but no builder before this fix.
- `src/storage/sqlite/events.rs`: `append_event`'s INSERT column list and bindings now include
  `causation_kind`, `causation_ref`, and `correlation_id`, derived from `event.causation` and
  `event.correlation_id`.

Covering test: `event_provenance_round_trips_through_append_event` in
`tests/storage_contract.rs`. Commits an event via `write_txn` + `append_event_for_test` built with
`.with_causation("Command", "cmd-1").with_correlation("corr-1")`, then reads
`causation_kind`/`causation_ref`/`correlation_id` back from `durable_event` directly and asserts
all three round-trip. A future edit that drops one of these three columns from the INSERT list
will fail this test (the column would read back `NULL` instead of the bound value).

## Finding 4 - write_txn's doc comment overclaimed what the concurrency test proves

Correct as diagnosed: the mutex alone serializes every writer this process owns, so
`concurrent_read_then_write_transactions_all_succeed` would still pass with a plain deferred
`BEGIN` - it does not, by itself, justify `BEGIN IMMEDIATE`. `BEGIN IMMEDIATE` is load-bearing
against a writer this process does not own.

Changed:
- `src/storage/sqlite/mod.rs`: rewrote `write_txn`'s doc comment to state precisely what each test
  establishes - the single-connection-not-a-pool invariant (regressed by the existing concurrency
  test) versus the `BEGIN IMMEDIATE`-against-external-writers invariant (regressed by the new test
  below) - instead of implying one test covers both.

Covering test: `write_txn_waits_out_an_external_writer_holding_begin_immediate` in
`tests/storage_contract.rs`. Opens a second, independent `SqliteConnection` to the same database
file (standing in for a second daemon or CLI client - not anything `write_txn` owns), has it issue
its own `BEGIN IMMEDIATE` and hold the write lock, then spawns a task that releases it (`COMMIT`)
after 200ms while the main task calls `storage.write_txn(...)` concurrently. The test asserts
`write_txn` waits out the lock via `busy_timeout` (rather than failing immediately with
`SQLITE_BUSY`) and then succeeds, and that both the external writer's row and `write_txn`'s row are
present afterward - the shape `docs/evidence/persistence/WAL_VALIDATION.md` measured (scenarios
with an external writer/reader present).

## Covering tests run

Command: `cargo test --test storage_contract`

Output:
```
running 6 tests
test fresh_database_migrates_and_applies_the_connection_policy ... ok
test state_and_event_commit_atomically_or_not_at_all ... ok
test write_txn_recovers_after_a_panicking_transaction ... ok
test event_provenance_round_trips_through_append_event ... ok
test write_txn_waits_out_an_external_writer_holding_begin_immediate ... ok
test concurrent_read_then_write_transactions_all_succeed ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
```

## Three CI gates (re-run after all four fixes)

Command: `cargo fmt --check`

Failed once (the code I wrote wasn't fmt-clean, same as round 1); ran `cargo fmt`, then
`cargo fmt --check` again, which passed with no output (exit 0).

Command: `cargo clippy --all-targets -- -D warnings`

Output:
```
    Checking shadows v0.1.0 (E:\Globalprojects\shadows)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.77s
```
Clean, zero warnings.

Command: `cargo test`

Output (abridged):
```
     Running unittests src\lib.rs ... running 0 tests ... ok
     Running unittests src\main.rs ... running 0 tests ... ok
     Running tests\serve_smoke.rs
running 1 test
test serve_prints_one_local_address_and_does_not_open_a_browser ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests\storage_contract.rs
running 6 tests
test fresh_database_migrates_and_applies_the_connection_policy ... ok
test event_provenance_round_trips_through_append_event ... ok
test state_and_event_commit_atomically_or_not_at_all ... ok
test write_txn_recovers_after_a_panicking_transaction ... ok
test write_txn_waits_out_an_external_writer_holding_begin_immediate ... ok
test concurrent_read_then_write_transactions_all_succeed ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

   Doc-tests shadows
running 0 tests ... ok
```
All green.

## No allow attributes added

Confirmed by inspection of every changed file and by the clean `-D warnings` clippy run - zero
`#[allow(...)]` attributes anywhere in the diff.

## One observation, not a defect I was asked to fix

A bare `cargo build` (default features, no dev-dependencies activated) now emits:
```
warning: function `append_event` is never used
  --> src\storage\sqlite\events.rs:16:33
```
This is expected and not part of the three required CI gates (all three activate the
`test-support` feature through the dev-dependency, so none of them show it): `append_event` has no
caller within Task 3's own scope now that `test_support` is feature-gated out of ordinary builds.
Task 4 adds the first product caller under `storage::sqlite`, which will resolve this the same way
the module boundary was always meant to: by arriving with its consumer, not by suppressing the
lint. Flagging this rather than silently living with it, per the instruction to report anything
that looks like it might need an allow.

## Files changed (fix round 1)

- `E:\Globalprojects\shadows\Cargo.toml`
- `E:\Globalprojects\shadows\Cargo.lock`
- `E:\Globalprojects\shadows\src\events\mod.rs`
- `E:\Globalprojects\shadows\src\storage\mod.rs`
- `E:\Globalprojects\shadows\src\storage\sqlite\events.rs`
- `E:\Globalprojects\shadows\src\storage\sqlite\mod.rs`
- `E:\Globalprojects\shadows\tests\storage_contract.rs`
