# Milestone 0 — Windows Acceptance Run

**Date:** 2026-09-23 (UTC; 2026-09-24 local)
**Status:** Windows acceptance recorded. Linux parent-death containment is NOT
established; see "What this run does not establish".
**Code:** branch `milestone-0/web-client` at `061f0ab`.
**Operator:** Mohammed drove the browser; the controller made the independent
process and durable-state observations below.

## Environment

| Component | Version |
|---|---|
| OS | Microsoft Windows 11 Pro, build 26200 |
| rustc | 1.96.0 (ac68faa20 2026-05-25) |
| Claude Code CLI (harness) | 2.1.278, at `~/.local/bin/claude.exe` |
| Node.js (web client) | v24.15.0 |
| Daemon | `target/release/shadows.exe serve --harness <claude.exe> --debug --db %TEMP%\shadows-accept\shadows.sqlite3` |
| Web client | `web/`, `npm run dev` on `http://localhost:5173` |

The machine also ran the Claude desktop application and other Claude Code
sessions throughout, which matters for the Stop proof: 19 unrelated
`claude.exe` processes were alive when it started.

## Gates

Each run bare and its exit code read directly, no pipe.

| Gate | Exit code |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo test` | 0 (107 passed) |
| `web/`: `npx tsc -b` | 0 |
| `web/`: `npx vitest run` | 0 (48 passed) |

## Spec §11.1 checklist

| Line | Result | What was observed |
|---|---|---|
| `shadows serve` starts and prints one local address without opening a browser | pass | Printed `shadows serve listening on http://127.0.0.1:4318` and the debug log path; no browser opened. |
| the user can manually open the Web client in any browser | pass | Opened `http://localhost:5173` by hand; the connection indicator read "Connected · 127.0.0.1:4318". |
| a local-directory project can be selected without exposing a path as project identity | pass | Project created from the in-page folder browser with directory `F:\abdo`; its identity is a `ProjectId` uuid, the directory is an attribute. Turns ran with `cwd=F:\abdo` (debug log). |
| a PlanningThread can be created and resumed | pass | Thread `660960d4…` created from the UI; later turns in it resumed the same Claude session and the model kept the conversation. |
| one real Claude Planner turn starts and streams output | pass | Replies streamed live into the page; turns ended `OperationCompleted` with outcome `{"subtype":"success","stop_reason":"end_turn"}`. |
| Stop terminates and reaps the managed process tree before durable Cancelled | pass | See "Stop proof". |
| daemon restart restores the durable thread and terminal operation | pass | See "Recovery proof". |
| structured logs correlate project, thread, and operation without sensitive payloads | partial | Every planner, process and transition line carries `operation_id` and `thread_id`; prompts appear only as `prompt_len`; process spawn logs pid, executable and cwd, never arguments or environment. `project_id` is not on those lines; it is reachable only through the thread. |
| the exact Windows acceptance run is recorded; Linux gaps are named honestly | pass | This document. |

## Stop proof

Operation `5ea0b0f5-425b-474c-b153-14bfd8425927`, thread `660960d4-cd41-48da-8521-8d61816f902f`.

Debug log, in order:

```text
process.spawn pid=15928 executable=C:\Users\Mohammed\.local\bin\claude.exe cwd=F:\abdo
operation.transition.committed event=OperationStarted from=Pending to=Running
operation.transition.committed event=OperationCancellationRequested from=Running to=Running
planner.stop
process.terminate pid=15928
process.exit status=exit code: 1
planner.stop: tree reaped
operation.transition.committed event=OperationCancelled from=Running to=Cancelled
```

Independent check after Stop, outside the daemon:

```text
Get-CimInstance Win32_Process | Where-Object { ProcessId -eq 15928 -or ParentProcessId -eq 15928 }
-> (empty)
```

Bystanders: the 19 unrelated `claude.exe` processes recorded before the run were
all still alive afterwards (19 of 19). Stop terminated only the turn's own Job
Object, never anything by name.

Durable row (read through `GET /api/threads/{id}/operations`; there is no
`sqlite3` CLI on this machine): `status_kind = Cancelled`,
`cancel_requested_at = 2026-09-23T21:47:50.4829215Z`,
`finished_at = 2026-09-23T21:47:50.580723Z`.

## Recovery proof

Operation `d9310aac-a983-49f3-ac78-f9436f67f798`, same thread.

1. Six seconds into a streaming turn the managed tree was
   `31868/claude.exe` and `32892/mxMCPProxy.exe` — a real grandchild, the MCP
   proxy the harness starts for itself.
2. The daemon (pid 13176) was terminated with `Stop-Process -Force` at
   2026-09-23T21:51:12Z. This is the unclean path, not Ctrl-C.
3. Two seconds later, a query for pid 31868 or any process whose parent is 31868
   returned **empty**. The Job Object's kill-on-close took the grandchild with
   it; nothing outlived the daemon.
4. On restart: `recovery.reconcile interrupted=1 anomalies=0`. The row reads
   `status_kind = Interrupted`, `interrupt_reason = PreviousRuntimeEndedDuringRun`.
5. The thread's entries were all present: 32 entries, ordinals 1 through 32 with
   no gap. The page, reloaded, showed the conversation and the interrupted turn.

A first attempt at this proof did not count and is not used above: that turn
finished on its own (26 s) before the daemon was killed.

## What this run establishes that was open before

`docs/evidence/harness/SERVE_STREAM_SPIKE.md` Finding 4 left open whether
`claude --print` spawns a process tree. It does: in this configuration it
starts an MCP proxy (`mxMCPProxy.exe`) as a child. Whether a `Bash` tool call
adds further descendants was not separately measured.

## What this run does not establish

- **Linux parent-death containment.** Spec §1.5 requires the harness tree to die
  when the daemon dies. On Linux, `process-wrap`'s `ProcessSession` covers
  deliberate termination only; a process session or group is explicitly not
  accepted as proof that descendants die with a crashed daemon, and Milestone 0
  implements nothing else. The Linux CI job is a compile and portable-test gate;
  its green status says nothing about this. The OPEN block in §1.5 names the
  trigger.
- **A harness that closes stdout but keeps running cannot be stopped.** The
  stream reader holds the handle first, so Stop finds nothing live to terminate;
  the tree lives until the daemon exits (where kill-on-close ends it) or shutdown
  reaches its bound. Found by the whole-branch review; not fixed in Milestone 0.
- **Starting a turn carries no `CommandId`.** CLAUDE.md requires one on every
  mutating command; only the client's in-flight guard prevents a duplicate turn.
- **`agent_invocation` is not persisted** (spec §8.2): a restart loses which
  model and flags produced a turn.
- **Remote access.** The daemon binds to loopback with no authentication (the
  §1 OPEN block). Nothing here was tested from another machine.
- Graceful shutdown (Ctrl-C) was covered by the automated suite
  (`tests/shutdown.rs`), not by this manual run.
