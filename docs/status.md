# Project Status

**Updated:** 2026-09-24

This file says where the project is. It decides nothing — the design and every
decision live in the topic owners indexed by
[`specs/README.md`](./superpowers/specs/README.md), and this file must never
restate them.

## Where we are

**Milestone 1 Phase A is implemented, and it has run on Linux.** A Planner
turn is now an ACP `session/prompt` on an adapter each open thread keeps
(spec §12). The per-turn `claude --print` path is deleted. Every Phase A step
passed against the real harness in a cloud container: streaming, Stop, a
refused permission, a graceful restart, and memory across it.
[`evidence/milestone1/PHASE_A_RUN.md`](./evidence/milestone1/PHASE_A_RUN.md).
**Mohammed's Windows run has not happened yet, and Phase B waits for it.**

- Milestone 0 is complete on Windows and on `main` (PRs #1-#3).
  [`evidence/milestone0/ACCEPTANCE.md`](./evidence/milestone0/ACCEPTANCE.md).
- Milestone 1 Phase A (Tasks 0, A1-A5): branch
  `milestone-1/harness-controls-7p9608`, no PR yet.
- The daemon serves an API only; the React client in `web/` is a separate
  application (spec §1). Its types are generated from `api/openapi.json`.
- 133 Rust tests and 48 web tests.

The plan is `superpowers/plans/2026-09-24-milestone-1-harness-controls.md`. Its
ledger is in `.superpowers/sdd/`, which must leave the branch before its PR.

## What has been measured

- **Milestone 0 acceptance on Windows.** Gates green; Stop terminates only the
  turn's own tree while 19 unrelated `claude.exe` processes survive; a daemon
  killed mid-turn takes the harness and its grandchild with it through the Job
  Object, and restart records the turn `Interrupted` with every entry intact.
  `evidence/milestone0/`.
- **Phase A over ACP, on Linux.** Adapter 0.81.1 over Claude Code 2.1.281:
  the harness itself confirmed a Stop (`cancelled`) and the adapter lived on,
  and a restarted daemon resumed the recorded session. Read-only shell commands
  are allowed by Claude Code without asking. `evidence/milestone1/`.
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

1. Mohammed's Windows run of Phase A (plan Task A5 Step 3), recorded next to
   the Linux run.
2. Remove `.superpowers/sdd/` from the branch, open the Phase A PR, merge it,
   delete the branch.
3. Phase B (the controls), which starts with its contract draft (plan Task 1).

## Standing risks

- **Linux parent-death containment is not implemented.** The Linux run and CI
  stop the daemon cleanly; neither proves that the adapter dies with a crashed
  daemon (spec §1.5 OPEN block). Never generalize a Windows
  result into a Linux claim.
- **A harness that stops answering but keeps running** was the Milestone 0 risk.
  Stop no longer waits on a stream: after `cancel_wait` it terminates the
  adapter's tree (§12.3). `fake_acp`'s `ignore-cancel` covers this; the real
  harness was not driven into that state.
- **Starting a turn carries no `CommandId`**, against the idempotency rule;
  **`agent_invocation` is not persisted** (spec §8.2). Both are Phase B, Task B3.
- **No authentication.** The daemon binds to loopback and refuses cross-site
  browser requests, but any local process can call it. Remote access is an OPEN
  block in spec §1 with its trigger.
- **Remove mx.** Its hooks are disabled, not deleted. The hook scripts, `mx*`
  skills, permission lines, the `mxai-knowledge` MCP server, and the mx block in
  the global `CLAUDE.md` still need removing.
