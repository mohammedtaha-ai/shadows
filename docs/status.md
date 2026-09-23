# Project Status

**Updated:** 2026-09-24

This file says where the project is. It decides nothing — the design and every
decision live in the topic owners indexed by
[`specs/README.md`](./superpowers/specs/README.md), and this file must never
restate them.

## Where we are

**Milestone 0 is complete on Windows.** A person starts `shadows serve`, opens
the web client in a browser, picks a project folder, holds a conversation with
a real Claude Planner that streams live, stops a turn with the process tree
confirmed dead, and gets the conversation back after the daemon is killed and
restarted. The run is recorded in
[`evidence/milestone0/ACCEPTANCE.md`](./evidence/milestone0/ACCEPTANCE.md).

- Tasks 1-4: `main` via PR #1. Tasks 5-11 (the backend): `main` via PR #2.
  Tasks 12a-12c (daemon API for an independent client, the React client) and
  Task 13 (shutdown, isolation, acceptance): branch `milestone-0/web-client`.
- The daemon serves an API only; the React client in `web/` is a separate
  application (spec §1). Its types are generated from `api/openapi.json`.
- 107 Rust tests and 48 web tests. CI runs three jobs: Windows is the acceptance
  gate, Linux is a compile gate only, and a web job builds, type-checks and
  tests the client.

The plan is `superpowers/plans/2026-09-21-milestone-0-browser-planner.md`; its
ledger and rulings are in the git-ignored `.superpowers/sdd/` workspace.

## What has been measured

- **Milestone 0 acceptance on Windows.** Gates green; Stop terminates only the
  turn's own tree while 19 unrelated `claude.exe` processes survive; a daemon
  killed mid-turn takes the harness and its grandchild with it through the Job
  Object, and restart records the turn `Interrupted` with every entry intact.
  `evidence/milestone0/`.
- **The harness does spawn a tree.** `claude --print` starts an MCP proxy as a
  child. This settles the question the serve/stream spike left open; whether a
  `Bash` call adds more descendants was not measured separately.
- **Persistence.** SQLx 0.9 + SQLite chosen; the delta validated SQLx and
  SeaORM 2.0.3 against PostgreSQL 16. `evidence/persistence/`.
- **Harness stream contract.** Measured against Claude Code 2.1.278: four stream
  classes, of which only `assistant`, `user`, and `result` are durable; turn end
  is an explicit `result` line; `--session-id`/`--resume` give continuity across
  processes. `evidence/harness/`.
- **Harness binary identity.** The machine carries more than one `claude-code`
  installation at different versions; spec §1.4 requires an explicit path and a
  recorded version.
- **SQLite writer strategy and `durable_seq` ordering.** One write connection
  plus `BEGIN IMMEDIATE`; no visibility inversion. Spec §6.23 and §6.18.
  `evidence/persistence/`.

## Next

1. Merge `milestone-0/web-client` (PR #3) and delete the branch.
2. Plan the next milestone. Its first candidates are the gaps below that
   Milestone 0 deliberately left open.

## Standing risks

- **Linux parent-death containment is not implemented.** The Linux CI job
  compiles and runs the portable suite; it does not prove that the harness dies
  with a crashed daemon (spec §1.5 OPEN block). Never generalize a Windows
  result into a Linux claim.
- **A harness that closes stdout but keeps running cannot be stopped** until the
  daemon exits. Found in the whole-branch review; the Stop/reader arbitration
  needs rework.
- **Starting a turn carries no `CommandId`**, against the idempotency rule;
  **`agent_invocation` is not persisted** (spec §8.2).
- **No authentication.** The daemon binds to loopback and refuses cross-site
  browser requests, but any local process can call it. Remote access is an OPEN
  block in spec §1 with its trigger.
- **Remove mx.** Its hooks are disabled, not deleted. The hook scripts, `mx*`
  skills, permission lines, the `mxai-knowledge` MCP server, and the mx block in
  the global `CLAUDE.md` still need removing.
