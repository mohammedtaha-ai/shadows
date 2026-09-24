# Task 7 — Managed process primitive and process-tree containment

## Status: DONE

## What was implemented

The `process/` module owns `tokio::process` and the OS containment primitive. Three files plus a test-support binary, all from the brief.

- `src/process/mod.rs` — public surface (`ProcessSpec`, `ProcessHandle`, `spawn`). `stdin(Stdio::null())`, `env_clear()` with `WINDOWS_ESSENTIAL_ENV` re-injected on Windows, containment wired before `cmd.spawn()` returns and the post-spawn `attach` opens/joins the handle.
- `src/process/containment_windows.rs` — Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. No `BREAKAWAY_OK` / `SILENT_BREAKAWAY_OK`, so descendants cannot escape. `Drop` on the job handle closes it, which kills the tree — the spec's "tree does not outlive its owning runtime" guarantee.
- `src/process/containment_unix.rs` — `setsid()` via `pre_exec`, `killpg(SIGKILL)` for deliberate termination. The brief is explicit that the parent-death half is unimplemented in Milestone 0.
- `src/bin/tree_probe.rs` — test-support binary. `--spawn-grandchild` builds the real grandchild the containment test needs; `--read-stdin` returns EOF immediately and is what the stdin-closed assertion relies on. Grandchild not `.wait()`ed because the Job Object (or the killpg on Linux) is what ends it; clippy allow with an explanatory comment.
- `tests/containment.rs` — two tests: real grandchild termination via Job Object, and stdin-closed returning EOF within 5 s.
- `Cargo.toml` — `windows = 0.58` (Win32_Foundation, Win32_Security, Win32_System_JobObjects, Win32_System_Threading), `libc = 0.2` for unix, `[[bin]] tree_probe`. **One small deviation from the brief**: `Win32_Security` had to be added because `CreateJobObjectW` takes `SECURITY_ATTRIBUTES` and is gated behind that feature in `windows` 0.58; without it the code does not compile.
- `src/lib.rs` — `pub mod process;` registered.

## What was tested and results

### RED (failing test before implementation)

Command: `cargo test --test containment`

Output before any `process/` code or `tree_probe` binary existed:

```
error: environment variable `CARGO_BIN_EXE_tree_probe` not defined at compile time
error[E0432]: unresolved import `shadows::process`
 --> tests\containment.rs:3:14
  |
3 | use shadows::process::{spawn, ProcessSpec};
  |              ^^^^^^^ could not find `process` in `shadows`
```

Compile failure on two independent fronts — `shadows::process` does not exist, and `CARGO_BIN_EXE_tree_probe` is unset because the binary is not declared. That's the RED.

### GREEN (passing tests after implementation)

Command: `cargo test --test containment`

```
running 2 tests
test a_spawned_child_has_no_inherited_stdin ... ok
test terminating_a_managed_tree_kills_the_grandchild_too ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; finished in 1.16s
```

The grandchild-killed test waits up to 5 s (50 × 100 ms) after `terminate_tree()` and asserts both pids are gone via `Get-Process`. The stdin test asserts EOF within 5 s; with inherited console stdin it would block and time out.

### Full suite

Command: `cargo test`

```
test result: ok. 2 passed  (containment)
test result: ok. 6 passed  (project_contract)
test result: ok. 4 passed  (recovery)
test result: ok. 1 passed  (serve_smoke)
test result: ok. 6 passed  (storage_contract)
test result: ok. 2 passed  (thread_contract)
```

No regressions in the 21 total tests.

### Lint and format

- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean after `cargo fmt` was applied (the brief's exact code had a few formatting nits that `cargo fmt` collapsed; semantics unchanged).

## Files changed

- `src/lib.rs` — register `pub mod process;`
- `src/process/mod.rs` — new
- `src/process/containment_windows.rs` — new
- `src/process/containment_unix.rs` — new
- `src/bin/tree_probe.rs` — new
- `tests/containment.rs` — new
- `Cargo.toml` — windows / libc deps, `[[bin]] tree_probe`
- `Cargo.lock` — updated by `cargo build` after the new deps

## Self-review findings

- **Brief deviation on `Cargo.toml`**: `Win32_Security` feature added because `CreateJobObjectW` requires it (its `SECURITY_ATTRIBUTES` parameter is gated behind that feature in `windows` 0.58). The brief's three-feature list compiled to an unresolved import on the first attempt. Logged here so the next reviewer sees why it is wider than the brief.
- **Brief deviation in `tests/containment.rs`**: removed an unused `use tokio::io::AsyncBufReadExt;` and the `mut` on `let mut lines = …` per clippy `-D warnings`. `Lines::next_line()` is the inherent method on `tokio::io::Lines`, so the trait import was dead, and clippy flagged the `mut` as unnecessary (the async block takes the value via the captured local). Both are visual no-ops; behaviour is identical. `cargo fmt` then collapsed a few cosmetic line breaks.
- **Brief deviation in `tree_probe.rs`**: added `#[allow(clippy::zombie_processes)]` on the grandchild `Command::spawn()` with a comment explaining why `.wait()` would defeat the test. The brief explicitly relies on the Job Object (Windows) / `killpg` (Linux) to reap the grandchild.
- **Containment verified on Windows**: the test ran against a real three-level process tree — `cargo test` daemon → `tree_probe --spawn-grandchild` → `tree_probe --sleep` — and `Get-Process` confirmed both pids absent within the 5 s window after `terminate_tree()`. That is the §3.7 Layer 4 evidence.
- **Linux half**: code is in place (`containment_unix.rs` with `setsid` + `killpg`) but the parent-death contract is unimplemented per the brief and the file's own doc comment. Milestone 0 only claims Windows; the `terminate_tree` path works on Unix by design.
- **No new module imports outside `process/`.** `agent/`, `planner/`, `runtime/` will pull `process/` in later tasks; nothing else in the tree references `process::*` yet. The private-only contract holds.

## Concerns

- The brief's `Cargo.toml` feature list omits `Win32_Security`, but `CreateJobObjectW` needs it. Treat this as a brief bug, not a Task 7 design choice — flagged in `git log` message and here so it does not get lost if the brief is reused elsewhere.

## Commits

- `5c07eee feat(process): managed spawn with Job Object tree containment on Windows`
