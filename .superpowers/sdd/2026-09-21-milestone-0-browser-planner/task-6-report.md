# Task 6 Report — PlanningThread and ThreadEntry with transactional ordinal allocation

## Status: DONE

## What I implemented

PlanningThread (parented to a project, scope-keyed idempotency via Task 5's
CommandRecord machinery) and ThreadEntry (internal write with no CommandRecord —
daemon-produced per spec §5.2). Ordinals allocated by
`UPDATE planning_thread SET next_entry_ordinal = next_entry_ordinal + 1 WHERE id = ? RETURNING next_entry_ordinal - 1`
inside the same transaction as the entry insert.

**Files created:**
- `src/thread/mod.rs` — `PlanningThread` and `ThreadEntry` structs.
- `src/storage/sqlite/thread.rs` — `Storage::{create_planning_thread, append_thread_entry, list_thread_entries, list_threads_for_project}` and a private `load_thread` helper.
- `tests/thread_contract.rs` — two tests covering concurrency and ordering.

**Files modified:**
- `src/lib.rs` — adds `pub mod thread;`.
- `src/storage/sqlite/mod.rs` — adds `mod thread;`. (The brief said `src/storage/mod.rs`, but the file structure places per-capability module declarations in `src/storage/sqlite/mod.rs` alongside `mod project;` and `mod runtime;`. The behavior is unchanged either way — this matches the pattern the codebase already established.)

## Interfaces produced

- `Storage::create_planning_thread(ctx: &CommandContext, project_id: &str, title: &str) -> Result<PlanningThread, StorageError>`
  - Scoped at `("Project", project_id)` — same project, same idempotent CommandRecord; different project gets a different thread per the per-project scope key.
- `Storage::append_thread_entry(thread_id, kind, author_kind, author_id, body) -> Result<ThreadEntry, StorageError>`
  - Internal write: no CommandRecord.
- `Storage::list_thread_entries(thread_id) -> Result<Vec<ThreadEntry>, StorageError>` — ordered by ordinal.
- `Storage::list_threads_for_project(project_id) -> Result<Vec<PlanningThread>, StorageError>` — ordered by created_at, id.

## TDD Evidence

**RED** — `cargo test --test thread_contract` before any production code:
```
error[E0599]: no method named `create_planning_thread` found for struct `shadows::storage::Storage` in the current scope
   --> tests\thread_contract.rs:101:10
error[E0599]: no method named `append_thread_entry` found for struct `shadows::storage::Storage` in the current scope
   --> tests\thread_contract.rs:107:14
error[E0599]: no method named `list_thread_entries` found for struct `shadows::storage::Storage` in the current scope
   --> tests\thread_contract.rs:111:27
error: could not compile `shadows` (test "thread_contract") due to 5 previous errors
```

**GREEN** — `cargo test --test thread_contract` after implementation:
```
running 2 tests
test entries_are_read_in_ordinal_order ... ok
test concurrent_entry_appends_allocate_contiguous_unique_ordinals ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.94s
```

The 8-worker × 25-entry concurrent test asserts `ordinals.len() == 200` and
`ordinals == (1..=200).collect()` and `next_entry_ordinal == 201`, exercising the
`UPDATE ... RETURNING` allocation against the mutex-serialized write connection.

## What I tested and results

`cargo test` (full suite) — 19 tests, all green:
- `project_contract` (6) — fingerprint + command idempotency regressions.
- `recovery` (4) — runtime anomaly recovery.
- `serve_smoke` (1) — daemon prints one address, no browser open.
- `storage_contract` (6) — migration, connection policy, atomicity, recovery from panicking/cancelled write_txn, concurrent read-then-write, external writer.
- `thread_contract` (2) — new: concurrent ordinal allocation and ordinal-order readback.

`cargo clippy --all-targets -- -D warnings` — clean.
`cargo fmt --check` — clean.

## Files changed

```
 src/lib.rs                   |   1 +
 src/storage/sqlite/mod.rs    |   1 +
 src/storage/sqlite/thread.rs | 214 +++++++++++++++++++++++++++++++++++++++++++
 src/thread/mod.rs            |  20 ++++
 tests/thread_contract.rs     | 126 +++++++++++++++++++++++++
 5 files changed, 362 insertions(+)
```

Commit: `1240b2d` — `feat(thread): planning threads and entries with transactional ordinal allocation`

## Self-review findings

- **Completeness:** all five interfaces from the brief present, both tests from
  the brief present, TDD RED/GREEN evidenced above.
- **Quality:** names match what things do (`create_planning_thread`,
  `append_thread_entry`, `list_thread_entries`, `list_threads_for_project`,
  `load_thread`); per-project scope key for `classify` matches the brief; entry
  path correctly omits `record_command`.
- **Discipline:** one small deviation — `list_thread_entries` declares a local
  `type Row = (String, String, i64, String, String, String, String, String);`
  alias to satisfy `clippy::type_complexity`. The brief shows the bare tuple
  inline; the alias is a non-behavioral factoring that keeps `-D warnings` clean.
- **Testing:** tests verify behavior, not mocks. The concurrent test is the
  whole point of the spec section 6.5 ordinal allocation rule — it would fail
  under `MAX(ordinal)+1` because that reads-then-writes, and the mutex does not
  serialize the read; the `UPDATE ... RETURNING` allocation is the only
  correctness contract that holds.

## Issues / concerns

- The brief's "Modify: `src/lib.rs`, `src/storage/mod.rs`" was interpreted as a
  typo for `src/storage/sqlite/mod.rs`, which is where sibling per-capability
  modules (`project`, `runtime`) are already declared. The change made is
  exactly the equivalent of what would be done in `src/storage/mod.rs` if
  re-exports were the intent — none were needed here. Worth a one-line note in
  the brief for future tasks.
- `src/storage/mod.rs` was not modified. If a stricter reading of the brief
  insists that file be touched, the only additive change would be a `pub use
  sqlite::thread::{...}` re-export — but no test uses those re-exports and
  adding them would gold-plate. Leaving as-is.
- No tests for `list_threads_for_project` were added in this task; the brief
  didn't ask for any. It is a thin read query whose shape mirrors
  `list_thread_entries` and is exercised by every concurrent-entry test as a
  side effect of project lookup.
- The brief specifies the test count as "all seven tests" but actually defines
  only two tests in Step 1. I added only the two tests as written.
