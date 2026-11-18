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
`449d4f95`. Executor choices per task, the Codex protocol check and global external connections are on
`main` (PR #71, `0e281585`). Now: B-159, the browser acceptance of executor choices (below).

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
| #71 | Executor choices per task, the Codex protocol check (plan `docs/backend/plans/2026-11-18-codex-protocol-check.md` at `0e281585`), global external connections (plan `docs/backend/plans/2026-11-18-external-agent-connections.md` at `0e281585`; [spec](specs/2026-11-18-external-agent-connections-design.md)); executor choices plan at `0e281585`, [shared request](../shared/requests/2026-11-18-executor-choices.md) | B-155 to B-158, B-160, B-161; F-125 | `0e281585` |

Replaced or dropped without being built: B-97, B-98, B-119 (by slice 2), B-102, B-110 (with the
critic), B-112 (by the Planner's judgment), B-115, B-116 (by B-125 and B-126). Their entries are in
this file at `db760593`.

### Known and left

- A delivery whose turn started but whose link keeps failing on storage retries without a bound and
  holds back every group's turn endings until it links (`crates/core-messages/src/deliveries.rs`);
  bounding it would drop a running turn's reply. Dispatch of other groups continues (B-162).
- `Plans::owner_thread` (planner context spec §6, slice 4) waits for messaging slice 3.
- Structure and depth slices 1 to 5 ran on a real provider on 2026-11-18 (`docs/evidence/2026-11-18-structure-and-depth-live.md`); the FastAPI example and the two-layers-at-one-place refusal were not run live.
- Recorded acceptances: §10 of the AYLUL agents design (2026-11-03, desktop app); planner context slices
  1 and 2 (evidence `docs/evidence/2026-11-04-planner-context-probe.md` at `4031168f`); slice 2's language servers on
  `testing4` (2026-11-13); slice 4's browser acceptance on `testing4` (2026-11-14, B-149); slice 6's
  browser acceptance on `testing6` (2026-11-16, B-118).
- Slice 4's live tests ran on 2026-11-18 (`docs/evidence/2026-11-18-slice-4-live.md`): 7 pass; `live_executor_asks_about_a_planned_consumer` failed and passes since B-166.
- Slice 6's live test (`stop_control_live`) and its evidence file were dropped (Mohammed, 2026-11-16).
- B-150 keeps its own per-thread message queue beside Turns' queue; reusing Turns' queue needs the executor session restored by thread.
- The web derives "stopped on the additions cap" itself; a Core field on `TaskRun` should own it.

## Open

### B-159 · Acceptance in the browser

- Status: todo (needs Mohammed's browser run and the signed-in live tests; B-155 to B-158, B-160, B-161 and F-125 are on `main` at `0e281585`)
- Plan: [B-159](plans/2026-11-18-executor-choices.md#task-b-159-acceptance-in-the-browser).
- Owner: integration acceptance through AppCore and the real UI.
- Touch: the task's Files list in the plan.
- Done when: the browser journey and its logs, and the named signed-in live tests (or "not run" with the reason), are recorded in evidence.

### B-162 · A delivery's storage error no longer stalls every group

- Status: done (the test is `a_delivery_failing_on_storage_is_refused_and_other_groups_still_dispatch`
  in `crates/shadows/tests/group_routes.rs`, beside the other dispatch tests: core-messages' own
  tests cannot build `Turns`)
- Plan: none; the defect is the "Known and left" entry above, written here in full.
  `Messages::dispatch_ready` (`crates/core-messages/src/deliveries.rs`) returns the first
  storage error of any candidate with `?`, so one delivery whose start keeps failing on storage
  stops the loop before the other groups' candidates, and the caller (`replies.rs`) retries the
  same order for ever. Find the root cause and fix it in the owner: one failing delivery must not
  keep the others from being dispatched, and a delivery that keeps failing must reach an end the
  person can see (a refusal notice in its group) instead of retrying for ever. Transient errors
  may still retry. Remove the "Known and left" entry when done.
- Read: `crates/core-messages/contract.yaml`; `crates/core-messages/src/deliveries.rs`,
  `replies.rs`, `store/`.
- Touch: `crates/core-messages/src/deliveries.rs`, `crates/core-messages/src/replies.rs`,
  `crates/core-messages/src/store/*.rs` if the end state needs a store write,
  `crates/core-messages/tests/groups.rs` (or a new `tests/deliveries.rs`),
  `crates/core-messages/contract.yaml`, this file.
- Done when: a test where one group's delivery fails on storage every time shows a second
  group's delivery still dispatched, and the failing one ends with a visible refusal after a
  bounded number of tries; `cargo test -p core-messages` passes.

### B-163 · One read gives every task's progress in a plan version

- Status: done
- Plan: none. The executors design (`specs/2026-11-08-executors-design.md`, "The plan page")
  promised the plan page's real progress; each task's `TaskRun` has `done`, but the plan page
  would need one call per task. Add `Runs::progress(workflow, version)` returning, in task
  order, one `TaskProgress { task, done, done_at, state }` per task, where `state` is the latest
  attempt's state or `null` with none (reuse the store reads `Runs::task` uses; prefer one store
  query over one per task). Route:
  `GET /api/workflows/{workflow}/versions/{version}/progress` returns `Vec<TaskProgress>`, 404 as
  the task read. Regenerate `api/openapi.json` and `web/src/api/schema.d.ts` with the repo's own
  generators. No write: marking done stays on the task page.
- Read: `crates/core-runs/contract.yaml`; `crates/core-runs/src/lib.rs` (`task`, `mark_done`);
  `crates/shadows-http/src/runs.rs` (`task`); `crates/shadows-http/src/contract.yaml`.
- Touch: `crates/core-runs/src/lib.rs` (or a new `src/progress.rs`), `crates/core-runs/src/model.rs`,
  `crates/core-runs/src/store/*.rs` if a new query is needed, `crates/core-runs/contract.yaml`,
  `crates/shadows-core/src/lib.rs` (re-export), `crates/shadows-http/src/runs.rs`,
  `crates/shadows-http/src/lib.rs` (route), `crates/shadows-http/src/contract.yaml`,
  `crates/shadows/tests/plan_routes.rs`, `api/openapi.json`, `web/src/api/schema.d.ts`, this file.
- Done when: a `plan_routes.rs` test reads progress for a three-task plan with one task marked
  done and one with an attempt, and gets them in order with the right `done` and `state`; an
  unknown version is 404; `cargo test -p shadows --test plan_routes` passes and the regenerated
  `api/openapi.json` is committed.

### B-164 · Run slice 4's live tests and record them

- Status: done (`docs/evidence/2026-11-18-slice-4-live.md`: 7 pass, 1 fail, B-166)
- Plan: none. `crates/shadows/tests/task_questions_live.rs` is written and has never run.
  Run it as its header says (`-- --ignored --test-threads=1`, gopls on PATH, Claude Code and
  Codex signed in) and record each test's outcome in
  `docs/evidence/2026-11-18-slice-4-live.md`: pass, fail with its first failing line, or not run
  with the reason. A failure is reported, not fixed in this task.
- Read: the test file's header; `crates/shadows/tests/fixtures/live.rs`.
- Touch: `docs/evidence/2026-11-18-slice-4-live.md`, this file and its "Known and left" entry.
- Done when: the evidence file names every test in the file with its outcome.

### B-166 · The planned-consumer live test asks while its attempt still runs

- Status: done (the test app never started the code service, so gopls stayed idle;
  `gopls_warm` starts it and warms gopls before the attempt; two live passes in
  `docs/evidence/2026-11-18-slice-4-live.md`)
- Plan: none. In `docs/evidence/2026-11-18-slice-4-live.md`,
  `live_executor_asks_about_a_planned_consumer` failed with GRANT_SCOPE "no running attempt holds
  this grant's task": the test asks `task_question` on the executor's grant, retrying while
  gopls starts, and the live executor can finish T1 first. Confirm the cause, then make the test
  hold the question to a running attempt (for example: warm gopls on the module before the
  attempt starts, or keep the executor from finishing until the test has asked) without
  weakening what it checks. A product change is out of scope; if the cause is in the product,
  stop and report.
- Read: the evidence file; `crates/shadows/tests/task_questions_live.rs` (`asked`, the test);
  `crates/shadows/tests/fixtures/live.rs`.
- Touch: `crates/shadows/tests/task_questions_live.rs`, the evidence file, this file.
- Done when: the test passes twice in a row live (`cargo test -p shadows --test
  task_questions_live live_executor_asks_about_a_planned_consumer -- --ignored`), recorded in
  the evidence file.

### B-165 · Structure and depth on a real provider

- Status: done (`docs/evidence/2026-11-18-structure-and-depth-live.md`; Mohammed confirms)
- Plan: none. Slices 1 to 5 of `specs/2026-11-05-structure-and-depth-design.md` have no
  real-provider acceptance. In the browser on an existing project, with the real Planner on
  Claude Code: the Planner proposes a map of parts, the person approves it, a part's waiver with
  a reason is kept, the roadmap gate refuses what the map lacks, and the result survives a
  daemon restart. Record each step with its daemon log lines in
  `docs/evidence/2026-11-18-structure-and-depth-live.md`. Mohammed confirms the record.
- Read: the spec's slices and its acceptance section.
- Touch: the evidence file, this file and its "Known and left" entry, the spec's status line.
- Done when: the evidence file records each step's observed result.
