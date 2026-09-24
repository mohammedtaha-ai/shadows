# Task 8 brief — the Claude harness and its stream contract

BASE: `46555ea` on branch `milestone-0/product-path`. Work on that branch.

## Authority — read these, do not trust this brief's summary of them

1. `CLAUDE.md` — project rules. The structure rules and the documentation rules
   both bind this task.
2. `docs/superpowers/plans/2026-09-21-milestone-0-browser-planner.md`:
   - `## Global Constraints` near the top — every task's requirements include it.
   - `## Task 8: The Claude harness and its stream contract` — your task, with
     the test code and the implementation code to write.
3. `docs/evidence/harness/SERVE_STREAM_SPIKE.md` — the **measured** invocation
   and the four stream classes. This is the contract. Where the plan and this
   evidence file disagree about a flag or a line shape, the evidence file was
   measured and the plan was typed; say so in your report rather than choosing
   silently.
4. `docs/codebase/README.md` and `docs/codebase/inventory.md` — what already
   exists. Read the inventory before writing a function, so you do not write a
   second one of something.

## Four pre-flight findings. These are rulings, not suggestions.

**A. The plan's Task 8 "Interfaces" block contradicts its own Step 4 code.**
Interfaces lists `AgentInvocation { ..., harness_path, harness_version, ... }`.
The struct in Step 4 has neither field and instead has `session_id`, and the
Step 2 tests construct the Step 4 shape.

Ruling: **Step 4's struct and the Step 2 tests are authoritative.** The
Interfaces block is stale. Do not add `harness_path`/`harness_version` to
`AgentInvocation`.

Consequence you must record in your report: spec §8.2 requires the harness
version to be frozen per Operation, and it now lives on `ClaudeHarness.version`
rather than on the invocation. Task 9 therefore has to read it from the harness.
State that plainly so Task 9's dispatch carries it.

**B. The capture command in Step 1 carries `--safe-mode`, which the product does
not emit.** `to_process_spec` does not produce it, and the measured invocation in
the evidence report does not contain it.

Ruling: **capture the fixture with exactly the flags `to_process_spec`
produces.** Drop `--safe-mode`. A fixture captured under flags the product never
sends is not evidence about the product's invocation, and the whole point of
Step 1 is that the classifier is tested against a real turn rather than an idea
of one. If you believe `--safe-mode` belongs in the contract, that is a spec
amendment to propose in your report — never a flag that exists only in the
capture.

**C. `< /dev/null` in the capture command is a POSIX redirect and this is
Windows.** Closing the child's stdin is not optional: evidence Finding 5.1 says
a child with open stdin waits three seconds every turn. Use the Bash tool (Git
Bash is available) so the redirect works.

Before committing the fixture, verify it yourself:
- it is non-empty;
- its last non-empty line has `"type":"result"`;
- at least one line has `"type":"assistant"`;
- at least one line has `"type":"system"` carrying a `session_id`.

If the capture fails or produces none of the above, **stop and report.** Do not
hand-write a fixture, and do not adjust the tests to pass against a partial one.
A hand-written fixture would make all four tests pass while proving nothing,
which is the exact failure this task exists to prevent.

**D. The code map is build-enforced as of commit `46555ea`, and you are creating
a new top-level module.** `src/agent/` must get its own row in
`docs/codebase/README.md`: its one job stated **without the word "and"**, plus a
reference file. Then regenerate the inventory in the same commit:

```bash
UPDATE_CODEMAP=1 cargo test --test codemap
```

`cargo test` fails if you skip either. Read `tests/codemap/main.rs` for why.

## Discipline

- **TDD, and prove the RED.** Write the Step 2 tests first, run them, and paste
  the actual failure output into your report. A test you never saw fail is not
  evidence of anything.
- **No `#[allow(...)]` attributes.** If something needs suppressing, the thing
  being suppressed is what to remove. This has been ruled on twice already.
- **File sizes.** At 300 lines a file must state its single responsibility; at
  500 it splits, by responsibility and never by line count. `src/agent/mod.rs`
  and `src/agent/claude.rs` are a deliberate split already — keep the trait and
  the types in `mod.rs`, the Claude-specific implementation in `claude.rs`.
- **Commit your work before you report.** A report describing uncommitted work
  has cost this project a whole dispatch before.
- Three gates must be clean, and paste the real output for each:
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## Deliverable

Commit on `milestone-0/product-path` with the message from the plan's Step 7,
plus the `docs/codebase` update from ruling D. Then report:

1. What you built, and the RED output you saw before it passed.
2. The three gates' real output.
3. The fixture's verified properties (the four checks in ruling C).
4. Anything where the plan, the evidence file, or this brief was wrong. Say it;
   a brief being wrong is a finding, not an obstacle to work around. This
   project has overturned four of its own rulings on exactly that basis.
