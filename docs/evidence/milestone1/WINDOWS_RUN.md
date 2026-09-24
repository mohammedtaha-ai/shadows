# Milestone 1 — Mohammed's run on Windows

**Date:** 2026-09-24
**Status:** Mohammed ran Milestone 1 in a browser on Windows 11 against the
real harness. Send, live output, Stop, and a daemon restart with the
conversation remembered all worked. This is the first Milestone 1 run on
Windows; Phase A and Phase B ran on Linux (`PHASE_A_RUN.md`, `PHASE_B_RUN.md`).
**Code:** `main` at `24a2ea5`, release build.
**Harness:** adapter `@agentclientprotocol/claude-agent-acp` 0.81.1 over
Claude Code 2.1.281 (`%USERPROFILE%\.local\bin\claude.exe`), Node 24.
**Operator:** Mohammed in the browser; Claude started the daemon and read the
debug log and `GET /api/threads/{id}/operations`.

## What ran

| Check | Observed |
|---|---|
| Turns | 7 turns on one thread, Opus and Sonnet, modes `acceptEdits` and `auto`, each `Completed` with `stop_reason: end_turn`. |
| Model per turn | `requested_model` / `observed_model`: `opus` → `claude-opus-5-5`, `sonnet` → `claude-sonnet-5`. |
| Effort per turn | `requested_effort` followed the menu: `high` for the first turns, `medium` after Mohammed picked it. |
| Stop | A turn running shell commands was stopped: `cancel_requested_at` 19:04:02.692, `finished_at` 19:04:02.748 (56 ms), status `Cancelled`. |
| Restart | The daemon was stopped and started again (20:24:34, `recovery.reconcile interrupted=0 anomalies=0`). The next turn on the same thread answered with the earlier conversation in mind and completed. |

## What the run found

- **A turn can fail on Claude's own login.** The first turn failed with
  `Failed to refresh OAuth token: another Claude Code process is refreshing it
  or exited mid-refresh`. Another Claude Code process on the machine was
  renewing the shared token at the same moment. The turn ended `Failed` with
  the harness's words shown in the conversation; the next turn worked.
- **The effort menu shows the session's value, not the pick, until a turn
  runs.** An effort is sent with the next turn (spec §12.7), so between the
  pick and the turn the menu checks the old value. Asking Claude its effort
  is not evidence: it cannot see the setting and answered `low` while
  `high` was set.
- **The effort list includes Claude's `default`.** Mohammed wants it removed
  and the effort set at once, as the model is. Not built yet.

## What this run does not establish

- Spec §12.13 items not exercised here: fork, the permission-refused line,
  and the context breakdown were not checked item by item.
- No process-tree count was taken for the Stop; Milestone 0's Windows
  acceptance measured tree termination (`evidence/milestone0/`).
