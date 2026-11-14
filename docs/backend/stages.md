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
| #68 | Slice 4: attempt context, task questions, conflicts and compatible ready groups ([design](specs/2026-11-14-task-questions-design.md)) | B-107 to B-109, B-147, B-148, F-102 | `d73bb646` |
| — | Codex in the real Planner and executors slice 1, accepted by Mohammed in the browser (2026-11-15); their plans deleted | B-88, B-94 | `7846c2d9` |

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
  `testing4` (2026-11-13); slice 4's browser acceptance on `testing4` (2026-11-14, B-149).
- Slice 4's live tests (`crates/shadows/tests/task_questions_live.rs`, B-149) are written and not run.

## Tasks

## Approved local delivery implementation, 2026-11-09

Order and rulings: [the program](plans/2026-11-09-local-delivery-program.md). Slice 4 is on `main`
at `d73bb646` (PR #68); its tasks B-108, B-148, B-107, B-109, B-147 and F-102 are shipped there.
Only B-149 remains in its [plan](plans/2026-11-14-slice-4-task-questions.md). Later-slice tasks
retain blocked draft pointers.

### B-105 · Minimal accepted decision revisions

- Status: blocked (deferred by the program until a task needs project decision revisions; no slice holds it)
- Plan: [Task 3](plans/2026-11-09-detailed-task-contracts.md#task-3-b-105-minimal-accepted-decision-revisions).
- Owner: Design.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: `docs/backend/stages.md` at `617d806f` (B-104); a task that needs a decision revision.
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### B-149 · Real slice 4 acceptance

- Status: doing. Browser acceptance is done (`testing4`, 2026-11-14): two tasks ran together with commits
  holding only their own files; a contract conflict stopped its attempt, freed its scope and reached the
  Planner, shown on the Report tab; `task_question` answered a planned consumer and, with `documentSymbol`,
  gopls's symbols. The live tests (`crates/shadows/tests/task_questions_live.rs`) are written and not run.
- Plan: [B-149](plans/2026-11-14-slice-4-task-questions.md#task-b-149-real-slice-acceptance).
- Owner: integration acceptance through AppCore and the actual UI.
- Read: approved design's Acceptance, B-149 steps and the live test patterns it names.
- Touch: exact Files of B-149 plus Common Touch paths; acceptance entries only in frontend stages.
- Needs: the slice 4 tasks and F-102, all shipped in `docs/backend/stages.md` at `d73bb646` (PR #68).
- Done when: named live tests and disposable browser/log scenarios pass with dated evidence and scoped commit; unavailable checks remain reported, not passed.

## Slice 5: checks and messages to a task's agent

Design: [executors §5.4 and §5.3](specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15),
Mohammed, 2026-11-14 and 2026-11-15. Plan: [slice 5](plans/2026-11-15-slice-5-checks.md). Order: B-111 → B-113 → B-114 →
B-150 → F-103 → B-151. Each task's files, interfaces, tests and commit are in the plan.

### B-111 · Checks in the task contract

- Status: todo
- Plan: [B-111](plans/2026-11-15-slice-5-checks.md#b-111--checks-in-the-task-contract).
- Owner: Plans; Harness for the instruction text.
- Needs: `d73bb646` (slice 4).
- Done when: `TaskContract.checks` is validated as the plan says, existing contracts still read, both instructions name it; `task_contract` and `planner_instructions` tests pass; OpenAPI regenerated.

### B-113 · Runs runs a task's checks after its commit

- Status: todo
- Plan: [B-113](plans/2026-11-15-slice-5-checks.md#b-113--runs-runs-a-tasks-checks-after-its-commit).
- Owner: Runs.
- Needs: B-111.
- Done when: a committed attempt's checks run one at a time per project and are recorded with their outcome, exit code, HEAD and output end; a moved HEAD is inconclusive; a restart reruns unfinished checks; the committed notice lists the results; `attempt_checks` passes.

### B-114 · The Planner reads checks and judges on them

- Status: todo
- Plan: [B-114](plans/2026-11-15-slice-5-checks.md#b-114--the-planner-reads-checks-and-judges-on-them).
- Owner: Runs, with its HTTP and MCP adapters.
- Needs: B-113.
- Done when: `task_checks` and `GET /api/attempts/{id}/checks` answer the results; a done mark over a check that did not pass needs a reason, recorded on the task; the named route, MCP and core tests pass.

### B-150 · The Planner messages a task's agent

- Status: todo
- Plan: [B-150](plans/2026-11-15-slice-5-checks.md#b-150--the-planner-messages-a-tasks-agent).
- Design: [executors design §5.3](specs/2026-11-08-executors-design.md#53-the-planner-messages-a-tasks-agent-mohammed-2026-11-14).
- Owner: Runs, through Turns' send and notice and Harness's executor session.
- Needs: B-114 (same files in Runs and `run_tools.rs`).
- Done when: the Planner's `message_agent` reaches a task's executor while it runs (waiting until its turn ends), after it committed, and after a done mark (session reopened); the reply reaches the Planner as "Reply from task agent · T<n>"; no attempt, addition count or plan changes; `agent_messages` passes.

### B-151 · Real slice 5 acceptance

- Status: todo
- Plan: [B-151](plans/2026-11-15-slice-5-checks.md#b-151--real-slice-5-acceptance).
- Owner: integration acceptance through AppCore and the real UI.
- Needs: B-111, B-113, B-114, B-150, F-103.
- Done when: `checks_live` passes on the signed-in subscription and the browser journey, with its logs, is recorded in evidence.

## Slice 6: stop control and full acceptance

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
