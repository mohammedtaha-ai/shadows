# Milestone 2.5 on Windows — Mohammed's run

- **Date:** 2026-09-29
- **Build:** `shadows serve --debug`, a debug build of branch `milestone-2.5/app-core` at `c6e5891`
- **Harness:** adapter 0.81.1, Claude Code 2.1.281 (`harness.versions`)
- **Database:** the Milestone 2 database, opened without a migration error
- **OS:** Windows 11

## What the log shows

Read from the debug log `shadows-20260929T181058Z-83804.log`:

- Startup ran recovery (`interrupted=0 anomalies=0`), revoked 2 stale thread
  grants, probed the harness versions, and listened on `127.0.0.1:4318`.
  The log targets are the new crates' (`shadows_core::…`, `shadows_http`,
  `shadows_process`); the messages are unchanged.
- Six Planner turns: every one `Pending → Running → Completed`.
- The Planner used Shadows' MCP server: 9 `POST /mcp` answered `200`.
- 2 `POST /mcp` answered `401`: a client presenting a grant that is no longer
  valid. That is the expected refusal, not a failure.
- SSE subscriptions replayed and reported `caught-up`, and closed with
  `client gone` when a tab left.

His verdict: it works.

## What this run did not check

The log records no Stop (no turn ended `Cancelled`), no daemon restart, no plan
Approve, and no Revoke. Those paths are covered by the automated tests the
branch pins (`stop_after_the_move_records_cancelled`,
`plan_commands_replay_after_the_move`, `revoked_grant_is_refused_after_the_move`
and the rest). Ctrl+C with a turn running has no automated test; its code is
unchanged from `main` (`crates/shadows/src/cli/mod.rs`).

## Observed, not yet explained

Mohammed saw the Planner's text appear in bursts with short pauses. The live
path forwards each harness chunk as one `delta` frame with no buffering
(`shadows-core/src/events/subscription.rs`); whether the pauses come from the
model or from the web client's rendering was not measured.
