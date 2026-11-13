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

ACP upgrade and conversation order: B-83 (conversation order) and B-84 (Rust SDK 5.0.0, adapter 0.101.0)
shipped in PR #62 (`3a443ea0`), with F-96; their entries are in this file at `3a443ea0`. The plan was
`docs/backend/plans/2026-11-07-acp-upgrade.md` at `3a443ea0`.

Codex app-server: B-85 to B-87 shipped in PR #63 (`217d7540`), with F-97; their entries are in this file at
`217d7540`. B-88 (the real Planner acceptance on `main`) is open. The plan is
`docs/backend/plans/2026-11-07-codex-app-server.md`, open for B-88.

Executors, slices 0 and 1: B-89 to B-93 shipped in PR #64 (`c71b263d`), with F-98; their entries are in this
file at `c71b263d`. B-94 (the browser run) is open. The plan is `docs/backend/plans/2026-11-08-executors.md`,
open for B-94.

Local delivery, slices 1, 1b and 3 (the Planner reads the project and its commits, runs and judges tasks, and
sets the project up first): B-95, B-96, B-99, B-100, B-101, B-103, B-104, B-106, B-120 to B-127 and B-128 to
B-136 shipped in PR #65 (`617d806f`), with F-106 to F-113. Their entries are in this file at `617d806f`.
Still open: B-97, B-98 and B-119 (slice 2, blocked), B-105 (blocked), and B-107 to B-118 (blocked, dropped
or replaced). B-102 is dropped. The plans `docs/backend/plans/2026-11-09-project-preparation.md`,
`2026-11-11-slice-3-setup-first.md`, `2026-11-11-planner-reads-commits.md` and
`2026-11-11-planner-reads-code-and-real-commits.md` stay, each for a dropped task or open items.

Review fixes (PR #66, `5a52a06a`): no task. The attempt watcher waits on the commit signal, a code scan writes
its tags in batches, a turn's entries share one transaction, dependencies no crate uses are dropped, and the
route table is built in groups.

## Tasks

### B-88 · Accept Codex in the real Planner

- Status: todo, after the merge (Mohammed, 2026-11-09: the branch merges without it; Mohammed runs it on `main`)
- Plan: [Acceptance](plans/2026-11-07-codex-app-server.md#b-88-acceptance-and-delivery).
- Read: §12.14, the Codex plan and the evidence record.
- Touch: Codex evidence, owner spec §12.14, the Codex plan, this stages file and `docs/frontend/stages.md`.
- Needs: `docs/backend/stages.md` at `217d7540` (B-87) and `docs/frontend/stages.md` at `217d7540` (F-97), merged to `main`. Not a merge blocker for `codex/codex-app-server`.
- Done when: real daemon/browser/log acceptance covers tools, refusal, Stop, restart/continuation, model/effort controls, fork and Send now; Claude still runs. Full workspace gate stays with CI.
- Watch during Stop: `Turns::stop` answers the operation as it stands, which may still be `Running` while the watcher writes `Cancelled` (core-turns contract, `stop`); the client takes the ending from the stream. If the browser shows a stale Running row after Stop, the follow-up is to make `stop` wait for `finished_at` as `stop_running` does.

## Executors, slices 0 and 1

Design: [executors](specs/2026-11-08-executors-design.md), approved 2026-11-08. Plan: [slices 0 and 1](plans/2026-11-08-executors.md),
open for B-94. Its tasks are in PR #64 (`c71b263d`) and PR #65 (`617d806f`), as the Now section says.

### B-94 · Run executors slice 1 in the browser
- Status: todo (B-93 at `c71b263d` and F-98 are built; runs after the merge on main)
- Plan: [B-94](plans/2026-11-08-executors.md#b-94-run-it).
- Needs: `docs/backend/stages.md` at `c71b263d` (B-93) and `docs/frontend/stages.md` at `c71b263d` (F-98). Mohammed runs it.
- Done when: T1 of the Laravel plan commits inside `backend/**` from its page; refusal, Stop and restart are observed in the browser and logs; evidence recorded.

## Approved local delivery implementation, 2026-11-09

Order and rulings: [the program](plans/2026-11-09-local-delivery-program.md). Slices 1, 1b and 3 are built in PR #65 (`617d806f`). The tasks below are still open; the program's later slices keep their draft pointers.

### B-97 · Server catalog and controlled process lifecycle

- Status: replaced (2026-11-13) by B-138 to B-145, [slice 2 plan](plans/2026-11-13-slice-2-language-servers.md), from the [language servers design](specs/2026-11-13-language-servers-design.md).
- Plan: [Task 2](plans/2026-11-09-repository-discovery.md#task-2-b-97-server-catalog-and-controlled-process-lifecycle), changed by the [program's slice 2 changes](plans/2026-11-09-local-delivery-program.md) (Mohammed, 2026-11-11): the catalog is the twelve `lspServers` of claude-plugins-official (`*-lsp/.claude-plugin/plugin.json`: command, args, extensionToLanguage; Apache-2.0) copied as one data file; a project's servers are chosen from its files' extensions in the inventory of `docs/backend/stages.md` at `617d806f` (B-96); a server not installed shows "not installed" with its install command. Not PHP-only.
- Owner: Code.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-96); slice 1 tried on a real Laravel project, then this slice detailed by Hamoud.
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-98 · Verified semantic queries with bounded results

- Status: replaced (2026-11-13) by B-138 to B-145, [slice 2 plan](plans/2026-11-13-slice-2-language-servers.md), from the [language servers design](specs/2026-11-13-language-servers-design.md).
- Plan: [Task 3](plans/2026-11-09-repository-discovery.md#task-3-b-98-verified-semantic-queries-with-bounded-results).
- Owner: Code.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-97; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-119 · Language-server status and question routes reach the Planner

- Status: replaced (2026-11-13) by B-138 to B-145, [slice 2 plan](plans/2026-11-13-slice-2-language-servers.md), from the [language servers design](specs/2026-11-13-language-servers-design.md).
- Plan: draft, the language-server part of [Task 4](plans/2026-11-09-repository-discovery.md#task-4-b-99-repository-and-lsp-apis-reach-the-planner), which `docs/backend/stages.md` at `617d806f` (B-99) no longer carries.
- Owner: Code, Grants and adapters.
- Read, Touch, Done when: written when slice 2 is detailed.
- Needs: B-98, and `docs/backend/stages.md` at `617d806f` (B-99).

### B-102 · Preparation HTTP/MCP and observable status

- Status: dropped (2026-11-11): the setup plan is read through the plan and task routes that exist; nothing is left for it to carry.

### B-105 · Minimal accepted decision revisions

- Status: blocked (deferred by the program until a task needs project decision revisions; no slice holds it)
- Plan: [Task 3](plans/2026-11-09-detailed-task-contracts.md#task-3-b-105-minimal-accepted-decision-revisions).
- Owner: Design.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-104); a task that needs a decision revision.
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-107 · Store a simple brief on each attempt

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 1](plans/2026-11-09-execution-briefs.md#task-1-b-107-store-a-simple-brief-on-each-attempt).
- Owner: Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-101, B-106); accepted shared API/model agreement (B-102 dropped).
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

- Status: replaced by the Planner's judgment (2026-11-11, `docs/backend/stages.md` at `617d806f`, B-124 and B-125; [§5.1](specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)).
- Plan: [Task 3](plans/2026-11-09-checks-and-critic.md#task-3-b-112-read-only-critic-session-and-one-review-per-result).
- Owner: Runs, Harness and Grants.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-111; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-113 · Evidence-backed outputs and honest task acceptance

- Status: blocked; re-scoped when slice 5 is detailed: honest task acceptance is the Planner's done mark (`docs/backend/stages.md` at `617d806f`, B-124, B-125); evidence-backed outputs remain, with no critic review as input.
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

- Status: absorbed by `docs/backend/stages.md` at `617d806f` (B-126), 2026-11-11: an addition is a new attempt on the same executor thread.
- Plan: [Task 1](plans/2026-11-09-repair-and-acceptance.md#task-1-b-115-retry-in-the-same-live-session-with-fresh-attempt-identity).
- Owner: Runs and Harness.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: B-114; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-116 · Find the actual Planner and enforce the three-attempt ladder

- Status: replaced by `docs/backend/stages.md` at `617d806f` (B-125), 2026-11-11: the Planner is the version's `written_by` thread, the notice waits through `Turns::queue`, and there is no ladder (§5.1's cap OPEN).
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
- Needs: `docs/backend/stages.md` at `617d806f` (B-126); accepted shared API/model agreement (B-115 absorbed, B-116 replaced).
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-118 · Disposable full journey and branch delivery evidence

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 4](plans/2026-11-09-repair-and-acceptance.md#task-4-b-118-disposable-full-journey-and-branch-delivery-evidence).
- Owner: Controller across existing owners.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 4; no other source paths.
- Needs: B-117, F-104; accepted shared API/model agreement. 
- Done when: all named RED/GREEN steps and focused verification commands in Task 4 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

## Codex images (2026-11-12)

Mohammed, 2026-11-12: a person can send images to a Codex conversation, as with Claude Code.
Codex 0.167's app-server takes an image turn input as `{"type":"image","url":<data URL>}`
(`UserInput::Image`, from `codex app-server generate-json-schema`), and its models list
`inputModalities` with `["text","image"]` as the default. The web composer already enables
its image button from the session's `capabilities.image`, so no web change.

### B-137 · A Codex conversation takes images

- Status: done. Live, 2026-11-12, real signed-in Codex (dedicated home `%LOCALAPPDATA%/Shadows/codex-home`): `live_a_codex_turn_sees_an_image` 1 passed (14.5 s). A 1x1 PNG reached Codex but read as "White", so the live test sends a 64x64 red fixture (`crates/shadows-core/tests/fixtures/red-64.png`). Browser: Mohammed attached an image in a Codex conversation on 2026-11-12, and it was answered.
- Owner: `shadows-agent`'s Codex connection (`crates/shadows-agent/src/codex/prompt.rs`): `prompt_capabilities` reports `image: true` with no reason, and `input` maps `PromptBlock::Image { media_type, bytes }` to `{"type":"image","url":"data:<media_type>;base64,<bytes>"}`, for both `prompt_blocks` and `steer_blocks`. The capability is the connection's, not the model's: a model that refuses images answers with its own provider error, which already reaches the person (simple boundary, 2026-11-08).
- Read: `crates/core-harness/contract.yaml` (`require_images`, `image_input`); `crates/shadows-agent/src/acp/prompt.rs` (how the ACP side maps an image); `crates/shadows-core/tests/codex_session.rs` (`codex_choices_follow_the_model_catalog`, `live_a_codex_planner_reads_a_file_and_writes_none`).
- Touch: `crates/shadows-agent/src/codex/prompt.rs`; `crates/shadows-agent/contract.yaml` (only if it names the Codex image refusal); `crates/core-harness/contract.yaml` (the `image_input` lines, if they name Codex); `crates/shadows-core/tests/codex_session.rs`; `docs/backend/stages.md` (this entry). If `fake-codex` refuses an image turn in a test that is not live, stop and name it; no new fake-codex behaviour.
- Tests:
  - `codex_choices_follow_the_model_catalog` asserts `capabilities.image` is true and `image_reason` is `None`.
  - New live test `live_a_codex_turn_sees_an_image`, `#[ignore]` with the same reason as the live Planner test: send `crates/shadows/tests/fixtures/red.png` with "What colour is this image? Answer with one word.", wait up to 180 s, the operation completes and the reply contains "red" (case-insensitive).
- Needs: nothing.
- Done when: `cargo test -p shadows-agent`; `cargo test -p shadows-core --test codex_session`; `cargo test -p shadows-core --test codex_session -- --ignored --test-threads=1 live_a_codex_turn_sees_an_image` passes against the signed-in Codex; `cargo test -p shadows-core --test contracts`; `cargo test -p shadows --test architecture --test codemap --test contract_tests`; fmt and the 100-column check on changed files. One commit, `feat(backend): B-137 a Codex conversation takes images`. Browser check: attach an image in a Codex conversation, and the reply describes it.

## Slice 2: language servers, the question card and Settings › Code (2026-11-13)

Design: [language servers](specs/2026-11-13-language-servers-design.md) (Mohammed, 2026-11-13). Plan: [slice 2](plans/2026-11-13-slice-2-language-servers.md). Each entry's Touch is its task's Files; its Done when is its task's run steps, plus `cargo test -p shadows --test architecture --test codemap --test contract_tests`, fmt and the 100-column check. One commit each, as the task names it. Order: B-138a, B-138, B-139, B-140, B-141, B-142, B-143, B-144, F-115, F-116, B-145.


### B-138a · `core-code` becomes `codebase`

- Status: done (Mohammed, 2026-11-13: the service is named without `core-`).
- Plan: [Task B-138a](plans/2026-11-13-slice-2-language-servers.md#task-b-138a-core-code-becomes-codebase).
- Needs: none. Every later slice 2 task reads `crates/codebase/` and `Codebase`.
### B-138 · The shadows-lsp client

- Status: done.
- Plan: [Task B-138](plans/2026-11-13-slice-2-language-servers.md#task-b-138-the-shadows-lsp-client).
- Needs: B-138a.

### B-139 · The catalog and a project's servers

- Status: done.
- Plan: [Task B-139](plans/2026-11-13-slice-2-language-servers.md#task-b-139-the-catalog-and-a-projects-servers).
- Needs: B-138.

### B-140 · Activity is a sent message, with an idle stop

- Status: done.
- Plan: [Task B-140](plans/2026-11-13-slice-2-language-servers.md#task-b-140-activity-is-a-sent-message-with-an-idle-stop).
- Needs: B-139.

### B-141 · code_lsp, and the name index suspended

- Status: done.
- Plan: [Task B-141](plans/2026-11-13-slice-2-language-servers.md#task-b-141-code_lsp-and-the-name-index-suspended).
- Needs: B-139, B-140.

### B-142 · The person's server settings over HTTP

- Status: done.
- Plan: [Task B-142](plans/2026-11-13-slice-2-language-servers.md#task-b-142-the-persons-server-settings-over-http).
- Needs: B-139.

### Review notes on B-139..B-142

- Status: done (`fix(backend): B-139..B-142 review notes`).
- Plan: the "Review fix" and "Review notes" lines of Tasks B-139, B-141 and B-142.
- Done: a scan's changed paths reach the running servers
  (`a_scanned_change_reaches_the_server`); `LANGUAGE_SERVER_UNAVAILABLE` with the server's
  state in its text; `add_server_for_test` and `ask_server_for_test` gone; Lua's id stays
  `lua` (spec amended).

### B-143 · person_ask, the question card

- Status: ready.
- Plan: [Task B-143](plans/2026-11-13-slice-2-language-servers.md#task-b-143-person_ask-the-question-card).
- Needs: none.

### B-144 · The Planner's shell, its permission cards, and the end-of-turn check

- Status: ready.
- Plan: [Task B-144](plans/2026-11-13-slice-2-language-servers.md#task-b-144-the-planners-shell-its-permission-cards-and-the-end-of-turn-check).
- Needs: B-138a, B-140 (`ActiveHold`), B-141 (`code_lsp`'s path check).

### B-145 · Live acceptance

- Status: ready.
- Plan: [Task B-145](plans/2026-11-13-slice-2-language-servers.md#task-b-145-live-acceptance).
- Needs: B-141, B-144, F-115, F-116.
