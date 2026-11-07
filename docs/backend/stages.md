# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

The architecture restructuring ([plan](plans/2026-10-21-architecture.md)) shipped in
PR #41 (`c7e7bac4`): A-0 to A-21, A-23, B-10 and B-11 are in this file at `c7e7bac4`.
The extraction tasks A-8c, A-8d, A-19b and A-19c finished in `d7a722cd`.
The rules are CLAUDE.md "Code structure"; its strict rule binds every task: start at
`AppCore`, work only in the owning service. Live keys: only `OPENROUTER` and
`API_KEY_MINIMAX`, read with a loop, never `. ./.env`, never printed or committed.

Before dispatch the controller expands each task's named scope into exact caller,
test, contract and map paths and records the affected test commands. A blocked
task is not a builder assignment. Per-task checks are focused; the full gate and
whole-branch review run once before each delivery PR.

A-6 is settled with no code change (Mohammed confirmed the approval-authority spec's §24.2
on 2026-10-27); its entry is in this file at `4d5c022d`.

A-7b (the breakdown route goes) and B-12 (test fixtures close their database before their
folder goes) shipped in PR #45 (`5a0af4ff`); their entries are in this file at `29ddfd88`.

A-22 (product workspaces) is deferred on 2026-10-28: its trigger is in
[docs/core/future.md](../core/future.md) "Housekeeping"; its entry is in this file at `3dc2437f`.

B-13 (the person record takes a partial update, `PATCH /api/me`) shipped in PR #46
(`e95b79c3`); its entry is in this file at `e95b79c3`.

A-8b1 and A-8b2 (the Hermes product daemon goes; the AYLUL project check and the master key
move into Core) and B-14 to B-18 (W-11's Core side) shipped in PR #47 (`f6551819`); their
entries are in this file at `4f23c717`.

Tool activity ([spec](specs/2026-10-31-tool-activity-design.md)) and B-19 to B-23 (ACP tool
facts, the `ToolCall` card, the task list, the `tool` and `todos` frames, the `TD` ids) shipped
in PR #48 (`ce31036d`); their entries are in this file at `ce31036d`.

AYLUL agents and panel, Core side: B-24 to B-34 shipped in PR #49 (`b40e077b`); their entries
are in this file at `b40e077b`. The design is `docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md`.
The plan was `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` at `b40e077b`.

Agent identity and background tasks ([tool activity §6](specs/2026-10-31-tool-activity-design.md#6-background-tasks)):
B-35 to B-37 shipped in PR #51 (`2790473b`); their entries are in this file at `16da6b8c`.

The real-app acceptance of the design's §10 is done (Mohammed, 2026-11-03, in the desktop app): an
HTTP MCP tool with a header token (Context7) answered in a conversation, and its token was found in
no database, WAL, log, agent home or session file.

Per-tool MCP permissions ([AYLUL agents and panel §7.2.1](../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md)):
B-38 and B-39 shipped in PR #52 (`6b2c66ed`), with F-80; their entries are in this file at
`9c7c7e5a`. The plan was `docs/backend/plans/2026-11-03-aylul-tool-permissions.md` and the request
`docs/shared/requests/2026-11-03-mcp-tool-permissions.md`, both at `9c7c7e5a`.
Its review findings (whole-token redaction, the listing's environment and folder, deny over
allow on a name clash, edits queued during a save) were fixed in PR #53 (`349606bf`).

Agent groups, messaging slice 1 ([spec](specs/2026-11-03-agent-messaging-design.md)): B-40 to B-46 shipped in PR #54
(`9e5b9ab2`), with F-81 to F-85, U-9 and U-10; their entries, and the commits outside their
`Touch:` lists, are in this file at `9e5b9ab2`. The plan was
`docs/backend/plans/2026-11-03-agent-groups.md` at `9e5b9ab2`.
Known and left: a persistent storage error while starting a delivery retries forever and
stalls dispatch for every group (`crates/core-messages/src/deliveries.rs`).

Planner context, slice 1 ([spec](specs/2026-11-04-planner-context-design.md)): B-47 to B-54 shipped in PR #55
(`fc78cb7`), with the three Planner defects in `docs/evidence/2026-11-04-planner-session-defects.md`. Their entries,
and the commits outside their `Touch:` lists, are in this file at `fc78cb7`. The plan was
`docs/backend/plans/2026-11-04-planner-context-slice-1.md` at `fc78cb7`. Its shared request (`docs/shared/requests/2026-11-04-planning-context-routes.md`, at `1b413fc4`) was built by the inspection view in PR #57. Local acceptance with real provider and browser is in the
[dated results](../evidence/2026-11-04-planner-context-probe.md#4-slice-1-implementation-acceptance-2026-11-04).
Planner context, slice 2 (the re-send after a compaction, spec §4.3): B-55 to B-58 shipped in PR #56
(`024432fc`); their entries, and the commits outside their `Touch:` lists, are in this file at
`024432fc`. The plan was `docs/backend/plans/2026-11-04-planner-context-slice-2.md` at `024432fc`. Its live
acceptance passed on 2026-11-04 ([evidence §5](../evidence/2026-11-04-planner-context-probe.md#5-slice-2-live-acceptance-2026-11-04)).
Its shared request (`docs/shared/requests/2026-11-04-planning-context-resend.md`, at `1b413fc4`) was built in PR #57.
Planner context, slice 3 (the inspection view): B-59 and F-86 to F-89 shipped in PR #57
(`1b413fc4`); their entries are in this file at `1b413fc4`. The plan was `docs/frontend/plans/2026-11-05-planner-context-view.md` at `1b413fc4`.
Its shared request (`docs/shared/requests/2026-11-05-turn-context-state.md`, at `1b413fc4`) was built by that PR.
Not built: `Plans::owner_thread` (spec §6, slice 4), which waits for messaging slice 3.

Structure and depth, slice 1 (one folder per part, no contract files): the design and its plan
were merged in PR #58 (`96f9ff3d`). Slice 1 shipped in PR #59 (`26bf739e`), with B-60 to B-62;
their entries, and the commits outside their `Touch:` lists (`d148c111`, `ec20c911`, `6874e9b2`,
`80c1b934`), are in this file at `26bf739e`. The plan is at `a4f89386`; its live acceptance is not recorded.

Structure and depth, slice 2 (standards v2 and layers on the map): B-63 to B-70 shipped in PR #60
(`b64242a7`), with F-90 and F-91. Their entries, and the commits outside their `Touch:` lists
(`ed4005a7`, `d0fdc70a`, `0774ac2a`), are in this file at `b64242a7`. B-68 to B-70 are test tooling:
a contract-to-test name check (`cargo test -p shadows --test contract_tests`), a weekly coverage
report and an on-demand mutation run. The plan is at `a4f89386`; its live acceptance is not recorded.

Structure and depth, slices 3 to 5 (principles, task context, task phases, and a map change that moves
a part): B-71 to B-82 shipped in PR #61 (`a4f89386`), with F-92 to F-95; their entries are in this file
at `a4f89386`, and so is the plan, deleted after the merge. Mohammed tried slices 3 to 5 in the browser
on 2026-11-07; real-provider acceptance is not recorded.
Elements and trace are dropped.

## Tasks

### B-83 · Order project conversations by the latest assistant reply

- Status: done (automated checks; Mohammed owns manual browser acceptance)
- Plan: Direct user request, 2026-11-07; owner spec §13.11 in `docs/core/specs/2026-09-25-planner-workflow-design.md`.
- Read: `crates/core-threads/contract.yaml`.
- Touch: `crates/core-threads/src/store/thread.rs`, `crates/core-threads/tests/entries.rs`, `crates/core-threads/contract.yaml`, this stages file.
- Needs: `docs/shared/requests/2026-11-07-conversation-reply-order.md` (accepted).
- Done when: core-threads tests, affected clippy, fmt, architecture, codemap and contract checks pass. Manual browser acceptance belongs to Mohammed.
- Verified: 9 core-threads tests, 49 architecture tests, codemap, contract tests and 8 OpenAPI tests; affected clippy with and without test-support; fmt and changed Rust line lengths.

The [design](specs/2026-11-05-structure-and-depth-design.md)
owns later structure work (outcome coverage, code observation). Existing projects are test data: no compatibility layer.
Coverage and code observation must not be recorded as completed execution.

B-A2 (Core's scoped context contract), stopped since 2026-10-19, is in this file at `f1459dcc`.

### B-84 · Upgrade ACP: the Rust crate to 5.0.0 and the adapter to 0.101.0
- Status: done (focused checks and live runs passed; Mohammed confirmed browser acceptance on 2026-11-07)
- Plan: [ACP upgrade](plans/2026-11-07-acp-upgrade.md).
- Base: `codex/acp-upgrade`, cut from `main` after #61. The original brief used B-83, which already tracks conversation ordering.
- Owner: Agent (`crates/shadows-agent`, the ACP `Connection`); the adapter install belongs to the daemon (`harness/claude`). No other service changes behaviour.
- Read: harness controls spec "The adapter", both crate contracts, and the migration guide linked in the plan.
- Measured on 2026-11-07 (bumped and reverted, nothing committed): `cargo check -p shadows-agent -p fake-acp` on `=5.0.0` gives 12 errors, all in `shadows-agent`. `agent_client_protocol::schema::v1` no longer exists (imports in `src/acp/session.rs`, `prompt.rs`, `permissions.rs`, `mapping.rs`, `connection.rs` and `src/events.rs`); `Client::builder` is gone (`src/acp/connection.rs:44`); `session.rs:212` expects `String` where `str` is passed; `mapping.rs:152,156` lose their inferred type. The features `unstable_session_fork` and `unstable_end_turn_token_usage` still resolve. `fake-acp` was not reached and may need the same fixes.
- Touch: the plan's exact scope. SDK pins, affected Agent and fake source, both crate contracts and code map; npm pins; the extension acceptance fixture and `remembered_mode.rs`; the fake's resume fixture and Core Harness contract; this stages file. Keep dated evidence and the standalone `harness_config` fixture intact.
- Needs: nothing.
- Input/output: the same wire behaviour as today. Sessions open, resume and fork; prompts stream; tool calls, permissions, the task list, slash commands, subagent cards and end-of-turn token usage map as before. No new feature rides on the upgrade.
- Failure cases: a renamed or removed unstable feature (stop and report, with the changelog line); a wire change in the adapter that alters a mapped event (fix on Shadows' side, never patch the adapter; report it); `session/fork` or `session/resume` behaviour changing (report with the measured result).
- Done when:
  - `cargo test -p shadows-agent`, `cargo test -p fake-acp`, `cargo test -p shadows-core` for the turns, harness and session targets the compiler touched, and `cargo test -p shadows --test harness_config --test planner_mcp --test mcp_tools`; fmt; clippy for the changed crates with and without `--features fake-acp/test-support`. No full gate locally.
  - The two ignored tests run once after installing the adapter. `aylul_extensions` sends real MiniMax prompts through it. The `aylul_profiles` probe lists models directly from OpenRouter and MiniMax.
  - A run: the daemon starts, the log line `harness.versions` shows `adapter_version="0.101.0"`, and a Planner turn streams in the browser. Mohammed tries it before the branch's PR.
- Verified on 2026-11-07: the SDK, fake, Core, Harness, Turns and CLI focused tests passed, including the mode, MCP, architecture, code map and contract checks. Formatting, both affected clippy modes and `cargo build -p shadows` passed.
- Live on 2026-11-07: adapter 0.101.0 and configured Claude Code 2.1.317. MiniMax extension isolation and three Core prompts passed (new acceptEdits, resume auto, fork auto). OpenRouter listed 362 models and MiniMax 9. A temporary daemon logged both version values. The local npm install omitted optional binaries after the bundled Windows executable download stalled; the lockfile still records the optional package.

## Executors, slices 0 and 1

Design: [executors](specs/2026-11-08-executors-design.md), approved 2026-11-08. Plan: [slices 0 and 1](plans/2026-11-08-executors.md). Branch
`feature/executors`. B-85 to B-88 are reserved by the Codex plan on `codex/codex-app-server`.

### B-89 · Probe the executor write boundary
- Status: ready (dispatch only when Mohammed says so)
- Plan: [B-89](plans/2026-11-08-executors.md#b-89-probe-the-write-boundary-slice-0).
- Owner: Harness (sessions and permissions); a throwaway probe, no product source.
- Touch: `docs/evidence/2026-11-08-executor-write-boundary-probe.md`, the design §7, this file.
- Needs: nothing.
- Done when: the four cases are measured on the Claude adapter 0.101.0 and §7's OPEN is replaced by the rule, or the gap is reported and B-90 stays blocked.

### B-90 · The executor session
- Status: blocked (B-89)
- Plan: [B-90](plans/2026-11-08-executors.md#b-90-the-executor-session).
- Owner: Threads (`ThreadKind`), Harness (executor setup, scoped policy), Grants (task scope), Agent (`PermissionPolicy::Scoped`), MCP (tools an executor grant sees).
- Touch: the plan's B-90 file list. Expand callers with the LSP before dispatch.
- Needs: B-89.
- Done when: the plan's B-90 tests pass and the Planner's isolation, session and MCP tests still pass.

### B-91 · Runs: attempts, Start, Stop and recovery
- Status: blocked (B-90)
- Plan: [B-91](plans/2026-11-08-executors.md#b-91-runs-attempts-and-startstop).
- Owner: Runs (`core-runs`, new, approved with the design); AppCore wiring.
- Touch: the plan's B-91 file list; the next unused migration number at dispatch.
- Needs: B-90.
- Done when: the plan's B-91 tests pass, with migration, architecture, codemap and contract checks.

### B-92 · Git and `task_commit`
- Status: blocked (B-91)
- Plan: [B-92](plans/2026-11-08-executors.md#b-92-git-and-task_commit).
- Owner: Runs (git through `shadows-process`); MCP (the tool).
- Needs: B-89's attribution rule, B-91.
- Done when: the plan's B-92 tests pass.

### B-93 · Task attempts over HTTP
- Status: blocked (B-92)
- Plan: [B-93](plans/2026-11-08-executors.md#b-93-runs-over-http).
- Owner: `shadows-http` (translation only).
- Needs: B-92.
- Done when: the route tests pass and `api/openapi.json` is regenerated, not hand-edited.

### B-94 · Run executors slice 1 in the browser
- Status: blocked (B-93, F-98)
- Plan: [B-94](plans/2026-11-08-executors.md#b-94-run-it).
- Needs: B-93, F-98. Mohammed runs it.
- Done when: T1 of the Laravel plan commits inside `backend/**` from its page; refusal, Stop and restart are observed in the browser and logs; evidence recorded.
