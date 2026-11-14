# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

Rules for every task: CLAUDE.md "Code structure"; its strict rule binds every task: start at
`AppCore`, work only in the owning service. Live keys: only `OPENROUTER` and `API_KEY_MINIMAX`,
read with a loop, never `. ./.env`, never printed or committed. Before dispatch the controller
expands each task's named scope into exact caller, test, contract and map paths and records the
affected test commands. A blocked task is not a builder assignment. Per-task checks are focused;
the full gate and whole-branch review run once before each delivery PR.

### Shipped

A shipped task's entry, and its plan once deleted, are in this file in Git at the commit named.

| PR | Work | Tasks | Entries at |
|---|---|---|---|
| #41 | Architecture restructuring ([plan](plans/2026-10-21-architecture.md)) | A-0 to A-21, A-23, B-10, B-11; A-8c, A-8d, A-19b, A-19c in `d7a722cd` | `c7e7bac4` |
| — | A-6 settled with no code change (approval-authority spec §24.2, 2026-10-27) | A-6 | `4d5c022d` |
| #45 | The breakdown route goes; fixtures close their database first | A-7b, B-12 | `29ddfd88` |
| — | A-22 (product workspaces) deferred, trigger in [future](../core/future.md) "Housekeeping" | A-22 | `3dc2437f` |
| #46 | `PATCH /api/me` | B-13 | `e95b79c3` |
| #47 | The Hermes daemon goes; AYLUL project check and master key move into Core | A-8b1, A-8b2, B-14 to B-18 | `4f23c717` |
| #48 | Tool activity ([spec](specs/2026-10-31-tool-activity-design.md)) | B-19 to B-23 | `ce31036d` |
| #49 | AYLUL agents and panel, Core side | B-24 to B-34 | `b40e077b` |
| #51 | Agent identity and background tasks | B-35 to B-37 | `16da6b8c` |
| #52, #53 | Per-tool MCP permissions, and its review fixes | B-38, B-39, F-80 | `9c7c7e5a` |
| #54 | Agent groups, messaging slice 1 ([spec](specs/2026-11-03-agent-messaging-design.md)) | B-40 to B-46, F-81 to F-85, U-9, U-10 | `9e5b9ab2` |
| #55 to #57 | Planner context slices 1 to 3 ([spec](specs/2026-11-04-planner-context-design.md)) | B-47 to B-59, F-86 to F-89 | `fc78cb7`, `024432fc`, `1b413fc4` |
| #58 to #61 | Structure and depth, slices 1 to 5 | B-60 to B-82, F-90 to F-95 | `26bf739e`, `b64242a7`, `a4f89386` |
| #62 | ACP upgrade and conversation order | B-83, B-84, F-96 | `3a443ea0` |
| #63 | Codex app-server | B-85 to B-87, F-97 | `217d7540` |
| #64 | Executors, slices 0 and 1 | B-89 to B-93, F-98 | `c71b263d` |
| #65, #66 | Local delivery slices 1, 1b and 3, and its review fixes | B-95, B-96, B-99 to B-101, B-103, B-104, B-106, B-120 to B-136, F-106 to F-113 | `617d806f` |
| #67 | Codex images and language servers, slice 2 ([design](specs/2026-11-13-language-servers-design.md)) | B-137, B-138a, B-138 to B-146, F-114 to F-116 | `db760593` |

Replaced or dropped without being built: B-97, B-98, B-119 (by slice 2), B-102, B-110 (with the
critic), B-112 (by the Planner's judgment), B-115, B-116 (by B-125 and B-126). Their entries are in
this file at `db760593`.

### Known and left

- A persistent storage error while starting a delivery retries forever and stalls dispatch for every
  group (`crates/core-messages/src/deliveries.rs`).
- `Plans::owner_thread` (planner context spec §6, slice 4) waits for messaging slice 3.
- Real-provider acceptance of structure and depth slices 1 to 5 is not recorded.
- Recorded acceptances: §10 of the AYLUL agents design (2026-11-03, desktop app); planner context slices
  1 and 2 ([evidence](../evidence/2026-11-04-planner-context-probe.md)); slice 2's language servers on
  `testing4` (2026-11-13).

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

Order and rulings: [the program](plans/2026-11-09-local-delivery-program.md). Slice 4 is
detailed in its [plan](plans/2026-11-14-slice-4-task-questions.md): B-108, B-148, B-107,
B-109, B-147, F-102, B-149. B-108 is ready; later-slice tasks retain blocked draft pointers.

### B-105 · Minimal accepted decision revisions

- Status: blocked (deferred by the program until a task needs project decision revisions; no slice holds it)
- Plan: [Task 3](plans/2026-11-09-detailed-task-contracts.md#task-3-b-105-minimal-accepted-decision-revisions).
- Owner: Design.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-104); a task that needs a decision revision.
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-107 · Store a simple brief on each attempt

- Status: blocked until B-108; scheduled after B-148.
- Plan: [B-107](plans/2026-11-14-slice-4-task-questions.md#task-b-107-saved-attempt-context-and-task_brief).
- Owner: Runs and Plans; grants/adapters translate.
- Read: [approved design](specs/2026-11-14-task-questions-design.md), [wire agreement](../shared/requests/2026-11-14-task-questions.md), owning contracts and the task's Interfaces.
- Touch: exact Files of B-107 plus the plan's Common Touch paths; no other sources.
- Needs: B-108; accepted slice 4 agreement.
- Done when: B-107's five steps and common checks pass, current contracts/maps updated, scoped commit recorded. Live/browser acceptance is B-149.

### B-108 · Readiness from real inputs and current plan state

- Status: done; first task of slice 4.
- Plan: [B-108](plans/2026-11-14-slice-4-task-questions.md#task-b-108-authoritative-admission-and-exact-required-files).
- Owner: Plans, Runs and Codebase.
- Read: AppCore, [approved design](specs/2026-11-14-task-questions-design.md), [wire agreement](../shared/requests/2026-11-14-task-questions.md), owner contracts and Interfaces.
- Touch: exact Files of B-108 plus the plan's Common Touch paths; no other sources.
- Needs: none; slice 2 shipped in #67 and the slice 4 design/agreement are accepted for planning.
- Done when: B-108's five steps and common checks pass, no refused admission inserts an attempt, scoped commit recorded.

### B-109 · Task-aware questions and executor tools

- Status: blocked until B-107.
- Plan: [B-109](plans/2026-11-14-slice-4-task-questions.md#task-b-109-questions-about-source-and-planned-consumers).
- Owner: Plans relationships, Runs composition, Grants authority and adapters.
- Read: approved slice 4 design/agreement, task Interfaces and owner contracts.
- Touch: exact Files of B-109 plus Common Touch paths; no other sources.
- Needs: B-107.
- Done when: B-109's five steps/common checks pass, source/planned facts stay distinct, scoped commit recorded.

### B-147 · Durable contract conflicts and selective stops

- Status: blocked until B-109.
- Plan: [B-147](plans/2026-11-14-slice-4-task-questions.md#task-b-147-conflicts-through-the-existing-attempt-lifecycle).
- Owner: Runs; existing Grants, Turns and Harness lifecycles.
- Read: approved slice 4 design/agreement, task Interfaces, owner contracts and executor cancellation ruling.
- Touch: exact Files of B-147 plus Common Touch paths; no other sources.
- Needs: B-107, B-109.
- Done when: B-147's five steps/common checks pass, race/recovery/termination cases keep their invariants, scoped commit recorded. Untested live cases stay explicit.

### B-148 · Compatible ready groups

- Status: blocked until B-108; second task of slice 4.
- Plan: [B-148](plans/2026-11-14-slice-4-task-questions.md#task-b-148-compatible-ready-groups).
- Owner: Runs, with Plans admission and existing scope matcher.
- Read: approved slice 4 design/agreement, task Interfaces and owner contracts.
- Touch: exact Files of B-148 plus Common Touch paths; no other sources.
- Needs: B-108 only; no dependency on conflicts. Includes native Git --only procedure and shared-check coordination instructions.
- Done when: B-148's five steps/common checks pass, groups are pairwise compatible and do not retry attempted tasks, scoped commit recorded.

### B-149 · Real slice 4 acceptance

- Status: blocked until B-148 and F-102.
- Plan: [B-149](plans/2026-11-14-slice-4-task-questions.md#task-b-149-real-slice-acceptance).
- Owner: integration acceptance through AppCore and the actual UI.
- Read: approved design's Acceptance, B-149 steps and the live test patterns it names.
- Touch: exact Files of B-149 plus Common Touch paths; acceptance entries only in frontend stages.
- Needs: B-107, B-108, B-109, B-147, B-148, F-102.
- Done when: named live tests and disposable browser/log scenarios pass with dated evidence and scoped commit; unavailable checks remain reported, not passed.

### B-111 · Run required checks with source provenance

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-checks-and-critic.md#task-2-b-111-run-required-checks-with-source-provenance).
- Owner: Runs.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-109; accepted shared API/model agreement (B-110 dropped with the critic).
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

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
