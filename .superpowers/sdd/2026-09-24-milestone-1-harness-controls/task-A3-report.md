# Task A3 report — one live ACP connection per thread

Status: DONE_WITH_CONCERNS
Commit: `250a4d0f13d7b67c4da295f4a0acb2f059e60b65`

## Scope delivered

- `src/planner/sessions.rs`: session opening and reuse, `acceptEdits` default, durable resume lookup, dead connection replacement, event receiver handoff, idle reaping, per-thread termination, and shutdown close.
- `src/agent/acp.rs`: exposes incoming transport closure to detect a dead connection before its process exit becomes observable.
- `src/planner/spawn.rs`: moved the existing project workspace check to the session owner; the old Planner path continues to call it until A4.
- `src/cli/mod.rs` and `src/protocol/mod.rs`: construct Sessions from configured paths, carry it in app state, and close all adapters during shutdown. Existing protocol test fixtures carry `None` until A4 migrates their turn paths.
- `tests/sessions.rs`: seven fake ACP integration tests. Code map owner line and generated inventory updated.

## RED / GREEN

- RED: `Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue; cargo test --test sessions` failed to compile because `OpenError`, `OpenSession`, `Sessions`, and `SessionsConfig` did not exist.
- First GREEN run: 4 of 5 tests passed. Dead connection replacement failed because ACP transport closure became visible before process exit.
- Added `Connection::is_closed`; next `cargo test --test sessions` passed 5 of 5.
- Added two focused coverage cases for reaper behavior while events are taken and explicit termination; final `cargo test --test sessions` passed 7 of 7.

## Exact verification commands and results

- `cargo fmt --all`: exit 0.
- `$env:UPDATE_CODEMAP='1'; cargo test --test codemap; Remove-Item Env:UPDATE_CODEMAP`: 2 passed.
- `Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue; cargo test`: exit 0, all tests passed, including 7 Sessions tests. Windows host only.
- `cargo clippy --all-targets -- -D warnings`: initially failed on two `collapsible_if` warnings in Sessions.
- `cargo fmt --all; cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings; Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue; cargo test --test sessions; cargo test --test codemap`: exit 0; fmt check and clippy clean, 7 Sessions tests and 2 code map tests passed.
- `git diff --check`: exit 0.

## Self-review and concerns

- The session registry uses one mutex, so simultaneous opens for one thread serialize and cannot spawn two adapters. The idle reaper skips a receiver held by a turn. Setup failure terminates and reaps its newly spawned process before returning `OpenError::Start`.
- `SessionsConfig::cancel_wait` is available for A4's cancellation integration; A3 does not use it.
- The existing Planner stream-json turn path still runs until A4 changes it. A3's Sessions object is constructed in the daemon and closed on shutdown, but no HTTP turn uses it yet. This is the intended A3/A4 task boundary.
- No real Claude adapter acceptance run or Linux CI run was performed in A3.
