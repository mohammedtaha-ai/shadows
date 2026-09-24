# Task A2 report — ACP connection and fake agent

## Status

DONE. Implemented on `milestone-1/harness-controls` from `a093401e963c1b2022650aa3927bed430f8281c0`.

## Changes

- Added `agent::acp::Connection` over the pinned `agent-client-protocol` crate, with initialize, new/resume/fork, option changes, prompt, cancel, event forwarding, permission refusal, stderr forwarding, and closed-transport mapping.
- Added `agent::events::HarnessEvent` for transient chunks, tool calls, permission refusals, context usage, and option updates.
- Added `ClaudeAdapter::process_spec` to launch the configured Node adapter with `CLAUDE_CODE_EXECUTABLE` and piped stdio. The existing stream-json harness and Planner caller remain intact for A4.
- Added `fake_acp`, a test-only ACP agent with session choices and prompt cases from the A2 brief. Its single responsibility is ACP test behavior; it is 232 lines after formatting.
- Added 11 ACP integration tests and updated the module ownership map plus generated code inventory.

## Verification

- RED: `cargo test --test acp_connection` failed to compile because `agent::acp`, `agent::events`, and `fake_acp` did not exist.
- Focused GREEN: `cargo test --test acp_connection` — 11 passed, 0 failed.
- Full suite: `cargo test` — 120 passed, 0 failed, including the old stream-json path. `Env:RUST_LOG` was removed before test runs.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo fmt --check` and `git diff --check` — passed.
- `UPDATE_CODEMAP=1 cargo test --test codemap` — 2 passed; inventory regenerated.

## Scope and limits

- A2 does not route Planner turns through ACP, persist ACP messages, or manage adapter idle lifetime. Those belong to later tasks under the binding spec.
- The brief listed nine required integration cases. Two additional cases verify the adapter launch specification and fake model/default-mode behavior.
- The real pinned Claude adapter was not invoked in this task. ACP protocol behavior was exercised against the crate-based fake agent on Windows.

## Deviations

- Wrote the failing integration tests before the fake agent, following the project's TDD workflow. The specified RED compile failure was observed before implementation.

## Commit

a5df0c4 — feat(agent): ACP client connection and the fake ACP agent (spec §12.2)

