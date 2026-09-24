# Task 10 report: Cancellation — request, confirmed termination, terminal Cancelled

## Summary

Implemented the two-phase cancellation interlock (spec §2.3): `Storage::request_cancellation`
(TX #1) and `Storage::mark_operation_cancelled` (TX #2) in `src/storage/sqlite/operation.rs`,
and `src/planner/mod.rs` (new module) that spawns a real Planner turn (`PlannerTurn::start`)
and drives cancellation end-to-end (`PlannerTurn::stop`), wired through `src/lib.rs`. All
three brief-mandated resolutions applied (newtype ids, `NewThreadEntry` struct,
`take_stdout_lines` ownership transfer). Full gate green: `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test` (45 tests, 0 failures). Codemap regenerated.

## Files changed

- `src/storage/sqlite/operation.rs` — added `request_cancellation`, `mark_operation_cancelled`.
- `src/planner/mod.rs` — **new**. `LiveHandles`, `PlannerTurnRequest`, `PlannerTurn::start`/`stop`.
- `src/process/mod.rs` — `stdout_lines(&mut self) -> Option<&mut Lines<...>>` replaced with
  `take_stdout_lines(&mut self) -> Option<Lines<...>>` (Task 7 deferred minor M2, closed here).
- `src/lib.rs` — `pub mod planner;`.
- `src/bin/fake_claude.rs` — **new** test-support binary (not product code, same pattern as
  `tree_probe.rs`). Speaks the Claude stream-JSON shape without requiring the real `claude` CLI.
- `Cargo.toml` — `[[bin]] fake_claude`.
- `tests/containment.rs` — updated the two `stdout_lines()` call sites to `take_stdout_lines()`.
- `tests/operation_lifecycle.rs` — appended the brief's four cancellation tests, adapted to
  `Actor` (see Deviations) instead of two `&str`.
- `tests/planner_turn.rs` — **new**. Three full-stack tests (brief gave no content for this
  file — see below).
- `docs/codebase/README.md` — added the `src/planner/` ownership row.
- `docs/codebase/inventory.md` — regenerated via `UPDATE_CODEMAP=1 cargo test --test codemap`.

## Deviations from the brief's literal signatures, and why

Beyond the three pre-authorized resolutions:

1. **`request_cancellation` takes `Actor`, not `(by_kind: &str, by_id: &str)`.** The task
   instructions explicitly called this out: "check how `Actor` is used by existing event
   writers and follow that pattern." `mark_operation_cancelled` takes only `&OperationId`
   (actor is always `Actor::system()` internally, matching the brief).

2. **`PlannerTurn::start` takes a `PlannerTurnRequest` struct, not eight positional
   parameters.** The brief's own signature (`runtime, handles, harness, thread_id, prompt,
   cwd, resume_session_id, bus`) is eight arguments, which trips
   `clippy::too_many_arguments` (default threshold 7) under `-D warnings`. The task rules
   forbid `#[allow(...)]` anywhere. Rather than suppress the lint, `thread_id`, `prompt`,
   `cwd`, `resume_session_id` — three of which are string-typed neighbours a positional call
   site cannot tell apart — are bundled into `PlannerTurnRequest`, the same rationale this
   project already used for `NewThreadEntry` (cited in the doc comment). `runtime`, `handles`,
   `harness`, `bus` stay positional since each has a distinct type.

3. **`LiveHandles::contains` (test-support only).** The brief marks the inner map
   `pub(crate)` so `cli::serve` can enumerate it. That does not reach an integration test,
   which is a separate crate. Added a `#[cfg(feature = "test-support")] pub async fn
   contains(&self, op_id: &OperationId) -> bool` following the exact pattern already used by
   `storage::test_support::append_event_for_test` — compiled in for `cargo test` only, never
   for `cargo build`.

4. **`tests/planner_turn.rs` content and `src/bin/fake_claude.rs` were designed from
   scratch.** The brief's Step 1 only gave test content for `tests/operation_lifecycle.rs`;
   it named `tests/planner_turn.rs` as a required new file but supplied no content (confirmed
   by reading the full 384-line brief). `PlannerTurn::start` needs a real child process
   speaking the harness's stream-JSON contract to exercise end-to-end; spawning the real
   `claude` CLI is not available in this environment and would be flaky/non-hermetic even if
   it were. `fake_claude` is a `src/bin/` test-support binary (the row already covering
   `src/bin/` in `docs/codebase/README.md` covers it too — no README change needed for it),
   selected by the invocation's trailing prompt argument (`"hang"` sleeps after one line so a
   test can cancel a Running turn; anything else emits one entry, one turn-end, and exits).

## The bug TDD caught

Writing `cancelling_a_running_turn_confirms_termination_before_writing_cancelled` against my
first implementation (transcribed faithfully from the brief's discarded-result pattern)
**failed** with `TransitionConflict { expected: "Pending or Running", found: "already
terminal" }` on `PlannerTurn::stop`'s own `mark_operation_cancelled` call. Root cause: killing
the process via `terminate_tree()` also closes its stdout, which the background reader task
observes as ordinary EOF — so the reader task's own `mark_operation_completed` call and
`stop()`'s `mark_operation_cancelled` call raced as two independent database CASes, and the
reader task's was reliably winning (its EOF-triggered path is faster than `stop`'s 10ms-polled
`wait()` confirmation loop). The result was a self-killed operation non-deterministically
recorded as Completed instead of Cancelled — exactly the kind of misattribution spec §2.3
exists to prevent, just from an angle the brief's own code did not close.

Fix: the live-handle map is now the single arbitration point. Whichever side's
`handles.0.lock().await.remove(&op_id)` returns `Some` is the one permitted to write a
terminal transition; the other side, finding `None`, does not attempt one. This is a strictly
stronger, deterministic version of the brief's original intent ("a natural exit wins the
race") — it now also correctly resolves the case where the "exit" is one we ourselves forced
by killing the tree. Verified stable across 5 repeated runs after the fix (see GREEN below).

## Decisions on the three discarded results

1. **`let _ = reader_runtime.storage.append_thread_entry(...)`** — **handled, not
   discarded.** Changed to `if let Err(error) = ... { tracing::error!(...) }`. This is the
   durable write of the conversation itself, not a shutdown path; silently losing a message
   here would have nothing anywhere saying so. The stream loop continues rather than aborting
   the turn on one failed write — one lost message must not also kill the user's in-flight
   turn — but the loss is now surfaced loudly via `tracing::error!` with operation id, thread
   id, and the underlying error.

2. **`let _ = reader_handles...remove(&reader_op)`** — **kept discarding the handle, but the
   boolean result is now load-bearing**, not discarded. Original brief code discarded both the
   removed handle *and* whether removal succeeded, then unconditionally called
   `mark_operation_completed`. That is exactly the bug described above. The fix
   (`let owns_outcome = ....remove(&reader_op).is_some();`) keeps discarding the `ProcessHandle`
   itself (dropping it is correct and intentional — the process is already gone, so dropping
   its containment wrapper is a no-op cleanup, documented inline) but now branches on whether
   the removal actually happened, which is the fact that decides whether this task may write a
   terminal state at all.

3. **`let _ = handle.wait().await` in `stop()` after confirmed `terminate_tree`** — **kept
   discarded, with justification.** We just killed this process ourselves, so its exit status
   reports the kill, not an outcome of the turn. `Cancelled` records that the operation was
   stopped, not what its exit code was, so trusting/propagating that status would misattribute
   an OS-assigned kill result to the turn's own outcome. `wait()` returning at all (rather than
   its `Ok`/`Err` payload) is what constitutes confirmation here.

## TDD evidence

### Storage layer (`tests/operation_lifecycle.rs`)

**RED** — `cargo test --test operation_lifecycle` after appending the four tests but before
touching `src/storage/sqlite/operation.rs`:

```
error[E0599]: no method named `request_cancellation` found for struct `shadows::storage::Storage`
error[E0599]: no method named `mark_operation_cancelled` found for struct `shadows::storage::Storage`
error: could not compile `shadows` (test "operation_lifecycle") due to 6 previous errors
```
Expected: the methods do not exist yet, exactly as the brief predicted.

**GREEN** — after implementing `request_cancellation`/`mark_operation_cancelled`:
```
running 9 tests
test a_cancellation_request_does_not_make_an_operation_terminal ... ok
test a_natural_exit_wins_over_an_in_flight_cancellation ... ok
test a_terminal_operation_never_transitions_again ... ok
test every_transition_appends_its_event_atomically ... ok
test phase_one_persists_pending_before_anything_spawns ... ok
test unconfirmed_termination_leaves_the_operation_non_terminal ... ok
test the_running_transition_is_an_exact_compare_and_swap ... ok
test cancelled_is_written_after_confirmation_and_closes_the_operation ... ok
test prepare_failure_and_spawn_failure_are_distinguishable ... ok
test result: ok. 9 passed; 0 failed
```

### Planner layer (`tests/planner_turn.rs`)

**RED** — `cargo test --test planner_turn` before `src/planner/mod.rs` existed:
```
error[E0432]: unresolved import `shadows::planner`
error: could not compile `shadows` (test "planner_turn") due to 1 previous error
```

**RED (again, design bug)** — first working `src/planner/mod.rs` (faithful transcription of
the brief's discard pattern) compiled, and 2 of 3 tests passed, but:
```
thread 'cancelling_a_running_turn_confirms_termination_before_writing_cancelled' panicked
called `Result::unwrap()` on an `Err` value:
  TransitionConflict { expected: "Pending or Running", found: "already terminal" }
test result: FAILED. 2 passed; 1 failed
```
This is the race described above — a real defect the test caught, not a test bug.

**GREEN** — after the arbitration fix:
```
running 3 tests
test a_completed_turn_persists_the_stream_and_releases_its_handle ... ok
test cancelling_a_running_turn_confirms_termination_before_writing_cancelled ... ok
test stop_without_a_live_handle_records_the_request_and_stays_non_terminal ... ok
test result: ok. 3 passed; 0 failed
```
Repeated 5x sequentially (`--test-threads=1`) with no flakes.

### Full gate

```
cargo fmt --check            -> exit 0, no diff
cargo clippy --all-targets -- -D warnings  -> Finished, 0 warnings
cargo test                   -> 45 tests total across all suites, 0 failed
  codemap: 2, containment: 3, harness_config: 2, harness_stream: 5,
  operation_lifecycle: 9, planner_turn: 3, project_contract: 6, recovery: 4,
  serve_smoke: 1, storage_contract: 6, thread_contract: 4
UPDATE_CODEMAP=1 cargo test --test codemap -> regenerated inventory.md, both
  codemap tests pass afterward (the ownership-map test required rewording the
  new README row to avoid "and" — see below)
```

One codemap friction: my first `docs/codebase/README.md` row for `src/planner/` read "the
two-phase Planner turn spawn and cancellation interlock", which `tests/codemap/main.rs`
correctly rejected as a two-job description (conjunction rule). Reworded to "the Planner
turn's spawn-through-termination lifecycle" — one job, and the codemap test now passes.

## Self-review

- **Completeness against the brief:** both storage methods and both `PlannerTurn` methods are
  present and behave per spec §2.3/§8.4 cases 4-6. `LiveHandles` is `Default` with a
  `pub(crate)` inner map as specified.
- **YAGNI:** did not add `mark_operation_interrupted` (brief explicitly forbids it — belongs
  to `reconcile_orphans` alone). Did not add a generic `AgentHarness` trait-object parameter
  to `PlannerTurn::start` — kept it concretely `Arc<ClaudeHarness>` as the brief specifies,
  even though a trait object would have made `fake_claude` unnecessary; introducing that
  wasn't authorized by the three resolutions and the concrete type is what production callers
  will actually use.
- **Naming:** `PlannerTurnRequest` fields mirror `AgentInvocation`'s naming
  (`resume_session_id`, `cwd`) for consistency.
- **Can each test actually fail?** Verified affirmatively for all 7 new tests via the RED
  evidence above — four by removing the storage methods (compile error), three by the
  `planner` module not existing, and one of the three additionally caught a live race once the
  module did exist.
- **File size:** `src/storage/sqlite/operation.rs` is now 296 lines (was 218) — under the
  300-line trigger, no justification owed yet, but it is the accretion point CLAUDE.md names
  explicitly and is close to the line; the next addition to it should look at splitting by
  transition group rather than adding a sixth method inline. `src/planner/mod.rs` is 276
  lines, comfortably under 300 for a new module with two public entry points and their shared
  background task.
- **Domain purity:** `src/planner/` has no `sqlx`/`Row`/`Entity` imports — confirmed by
  reading the file; it only calls `Storage` methods and holds `ProcessHandle`/`OperationId`.
- **No `#[allow(...)]`** introduced anywhere (checked via grep across all changed/new files).

## Concerns

- `PlannerTurn::stop`'s cancellation actor is hardcoded to `Actor::user("local")`, matching
  the brief's own hardcoded `"User", "local"`. This is almost certainly a placeholder for a
  real principal once HTTP auth exists — flagging for whoever wires `cli::serve` to
  `PlannerTurn`, not fixing here since it is out of this task's scope and the brief specifies
  the same literal.
- `FailureStage::Run` is defined in `src/operation/mod.rs` but still unused by any caller
  (including this task's code) — pre-existing, not introduced or worsened here.
- The arbitration-point fix is my own design addition beyond the brief's transcribed intent.
  It is the correct fix for a real, reproduced race, not speculative hardening — I verified it
  fires deterministically (the un-fixed version failed 1 run out of 1 attempted, consistently)
  and the fixed version passed 5/5 repeated runs.

---

# Task 10 fix report — round 1 of 5

Commit: `558171a` — `fix(planner): the interlock stops guessing which ending a turn had`.
All eight findings fixed. Full gate green on Windows: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test` (48 tests, 0 failures — 45
before, 3 new). Codemap regenerated in the same commit. No `#[allow(...)]` added.

## Per finding

**1. `stop` claimed outcome ownership before it could confirm termination.**
`stop` no longer keeps the registration it cannot back up. The handle is removed under
the lock, and on a `terminate_tree` error it is put straight back
(`src/planner/mod.rs`, the §8.4-case-6 branch) before returning `Ok(())`. Ownership —
which is what the registration now means — is claimed only once the kill is under way.
The reader therefore still finds `Some` on its own `remove`, still owns the outcome,
and the `ProcessHandle` that owns the containment is still reachable. New test:
`unconfirmed_termination_keeps_the_handle_and_leaves_the_operation_non_terminal`.

**2. The arbitration inverted §8.4 case 4.** The tiebreak was "whoever takes the lock
first". It is now the observed fact, carried on the registration: `LiveTurn` holds an
`Arc<AtomicBool>` the reader sets the instant it classifies the turn's `TurnEnd`
(before the item reaches the bus), and `LiveTurn::ended_on_its_own` is that flag OR
`ProcessHandle::has_exited` (a new non-blocking `try_wait`). Either signal alone is
incomplete — a `TurnEnd` precedes the exit, and a child can die without one — so both
are consulted. When it fires, `stop` puts the registration back and writes no
`Cancelled`: the reader owns the outcome and persists the real exit. New test:
`a_turn_that_ended_on_its_own_is_never_overwritten_by_a_cancellation`.

*How the window is made observable rather than hoped for:* `fake_claude` gained a
`slow-exit` mode that prints its entry and its turn-end result and then stays alive for
two seconds with stdout open. The reader is therefore blocked in `next_line` and cannot
have reached its arbitration, while the turn has demonstrably produced its ending. The
test subscribes to the stream bus and proceeds only after receiving `StreamItem::TurnEnd`
— which the reader publishes strictly after setting the flag — so the contested state is
entered deterministically, not raced for.

**3. The terminal-state write was a silent `let _ =`.** It is now
`if let Err(error) = … { tracing::error!(…, "planner.terminal_transition_failed: a
finished turn was left non-terminal") }`, matching the treatment the same task gave
`append_thread_entry` forty lines above. Since fix 2 this branch is the sole writer for
a naturally-ending turn.

**4. The stream `uuid` was written into `Actor.id`.** The author is now
`Actor { kind: "Agent", id: <the invocation's role, "Planner"> }` — one actor for every
line of the turn. The stream's per-line `role` ("assistant", or "user" on a tool-result
echo) is a label on the transport, not a second author, and is dropped with the uuid;
the reasoning is stated at the call site. The uuid gets no column: an OPEN block was
added to §6.5 `thread_entry` in `docs/superpowers/specs/2026-09-21-sqlite-schema-design.md`,
written in the form and tone of `e302ac4`'s `agent_invocation` block, with the trigger
that closes it — the first feature that must match a stored entry to a harness-side
line (resume dedupe, or replay). Covered by a new assertion in
`a_completed_turn_persists_the_stream_and_releases_its_handle`.

**5. A child that died without a `TurnEnd` was recorded `Completed`.** The reader owns
the handle once it owns the outcome, so it now calls `wait()` and consults the real
exit. `TurnEnd` present and a success status → `mark_operation_completed` with that
outcome; `TurnEnd` present and a bad status → `Failed`/`Run`, "the harness emitted its
turn-end result, then {status}"; no `TurnEnd` → `Failed`/`Run`, "the harness ended
without a turn-end result: {status}". The reason names which of the two it was, as
required. The `while let Ok(Some(line))` loop became an explicit `match`, so a stdout
read error is logged as one (`planner.stream_read_failed`) instead of being
indistinguishable from EOF. `FailureStage::Run` now has its caller. The plan's Task 10
Step 4 is amended in place with a paragraph stating the rule and why the block below it
is superseded (also covering fixes 1 and 2), so the defect does not outlive this round.
New test: `a_child_that_dies_without_a_turn_end_is_failed_at_the_run_stage`, backed by a
`crash` mode in `fake_claude` (entry, no result line, `exit(3)`).

**6. A `mark_operation_started` failure leaked a live process.** The `?` became an
explicit `if let Err(error)` that removes the registration, calls `terminate_tree`, and
awaits `wait()` before propagating. No test: forcing that storage call to fail needs a
fault-injection seam in `Storage` that nothing else in the suite has, which is a larger
change than the fix. Stated here rather than claimed as covered.

**7. Nothing asserted `outcome_json`.** `a_completed_turn_persists_the_stream_and_releases_its_handle`
now parses the persisted outcome and compares it to
`{"subtype":"success","stop_reason":"end_turn"}` (parsed, not string-compared, so key
ordering is not load-bearing), plus the author assertion from fix 4.

**8. `AgentInvocation.operation_id` was a `String`.** Now `OperationId`.
`src/planner/mod.rs` drops its `op_id.as_str().to_string()`; `tests/harness_stream.rs`
uses `OperationId::from_literal`, the existing `test-support`-gated constructor.

## The test-support seam, and why it exists

§8.4 case 6's live-handle branch (finding 1) cannot be reached otherwise: on Windows —
this project's acceptance gate — `start_kill` against a live child does not fail on
demand. `ProcessHandle` gained a `#[cfg(feature = "test-support")] termination_fails`
field, a `force_termination_failure()` that sets it, and a check at the top of
`terminate_tree`; `LiveHandles::force_termination_failure(op_id) -> bool` reaches the
registered handle from an integration test and returns whether there was one to arm, so
a test cannot pass by arming nothing. This follows `storage::test_support`,
`OperationId::from_literal` and `LiveHandles::contains`: `cargo test` enables the
feature through the self dev-dependency, `cargo build` does not, and the field does not
exist in anything that ships (`cargo build` verified green).

## Evidence

### RED — every new or strengthened assertion was shown to bite

Each was produced by breaking the fix in `src/planner/mod.rs`, running the test, and
restoring the file.

Finding 1 — removed the `map.insert(op_id.clone(), turn)` on the `terminate_tree` error
path (i.e. the pre-fix behaviour), `cargo test --test planner_turn unconfirmed_termination_keeps`:

```
test unconfirmed_termination_keeps_the_handle_and_leaves_the_operation_non_terminal ... FAILED
panicked at tests\planner_turn.rs:256:5:
an unconfirmed kill must leave the handle registered — it is the only thing that can still reach the tree
test result: FAILED. 0 passed; 1 failed
```

Finding 2 — deleted the `if turn.ended_on_its_own()` branch,
`cargo test --test planner_turn a_turn_that_ended_on_its_own`:

```
test a_turn_that_ended_on_its_own_is_never_overwritten_by_a_cancellation ... FAILED
panicked at tests\planner_turn.rs:307:5:
assertion `left == right` failed: a turn that ended on its own is not something Shadows stopped
  left: "Cancelled"
 right: "Completed"
test result: FAILED. 0 passed; 1 failed
```

This is the reviewer's finding reproduced exactly: `Cancelled` written over a turn that
had already ended on its own.

Finding 5 — restored the unconditional `mark_operation_completed` with the default
outcome, `cargo test --test planner_turn a_child_that_dies`:

```
test a_child_that_dies_without_a_turn_end_is_failed_at_the_run_stage ... FAILED
panicked at tests\planner_turn.rs:349:5:
assertion `left == right` failed: a crashed turn is not a completed one
  left: "Completed"
 right: "Failed"
test result: FAILED. 0 passed; 1 failed
```

Finding 7 — made the `TurnEnd` branch store the default outcome,
`cargo test --test planner_turn a_completed_turn`:

```
panicked at tests\planner_turn.rs:116:5:
assertion `left == right` failed: the persisted outcome must be the one the harness's turn-end reported
  left: Object {"stop_reason": Null}
 right: Object {"stop_reason": String("end_turn"), "subtype": String("success")}
```

Finding 4 — restored `Actor { kind: "assistant", id: "fake-entry-1" }`, same command:

```
panicked at tests\planner_turn.rs:132:5:
assertion `left == right` failed
  left: "assistant"
 right: "Agent"
```

### GREEN

`cargo test --test planner_turn`:

```
running 6 tests
test stop_without_a_live_handle_records_the_request_and_stays_non_terminal ... ok
test unconfirmed_termination_keeps_the_handle_and_leaves_the_operation_non_terminal ... ok
test cancelling_a_running_turn_confirms_termination_before_writing_cancelled ... ok
test a_completed_turn_persists_the_stream_and_releases_its_handle ... ok
test a_child_that_dies_without_a_turn_end_is_failed_at_the_run_stage ... ok
test a_turn_that_ended_on_its_own_is_never_overwritten_by_a_cancellation ... ok
test result: ok. 6 passed; 0 failed
```

Repeated 5x with `--test-threads=1`: 6 passed each time, no flakes (3.7s–4.6s).

Full gate:

```
cargo fmt --check                          -> exit 0, no diff
cargo clippy --all-targets -- -D warnings  -> Finished, 0 warnings
cargo build                                -> Finished (test-support off: the seam does not ship)
cargo test                                 -> 48 tests, 0 failed
  codemap 2, containment 3, harness_config 2, harness_stream 5, operation_lifecycle 9,
  planner_turn 6, project_contract 6, recovery 4, serve_smoke 1, storage_contract 6,
  thread_contract 4
UPDATE_CODEMAP=1 cargo test --test codemap -> regenerated, both codemap tests pass
```

## File size

`src/planner/mod.rs` is 447 lines, past the 300-line trigger and under the 500-line
split. Its one job is the Planner turn's spawn-through-termination lifecycle. The new
code is that job: the stream reader and `stop` are the two sides of one arbitration that
§8.4 requires exactly one winner of, and the rule deciding which side wins is only
reviewable if both are read together — separating them yields two files that must be
read as one, the split CLAUDE.md says does not count. That argument is now in the module
doc so the next person meets it before the line count. Most of the growth is comments.
`src/storage/sqlite/operation.rs` is untouched at 296 lines.

## Concerns for the re-review

1. **A harness that hangs after its turn-end cannot be stopped.** By design, `stop`
   declines to terminate once `ended_on_its_own` is true, so a child that emits
   `TurnEnd` and then never exits keeps running with its registration held and the
   operation `Running`. This is the cost of finding 2's prescribed tiebreak. The
   alternative — terminate anyway but yield the outcome write — needs a second flag
   ("we killed it, you still own it") so the reader does not read our own kill as a
   non-success exit and record `Failed`. That is more machinery inside the subtlest
   part of the interlock; I judged the current shape the better trade, but it is a
   judgement and the controller may disagree. Containment still covers the process:
   the registration is held, so daemon shutdown and `KillOnDrop` both reach the tree.
2. **The reader blocks in `wait()` after EOF.** A harness that closes stdout and keeps
   running holds the reader there, with the registration already claimed, so `stop`
   finds nothing. The operation stays `Running`, which is honest — the turn has not
   ended — and recovery resolves it as `Interrupted` after a restart. Noted rather
   than fixed: consulting the status without waiting would misreport the common case,
   where the child exits milliseconds after EOF.
3. **Finding 6 has no test**, for the reason given above.
4. `PlannerTurn::stop`'s cancellation actor is still hardcoded `Actor::user("local")`
   (unchanged, out of scope, carried from the previous round's concerns).

---

# Task 10 fix report — round 2 of 5

Commit: `4d7a13e` — `fix(planner): terminating a tree and naming its ending are two decisions`.
All three items addressed. Full gate green on Windows: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo build` (feature off),
`cargo test` (49 tests, 0 failures — 48 after round 1, 1 new). Codemap regenerated in
the same commit. No `#[allow(...)]` added.

## 1 (Important) — `stop` declined to terminate a live tree once `turn_end_seen` was set

Ruling accepted; the round-1 reasoning was wrong for the reason given. §8.4 case 4's
precondition is that the process *exited*, and a `TurnEnd` is not that fact — round 1's
own case-4 test demonstrates the gap, since its premise is a harness that is
demonstrably alive after reporting its result.

The fusion is undone. `LiveTurn`'s doc comment now states the split as the rule, and
each branch of `stop` names the case it serves:

- `handle.has_exited()` → nothing to terminate, registration goes back, the reader
  writes the real exit (case 4). This is the only thing that branch asks; the comment
  says so explicitly, including what it is *not* asking.
- still alive → `terminate_tree`, and on error the registration goes back with no
  `Cancelled` (case 6, unchanged from round 1).
- terminated, and `turn_end_seen` → case 3 is served, and the ending is the turn's own.
  `stop` sets `LiveTurn::terminated_by_stop`, puts the registration back, and writes
  nothing. The reader then owns the outcome and persists it.
- terminated, no turn end → `wait`, then `Cancelled` (case 3, as before).

`terminated_by_stop` is a plain `bool` on `LiveTurn`, not an `Arc<AtomicBool>`: `stop`
mutates it under the map lock before re-inserting, and only whoever removes the
registration reads it, so there is nothing to share. The reader consults it in exactly
one place — `exit_is_the_turns` — so a killed process's status is not read as the turn's
verdict. Without that field the reader records `Failed`/`Run` for a turn the harness
reported as successful; the RED evidence below shows precisely that.

`fake_claude`'s `slow-exit` sleep went from 2 seconds to 120. At 2 seconds a `stop` that
terminated nothing would still see the process disappear on its own, so the test could
not distinguish "we killed it" from "we waited for it" — which is the whole claim under
test.

Test (renamed, since it now covers both obligations):
`a_cancelled_turn_that_had_already_ended_keeps_its_outcome_and_loses_its_tree`. It
enters the contested window deterministically through the bus as before, captures the
leader pid via a new `test-support` accessor `LiveHandles::pid`, and asserts both halves:
the persisted state is `Completed` carrying the harness's own outcome with
`cancel_requested_at` retained as history, **and** the pid is gone within a bounded poll.
`is_alive` is duplicated from `tests/containment.rs` (each integration test is its own
crate); the Unix arm keeps that file's zombie distinction.

*Note on what this test no longer covers.* With `slow-exit` alive for two minutes, the
`has_exited` branch of `stop` is no longer exercised by it. That branch's behaviour is
covered at the storage layer by `a_natural_exit_wins_over_an_in_flight_cancellation`
(`tests/operation_lifecycle.rs`); a full-stack version would need the process to exit
between the request and the lock, which cannot be made deterministic from a test.
Flagged rather than left implied.

## 2 (Minor) — a failing turn-end subtype with a clean exit was `Completed`

Fixed in the same match. A turn is `Completed` only when both verdicts agree: the
harness's (`subtype`) and the process's (exit status, or `terminated_by_stop`). A
`subtype` other than success is `Failed`/`Run`, reason "the harness reported a failing
turn end: {subtype}".

**On the subtype values — please read this.** The coordinator's message says
`error_max_turns` is "a real subtype of the measured harness contract in
`docs/evidence/harness/`". It is not in that directory: `grep -rn "error_max_turns\|max_turns"
docs/` returns nothing, and `SERVE_STREAM_SPIKE.md` contains exactly one `subtype` value
for a `result` line, `"success"` (the other `subtype` mentions are `system/*` lines). So
I did not branch on a list. The code holds `const TURN_END_SUCCESS: &str = "success"` —
the one value measured to mean success — and treats everything else as a failing
verdict, with the comment stating why a blacklist would be invented and would record the
next unmeasured failure subtype as a completed turn. If `error_max_turns` is known from
outside the evidence file, that knowledge belongs in the evidence file; this code does
not depend on it either way. `fake_claude`'s new `failing-turn-end` mode uses the string
`error_max_turns` purely as *a* non-success value, and its comment says the specific
string is not load-bearing.

New test: `a_failing_turn_end_is_not_completed_even_on_a_clean_exit`.

## 3 (Small) — nothing failed if `test-support` stopped being off

A plain `cargo build` does not actually catch this: if the feature became default the
build would simply succeed with the seams compiled in. So the new CI step builds what
ships **and then asks Cargo**:

```yaml
- name: Ship build — the test-only seams must not be in it
  shell: bash
  run: |
    cargo build
    if cargo tree -e features,no-dev --package shadows | grep -q 'test-support'; then
      echo "::error::test-support is enabled in the shipping build - a test-only seam would ship"
      exit 1
    fi
```

Added to both jobs, before `Test`, with a comment naming all three seams
(`storage::test_support`, `<Id>::from_literal`,
`ProcessHandle::force_termination_failure`), why no other step can notice, and that a
failure means fixing the gating rather than relaxing the step. Verified locally that the
check discriminates: `cargo tree -e features,no-dev` matches `test-support` 0 times,
`cargo tree -e features` (dev graph, what `cargo test` builds) matches it once.

## Evidence

### RED — the round-2 fix, both halves

Termination half — restored the fused condition
(`if turn.handle.has_exited() || turn.turn_end_seen.load(...)`, i.e. round 1),
`cargo test --test planner_turn a_cancelled_turn_that_had_already_ended`:

```
test a_cancelled_turn_that_had_already_ended_keeps_its_outcome_and_loses_its_tree ... FAILED
panicked at tests\planner_turn.rs:70:5:
operation did not reach a terminal state in time
test result: FAILED. 0 passed; 1 failed
```

The process was never killed, so it was still sleeping when the bounded wait ran out —
the unstoppable harness, reproduced.

Outcome half — dropped `terminated_by_stop` from `exit_is_the_turns`, same command:

```
panicked at tests\planner_turn.rs:380:5:
assertion `left == right` failed: the turn's own ending wins: not Cancelled, and not a
Run failure read off the exit status of a kill Shadows itself performed
  left: "Failed"
 right: "Completed"
test result: FAILED. 0 passed; 1 failed
```

The reader read our own kill as the turn's verdict — exactly the reason the second field
exists.

Item 2 — disabled the subtype check (`if false && verdict != TURN_END_SUCCESS`),
`cargo test --test planner_turn a_failing_turn_end`:

```
test a_failing_turn_end_is_not_completed_even_on_a_clean_exit ... FAILED
panicked at tests\planner_turn.rs:318:5:
assertion `left == right` failed
  left: "Completed"
 right: "Failed"
```

### GREEN

```
running 7 tests
test stop_without_a_live_handle_records_the_request_and_stays_non_terminal ... ok
test cancelling_a_running_turn_confirms_termination_before_writing_cancelled ... ok
test unconfirmed_termination_keeps_the_handle_and_leaves_the_operation_non_terminal ... ok
test a_child_that_dies_without_a_turn_end_is_failed_at_the_run_stage ... ok
test a_completed_turn_persists_the_stream_and_releases_its_handle ... ok
test a_failing_turn_end_is_not_completed_even_on_a_clean_exit ... ok
test a_cancelled_turn_that_had_already_ended_keeps_its_outcome_and_loses_its_tree ... ok
test result: ok. 7 passed; 0 failed
```

Repeated 5x with `--test-threads=1`: 7 passed each time, no flakes (2.3s–4.1s).

Full gate:

```
cargo fmt --check                                     -> exit 0, no diff
cargo clippy --all-targets -- -D warnings             -> Finished, 0 warnings
cargo build                                           -> Finished
cargo tree -e features,no-dev --package shadows       -> 0 matches for test-support
cargo test                                            -> 49 tests, 0 failed
  codemap 2, containment 3, harness_config 2, harness_stream 5, operation_lifecycle 9,
  planner_turn 7, project_contract 6, recovery 4, serve_smoke 1, storage_contract 6,
  thread_contract 4
UPDATE_CODEMAP=1 cargo test --test codemap            -> regenerated, both tests pass
```

## File size — this crosses 500 and I am asking for the judgement, not assuming it

`src/planner/mod.rs` is **527 lines**, past CLAUDE.md's split threshold. 239 of those
lines are comments or blank, so the code is about 288 lines; the growth this round is
one field, one branch, and the comments that name which §8.4 case each branch serves.
CLAUDE.md's own wording is "a 520-line file with one genuine responsibility survives
review by saying so", so here is the saying-so, and the controller can overrule it.

Its one job: the Planner turn's spawn-through-termination lifecycle. The test CLAUDE.md
gives — name each resulting file's one job without using "and" — is what argues against
splitting here. The only seam I can find is `LiveHandles` + `LiveTurn` (the registry and
the two facts the arbitration reads) in one file, `PlannerTurn::start`/`stop` in another.
That produces exactly the split the rule forbids: `LiveTurn`'s field comments state the
arbitration rule (`has_exited` decides termination, `turn_end_seen` decides only who
names the ending) and the branches in `stop` are that rule executed. A reviewer checking
§8.4 would have to hold both files open, which is the "same pile under a new name"
outcome CLAUDE.md describes.

What would genuinely relieve it is not a split but a subtraction: the file carries
`PlannerTurnRequest`, the `start` prepare/spawn path, the stream reader, and the
arbitration, and only the last two are inseparable. If the controller wants it under 500
now, the honest cut is `src/planner/spawn.rs` owning "turn a request into a registered,
Running operation" with `mod.rs` keeping "decide and persist how a live turn ends" —
those are two jobs statable without "and", and `start`'s prepare/spawn half does not
participate in the interlock. I did not do it in a fix round that the review scoped to
eight findings plus three items; say the word and it is the next commit.

## Concerns for the re-review

1. **The `has_exited` branch of `stop` has no full-stack test**, as described under item
   1 above. Storage-layer coverage exists; the full-stack window is not deterministically
   constructible.
2. **`error_max_turns` is not in `docs/evidence/harness/`.** Reported under item 2. The
   code does not depend on the specific value, but the discrepancy is worth resolving in
   the evidence file rather than in my memory or the coordinator's.
3. Round-1 concerns 2–4 stand as deferred by this round's message (reader holds the
   registration across `wait()`; hardcoded `Actor::user("local")`; `stop` returning
   `Ok(())` for several outcomes; finding 6 untested).
