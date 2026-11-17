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

The local delivery program is complete: its slices 1, 1b, 3, 2, 4, 5 and 6 are on `main`, the last
at `449d4f95` (PR #70). Its plan is `docs/backend/plans/2026-11-09-local-delivery-program.md` at
`449d4f95`. Now: executor choices on `feature/executor-choices` (tasks below).

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
| #68 | Slice 4: attempt context, task questions, conflicts and compatible ready groups ([design](specs/2026-11-14-task-questions-design.md)) | B-107 to B-109, B-147, B-148, F-102; B-149 (acceptance: browser on `testing4`, 2026-11-14; closed 2026-11-18, live tests written and not run) | `d73bb646` |
| — | Codex in the real Planner and executors slice 1, accepted by Mohammed in the browser (2026-11-15); their plans deleted | B-88, B-94 | `7846c2d9` |
| #69 | Slice 5: checks, returns and messages to a task's agent ([design §5.3, §5.4](specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15)) | B-111, B-113, B-114, B-150, B-151 (acceptance, evidence (`docs/evidence/2026-11-15-slice-5-checks.md` at `4031168f`)) | `4031168f` |
| #70 | Slice 6: stop control, interruption, project rechecks and decisions (plan `docs/backend/plans/2026-11-16-slice-6-stop-control.md` at `449d4f95`; accepted in the browser on `testing6`, 2026-11-16) | B-117, B-152, B-153, B-154, B-105, B-118; F-121 to F-124 | `449d4f95` |

Replaced or dropped without being built: B-97, B-98, B-119 (by slice 2), B-102, B-110 (with the
critic), B-112 (by the Planner's judgment), B-115, B-116 (by B-125 and B-126). Their entries are in
this file at `db760593`.

### Known and left

- A persistent storage error while starting a delivery retries forever and stalls dispatch for every
  group (`crates/core-messages/src/deliveries.rs`).
- `Plans::owner_thread` (planner context spec §6, slice 4) waits for messaging slice 3.
- Real-provider acceptance of structure and depth slices 1 to 5 is not recorded.
- Recorded acceptances: §10 of the AYLUL agents design (2026-11-03, desktop app); planner context slices
  1 and 2 (evidence `docs/evidence/2026-11-04-planner-context-probe.md` at `4031168f`); slice 2's language servers on
  `testing4` (2026-11-13); slice 4's browser acceptance on `testing4` (2026-11-14, B-149); slice 6's
  browser acceptance on `testing6` (2026-11-16, B-118).
- Slice 4's live tests (`crates/shadows/tests/task_questions_live.rs`) are written and not run; B-149 is closed on the browser acceptance.
- Slice 6's live test (`stop_control_live`) and its evidence file were dropped (Mohammed, 2026-11-16).
- B-150 keeps its own per-thread message queue beside Turns' queue; reusing Turns' queue needs the executor session restored by thread.
- The web derives "stopped on the additions cap" itself; a Core field on `TaskRun` should own it.

## Executor choices: CLI, model and effort per task

Plan: [executor choices](plans/2026-11-18-executor-choices.md), approved by Mohammed (2026-11-18) after
three Codex reviews (last plan commit `a1561e20`). Rulings (Mohammed, 2026-11-18):
- The choice is saved on the task and the last save wins.
- It follows the task across versions, and a replaced task starts empty.
- Lists come from each CLI; Claude Code's are read by one probe session per daemon run.
- No choice refuses Start.
- A Codex executor uses Codex's own tools in mode `ask`.
- The Planner picks the smallest model that can build the task from its contract.

Order: B-155 → B-156 → B-157 → B-158 → web tasks → B-159. B-155 and B-156 change nothing the web sees.
B-157 and B-158 do, and build on the [shared request](../shared/requests/2026-11-18-executor-choices.md), accepted by Masada (`da7a6950`).

### B-155 · Harness reads each CLI's catalog

- Status: done
- Plan: [B-155](plans/2026-11-18-executor-choices.md#task-b-155-harness-reads-each-clis-catalog).
- Owner: Harness, with Codex's catalog read in `shadows-agent`.
- Read: `crates/core-harness/contract.yaml` (`list`, remembered settings, the Codex choices rule); `crates/shadows-agent/src/choices.rs` (`Offered`); `crates/shadows-agent/src/codex/session.rs` (`catalog`); the plan's Rulings.
- Touch: the task's Files list in the plan; no other path.
- Needs: nothing open.
- Done when: the seven `harness_catalog` tests (live ones on the signed-in machine) and `harness_choices` pass; contract and code map updated; scoped commit.

### B-156 · Harness opens an executor on its attempt's CLI, model and effort

- Status: done
- Plan: [B-156](plans/2026-11-18-executor-choices.md#task-b-156-harness-opens-an-executor-on-its-attempts-cli-model-and-effort).
- Owner: Harness, with Turns' executor mode check and the two Runs call sites.
- Read: `crates/core-harness/contract.yaml` (the executor comment block, `open_executor`); `crates/core-turns/contract.yaml` (the executor's mode); `crates/shadows-agent/src/policy.rs` (Codex modes).
- Touch: the task's Files list in the plan; no other path.
- Needs: B-155 (its tests read the catalog).
- Done when: `executor::` unit tests, `executor_session`, `runs_attempts` and `core-turns` pass; Harness and Turns contracts updated; scoped commit.

### B-157 · Runs stores each task's choice and starts on it

- Status: todo (the request is accepted, `da7a6950`)
- Plan: [B-157](plans/2026-11-18-executor-choices.md#task-b-157-runs-stores-each-tasks-choice-and-starts-on-it).
- Owner: Runs; Plans for `copied_from` and `task_lineage_in`; Events for the tail kind.
- Read: `crates/core-runs/contract.yaml` (`start`, `start_for`, `task`, `amend`, `run_turn`, `ready_group`); `crates/core-plans/contract.yaml` (`start_draft`, `shared_in_transaction`); `crates/core-db/contract.yaml` (`command`); the plan's Rulings and Review Focus.
- Touch: the task's Files list in the plan, including every named caller of `Runs::start` and `Runs::start_for`; no other path.
- Needs: B-156; the accepted request.
- Done when: the `executor_choice::` tests, the changed `plan_storage` copy test and the suites Step 5 names (with `openapi`) pass; Runs, Plans and Events contracts and spec D6 updated; scoped commit.

### B-158 · The person's and the Planner's way to the choice

- Status: todo (the request is accepted, `da7a6950`)
- Plan: [B-158](plans/2026-11-18-executor-choices.md#task-b-158-the-persons-and-the-planners-way-to-the-choice).
- Owner: the HTTP and MCP adapters over Runs and Harness; Grants for the tool list.
- Read: `crates/shadows-http/src/contract.yaml`, `crates/shadows-mcp/src/contract.yaml` (`task_start`); the plan's web API table.
- Touch: the task's Files list in the plan; no other path.
- Needs: B-157; the accepted request.
- Done when: the named `runs_routes`, `mcp_server`, `planner_instructions`, `harness_choices` tests and `openapi` pass; contracts updated; scoped commit.

### B-159 · Acceptance in the browser

- Status: blocked (needs B-155 to B-158 and the web tasks)
- Plan: [B-159](plans/2026-11-18-executor-choices.md#task-b-159-acceptance-in-the-browser).
- Owner: integration acceptance through AppCore and the real UI.
- Touch: the task's Files list in the plan.
- Done when: the browser journey and its logs, and the named signed-in live tests (or "not run" with the reason), are recorded in evidence.
