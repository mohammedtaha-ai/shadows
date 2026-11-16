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
| #41 | Architecture restructuring (plan at `c604d06c`) | A-0 to A-21, A-23, B-10, B-11; A-8c, A-8d, A-19b, A-19c in `d7a722cd` | `c7e7bac4` |
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
| #69 | Slice 5: checks, returns and messages to a task's agent ([design §5.3, §5.4](specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15)) | B-111, B-113, B-114, B-150, B-151 (acceptance, [evidence](../evidence/2026-11-15-slice-5-checks.md)) | `4031168f` |

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
- B-150 keeps its own per-thread message queue beside Turns' queue; reusing Turns' queue needs the executor session restored by thread.
- The web derives "stopped on the additions cap" itself; a Core field on `TaskRun` should own it.

## Tasks

## Approved local delivery implementation, 2026-11-09

Order and rulings: [the program](plans/2026-11-09-local-delivery-program.md). Slice 4 is on `main`
at `d73bb646` (PR #68); its tasks B-108, B-148, B-107, B-109, B-147 and F-102 are shipped there.
Slice 5 is on `main` at `4031168f` (PR #69). Only B-149 remains in its [plan](plans/2026-11-14-slice-4-task-questions.md).
B-105 moved into slice 6 (Mohammed, 2026-11-16).

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

## Slice 6: stop control and full acceptance

Plan: [slice 6](plans/2026-11-16-slice-6-stop-control.md), approved by Mohammed (2026-11-16) after two
Codex reviews. Rulings: a contract problem stops only its own attempt and the Planner
stops other affected work (2026-11-14); leftovers continue, a done mark rechecks the plan's done
tasks, decisions are accepted on the person's card (2026-11-16).
Order: B-117 → B-153 → B-152 → B-154 → B-105 → web tasks from F-121 → B-118.
B-152, B-154 and B-105 build on the [shared request](../shared/requests/2026-11-16-slice-6-stop-control.md), accepted by Masada 2026-11-16 (`9dbb015c`).

### B-117 · The Planner lists the attempts that hold a scope (`task_runs`)

- Status: done (automated checks passed; manual acceptance remains with Mohammed)
- Plan: [B-117](plans/2026-11-16-slice-6-stop-control.md#task-b-117-the-planner-lists-the-attempts-that-still-hold-a-scope).
- Owner: Runs, with its MCP adapter.
- Read: `crates/core-runs/contract.yaml` (`stop_for`, `planner_plan`, `start_for`); the plan's Global Constraints.
- Touch: the task's Files list in the plan; no other path.
- Needs: slices 4 and 5 (shipped, `4031168f`).
- Done when: the four `planner::task_runs_*` tests, `the_planner_lists_live_attempts_over_mcp`, `mcp_server` and `planner_instructions` pass; contracts updated; scoped commit.

### B-153 · A new Start continues from the files a stopped attempt left

- Status: done (automated checks passed; manual acceptance remains with Mohammed)
- Plan: [B-153](plans/2026-11-16-slice-6-stop-control.md#task-b-153-a-new-start-continues-from-the-files-a-stopped-attempt-left).
- Owner: Runs.
- Read: `crates/core-runs/contract.yaml` (`start`, `start_answer`, the dirty-scope obligations); the plan's rulings.
- Touch: the task's Files list in the plan; no other path.
- Needs: B-117 (same `lib.rs` and contract).
- Done when: the four `leftovers::` tests and the changed `conflict_keeps_uncommitted_files` pass, with all of `runs_attempts` and `task_conflicts`; contract updated; scoped commit.

### B-152 · The Planner hears of an interrupted attempt and of the person's Stop

- Status: done (automated checks passed; manual acceptance remains with Mohammed)
- Plan: [B-152](plans/2026-11-16-slice-6-stop-control.md#task-b-152-the-planner-hears-of-an-interrupted-attempt-and-of-the-persons-stop).
- Owner: Runs.
- Read: `crates/core-runs/contract.yaml` (`recover`, `stop`, `stop_for`, `notify`, transitions); the plan's Review Focus.
- Touch: the task's Files list in the plan; no other path.
- Needs: B-153; the accepted request (`9dbb015c`).
- Done when: the six `stops::` tests and `a_stopped_conflict_keeps_its_one_notice` pass, with all of `runs_attempts` and `recovery`; contract updated; scoped commit.

### B-154 · A done mark rechecks every done task of its plan

- Status: done (automated checks passed; manual acceptance remains with Mohammed)
- Plan: [B-154](plans/2026-11-16-slice-6-stop-control.md#task-b-154-a-done-mark-rechecks-every-done-task-of-its-plan).
- Owner: Runs.
- Read: `crates/core-runs/contract.yaml` (`mark_done`, `confirmation`, `done_checks`, `recover`); `crates/core-plans/contract.yaml` (`plan`, `shared_in_transaction`); migration `0063_run_checks.sql`.
- Touch: the task's Files list in the plan; no other path.
- Needs: B-152 (same `recover.rs` and contract); the accepted request.
- Done when: the nine `rechecks::` tests pass, with all of `runs_attempts`, `recovery` and `openapi`; contract and code map updated; scoped commit.

### B-105 · Project decisions, proposed by the Planner and accepted by the person

- Status: todo (Files widened again by Hamoud, 2026-11-16: the `TaskContract` literals, after `483adee3`)
- Plan: [B-105](plans/2026-11-16-slice-6-stop-control.md#task-b-105-project-decisions-proposed-by-the-planner-and-accepted-by-the-person).
- Owner: Design, with Plans for the pins and the adapters.
- Read: `crates/core-design/contract.yaml` (`propose_vision`, `approve_vision_proposal`); `crates/core-plans/contract.yaml` (`TaskContract`, `TaskContext`, `task_context_in`); spec §17.3.
- Touch: the task's Files list in the plan; no other path.
- Needs: B-154 (migration order); the accepted request.
- Done when: the named tests in `design_decisions`, `task_contract`, `attempt_briefs`, `design_routes` and `mcp_server` pass, with `openapi`; contracts, code map and spec updated; scoped commit.

### B-118 · The full journey, live and in the browser

- Status: blocked (needs B-117, B-153, B-152, B-154, B-105 and the web tasks)
- Plan: [B-118](plans/2026-11-16-slice-6-stop-control.md#task-b-118-the-full-journey-live-and-in-the-browser).
- Owner: integration acceptance through AppCore and the real UI.
- Read: the plan's B-118; `crates/shadows/tests/checks_live.rs` for the live setup.
- Touch: the task's Files list in the plan.
- Needs: every task above and the web tasks.
- Done when: `stop_control_live` passes on the signed-in subscription and the browser journey, with its logs, is recorded in evidence.
