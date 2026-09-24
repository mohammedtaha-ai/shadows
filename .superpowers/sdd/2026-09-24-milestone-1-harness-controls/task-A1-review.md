# Task A1 review — 2026-09-24

Base `b98b565`; reviewed head `a093401e963c1b2022650aa3927bed430f8281c0` on `milestone-1/harness-controls`. Scope: Task A1 brief, binding spec §12.2, and the prepared diff. No tracked changes or fix commit resulted from this review.

## Verdicts

- **Spec compliance: PASS for A1.** The pinned adapter and lockfile are present (`harness/claude/package.json:1-7`, `harness/claude/package-lock.json:9,16`); the Rust ACP dependency is exactly pinned (`Cargo.toml:10`). All three CLI paths use the same absolute and existing-path parser (`src/cli/args.rs:27-36,55-58`) and enter `Config` (`src/cli/args.rs:75-83`). The adapter version reads the package two levels above its entry (`src/config.rs:105-120`), while startup logs that version with the Claude version probe (`src/cli/mod.rs:25-27`). The process layer pipes stdin on request and retains all three stdio handles for one-time transfer (`src/process/mod.rs:71-81,184-200,242-264`). Existing callers explicitly use `pipe_stdin: false` (`src/agent/claude.rs:50-56`, `src/cli/mod.rs:101-108`). Running `node <adapter entry>`, setting `CLAUDE_CODE_EXECUTABLE`, and live ACP exchange in spec §12.2 are subsequent integration work, not claimed by A1's brief.
- **Code quality: PASS.** CLI argument ownership is in `src/cli/args.rs:10-88`; `src/main.rs:1-7` only parses and dispatches. Process ownership stays within `src/process/mod.rs`; the all-or-nothing check in `take_stdio` prevents partial consumption when a handle is absent (`src/process/mod.rs:71-80`). The current tests cover piped echo, one-time stdio transfer, closed stdin for old callers, and managed tree termination (`tests/containment.rs:242-286`).

## Prioritized issues

No confirmed in-scope defects. No corrections were made. The A1 brief prescribed a failing-test step, but the implementer report says the original red run was not recoverable; this is a process-evidence gap, not a code defect (`task-A1-report.md`, Verification). The full §12.2 behavior is not validated by these tests: no live adapter session or Linux run has been demonstrated. A2 and later acceptance gates must supply that evidence.

## Strengths

- Exact package and Rust pins prevent unplanned adapter or protocol drift (`harness/claude/package.json:6`, `Cargo.toml:10`).
- Config validation rejects relative and absent paths before subprocess startup (`src/config.rs:89-103`; `tests/harness_config.rs:14-35`).
- Interactive stdio preserves stderr for the protocol caller, while legacy noninteractive children continue to drain it (`src/process/mod.rs:242-254`).
- The version probe has a startup timeout, so a hanging `--version` executable does not hold the daemon indefinitely (`src/cli/mod.rs:97-138`).

## Verification performed in this review

- `cargo test --test containment --test harness_config` with `RUST_LOG` removed from the PowerShell environment: **11 passed, 0 failed** on Windows.
- `git diff --check b98b565..a093401e963c1b2022650aa3927bed430f8281c0`: exit 0.
- `git status --short --branch` before review: clean tracked tree on `milestone-1/harness-controls`.

The implementer separately reports a final `cargo test` (109 passed), Clippy, fmt, and code map checks. Those results were not rerun in this review and are reported as implementer evidence, not reviewer execution.
