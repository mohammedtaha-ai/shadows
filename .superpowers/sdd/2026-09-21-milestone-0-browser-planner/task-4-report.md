# Task 4 Report: Runtime instance lifecycle and startup orphan reconciliation

## Summary

Implemented `Storage::register_runtime_instance`, `Storage::stop_runtime_instance`,
`Storage::reconcile_orphans` in `src/storage/sqlite/runtime.rs`, re-exported
`ReconcileReport` / `StopKind` from `src/storage/mod.rs`, and added
`src/runtime/mod.rs` with `Runtime::start` / `Runtime::stop` as the orchestration
entry point used at daemon startup. `pub mod runtime;` added to `src/lib.rs`.

## Files changed

- `src/storage/sqlite/runtime.rs` (new) — `StopKind`, `ReconcileReport`,
  `register_runtime_instance`, `stop_runtime_instance`, `reconcile_orphans`.
- `src/storage/sqlite/mod.rs` — declares `mod runtime;`, re-exports
  `ReconcileReport, StopKind`.
- `src/storage/mod.rs` — re-exports `ReconcileReport, StopKind` alongside the
  existing `Storage, StorageError` (the named accretion point; added one
  capability, did not reorganise it).
- `src/runtime/mod.rs` (new) — `Runtime { instance_id, storage }`,
  `Runtime::start` (register + reconcile + log), `Runtime::stop`.
- `src/lib.rs` — added `pub mod runtime;`.
- `tests/recovery.rs` (new) — the four brief tests, verbatim content
  (rustfmt reformatted whitespace only).

## TDD evidence

**RED** — `cargo test --test recovery` before implementation:

```
error[E0432]: unresolved import `shadows::storage::StopKind`
...
error[E0599]: no method named `register_runtime_instance` found for struct `shadows::storage::Storage`
error[E0599]: no method named `stop_runtime_instance` found for struct `shadows::storage::Storage`
error[E0599]: no method named `reconcile_orphans` found for struct `shadows::storage::Storage`
error: could not compile `shadows` (test "recovery") due to 14 previous errors
```

Expected: yes — `StopKind` and all three methods did not exist yet, matching
the brief's Step 2 expectation exactly.

**GREEN** — `cargo test --test recovery` after implementation:

```
running 4 tests
test the_current_runtimes_own_operations_are_left_alone ... ok
test a_lost_runtimes_operations_become_interrupted ... ok
test a_graceful_runtime_owning_unfinished_work_is_reported_as_an_anomaly ... ok
test an_escalated_shutdowns_operations_are_not_stranded ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Full verification (all three required clean before commit)

- `cargo fmt --check` — clean (ran `cargo fmt` once to normalize the
  brief's inline code into rustfmt style; no logic changed).
- `cargo clippy --all-targets -- -D warnings` — clean, zero warnings.
- `cargo test` — all 11 tests pass across `recovery.rs` (4),
  `serve_smoke.rs` (1), `storage_contract.rs` (6), 0 unit tests, 0 doc
  tests.

`cargo build` (plain, no test-support feature) also emits no `dead_code`
warning on `append_event` — confirmed: `register_runtime_instance`,
`stop_runtime_instance`, and `reconcile_orphans` are its first product
callers, so the warning mentioned in the task context is gone as expected.

## Self-review

- **Recovery predicate**: implemented exactly as specified — selects
  `status_kind IN ('Pending','Running') AND runtime_instance_id <> ?`,
  with no condition on `stopped_at` or `stop_kind`. Those columns are read
  only to decide the anomaly flag (`stop_kind = 'Graceful'`), never used to
  filter membership in the scan. Verified against both the "lost" runtime
  (never stopped) and the "Escalated" runtime (stopped, non-NULL
  `stopped_at`) tests — both are reconciled identically by ownership.
- **Test quality**: each recovery test constructs a real row owned by a
  real previous runtime — `seed_operation` inserts directly against the
  `operation` table with a `runtime_instance_id` that came from an actual
  `register_runtime_instance` call, not a fabricated id. The Escalated and
  Graceful tests call the real `stop_runtime_instance` to produce the
  `stop_kind` the assertions depend on, so the production code path that
  sets `stop_kind` is exercised, not assumed. None of the four tests
  assert against a state the production code could not itself produce.
- **No `allow` attributes**: none added; `cargo clippy --all-targets -- -D
  warnings` is clean without any.
- **Naming/YAGNI**: kept `runtime.rs` scoped to runtime-instance and
  orphan-scan concerns only, per the brief's note that Tasks 5/6/9 add
  their own sibling files under `storage/sqlite/`. `Runtime::start` /
  `Runtime::stop` in `src/runtime/mod.rs` are not yet called from
  `cli::serve` — wiring the daemon startup path to actually invoke
  `Runtime::start` is not in this task's Step list (the brief's milestone
  description says the full serve wiring is a later integration point),
  so I left `cli::serve` untouched. Both methods are `pub` or exercised
  indirectly through the storage-layer tests, so nothing is unreachable
  dead code, and clippy confirms it.
- **Files-list discrepancy** (concern, not blocking): the brief's header
  lists `src/runtime/recovery.rs` as a file to create, but none of Steps
  1–7 (the literal, verbatim-value instructions) ever define content for
  it, and the Step 7 commit command only adds `src/runtime` as a
  directory. Creating an empty `recovery.rs` would both be unused (barred
  by the "no `allow`" constraint, since an empty/unreferenced module
  triggers no code but adds a file with no responsibility) and would
  contradict CLAUDE.md's "No module is created before the task that fills
  it... An empty module documenting its own absence is a defect." I
  treated the Steps as authoritative over the stale Files header and did
  not create `src/runtime/recovery.rs`. Flagging this for the reviewer in
  case the intent was to split `Runtime::start`'s reconciliation-logging
  loop into its own file — as written, that logic is small (two `for`
  loops of `tracing` calls) and does not yet earn a second file.
- **Ids-are-String discrepancy** (checked, resolved): the brief's code
  returns `Result<String, StorageError>` from `register_runtime_instance`,
  and the Interfaces section names the return type `RuntimeInstanceId`
  informally. This appears to contradict the Global Constraints' "Ids are
  UUID-v4 newtypes" rule, but commit `ed0b991` already records this as a
  deliberate, spec-documented Milestone-0 exception (`docs/superpowers/
  specs/2026-09-21-domain-model-design.md`, OPEN block), closing at
  Task 9. I followed the brief's `String`-based signatures as written.

## Concerns

- The `src/runtime/recovery.rs` files-list vs. steps discrepancy noted
  above — no action taken beyond documenting it, since the Steps are
  self-consistent and complete without it.
- `Runtime::start`/`Runtime::stop` are not yet wired into `cli::serve`;
  this task's brief does not ask for that wiring, but a later task
  presumably will call `Runtime::start` at daemon boot to actually gate
  work acceptance on recovery completing, per spec §8.1.
