# Project Status

**Updated:** 2026-09-21

This file says where the project is. It decides nothing — the design and every
decision live in the topic owners indexed by
[`specs/README.md`](./superpowers/specs/README.md), and this file must never
restate them.

## Where we are

Architecture phase. No product code exists yet.

Documentation was consolidated on 2026-09-21 and then split by topic under
`docs/superpowers/specs/`. Each decision has one owner file, the directory
README is the index, `status.md` records progress, and `evidence/` records dated
measurements.

## What has been measured

- **Persistence.** SQLx 0.9 + SQLite chosen; the delta validated SQLx and
  SeaORM 2.0.3 against PostgreSQL 16. `evidence/persistence/`.
- **Harness stream contract.** Measured against Claude Code 2.1.278: four stream
  classes, of which only `assistant`, `user`, and `result` are durable; turn end
  is an explicit `result` line; durable lines arrive with a harness-assigned
  uuid; `--session-id`/`--resume` give continuity across processes. Serving the
  web client needs no framework. `evidence/harness/`.
- **Harness binary identity.** The machine carries more than one `claude-code`
  installation at different versions. The measured contract belongs to one of
  them. Spec §1.4 now requires an explicitly configured path and a recorded
  version.
- **SQLite writer strategy and `durable_seq` ordering.** Deferred `BEGIN` fails
  on 73–97 % of read-then-write transactions with `SQLITE_BUSY_SNAPSHOT`, and
  `busy_timeout` does not rescue it. One write connection plus `BEGIN IMMEDIATE`
  gives zero failures at higher throughput than `BEGIN IMMEDIATE` alone. No
  visibility inversion in 21,798 reader polls, and none is structurally possible
  on SQLite. Spec §6.23 and §6.18 amended. `evidence/persistence/`.

## What is not measured, and was wrongly claimed to be

Whether `claude --print` spawns a process tree when a command-executing tool
runs. An earlier claim that this was observed has been withdrawn: the process
filter matched on the name `claude`, which on this machine also matches the
Electron desktop application, and Task Manager groups by application rather than
by parent. The Job Object decision (spec §1.5) stands on its own reasoning.

Settling it needs one turn that actually invokes `Bash`, with the tree walked by
`ParentProcessId` from the daemon's own PID.

## Next

1. **Write the Milestone 0 implementation plan.** Its order and acceptance are
   spec §11.1; the plan turns those into executable tasks.
2. **Create `docs/codebase/roadmap/`** once that plan exists — a living code map
   an agent reads before writing, and updates when it finishes.
3. **Implement**, only after the plan is reviewed.

## Standing risks

- **Everything is horizontal until the vertical runs.** The previous `shadow`
  repository reached 72k lines of Rust and 46k lines of documentation before
  anyone ran the product path end to end; the first attempt found a reversed
  argument pair that 413 commits had not caught. No module here is finished
  until the vertical path reaches it.
- **Windows is the development target and Linux is not yet exercised.** The WAL
  validation in particular ran only on Windows, and SQLite's locking primitives
  differ by platform. Process
  environment, path handling, and file bytes read at compile time behave
  differently on each. A single-platform run is never evidence about the other.
- **Remove mx.** Its hooks are disabled, not deleted. The hook scripts, `mx*`
  skills, permission lines, the `mxai-knowledge` MCP server, and the mx block in
  the global `CLAUDE.md` still need removing once the other project migrates.
