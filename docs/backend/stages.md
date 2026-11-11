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

### B-85 · Prove Codex MCP tools and effective permissions

- Status: done (signed-in Windows permission proof, 2026-11-08)
- Plan: [Codex app-server](plans/2026-11-07-codex-app-server.md#b-85-prove-tools-and-permissions-first).
- Owner: Harness, through its existing Grants and planning owners; this task is a throwaway probe, not product implementation.
- Read: harness controls spec §12.14; `crates/core-harness/contract.yaml`; `crates/core-grants/contract.yaml`; the existing Codex evidence at `41dd19ef`.
- Touch: `docs/evidence/2026-11-07-codex-app-server-probe.md`, `docs/core/specs/2026-09-24-harness-controls-design.md`, `docs/backend/plans/2026-11-07-codex-app-server.md`, this stages file. Temporary probe source is deleted after evidence is saved.
- Needs: none. Only this task is initially executable.
- Verified on Windows with stable 0.167.0: real Planner thread-grant marker and draft, denied approval, read-only MXC write refusal, required-server failure, pending-tool Stop, same-thread continuation with a fresh project grant, and revoked-project-grant write refusal. See the dated Windows evidence for launch details and test boundaries.
- Isolated launch: the same ChatGPT account signed into the dedicated Shadows `CODEX_HOME`. The regular home was excluded. Forcing the test project untrusted excluded both its original MCP fixture and one inserted after preflight. The exact policy and readback are in §12.14 and the evidence.
- Done when: real 0.167.0 uses the intended Shadows MCP tools, creates/reads a marked draft, rejects forbidden authority and an old grant, cancels a pending tool call and resumes with fresh authority. Evidence records the exact effective tool/mode policy and secret-absence check. If policy cannot be enforced, leave B-86 blocked and report the measured gap.

### B-86 · Connect Codex through the existing session owner

- Status: done (fixtures, Claude regressions and a real signed-in daemon run, 2026-11-08)
- Plan: [First real Planner path](plans/2026-11-07-codex-app-server.md#b-86-first-real-planner-path).
- Owner: Agent owns wire translation; Harness owns sessions/grants; AppCore wires the programs.
- Read: §12.14, B-85 evidence, Agent/Harness/Turns contracts and the plan's shared interfaces.
- Touch: the exact create/modify lists under B-86 in the plan, including constructor callers, tests, contracts and code map. Recheck caller scope before dispatch.
- Needs: B-85; `docs/shared/requests/2026-11-07-codex-planner.md` (accepted).
- Done when: the managed second transport and AppCore path pass the plan's transport/lifecycle/MCP fixtures and first real tool path; failures revoke grants; Stop confirms its terminal result; Claude regressions pass. Public Codex availability remains disabled until B-87.
- Verified on 2026-11-08: `shadows-agent --test codex` 6, `shadows-core --test codex_session` 5, `sessions` 12, `acp_connection` 21, `aylul_session` 55, `contracts` 6, `design_migration` 9, `shadows --test harness_config` 5 (two new Codex launch/config tests), `planner_mcp` 24, `architecture` 49 and `codemap` passed; fmt and clippy for the changed crates with and without test support.
- Live on 2026-11-08: a worktree daemon with `--codex` (native 0.167.0) and the dedicated signed-in home opened a Planner thread (20-tool inventory verified, 8 catalog models, `read-only`). A real `gpt-6.1-sol`/`low` turn called `workspace_get` and `vision_propose`; the API showed the pending proposal. `vision_approve` was refused as a visible refusal and the vision stayed at revision 0. The model declined both shell-write requests under the Planner instructions, so filesystem enforcement still rests on B-85's MXC measurement and the opening's `readOnly` check. Stop during a streamed turn recorded one `Cancelled`. After a daemon restart the same thread resumed and recalled the earlier tool answer. No bearer text appeared in either daemon log or the new rollout. `GET /api/harnesses` still listed Codex as unavailable.

### B-87 · Complete Codex choices, usage and conversation actions

- Status: done (focused checks and a real signed-in daemon run, 2026-11-09)
- Plan: [Controls](plans/2026-11-07-codex-app-server.md#b-87-complete-controls-and-enable-the-harness).
- Owner: Agent translates reports; Harness owns choices; Projects owns allowed modes; DB owns the migration catalog.
- Read: §12.14 and the Agent, Harness, Projects and DB contracts.
- Touch: the exact B-87 file list in the plan. Confirm the next unused migration number before dispatch.
- Needs: B-86; the accepted Codex Planner request.
- Done when: configured Codex is available, per-model effort/mode rules hold, existing authority is not widened by migration, fork/Send now work, and last-request usage is neither cumulative nor counted twice on resume. Run the plan's focused checks.
- Verified on 2026-11-09: `shadows-core --test codex_session` 16 (11 new B-87 tests), `thread_session` 2, `harness_schema` 5; `shadows-agent` lib 38 and `--test codex` 6; `core-harness --test list` 1; `core-db --test migration_checksums` 1; `shadows --test harness_choices` 13, `harness_effort` 7, `harness_observation` 5, `harness_config` 5, `openapi` 8 (no API change), `architecture` 49 and `codemap`. fmt and clippy for the changed crates with and without test support passed.
- Migration: `0050_codex_modes.sql`. This branch merges first and keeps 0050 (Mohammed, 2026-11-09); `feature/executors` renumbers its `0050_executor_sessions.sql` and updates the checksum fixture when it merges.
- Live on 2026-11-09 (native 0.167.0, dedicated home, fresh database): Codex listed available with `read-only`; a new project allowed only `read-only`. The catalog listed 8 models in order with per-model efforts. One turn called `workspace_get` and recorded context 18127/475000 (last request), with five-hour and weekly limits from the real 300/10080-minute windows. The model and effort were remembered only after that turn. Send now steered into a running turn (2 operations, not 3). A fork answered from the source's history while the source's entries stayed unchanged. No bearer text appeared in the daemon logs.
- Found live and fixed: the opening's `account/rateLimits/read` (about 3.7 s) pushed setup to the 20 s limit, so it now runs in the background. Real `thread/start` reports no effort, so the opening now reports the model's catalog default.

### B-88 · Accept Codex in the real Planner

- Status: todo, after the merge (Mohammed, 2026-11-09: the branch merges without it; Mohammed runs it on `main`)
- Plan: [Acceptance](plans/2026-11-07-codex-app-server.md#b-88-acceptance-and-delivery).
- Read: §12.14, the Codex plan and the evidence record.
- Touch: Codex evidence, owner spec §12.14, the Codex plan, this stages file and `docs/frontend/stages.md`.
- Needs: B-87 and F-97 merged to `main`. Not a merge blocker for `codex/codex-app-server`.
- Done when: real daemon/browser/log acceptance covers tools, refusal, Stop, restart/continuation, model/effort controls, fork and Send now; Claude still runs. Full workspace gate stays with CI.
- Watch during Stop: `Turns::stop` answers the operation as it stands, which may still be `Running` while the watcher writes `Cancelled` (core-turns contract, `stop`); the client takes the ending from the stream. If the browser shows a stale Running row after Stop, the follow-up is to make `stop` wait for `finished_at` as `stop_running` does.

## Executors, slices 0 and 1

Design: [executors](specs/2026-11-08-executors-design.md), approved 2026-11-08. Plan: [slices 0 and 1](plans/2026-11-08-executors.md). Branch
`feature/executors`. B-85 to B-88 are reserved by the Codex plan on `codex/codex-app-server`.

### B-89 · Probe the executor write boundary
- Status: done 2026-11-08, run by the controller ([evidence](../evidence/2026-11-08-executor-write-boundary-probe.md))
- Plan: [B-89](plans/2026-11-08-executors.md#b-89-probe-the-write-boundary-slice-0--done-2026-11-08).
- Owner: Harness (sessions and permissions); a throwaway probe, no product source.
- Touch: `docs/evidence/2026-11-08-executor-write-boundary-probe.md`, the design §7, this file.
- Needs: nothing.
- Outcome: Mohammed's ruling on the evidence: the executor is built as the Planner is, with only its task's tools and permission rules; shell commands are not policed; the commit takes only paths in `writes` (design §7).

### B-90 · The executor session
- Status: done (migration 0051_executor_sessions; 0050 belongs to the Codex branch, which merges first)
- Plan: [B-90](plans/2026-11-08-executors.md#b-90-the-executor-session).
- Owner: Threads (`ThreadKind`), Harness (executor setup: tools, the task's permission rules, `Refuse`), Agent (`SessionSetup.permissions`), Grants (`GrantKind::Executor`, opaque task scope, its tool list), Plans (`task_context` checks the scope), MCP (`server.rs` router for the executor kind).
- Touch: the plan's B-90 file list. Expand callers with the LSP before dispatch.
- Needs: nothing.
- Done when: the plan's B-90 tests pass and the Planner's isolation, session and MCP tests still pass.

### B-91 · Runs: attempts, Start, Stop and recovery
- Status: done (migration 0052_runs)
- Plan: [B-91](plans/2026-11-08-executors.md#b-91-runs-attempts-and-startstop).
- Owner: Runs (`core-runs`, new, approved with the design); AppCore wiring.
- Touch: the plan's B-91 file list; the next unused migration number at dispatch.
- Needs: B-90 (done, 81ccae6d).
- Carry from B-90: an executor's registration in `Setups` is never cleared, so reopening its thread rebuilds an executor setup; clear it when the attempt releases its scope. `context_before_turn` has no executor case, so an executor's first turn may compare against Planner instructions and standards; give it one, with a test.
- Done when: the plan's B-91 tests pass, with migration, architecture, codemap and contract checks.

### B-92 · Git and `task_commit`
- Status: done (commit tests in `shadows-core/tests/runs_attempts/commit.rs`, real git; live mode test passed)
- Plan: [B-92](plans/2026-11-08-executors.md#b-92-git-and-task_commit).
- Owner: Runs (git through `shadows-process`); MCP (the tool).
- Needs: B-91 (done, 8442d973).
- Carry from B-91: the Planner's planning-context block is still added to an executor's prompts; leave it out for executor threads, with a test.
- Carry from the B-90/B-91 review (4a5542ba): no test sends an executor turn in a mode other than `default`; add one. `ready.rs` ignores `needs` links to another plan's tasks; make them count, with a test.
- Done when: the plan's B-92 tests pass.

### B-93 · Task attempts over HTTP
- Status: done (routes under `/api/workflows/{workflow}/versions/{version}/tasks/{task}` and `/api/attempts/{id}`, no project segment; live end-to-end over HTTP passed)
- Plan: [B-93](plans/2026-11-08-executors.md#b-93-runs-over-http).
- Owner: `shadows-http` (translation only).
- Needs: B-92; `docs/shared/requests/2026-11-08-task-run-page.md` (accepted).
- Carry from the B-90/B-91 review (4a5542ba): Stop while Start is still opening the session lets Start send the turn on an attempt already Stopped; Start must check the attempt is still Running before it sends.
- Done when: the route tests pass and `api/openapi.json` is regenerated, not hand-edited.

### B-94 · Run executors slice 1 in the browser
- Status: todo (B-93 and F-98 are built; runs after the merge on main)
- Plan: [B-94](plans/2026-11-08-executors.md#b-94-run-it).
- Needs: B-93, F-98. Mohammed runs it.
- Done when: T1 of the Laravel plan commits inside `backend/**` from its page; refusal, Stop and restart are observed in the browser and logs; evidence recorded.

### B-95 · The executor commits its own work and reports it
- Status: done on `fix/first-run` (migration 0053_attempt_report; live over HTTP passed)
- Why: the first browser run (2026-11-09) committed 2354 files of a Python runtime through
  `task_commit`, and the agent's own `git commit --amend` then orphaned the recorded commit.
  Mohammed's ruling: the agent commits its own files and reports; Shadows checks and records.
- Owner: Runs (`Runs::report` replaces `Runs::commit`; recovery reads no git); Grants (the
  executor tool list); Harness (`executor.txt`); MCP (`task_report` replaces `task_commit`).
- Design: [§7](specs/2026-11-08-executors-design.md#7-the-write-boundary), amended 2026-11-09.
- Left: the web shows `report` on the Report tab (a frontend task; `schema.d.ts` is regenerated).


## Approved local delivery implementation, 2026-11-09

Order and rulings: [the program](plans/2026-11-09-local-delivery-program.md). Slice 1 (B-96, B-99, B-103, B-104, B-106) is detailed in [its plan](plans/2026-11-09-slice-1-files-and-contract.md) and goes to one builder, in that order, one commit each, with no stop between tasks. Later slices keep their draft pointers until their slice is detailed.

### B-96 · The file inventory

- Status: done. Ran: `cargo test -p core-db --test migration_checksums` 1 passed; `cargo test -p core-code` 11 passed (3 unit, 2 index, 6 inventory); `cargo test -p shadows-core --test code_index` 3 passed; `cargo test -p shadows --test project_remove` 4 passed; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` 8 passed (`INVENTORY_STALE` the only diff); `cargo test -p shadows-http mapping` 1 passed; `cargo test -p shadows --test architecture --test codemap --test contract_tests` 49+1+1 passed; fmt, both clippy runs and the line check clean.
- Plan: [Task 1](plans/2026-11-09-slice-1-files-and-contract.md#task-1-b-96-the-file-inventory). Its Interfaces and steps are binding.
- Owner: Code (`crates/core-code`).
- Read: the plan's Global Constraints and this task's Interfaces; [§17.0](../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09); the [shared request](../shared/requests/2026-11-09-local-delivery.md) with its "Backend answer, slice 1"; `crates/core-code/contract.yaml`, `crates/core-db/contract.yaml`.
- Touch: `crates/core-code/src/inventory.rs` (new); `crates/core-code/src/store/inventory.rs` (new); `crates/core-db/migrations/0054_code_inventory.sql` (new); `crates/core-code/tests/inventory.rs` (new); `crates/core-code/tests/inventory/invariants.rs` (new); `crates/core-code/src/lib.rs`; `crates/core-code/src/model.rs`; `crates/core-code/src/error.rs`; `crates/core-code/src/scan.rs`; `crates/core-code/src/watch.rs`; `crates/core-code/src/store/mod.rs`; `crates/core-code/contract.yaml`; `crates/core-db/tests/fixtures/base_migrations.sha384`; `crates/core-db/tests/migration_checksums.rs`; `crates/core-db/contract.yaml`; `crates/shadows/tests/architecture/table-owners.txt`; `crates/shadows-core/src/error.rs`; `crates/shadows-core/src/lib.rs`; `crates/core-plans/src/error.rs`; `crates/shadows-http/src/failure/mod.rs`; `api/openapi.json` (regenerated); `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: none.
- Done when: `cargo test -p core-db --test migration_checksums` (1 test); `cargo test -p core-code` (the 7 new tests and every earlier one); `cargo test -p shadows-core --test code_index` (3 tests); `cargo test -p shadows --test project_remove`; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`; `cargo test -p shadows-http mapping`; then `cargo test -p shadows --test architecture --test codemap --test contract_tests` and the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `feat(backend): B-96 <goal>`.
- Review finding R-96-1 (fixed by `backend-fixer` in the commit `fix(backend): R-96-1 the inventory contract in the code's order`): in `crates/core-code/contract.yaml`, the comment of `functions.inventory.inventory` says "In order" and puts "no scan row: Indexing" before the cursor's shape check. The code (`Code::inventory` in `crates/core-code/src/inventory.rs`) is right and stays: it answers Missing, then refuses a cursor that is not `<generation>:<key>` with its key at or under the prefix, and only then reads storage and answers Indexing or `InventoryStale`. Rewrite that comment in the code's order. Change no `.rs` file. Touch: `crates/core-code/contract.yaml`; `docs/backend/stages.md` (this line: set it to fixed with the commit). Done when: `cargo test -p shadows --test contract_tests` passes.

### B-97 · Server catalog and controlled process lifecycle

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-repository-discovery.md#task-2-b-97-server-catalog-and-controlled-process-lifecycle), changed by the [program's slice 2 changes](plans/2026-11-09-local-delivery-program.md) (Mohammed, 2026-11-11): the catalog is the twelve `lspServers` of claude-plugins-official (`*-lsp/.claude-plugin/plugin.json`: command, args, extensionToLanguage; Apache-2.0) copied as one data file; a project's servers are chosen from its files' extensions in the B-96 inventory; a server not installed shows "not installed" with its install command. Not PHP-only.
- Owner: Code.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-96; slice 1 tried on a real Laravel project, then this slice detailed by Hamoud.
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-98 · Verified semantic queries with bounded results

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 3](plans/2026-11-09-repository-discovery.md#task-3-b-98-verified-semantic-queries-with-bounded-results).
- Owner: Code.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-97; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-99 · The inventory reaches HTTP and the Planner

- Status: done. Ran: `cargo test -p shadows --test code_scope --test mcp_server --test mcp_tools` 3+6+18 passed; `cargo test -p shadows --test planner_mcp` 24 passed; `cargo test -p shadows-core --test codex_session --test executor_session` 17 passed; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` 8 passed (two paths, four `Inventory*` schemas); `cargo test -p shadows --test architecture --test codemap --test contract_tests` 49+1+1 passed; fmt, both clippy runs and the line check clean. Live `live_the_planner_lists_the_real_files` not run: no `API_KEY_MINIMAX` in this environment.
- Plan: [Task 2](plans/2026-11-09-slice-1-files-and-contract.md#task-2-b-99-the-inventory-reaches-http-and-the-planner). Its Interfaces and steps are binding.
- Owner: `shadows-http` and `shadows-mcp` (translation), Grants (the Planner's tool list), Harness (the Planner's instructions).
- Read: the plan's Global Constraints and this task's Interfaces; [§17.0](../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09); the [shared request](../shared/requests/2026-11-09-local-delivery.md) with its "Backend answer, slice 1"; the contracts of `shadows-http`, `shadows-mcp`, `core-grants` and `core-harness`.
- Touch: `crates/shadows/tests/fixtures/live.rs` (new); `crates/shadows/tests/planner_live.rs` (new); `crates/shadows-http/src/code.rs`; `crates/shadows-http/src/lib.rs`; `crates/shadows-http/src/contract.yaml`; `crates/shadows-mcp/src/code_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-grants/src/tools.rs`; `crates/core-grants/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/contract.yaml`; `crates/shadows/tests/code_scope.rs`; `crates/shadows/tests/mcp_server.rs`; `crates/shadows/tests/openapi.rs` (the two new routes in the list of `the_document_names_every_route`); `api/openapi.json` (regenerated); `docs/codebase/README.md` (only if `codemap` asks for a row); `docs/backend/stages.md` (this entry).
- Needs: B-96. Only `project_files` and `project_refresh`; the language-server routes are B-119.
- Done when: `cargo test -p shadows --test code_scope --test mcp_server --test mcp_tools`; `cargo test -p shadows --test planner_mcp`; `cargo test -p shadows-core --test codex_session --test executor_session`; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`; live, where `API_KEY_MINIMAX` is present: `cargo test -p shadows --test planner_live live_the_planner_lists_the_real_files -- --ignored --test-threads=1`, its reply line recorded here; then `cargo test -p shadows --test architecture --test codemap --test contract_tests` and the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `feat(backend): B-99 <goal>`.

### B-119 · Language-server status and question routes reach the Planner

- Status: blocked (slice 2; prerequisites below must ship first)
- Plan: draft, the language-server part of [Task 4](plans/2026-11-09-repository-discovery.md#task-4-b-99-repository-and-lsp-apis-reach-the-planner), which B-99 no longer carries.
- Owner: Code, Grants and adapters.
- Read, Touch, Done when: written when slice 2 is detailed.
- Needs: B-98, B-99.

### B-100 · Preparation request, scope and persistence

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 1](plans/2026-11-09-project-preparation.md#task-1-b-100-preparation-request-scope-and-persistence).
- Owner: Runs and Grants.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: B-99; accepted shared API/model agreement.
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-101 · Bounded preparation worker and refreshed result

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-project-preparation.md#task-2-b-101-bounded-preparation-worker-and-refreshed-result).
- Owner: Runs and Harness.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-100; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-102 · Preparation HTTP/MCP and observable status

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 3](plans/2026-11-09-project-preparation.md#task-3-b-102-preparation-httpmcp-and-observable-status).
- Owner: Runs adapters.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-101; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-103 · A task carries its file work and symbol work

- Status: done. Ran: `cargo test -p core-plans` 29 passed (5 new, 24 workflow_rules); `cargo test -p shadows-core --test task_contract --test task_context --test design_parts --test design_outcomes --test runs_attempts` 1+3+46+3+27 passed, 5 live ignored (no API_KEY_MINIMAX); `cargo test -p shadows --test plan_routes --test plan_storage --test mcp_tools` 7+16+18 passed; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` 8 passed (`TaskContent.contract` optional, six new schemas); `cargo test -p shadows --test architecture --test codemap --test contract_tests` 49+1+1 passed; fmt, both clippy runs and the line check clean.
- Plan: [Task 3](plans/2026-11-09-slice-1-files-and-contract.md#task-3-b-103-a-task-carries-its-file-work-and-symbol-work). Its Interfaces and steps are binding.
- Owner: Plans (`crates/core-plans`).
- Read: the plan's Global Constraints and this task's Interfaces; [§17.0](../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09); the [shared request](../shared/requests/2026-11-09-local-delivery.md) with its "Backend answer, slice 1"; `crates/core-plans/contract.yaml`.
- Touch: `crates/core-plans/src/task_contract.rs` (new); `crates/core-plans/tests/task_contract.rs` (new); `crates/shadows-core/tests/task_contract.rs` (new); `crates/core-plans/src/lib.rs`; `crates/core-plans/src/model.rs`; `crates/core-plans/src/rules.rs`; `crates/core-plans/src/store/task.rs`; `crates/core-plans/contract.yaml`; `crates/shadows-core/src/lib.rs`; `crates/core-plans/tests/workflow_rules.rs`; `crates/shadows-core/tests/runs_attempts.rs`; `crates/shadows-core/tests/design_outcomes.rs`; `crates/shadows-core/tests/design_parts.rs`; `crates/shadows-core/tests/design_parts/roadmap_gate.rs`; `crates/shadows-core/tests/fixtures/cross_plan.rs`; `crates/shadows/tests/fixtures/plan.rs`; `api/openapi.json` (regenerated); `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: none in code; it runs after B-99 because every task of the slice writes `stages.md`, the code map and `api/openapi.json`.
- Done when: `cargo test -p core-plans` (the 5 new tests and all 24 of `workflow_rules`); `cargo test -p shadows-core --test task_contract --test task_context --test design_parts --test design_outcomes --test runs_attempts`; `cargo test -p shadows --test plan_routes --test plan_storage --test mcp_tools`; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`; then `cargo test -p shadows --test architecture --test codemap --test contract_tests` and the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `feat(backend): B-103 <goal>`.
- Review finding R-103-1 (fixed by `backend-fixer` in the commit `fix(backend): R-103-1 a test for a symbol described twice`): the rule "`{name}` is described more than once in `{path}`" in `ContractRules::task` (`crates/core-plans/src/task_contract.rs`) has no test; deleting it passes every test, and spec 17.0 names duplicate symbols among what is validated. In `a_contract_is_refused_with_every_problem_named` (`crates/core-plans/tests/task_contract.rs`): first add ``"T1 contract: `two` is described more than once in `app/A.php`"`` to the expected list and see the test fail; then add a third symbol `{ "id": "s2", "path": "app/A.php", "kind": "function", "name": "two", "returns": { "type_name": "int", "meaning": "x" }, "behavior": ["Answers two again"] }` and see it pass. In `crates/core-plans/contract.yaml`, that test's `checks` says twelve problems. Change no file under `src/`. Touch: `crates/core-plans/tests/task_contract.rs`; `crates/core-plans/contract.yaml`; `docs/backend/stages.md` (this line: set it to fixed with the commit). Done when: `cargo test -p core-plans --test task_contract` (8 passed) and `cargo test -p shadows --test contract_tests`.

### B-104 · A consumer uses a producer's output

- Status: done. Ran: `cargo test -p core-plans` 32 passed (8 in task_contract); `cargo test -p shadows-core --test task_contract --test task_context --test phase_dependencies --test cross_plan_resolution` 2+3+3+4 passed; `cargo test -p shadows --test mcp_tools --test plan_routes` 18+7 passed; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` 8 passed (`produces`, `uses`, four new schemas); `cargo test -p shadows --test architecture --test codemap --test contract_tests` 49+1+1 passed; fmt, both clippy runs and the line check clean.
- Plan: [Task 4](plans/2026-11-09-slice-1-files-and-contract.md#task-4-b-104-a-consumer-uses-a-producers-output). Its Interfaces and steps are binding.
- Owner: Plans (`crates/core-plans`).
- Read: the plan's Global Constraints and this task's Interfaces; [§17.0](../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09); the [shared request](../shared/requests/2026-11-09-local-delivery.md) with its "Backend answer, slice 1"; `crates/core-plans/contract.yaml`.
- Touch: `crates/core-plans/src/contract_uses.rs` (new); `crates/core-plans/src/task_contract.rs`; `crates/core-plans/src/rules.rs`; `crates/core-plans/src/task_context.rs`; `crates/core-plans/src/store/task_context.rs`; `crates/core-plans/src/lib.rs`; `crates/core-plans/contract.yaml`; `crates/shadows-core/src/lib.rs`; `crates/core-plans/tests/task_contract.rs`; `crates/shadows-core/tests/task_contract.rs`; `api/openapi.json` (regenerated); `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: B-103.
- Done when: `cargo test -p core-plans` (8 tests in `task_contract`); `cargo test -p shadows-core --test task_contract --test task_context --test phase_dependencies --test cross_plan_resolution`; `cargo test -p shadows --test mcp_tools --test plan_routes`; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`; then `cargo test -p shadows --test architecture --test codemap --test contract_tests` and the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `feat(backend): B-104 <goal>`.

### B-105 · Minimal accepted decision revisions

- Status: blocked (deferred by the program until a task needs project decision revisions; no slice holds it)
- Plan: [Task 3](plans/2026-11-09-detailed-task-contracts.md#task-3-b-105-minimal-accepted-decision-revisions).
- Owner: Design.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-104; a task that needs a decision revision.
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-106 · The contract reaches the Planner, the person and the executor

- Status: done. Ran: `cargo test -p shadows --test mcp_tools --test planner_mcp --test plan_routes` 20+24+7 passed; `cargo test -p shadows-core --test executor_session --test runs_attempts` 8+27 passed, 5 live ignored (no executor.txt sentence pinned); `cargo test -p shadows --test openapi` 8 passed, `api/` unchanged; `cargo test -p shadows --test planner_live --test runs_routes` builds, 8 passed, 4 live ignored; `cargo test -p shadows --test architecture --test codemap --test contract_tests` 49+1+1 passed; fmt, both clippy runs and the line check clean. Live `live_the_planner_writes_a_contract_without_a_function_body` and `live_an_executor_builds_the_function_its_contract_names` not run: no `API_KEY_MINIMAX` in this environment, so no `live contract:` line is recorded.
- Plan: [Task 5](plans/2026-11-09-slice-1-files-and-contract.md#task-5-b-106-the-contract-reaches-the-planner-the-person-and-the-executor). Its Interfaces and steps are binding.
- Owner: `shadows-mcp` (two tool descriptions), Harness (both instruction texts). No new route.
- Read: the plan's Global Constraints and this task's Interfaces; [§17.0](../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09); the [shared request](../shared/requests/2026-11-09-local-delivery.md) with its "Backend answer, slice 1"; the contracts of `shadows-mcp`, `core-harness` and `shadows-http`.
- Touch: `crates/shadows-mcp/src/plan_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/src/executor.txt`; `crates/core-harness/contract.yaml`; `crates/shadows-http/src/contract.yaml`; `crates/shadows/tests/mcp_tools.rs`; `crates/shadows/tests/planner_live.rs`; `crates/shadows/tests/runs_routes/live.rs`; `docs/backend/stages.md` (this entry).
- Needs: B-99, B-104. Decisions are not exposed here; they wait for B-105.
- Done when: `cargo test -p shadows --test mcp_tools --test planner_mcp --test plan_routes`; `cargo test -p shadows-core --test executor_session --test runs_attempts`; `cargo test -p shadows --test openapi` with no change to `api/`; live, where `API_KEY_MINIMAX` is present: `cargo test -p shadows --test planner_live live_the_planner_writes_a_contract -- --ignored --test-threads=1` and `cargo test -p shadows --test runs_routes live_an_executor_builds_the_function -- --ignored --test-threads=1`, their printed lines recorded here; then `cargo test -p shadows --test architecture --test codemap --test contract_tests` and the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `feat(backend): B-106 <goal>`.

### B-120 · The Planner is told what an unfinished listing means, and a misspelt task key is refused

- Status: done. All three parts built. Part 3 is safe: stored rows read through the store's own `Contract`/`Scope` structs, never `TaskContent`; fingerprints are built from parsed values; `web/src` sends no task edit; `PlanTask` flattens `TaskContent`, so its `id` is not an unknown key. Red run first: `cargo test -p core-plans --test task_contract` 8 passed, 1 failed (`a_misspelt_key_on_a_task_is_refused_by_name`). Then green: `cargo test -p core-plans` 0 + 9 + 24; `-p shadows --test plan_routes --test plan_storage --test mcp_tools --test planner_mcp --test mcp_server` 7, 16, 21, 24, 6; `-p shadows-core --test task_contract --test task_context --test design_parts --test runs_attempts` 2, 3, 46, 27 (5 ignored); `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` 8 (`api/openapi.json`: `TaskContent` gains `additionalProperties: false`); `-p shadows --test architecture --test codemap --test contract_tests` 49, 1, 1; fmt, both clippy passes and the 100-character check are clean.
- Why: Hamoud's slice 1 review (2026-11-10) named three things a person would meet on a first real project; Mohammed asked for them as one task before he tries the slice.
- Owner: `shadows-mcp` (one tool description), Harness (the Planner's instructions), Plans (`TaskContent`).
- Read: `crates/core-plans/contract.yaml`, `crates/core-harness/contract.yaml`, `crates/shadows-mcp/src/contract.yaml`; [Task 2 and Task 5](plans/2026-11-09-slice-1-files-and-contract.md) for the texts as built.
- Parts, in this order:
  1. **An unfinished listing.** The description of `project_files` in `crates/shadows-mcp/src/code_tools.rs` and the `project_files` line of `crates/core-harness/src/prompt.txt` say what each `state` of a page means, and that `indexing` means the listing is not complete yet: ask again, never conclude the folder is empty. Take the state names from `InventoryState` in `crates/core-code/src/model.rs`; add none. `the_planner_is_told_what_a_contract_holds` in `crates/shadows/tests/mcp_tools.rs`, or a new test beside it, pins the sentence in both texts.
  2. **`task_update` without its contract removes it.** `plan_edit`'s description and the prompt already say `task_update` carries the whole task, contract included. Add to the prompt's line on `task_add`/`task_update` that a `task_update` sent without the task's contract leaves the task with none. The rule in `crates/core-plans/src/ops.rs` does not change. The same test pins the sentence.
  3. **A misspelt key on a task is refused.** `TaskContent` in `crates/core-plans/src/model.rs` refuses an unknown field, as the contract types already do. First show it is safe: every stored task row and every fixture still reads (the store's own struct in `crates/core-plans/src/store/task.rs`, the command fingerprint, `cargo test -p core-plans`, `-p shadows --test plan_routes --test plan_storage --test mcp_tools --test planner_mcp`, `-p shadows-core --test task_contract --test task_context --test design_parts --test runs_attempts`). A new test in `crates/core-plans/tests/task_contract.rs` gives a task with the key `contracts` and expects the refusal to name it; write it first and see it fail. **If a stored row, an existing client or a test outside Touch cannot hold this, do not build part 3:** leave `TaskContent` as it is, say exactly what broke, and commit parts 1 and 2.
- Touch: `crates/shadows-mcp/src/code_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/contract.yaml`; `crates/core-plans/src/model.rs`; `crates/core-plans/contract.yaml`; `crates/core-plans/tests/task_contract.rs`; `crates/shadows/tests/mcp_tools.rs`; `api/openapi.json` (regenerated, only if part 3 changes it); `docs/shared/requests/2026-11-09-local-delivery.md` (one line in the backend answer, only if `api/openapi.json` changed); `docs/backend/stages.md` (this entry).
- Needs: B-106.
- Done when: the tests named in the parts pass; `cargo test -p shadows --test mcp_tools --test planner_mcp --test mcp_server`; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`; `cargo test -p shadows --test architecture --test codemap --test contract_tests`; the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `fix(backend): B-120 <goal>`.
- Not in this task: collapsing ignored files of a tracked folder into one line, any check on the text of `behavior`, PHP symbols (slice 2).

### B-121 · The executor is one file, runs in `auto`, and changes files with its file tools

- Status: done. Ran: `core-harness` 2 unit + 3 doc-less targets ok; `executor_session` 7, `runs_attempts` 27 (5 ignored); signed in to Claude Code (`sonnet`): `runs_attempts live_an_executor_edits_a_file_in_a_subfolder_in_mode_auto_and_its_commit_is_accepted` passed (commit made with `Edit`, no refusal) and `runs_routes live_an_executor_builds_the_function` passed (committed `src/cart.rs` with `cart_total_cents(&[u32]) -> u32`); `architecture` 49, `codemap` 1, `contract_tests` 1; rule 5's checks clean. The executor test module imports `parse` and `FILE` by name (the ratchet refuses a glob import). `runs_routes live_an_executor_commits_its_own_work_and_reports_it` still runs on MiniMax, where `auto` refuses `Bash`; it was not run.
- Why: Mohammed's first real run of slice 1 (2026-11-11, a Laravel project, Foundation v3 T3). Two attempts of two ended without a commit. Each wrote its files through the shell (heredocs, `sed`, a `python -` script); the shell dropped the backslashes of the PHP namespaces; the `Edit` or `Write` it then asked for, on a path inside its task's `writes` (`backend/routes/web.php`, `backend/tests/Unit/ArchitectureTest.php`), was refused and recorded as `PermissionRefused`. The `python` call installed a runtime in `backend/app/Python/`. The attempts that did commit before this never called `Edit` or `Write`. Why the rule did not match is not established (every refusal came after a `cd backend`; Windows separators are the other suspect); the ruling below removes the rules, so do not spend the task on it beyond one sentence in the report if you see it.
- Ruling (Mohammed, 2026-11-11; [spec §7](specs/2026-11-08-executors-design.md#7-the-write-boundary)): an executor runs in `auto`; the check on the reported commit is the one write boundary; the executor's instructions and its tool list live in one Markdown file.
- Owner: Harness (the executor session), `shadows-agent` (the mode).
- Read: [spec §7](specs/2026-11-08-executors-design.md#7-the-write-boundary); `crates/core-harness/contract.yaml`, `crates/core-runs/contract.yaml`, `crates/shadows-agent/contract.yaml`.
- Parts, in this order:
  1. **One file.** `crates/core-harness/src/executor.md` replaces `executor.txt`. Its front matter carries `tools:` (a comma list, the session's built-in tools: `Read, Grep, Glob, Edit, Write, MultiEdit, Bash, TodoWrite`); its body is the instructions. Harness reads both from this file (`include_str!`, parsed by a private function of `executor.rs`), and the `EXECUTOR_TOOLS` constant goes. No model and no mode in the file: the model is Start's, the mode is `shadows-agent`'s. A test holds that the file parses and names those eight tools.
  2. **The instructions.** The body keeps today's text and gains a "Tools" section with two rules: never use Python (`python`, `py`, `pip`), for anything; create and change files with `Write`, `Edit` and `MultiEdit`, never with a heredoc, `sed` or a script. The shell is for running things: tests, git, generators.
  3. **The mode.** `EXECUTOR_MODE` in `crates/shadows-agent/src/policy.rs` becomes `auto`. The per-`writes` `Edit`/`Write`/`MultiEdit` permission rules go, with whatever only they needed. Keep what still decides something (an executor thread runs in this mode and no other); delete what `auto` makes dead. `Runs::report`'s `OutsideWrites` check does not change.
  4. **The tests.** `an_executor_session_opens_with_its_instructions_tools_and_rules` holds the new opening. `live_an_executor_writes_inside_its_task_in_mode_default_and_is_refused_outside` becomes a live test, renamed for what it now holds, in which the executor changes a file that already exists in a subfolder with `Edit` and its commit is accepted; the refusal of a commit outside `writes` stays held by `a_commit_with_paths_outside_writes_is_refused_until_fixed`. No new fake-acp test.
  5. **The live test runs on the Claude subscription** (Mohammed, 2026-11-11). On MiniMax, `auto` refuses every `Bash` call ("Auto mode is unavailable: its safety check does not run for this session"), so an executor there cannot commit. An executor live test that needs `auto` opens the real adapter on the person's signed-in Claude Code, with no provider override and model `sonnet`; its `#[ignore]` reason says so. The other live tests stay on MiniMax.
  6. **The fake attempts ask for a model that takes `auto`.** fake-acp accepts `auto` on `fake-large` only; the `start` helper of `crates/shadows-core/tests/runs_attempts.rs` and the assertion that names its model use it.
  7. **The Planner's prompt names a symbol's field as the schema does** (left over from B-122, which could not touch this crate while this task's tree was open). In `crates/core-harness/src/prompt.txt`, the symbols line says "its path (the file it lives in)" where it says "the file it lives in", and "How to edit" gains: "plan_edit's description ends with one complete, valid edit; copy its shape." The test that pins the prompt's contract section holds both. Its own commit, after this task's: `fix(backend): B-122 the prompt names a symbol's path`.
- Touch: `crates/core-harness/src/executor.md` (new); `crates/core-harness/src/executor.txt` (deleted); `crates/core-harness/src/executor.rs`; `crates/core-harness/src/setup.rs`; `crates/core-harness/src/lib.rs`; `crates/core-harness/contract.yaml`; `crates/shadows-agent/src/policy.rs`; `crates/shadows-agent/src/choices.rs`; `crates/shadows-agent/contract.yaml`; `crates/core-runs/src/start.rs`; `crates/core-runs/contract.yaml`; `crates/core-turns/src/record.rs`; `crates/core-turns/contract.yaml`; `crates/core-grants/src/tools.rs` (only a comment or name that cites the old file); `crates/shadows-core/tests/executor_session.rs`; `crates/shadows-core/tests/runs_attempts.rs` (part 6 only); `crates/shadows-core/tests/runs_attempts/live.rs` and the live fixture it uses (part 5); `crates/core-harness/src/prompt.txt` and `crates/shadows/tests/mcp_tools.rs` (part 7 only); `crates/shadows/tests/runs_routes/live.rs` (only where it cites `executor.txt`, or to run its executor on the subscription); `docs/codebase/README.md`; `docs/backend/stages.md` (this entry). If the change needs another source file, stop and say which and why.
- Needs: B-120.
- Done when: `cargo test -p core-harness`; `cargo test -p shadows-core --test executor_session --test runs_attempts`; signed in to the Claude Code subscription (no provider override, model `sonnet`), the renamed live test and `cargo test -p shadows --test runs_routes live_an_executor_builds_the_function -- --ignored --test-threads=1` pass; `cargo test -p shadows --test architecture --test codemap --test contract_tests`; the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `fix(backend): B-121 <goal>`.
- Not in this task: a per-project executor file, any policing of shell commands, the executor's environment (`LOCALAPPDATA`), cleaning up after a failed attempt.

### B-122 · The Planner is told the exact shape of each `plan_edit` operation

- Status: done for `shadows-mcp`. The published schema already stated all six refused shapes (`task` and `link` required, `AcceptanceItem` an object, `SymbolWork` takes `path` and refuses others, `after` required, `ContractTarget` only `local`); it left none open. `plan_edit`'s description now names each op's object and ends with one valid ops list, parsed as `Vec<PlanOp>` by `mcp_tools::the_plan_edit_example_parses_as_the_ops_it_shows`. The prompt was left to B-121's open tree: its symbols line says "the file it lives in" where the field is `path`. Tests: `mcp_tools` 22, `planner_mcp` 24, `mcp_server` 6, `planner_live` 2 (live), `codemap` 1, `contract_tests` 1; `architecture` fails only on B-121's uncommitted `core-harness/src/executor.rs`. Prompt part done after B-121: the symbols line names "its path (the file it lives in)", "How to edit" points at the description's example, and `mcp_tools::the_planner_is_told_what_a_contract_holds` holds both (`mcp_tools` 22, `architecture` 49).
- Why: the same run. In a new Codex conversation (`gpt-6.1-sol`) `plan_edit` failed six times in a row before it succeeded, each time on the shape of an operation: `missing field task`, `invalid type: string ..., expected struct AcceptanceItem`, `unknown field file, expected one of id, path, kind, name, inputs, returns, behavior, errors`, `missing field link`, `missing field after`. In the conversation that already held plans it failed once per edit (`unknown field file`; `unknown variant plan, expected local`). The Planner guesses the shape and learns it from the refusals.
- Owner: `shadows-mcp` (the tool's input schema and description), Harness (the Planner's instructions).
- Read: `crates/shadows-mcp/src/contract.yaml`, `crates/core-plans/contract.yaml`, `crates/core-harness/contract.yaml`.
- Parts:
  1. **Find the cause first.** See what a client is told about one operation today: the JSON schema `plan_edit` publishes for `ops` and the texts beside it. Say in the report which of the refused shapes the schema already states and which it leaves open.
  2. **State the shape once.** Where the schema leaves a shape open, close it in the schema; where the schema states it and the Planner still missed it, the description of `plan_edit` carries one complete, valid example: a `task_add` whose task has acceptance items and a contract with one file, one symbol, one produced output and one use, and a `link_put`. The example is built from the real types by a test, so it cannot drift: the test feeds the example text to the same deserializer `plan_edit` uses and expects it to parse.
  3. No rule of Plans changes: nothing that is refused today becomes accepted.
- Touch: `crates/shadows-mcp/src/plan_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-harness/src/prompt.txt` and `crates/core-harness/contract.yaml` (only if the prompt must point at the example); `crates/shadows/tests/mcp_tools.rs`; `docs/backend/stages.md` (this entry). If the cause lies in `core-plans`' types, stop and say so.
- Needs: B-120.
- Done when: the new test passes; `cargo test -p shadows --test mcp_tools --test planner_mcp --test mcp_server`; with `API_KEY_MINIMAX` set, `cargo test -p shadows --test planner_live -- --ignored --test-threads=1`; `cargo test -p shadows --test architecture --test codemap --test contract_tests`; the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `fix(backend): B-122 <goal>`.
- Not in this task: a contract `uses` that names another plan's output, done marks carried to a new version, one line for a folder whose files are all ignored. Mohammed's run found all three; they wait for a ruling.

## Slice 1b: the Planner runs and judges tasks (2026-11-11)

Mohammed's rulings after slice 1 ran on a real Go project: [executors design §5.1](specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11). Plan: [slice 1b](plans/2026-11-11-slice-1b-planner-runs-tasks.md); each task's Interfaces and steps there are binding. It comes before slice 2. One builder, B-123 to B-125 in order, one commit each; B-126 waits for its shared request. Every task: new tests are live (the real adapter, real git) or run no agent, never a new fake-acp test; symbols through the LSP; only the tests named, with `-p` and the target, never `--workspace`; one build cache in `target/`, no `CARGO_TARGET_DIR`, `cargo clean` or `--release`; no `crates/**/*.rs` line over 100 columns.

### B-123 · The Planner initialises git in a folder that has none

- Status: done. `Runs::init_git` and the `project_git_init` tool (23 Planner tools). Tests: `runs_attempts` 30 (5 ignored; the three `init` tests among them), `mcp_server` 6, `mcp_tools` 23, `planner_mcp` 24, `architecture` 49, `codemap` 1, `contract_tests` 1; rule 5's checks clean. `mcp_server::the_discover_lifecycle_works_without_a_session` counts the Planner's tools too (22 to 23).
- Plan: [Task B-123](plans/2026-11-11-slice-1b-planner-runs-tasks.md#task-b-123-the-planner-initialises-git-in-a-folder-that-has-none).
- Owner: Runs (git calls in a project folder; Start's `NotGit` is its refusal); `shadows-mcp` (the tool); Grants (the Planner's tool list); Harness (the Planner's instructions).
- Read: `crates/core-runs/contract.yaml`, `crates/shadows-mcp/src/contract.yaml`, `crates/core-grants/contract.yaml`, `crates/core-harness/contract.yaml`; §5.1.
- Touch: `crates/core-runs/src/init.rs` (new); `crates/core-runs/src/git.rs`; `crates/core-runs/src/lib.rs`; `crates/core-runs/src/error.rs`; `crates/core-runs/contract.yaml`; `crates/shadows-mcp/src/run_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-grants/src/tools.rs`; `crates/core-grants/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/contract.yaml`; `crates/shadows-core/tests/runs_attempts.rs` (the module line and `planner_grant`); `crates/shadows-core/tests/runs_attempts/init.rs` (new); `crates/shadows/tests/mcp_server.rs`; `crates/shadows/tests/mcp_tools.rs`; `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: B-121 and B-122's prompt part committed (they hold `prompt.txt`, `core-runs/contract.yaml` and `runs_attempts.rs` open in the tree).
- Done when: `cargo test -p shadows-core --test runs_attempts` (the three new tests among them); `cargo test -p shadows --test mcp_server --test mcp_tools --test planner_mcp`; `cargo test -p shadows --test architecture --test codemap --test contract_tests`; the pre-commit checks of [AGENTS.md](AGENTS.md) rule 5. One commit, `feat(backend): B-123 <goal>`.

### B-124 · The Planner starts a task and marks it done or not done

- Status: done. `Runs::start_for`, `mark_done_for` (with `DoneMark.next`), `NoModel`; tools `task_start` and `task_done` (25 Planner tools); the "Running tasks" prompt paragraph. Tests: `runs_attempts` 34 (5 ignored; four new `planner` tests), `mcp_server` 6, `mcp_tools` 23, `planner_mcp` 24, `runs_routes` 8 (2 ignored), `planner_live` and `planner_runs_live` build; signed in (`sonnet`): `planner_runs_live live_the_planner_starts` passed, printing `live start: Stopped`; `architecture` 49, `codemap` 1, `contract_tests` 1; rule 5's checks.
- Plan: [Task B-124](plans/2026-11-11-slice-1b-planner-runs-tasks.md#task-b-124-the-planner-starts-a-task-and-marks-it-done-or-not-done).
- Owner: Runs (`start_for` and `mark_done_for`: the Planner's grant checked, then the same `start` and `mark_done` the HTTP routes call; a done mark answers `next`, which the Planner starts on its own, Mohammed 2026-11-11); `shadows-mcp`; Grants; Harness.
- Read: as B-123, and `crates/shadows-http/src/contract.yaml` (the Start route, unchanged).
- Touch: `crates/core-runs/src/planner.rs` (new); `crates/core-runs/src/lib.rs`; `crates/core-runs/src/error.rs`; `crates/core-runs/src/model.rs`; `crates/core-runs/contract.yaml`; `crates/shadows-mcp/src/run_tools.rs`; `crates/shadows-mcp/src/refusal.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-grants/src/tools.rs`; `crates/core-grants/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/contract.yaml`; `crates/shadows-core/tests/runs_attempts.rs` (module line); `crates/shadows-core/tests/runs_attempts/planner.rs` (new); `crates/shadows/tests/fixtures/live.rs`; `crates/shadows/tests/planner_runs_live.rs` (new); `crates/shadows/tests/mcp_server.rs`; `crates/shadows/tests/mcp_tools.rs`; `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: B-123.
- Done when: `cargo test -p shadows-core --test runs_attempts`; `cargo test -p shadows --test mcp_server --test mcp_tools --test planner_mcp --test runs_routes`; `cargo test -p shadows --test planner_live --test planner_runs_live` builds; signed in to Claude Code: `cargo test -p shadows --test planner_runs_live live_the_planner_starts -- --ignored --test-threads=1`, its printed line recorded here; architecture, codemap, contract_tests; rule 5's checks. One commit, `feat(backend): B-124 <goal>`.

### B-125 · A finished attempt reaches its Planner, and a committed one waits for its judgment

- Status: done. Tests: `runs_attempts` 36 (5 ignored), `runs_routes` 8 (2 ignored; no expectation needed changing), `mcp_tools` 23, `mcp_server` 6, `architecture` 49, `codemap` 1, `contract_tests` 1; rule 5's checks clean. Signed in (`sonnet`): `live_a_committed_attempt_reaches_its_planner_and_waits_for_its_judgment` printed `live notice: [Shadows] T1 "Add t1" of plan version 142f71e8-…: attempt 1 committed ea219480….`; `live_an_attempt_without_a_commit_reaches_its_planner` printed `live notice: [Shadows] T1 "Add t1" of plan version f001e4da-…: attempt 1 failed: ended without a commit`.
- Plan: [Task B-125](plans/2026-11-11-slice-1b-planner-runs-tasks.md#task-b-125-a-finished-attempt-reaches-its-planner-a-committed-one-waits-for-its-judgment).
- Owner: Runs (the watcher, the notice, the release on a done mark); Turns is used through `Turns::queue` only; `shadows-mcp` and Harness (texts).
- Read: `crates/core-runs/contract.yaml`, `crates/core-turns/contract.yaml` (`queue`), `crates/shadows-mcp/src/contract.yaml`, `crates/core-harness/contract.yaml`; §5.1.
- Touch: `crates/core-runs/src/notify.rs` (new); `crates/core-runs/src/lib.rs`; `crates/core-runs/src/watch.rs`; `crates/core-runs/src/store/mod.rs`; `crates/core-runs/src/store/recover.rs` (Mohammed, 2026-11-11: "Recovery does not change" was wrong; recovery must keep a committed attempt's scope held while it waits for the Planner's judgment); `crates/core-runs/contract.yaml`; `crates/shadows-mcp/src/run_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/contract.yaml`; `crates/shadows-core/tests/runs_attempts.rs` (the renamed test); `crates/shadows-core/tests/runs_attempts/planner.rs`; `crates/shadows/tests/planner_runs_live.rs`; `crates/shadows/tests/mcp_tools.rs`; `crates/shadows/tests/runs_routes.rs` and `crates/shadows/tests/runs_routes/live.rs` (only an expectation that a committed attempt's scope is freed at its turn's end); `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: B-124.
- Done when: `cargo test -p shadows-core --test runs_attempts`; `cargo test -p shadows --test runs_routes --test mcp_tools --test mcp_server`; signed in: the two live tests of the task with `-- --ignored --test-threads=1`, their printed lines recorded here; architecture, codemap, contract_tests; rule 5's checks. One commit, `feat(backend): B-125 <goal>`.

### B-126 · An addition continues the executor in its session, recorded on the task

- Status: blocked (its shared request is not accepted yet)
- Plan: [Task B-126](plans/2026-11-11-slice-1b-planner-runs-tasks.md#task-b-126-an-addition-continues-the-executor-in-its-session-recorded-on-the-task).
- Owner: Runs (the amendment is a new attempt on the same thread with its addition; `run_turn` is Start's tail, shared); core-db (migration 0055); `shadows-mcp`; Grants; Harness.
- Read: `crates/core-runs/contract.yaml`, `crates/core-db/contract.yaml`, `crates/core-harness/contract.yaml` (`open_executor`), `crates/shadows-mcp/src/contract.yaml`; §5.1; [the request](../shared/requests/2026-11-11-planner-review.md).
- Touch: `crates/core-runs/src/amend.rs` (new); `crates/core-db/migrations/0055_attempt_addition.sql` (new); `crates/core-runs/src/start.rs`; `crates/core-runs/src/lib.rs`; `crates/core-runs/src/model.rs`; `crates/core-runs/src/error.rs`; `crates/core-runs/src/store/mod.rs`; `crates/core-runs/contract.yaml`; `crates/core-db/tests/fixtures/base_migrations.sha384` and `crates/core-db/tests/migration_checksums.rs` (if the checksum test asks); `crates/core-db/contract.yaml` (if it lists migrations); `crates/shadows-mcp/src/run_tools.rs`; `crates/shadows-mcp/src/contract.yaml`; `crates/core-grants/src/tools.rs`; `crates/core-grants/contract.yaml`; `crates/core-harness/src/prompt.txt`; `crates/core-harness/contract.yaml`; `crates/shadows-core/tests/runs_attempts/planner.rs`; `crates/shadows-core/tests/runs_attempts/live.rs`; `crates/shadows/tests/planner_runs_live.rs`; `crates/shadows/tests/mcp_server.rs`; `crates/shadows/tests/mcp_tools.rs`; `crates/shadows/tests/runs_routes.rs` (only an `Attempt` read that must now name `addition`); `api/openapi.json` (regenerated); `docs/shared/requests/2026-11-11-planner-review.md` (one line in the backend answer: built); `docs/codebase/README.md`; `docs/backend/stages.md` (this entry).
- Needs: B-125; `docs/shared/requests/2026-11-11-planner-review.md` accepted.
- Done when: `cargo test -p core-db --test migration_checksums`; `cargo test -p shadows-core --test runs_attempts`; `cargo test -p shadows --test runs_routes --test mcp_server --test mcp_tools --test planner_mcp`; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` (only `Attempt.addition` changes); signed in: `cargo test -p shadows-core --test runs_attempts live_an_addition -- --ignored --test-threads=1` and `cargo test -p shadows --test planner_runs_live live_the_planner_runs_judges -- --ignored --test-threads=1` (the slice's acceptance), their printed lines recorded here; architecture, codemap, contract_tests; rule 5's checks. One commit, `feat(backend): B-126 <goal>`.
- Then Mohammed runs the slice on the Go project from the browser: the Planner starts T1, judges it, and sends an addition or marks it done.

### B-107 · Store a simple brief on each attempt

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 1](plans/2026-11-09-execution-briefs.md#task-1-b-107-store-a-simple-brief-on-each-attempt).
- Owner: Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: B-102, B-106; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-108 · Readiness from real inputs and current plan state

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-execution-briefs.md#task-2-b-108-readiness-from-real-inputs-and-current-plan-state).
- Owner: Runs with Plans.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-107; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-109 · Task-aware questions and executor tools

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 3](plans/2026-11-09-execution-briefs.md#task-3-b-109-task-aware-questions-and-executor-tools).
- Owner: Runs, Grants and adapters.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-108; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-110 · Bounded source from the reported Git SHA

- Status: dropped with the critic (2026-11-11, slice 1b): its only reader was the critic; it returns only if §5.1's critic OPEN closes that way.
- Plan: [Task 1](plans/2026-11-09-checks-and-critic.md#task-1-b-110-bounded-source-from-the-reported-git-sha).
- Owner: Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: B-109; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-111 · Run required checks with source provenance

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-checks-and-critic.md#task-2-b-111-run-required-checks-with-source-provenance).
- Owner: Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-109; accepted shared API/model agreement (B-110 dropped with the critic).
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-112 · Read-only critic session and one review per result

- Status: replaced by the Planner's judgment (2026-11-11, B-124 and B-125; [§5.1](specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)).
- Plan: [Task 3](plans/2026-11-09-checks-and-critic.md#task-3-b-112-read-only-critic-session-and-one-review-per-result).
- Owner: Runs, Harness and Grants.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-111; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-113 · Evidence-backed outputs and honest task acceptance

- Status: blocked; re-scoped when slice 5 is detailed: honest task acceptance is the Planner's done mark (B-124, B-125); evidence-backed outputs remain, with no critic review as input.
- Plan: [Task 4](plans/2026-11-09-checks-and-critic.md#task-4-b-113-evidence-backed-outputs-and-honest-task-acceptance).
- Owner: Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 4; no other source paths.
- Needs: B-111; accepted shared API/model agreement (B-112 replaced).
- Done when: all named RED/GREEN steps and focused verification commands in Task 4 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-114 · Expose checks, critic and result detail

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 5](plans/2026-11-09-checks-and-critic.md#task-5-b-114-expose-checks-critic-and-result-detail).
- Owner: Runs adapters.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 5; no other source paths.
- Needs: B-113; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 5 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-115 · Retry in the same live session with fresh attempt identity

- Status: absorbed by B-126 (2026-11-11): an addition is a new attempt on the same executor thread.
- Plan: [Task 1](plans/2026-11-09-repair-and-acceptance.md#task-1-b-115-retry-in-the-same-live-session-with-fresh-attempt-identity).
- Owner: Runs and Harness.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: B-114; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-116 · Find the actual Planner and enforce the three-attempt ladder

- Status: replaced by B-125 (2026-11-11): the Planner is the version's `written_by` thread, the notice waits through `Turns::queue`, and there is no ladder (§5.1's cap OPEN).
- Plan: [Task 2](plans/2026-11-09-repair-and-acceptance.md#task-2-b-116-find-the-actual-planner-and-enforce-the-three-attempt-ladder).
- Owner: Plans and Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-115; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-117 · Stop obsolete plans and rerun pending evidence

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 3](plans/2026-11-09-repair-and-acceptance.md#task-3-b-117-stop-obsolete-plans-and-rerun-pending-evidence).
- Owner: Runs with Plans.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-126; accepted shared API/model agreement (B-115 absorbed, B-116 replaced).
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-118 · Disposable full journey and branch delivery evidence

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 4](plans/2026-11-09-repair-and-acceptance.md#task-4-b-118-disposable-full-journey-and-branch-delivery-evidence).
- Owner: Controller across existing owners.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 4; no other source paths.
- Needs: B-117, F-104; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 4 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.
