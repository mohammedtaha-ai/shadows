# Task 9 brief — Operation lifecycle, two-phase spawn for a Planner turn

BASE: `7f3a0f4` on branch `milestone-0/product-path`. Work on that branch.

## Authority — read these, do not trust this brief's summary of them

1. `CLAUDE.md` — project rules. The structure rules, the ordering rule, and the
   documentation rules all bind this task.
2. `docs/superpowers/plans/2026-09-21-milestone-0-browser-planner.md`:
   - `## Global Constraints` near the top — every task's requirements include it.
   - `## Task 9: Operation lifecycle — two-phase spawn for a Planner turn`.
3. `docs/superpowers/specs/README.md` — the index. Follow it to §2.7 (two-phase
   spawn), §8.3 (failure stages), §8.6 (terminal states and recovery CAS), and
   §4.1 (identity types). Read §4.1's **OPEN block** in full; ruling A below is
   that block, and the block was amended today.
4. `docs/codebase/README.md` and `docs/codebase/inventory.md` — what already
   exists. Read the inventory before writing a function; it lists every
   reachable declaration with its full signature.

## Rulings. These are decisions already taken, not suggestions.

**A. You land two newtypes, and only two: `OperationId` and `RuntimeInstanceId`.**

Spec §4.1 requires UUID newtypes and Milestone 0 has been carrying `String` ids
against that rule, deliberately and recorded. Task 9 is the trigger that closes
part of it, because your own API is where the hazard first becomes reachable:

```rust
mark_operation_started(op_id, expected_runtime)   // two Strings, indistinguishable
```

A caller that swaps those two arguments compiles cleanly and silently asserts the
wrong ownership. The predecessor project `shadow` reached 72k lines and died
partly on exactly one such reversed pair that 413 commits did not catch.

So:
- `OperationId` lives in `src/operation/mod.rs`. `RuntimeInstanceId` lives in
  `src/runtime/mod.rs` — its module already exists; do not create a second home
  for it.
- Both are UUID-v4 newtypes over `String`. Give each what its call sites need and
  nothing more; do not build a trait hierarchy or a macro for two types.
- `register_runtime_instance` currently returns `Result<String, StorageError>`.
  Change it to return `RuntimeInstanceId`, and update `stop_runtime_instance`,
  `reconcile_orphans` and `tests/recovery.rs` accordingly. That is in scope.
- **Do NOT touch `ProjectId`, `ThreadId` or `ThreadEntryId`.** That sweep is a
  separate change after your review, for reviewability. `thread_id` stays a
  `String` in your signatures and in `Operation`. Do not "improve" this.

**B. The harness version is frozen from the harness, not from the invocation.**

Spec §8.2 requires the harness path and version to be recorded when an Operation
starts. The plan's Task 9 Interfaces block inherits an error from Task 8's: it
implies `AgentInvocation` carries `harness_path`/`harness_version`. **It does
not** — Task 8 shipped `AgentInvocation` without them, and the version lives on
`ClaudeHarness.version`. Read `src/agent/mod.rs` and `src/agent/claude.rs` and
believe the code.

If recording the harness path and version per Operation needs a column the
migration does not have, say so in your report rather than inventing a column or
silently dropping the requirement. Adding a column to
`migrations/0001_milestone0.sql` is acceptable if §6 defines it; adding one §6
does not define is a spec question for your report.

**C. Every test must be shown failing before it passes.**

This is the standing rule on this project, from a ruling earned twice. Task 8's
suite counted delta lines and discarded the count, and the classifier's entire
transient arm could be disabled with every test still green. A counted-but-
discarded observation is not a test, and neither is an assertion that cannot
distinguish the behaviour it names.

For each test you write, ask: what one-line change to the implementation would
make this fail? If the answer is "none", the test is decoration. In particular,
your CAS tests must prove the CAS, not merely that a happy path works: a
transition fired against the wrong runtime, and a transition fired twice, must
both be refused, and the stored row must be shown unchanged after the refusal.

**D. Ordering is explicit.** CLAUDE.md: no `rowid`, no physical insertion order,
no implicit `SELECT` order. When you read events back to assert their sequence,
order by the explicit durable sequence.

**E. The code map is build-enforced.** You are creating `src/operation/` — a new
top-level module. It needs a row in `docs/codebase/README.md` giving its one job
**without the word "and"**, plus a reference file. Then, in the same commit:

```bash
UPDATE_CODEMAP=1 cargo test --test codemap
```

`cargo test` fails if you skip either. Your newtype changes also move signatures
the inventory records, so it must be regenerated regardless.

## Discipline

- **No `#[allow(...)]` attributes.** If something needs suppressing, the thing
  being suppressed is what to remove. Ruled on twice.
- **File sizes.** At 300 lines a file states its single responsibility; at 500 it
  splits, by responsibility and never by line count. `storage/sqlite/` is a named
  accretion point: your work goes in a new `operation.rs` there, never into
  `storage/sqlite/mod.rs` or another entity's file.
- **Run the whole suite, not just your own file.** Changing
  `register_runtime_instance`'s return type will break callers elsewhere. The
  last task shipped a green new file over a broken suite because only the new
  file was run.
- **Commit your work before you report.**
- Three gates, and paste the real output for each: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## Deliverable

Commit on `milestone-0/product-path` with the plan's Step 7 message, plus the
`docs/codebase` update from ruling E. Then report:

1. What you built, and the real RED output you saw before it passed.
2. For each test: the one-line implementation change that would break it.
3. The three gates' real output, for the whole suite.
4. Anything where the plan, a spec section, or this brief was wrong. Say it. This
   project has overturned five of its own rulings on exactly that basis, and the
   last two tasks each found a real defect in the plan's own shipped code.
