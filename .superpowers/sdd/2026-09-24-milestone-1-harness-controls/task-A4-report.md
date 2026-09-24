# Task A4 report — a turn over the ACP connection

Status: DONE_WITH_CONCERNS. Finished in the cloud session from the in-progress tree
at d3bc6aa (the Sol/medium implementer's work: collector, watcher, stop, spawn,
conversation start, SSE delta). Linux only; no Windows run, no real adapter.

## Gate (Linux)
- `cargo test --features test-support --no-fail-fast`: 131 passed, 0 failed.
- `cargo clippy --all-targets --features test-support -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean. Code map regenerated; codemap 2/2.
- web: `gen:api` regenerated `schema.d.ts`; typecheck, lint, 48/48 tests.

## What was added or changed beyond d3bc6aa
- Tests migrated to `fake_acp` via a shared `tests/fixtures/acp.rs` (used by 12 files).
- `tests/planner_turn.rs` rewritten: the plan's 8 tests plus the two §8.4 case 6
  tests Milestone 0 had (stop with no live turn; failed termination keeps the registration).
- Deleted: `ClaudeHarness`, `AgentHarness`, `AgentInvocation`, `StreamItem`,
  `src/bin/fake_claude.rs`, `tests/harness_stream.rs`, `tests/fixtures/claude_turn.jsonl`,
  and `harness_config.rs`'s tool-block classifier test. Their behaviour is now covered by
  `entries.rs` unit tests and `planner_turn::a_turn_streams_and_stores_one_entry_per_message`.
- `Collector` kept unfinished tool calls in a `HashMap`, so `finish()` emitted them in
  random order (CLAUDE.md: ordering is explicit). Now a `Vec` in first-seen order, with a test.
- `stop` now reads `turn_end_seen`: once the prompt has answered, a timed-out Stop returns
  `ResolvedByTurn` instead of killing an adapter whose turn already ended.
- `turn-end` SSE frame kept (the web client clears streamed text on it): new
  `HarnessEvent::TurnEnd`, sent by the watcher after the turn's entries are durable.
  The `meta` frame is no longer sent, and it is gone from the stream description.
- Log lines restored for §8.7 and `tests/debug_log.rs`: `agent.invocation.start`,
  `planner.first_output`, `planner.turn_end`, `planner.stop*`; `Sessions::open` runs
  under a `sessions.open{thread_id}` span, so `process.spawn` names its thread.
- `fake_acp`: session ids unique per process (`fake-<pid>-<n>`); new `wait-for-release` prompt.
- Test-support hooks on `Sessions`: `pid(thread)`, `force_termination_failure(thread)`.
- Start-turn route documents 502 `HARNESS_START_FAILED`; `api/openapi.json` regenerated.

## Concerns for the reviewer / Mohammed
1. A project with no directory used to answer 202 and fail at `Prepare`. It now answers
   409 `PATH_NOT_FOUND` with the reason as its message, and writes nothing (§12.7, amended).
   Resolved in the follow-up commit, at Mohammed's request.
2. Owner line for `planner/turn.rs` in the plan ("decide and persist ...") fails the codemap
   "and" rule; recorded as "the recorded ending of a live Planner turn".
3. `planner/sessions.rs` is 317 lines, over 300. Single job, unchanged: the live adapter each
   thread holds; the additions are two test-support hooks and the open span.
4. Kept as is (Mohammed agreed): a first turn the harness confirms `cancelled` still records its session (only a
   terminated or failed-RPC turn does not). `thread_session.rs` tests the terminated case.
