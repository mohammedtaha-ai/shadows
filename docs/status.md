# Project Status

**Updated:** 2026-09-29 (Milestone 2.5 merged to `main`)

This file says where the project is. It decides nothing — the design and every
decision live in the topic owners indexed by
[`specs/README.md`](./superpowers/specs/README.md), and this file must never
restate them.

## Where we are

**Milestone 2.5 (§14, one application core) is on `main`** (PR #7,
2026-09-29; 328 Rust tests). Shadows is now a Cargo workspace under `crates/`:
the HTTP and MCP adapters are their own crates and reach the application only
through `AppCore` in `shadows-core`, whose eight services each own their
operations and carry a `contract.yaml` that
`crates/shadows-core/tests/contracts.rs` keeps current. Behaviour did not
change, and `api/openapi.json` is byte-identical. Mohammed ran it on Windows:
six Planner turns completed and the Planner used Shadows' MCP server; Stop, a
restart, Approve and Revoke were not exercised in that run
([`evidence/milestone2_5/WINDOWS_RUN.md`](./evidence/milestone2_5/WINDOWS_RUN.md)).
The plan is `superpowers/plans/2026-09-26-milestone-2-5-app-core.md`; its
execution ledger was deleted after the merge.

**Milestone 2 (§13, the Planner writes a plan) is on `main`** (PR #6,
2026-09-25; 304 Rust, 135 web tests), and Mohammed ran it on Windows. A
12-task plan, Approve freezing v1 while an edit made v2, project instructions
reaching both a new and an existing conversation, and Connect and Revoke from
an external Claude Code all worked. The run found that a harness opening took
up to 5.7 s against a 5 s bound, which is now fixed:
[`evidence/milestone2/WINDOWS_RUN.md`](./evidence/milestone2/WINDOWS_RUN.md).
The plan is `superpowers/plans/2026-09-25-milestone-2-plan-workflow.md`; its
execution ledger was removed from the branch before merge.

**Milestone 1 is implemented, Phase A and Phase B, and both have run on
Linux against the real harness.** A Planner turn is an ACP `session/prompt` on
an adapter each open thread keeps (spec §12). The person chooses the CLI per
conversation and the model, mode and effort per message, all read from the
harness. The composer shows context and limits. Any message can be copied, and
the last one forked. The Phase B run found four defects, all fixed and run
again: [`evidence/milestone1/PHASE_B_RUN.md`](./evidence/milestone1/PHASE_B_RUN.md).
The Phase A run is [`PHASE_A_RUN.md`](./evidence/milestone1/PHASE_A_RUN.md).
**Mohammed ran it on Windows:** send, Stop (56 ms to `Cancelled`) and a
daemon restart with the conversation remembered all worked:
[`evidence/milestone1/WINDOWS_RUN.md`](./evidence/milestone1/WINDOWS_RUN.md).
Fork, the permission-refused line and the breakdown were not checked item by
item.

- Milestone 0 is complete on Windows and on `main` (PRs #1-#3).
  [`evidence/milestone0/ACCEPTANCE.md`](./evidence/milestone0/ACCEPTANCE.md).
- Milestone 1 is on `main` (PRs #4 and #5). The
  whole-branch review ran before the merge.
- Mohammed's three rulings after the run are built and ran on the real
  harness (spec §12.5, §12.6/§12.9, §12.7): the mode menu says what Accept
  edits allows, a fork is locked to its harness, and a chosen model is set at
  once.
- The daemon serves an API only; the React client in `web/` is a separate
  application (spec §1). Its types are generated from `api/openapi.json`.
- 202 Rust tests and 95 web tests.

The plan is `superpowers/plans/2026-09-24-milestone-1-harness-controls.md`.
The PR is #4; its execution ledger was removed from the branch before merge.

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
- **Phase B on Linux.** The model list, each model's efforts and the modes all
  come from the session. The adapter needs `PATH` and `HOME` from the daemon, or
  no shell command runs. Accept edits runs file commands in the project folder,
  `rm` included, without a permission request. `evidence/milestone1/`.
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

1. **The five pre-existing defects Milestone 2.5 found,** each a `gap` in its
   service's contract: a model change is not undone when a later check refuses
   the turn; `Sessions` opens a second SQLite pool, so its events do not wake
   SSE; `from_workflow_id` is not fingerprinted; the harness-session route's
   OpenAPI text says nothing durable is written.
2. **Milestone 3: the code index** (`vision.md` §2.4), with tree-sitter embedded.
3. **Effort at once, without `default`.** Mohammed's ruling after the
   Windows run: picking an effort sets it on the session at once, as the
   model is, and Claude's `default` effort is not offered. Amends §12.4 and
   §12.7.
4. **One lock for every open session.** `Sessions` holds a single lock through
   an adapter's startup (typically 3–6 s on Windows, bounded at 20 s) and
   through each termination wait, so
   opening one conversation can delay Stop on another. This is latency, not a
   correctness defect. The fix is one slot per thread, as its own task.

## Standing risks

- **Linux parent-death containment is not implemented.** The Linux run and CI
  stop the daemon cleanly; neither proves that the adapter dies with a crashed
  daemon (spec §1.5 OPEN block). Never generalize a Windows
  result into a Linux claim.
- **A harness that stops answering but keeps running** was the Milestone 0 risk.
  Stop no longer waits on a stream: after `cancel_wait` it terminates the
  adapter's tree (§12.3). `fake-acp`'s `ignore-cancel` covers this; the real
  harness was not driven into that state.
- **Accept edits lets Claude Code delete files in the project folder without
  asking** (spec §12.5, measured in the Phase B run). This is the harness's
  behaviour, kept by decision; an uncommitted file it deletes is lost.
- **No authentication.** The daemon binds to loopback and refuses cross-site
  browser requests, but any local process can call it. Remote access is an OPEN
  block in spec §1 with its trigger.
- **Remove mx.** Its hooks are disabled, not deleted. The hook scripts, `mx*`
  skills, permission lines, the `mxai-knowledge` MCP server, and the mx block in
  the global `CLAUDE.md` still need removing.
