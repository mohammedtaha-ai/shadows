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
`main` (PR #71, `0e281585`). Plan progress for a plan version, the fix that keeps one delivery's storage
error from stalling the other groups, and the slice 4 and structure-and-depth live records are on `main`
(PR #72, `753b26a4`). Executor choices were accepted by Mohammed in the browser on 2026-11-18 (B-159).
The contract check and the code record's first slice are on `main` (PR #73, `7b7abec`): file refusals
(B-167, a task may not modify or delete a file the project does not have; B-169, nor create one it
has), a big project's file listing no longer stalls Shadows (B-168), the `contract` check and the
record written with the done mark (B-170 to B-173), its acceptance on a real provider (B-174), and the
Report tab's contract row (F-129). Now, on `ccr-9e1b02d8-b0ylm1`: slice 2 of the code record as two
plans on this one branch (Mohammed, 2026-11-19; design §5, §8, §9 of
`docs/backend/specs/2026-11-19-project-code-record-design.md`). Plan 2a,
`docs/backend/plans/2026-11-19-code-record-2a.md`: tables and screens as Design's elements (§18.13),
pinned by task contracts, their states from the record, and `project_state` (B-175 to B-181, B-187). Plan 2b,
`docs/backend/plans/2026-11-19-code-record-2b.md`: `code_where` (B-182 to B-186). The controller
dispatches the builders and fixers (Mohammed, 2026-11-19); the backend lead prepares and checks.

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
| #68 | Slice 4: attempt context, task questions, conflicts and compatible ready groups ([design](specs/2026-11-14-task-questions-design.md)) | B-107 to B-109, B-147, B-148, F-102; B-149 (acceptance: browser on `testing4`, 2026-11-14; closed 2026-11-18; its live tests ran in #72, B-164) | `d73bb646` |
| — | Codex in the real Planner and executors slice 1, accepted by Mohammed in the browser (2026-11-15); their plans deleted | B-88, B-94 | `7846c2d9` |
| #69 | Slice 5: checks, returns and messages to a task's agent ([design §5.3, §5.4](specs/2026-11-08-executors-design.md#54-checks-shadows-runs-mohammed-2026-11-15)) | B-111, B-113, B-114, B-150, B-151 (acceptance, evidence (`docs/evidence/2026-11-15-slice-5-checks.md` at `4031168f`)) | `4031168f` |
| #70 | Slice 6: stop control, interruption, project rechecks and decisions (plan `docs/backend/plans/2026-11-16-slice-6-stop-control.md` at `449d4f95`; accepted in the browser on `testing6`, 2026-11-16) | B-117, B-152, B-153, B-154, B-105, B-118; F-121 to F-124 | `449d4f95` |
| #71 | Executor choices per task, the Codex protocol check (plan `docs/backend/plans/2026-11-18-codex-protocol-check.md` at `0e281585`), global external connections (plan `docs/backend/plans/2026-11-18-external-agent-connections.md` at `0e281585`; [spec](specs/2026-11-18-external-agent-connections-design.md)); executor choices plan at `0e281585`, shared request `docs/shared/requests/2026-11-18-executor-choices.md` at `0e281585` | B-155 to B-158, B-160, B-161; F-125 | `0e281585` |
| #72 | Plan progress for a plan version (B-163, F-127), one delivery's start failure refused without stalling the others (B-162), slice 4's live tests and the planned-consumer fix (B-164, B-166; evidence `docs/evidence/2026-11-18-slice-4-live.md`), structure and depth on a real provider (B-165; evidence `docs/evidence/2026-11-18-structure-and-depth-live.md`), Settings › Code as one row (F-126), visual acceptances (F-128; evidence `docs/evidence/2026-11-18-visual-acceptance.md`) | B-162 to B-166; F-126 to F-128 | `753b26a4` |
| #73 | Task contracts checked against the code: file refusals (B-167, B-169), a big listing no longer stalls Shadows (B-168), the code record's first slice (plan `docs/backend/plans/2026-11-19-code-record.md` at `7b7abec`; the `contract` check and the record written with done, B-170 to B-173; acceptance on Sonnet with pyright, B-174; evidence `docs/evidence/2026-11-19-code-record-live.md`), the Report tab's contract row (F-129) | B-167 to B-174; F-129 | `7b7abec` |
| — | Executor choices accepted by Mohammed in the browser (2026-11-18); the signed-in live tests were not recorded. Its plan is deleted (`docs/backend/plans/2026-11-18-executor-choices.md` at `7029198`) | B-159 | `7029198` |

Replaced or dropped without being built: B-97, B-98, B-119 (by slice 2), B-102, B-110 (with the
critic), B-112 (by the Planner's judgment), B-115, B-116 (by B-125 and B-126). Their entries are in
this file at `db760593`.

### Known and left

- A delivery whose turn started but whose link keeps failing on storage retries without a bound and
  holds back every group's turn endings until it links (`crates/core-messages/src/deliveries.rs`);
  bounding it would drop a running turn's reply. Dispatch of other groups continues. A start that
  keeps failing is refused after three tries (`docs/backend/stages.md` at `753b26a4`).
- `Plans::owner_thread` (planner context spec §6, slice 4) waits for messaging slice 3.
- Structure and depth slices 1 to 5 ran on a real provider on 2026-11-18 (`docs/evidence/2026-11-18-structure-and-depth-live.md`); the FastAPI example and the two-layers-at-one-place refusal were not run live.
- Recorded acceptances: §10 of the AYLUL agents design (2026-11-03, desktop app); planner context slices
  1 and 2 (evidence `docs/evidence/2026-11-04-planner-context-probe.md` at `4031168f`); slice 2's language servers on
  `testing4` (2026-11-13); slice 4's browser acceptance on `testing4` (2026-11-14, B-149); slice 6's
  browser acceptance on `testing6` (2026-11-16, B-118); executor choices in the browser (2026-11-18, B-159).
- Slice 4's live tests ran on 2026-11-18 (`docs/evidence/2026-11-18-slice-4-live.md`): 7 pass; `live_executor_asks_about_a_planned_consumer` failed and passes since the fix in `docs/backend/stages.md` at `753b26a4`.
- Slice 6's live test (`stop_control_live`) and its evidence file were dropped (Mohammed, 2026-11-16).
- B-150 keeps its own per-thread message queue beside Turns' queue; reusing Turns' queue needs the executor session restored by thread.
- The web derives "stopped on the additions cap" itself; a Core field on `TaskRun` should own it.
- When the file check of B-167 (`docs/backend/stages.md` at `7b7abec`) is skipped (no project folder, first listing running), the Planner is not told; it saw a like edit refused before and could not tell why.
- TypeScript's language server is `tsc --lsp` (TypeScript 7); on a machine with TypeScript 6 a TypeScript task's `contract` check is `not_run` with the server's error line (`docs/evidence/2026-11-19-code-record-live.md`).
- The contract check's Go shapes (`(*X).m`) are tested on recorded gopls answers only; no live gopls run yet.
- A Planner's `Read` (Claude) and read-only sandbox (Codex) can read any file the account can, outside the project too; only its instructions keep it in the project. Accepted as is by Mohammed on 2026-11-18.

## Open

Slice 2a of the code record. Plan: `docs/backend/plans/2026-11-19-code-record-2a.md`. Shared request:
`docs/shared/requests/2026-11-19-project-state-and-diagrams.md` (answered again on 2026-11-20 after
Masada's dispute: progress events, B-187; accepted by Masada on 2026-11-20, `b6b158d`).
The plan's "Rulings for every task" and "Gate files every task here touches" bind every task below.
Dispatch is the controller's (Mohammed, 2026-11-19): the controller runs `backend-builder` and
`backend-fixer` on these tasks; the backend lead prepares them and checks each report.

### B-175 · Design owns tables and screens

- Status: done
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-175--design-owns-tables-and-screens](plans/2026-11-19-code-record-2a.md#task-b-175--design-owns-tables-and-screens)
- Read: the plan's rulings and reuse table; `crates/shadows-core/src/app.rs`;
  `crates/core-design/contract.yaml`; `crates/core-design/src/decisions.rs` and
  `src/store/decisions.rs` (the pattern copied); §18.13 of
  `docs/core/specs/2026-10-03-project-planning-workspace-design.md`.
- Touch: the plan task's Files.
- Needs: nothing.
- Done when: the plan task's "Done when".

### B-176 · The Planner writes tables and screens

- Status: done
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-176--the-planner-writes-tables-and-screens](plans/2026-11-19-code-record-2a.md#task-b-176--the-planner-writes-tables-and-screens)
- Read: the plan's rulings; `crates/shadows-mcp/src/contract.yaml`;
  `crates/shadows-mcp/src/decision_tools.rs`; `crates/core-grants/contract.yaml` (`tools_for`);
  `crates/core-harness/contract.yaml`.
- Touch: the plan task's Files.
- Needs: B-175.
- Done when: the plan task's "Done when".

### B-177 · A task pins the elements it builds; the executor receives them

- Status: done
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-177--a-task-pins-the-elements-it-builds-the-executor-receives-them](plans/2026-11-19-code-record-2a.md#task-b-177--a-task-pins-the-elements-it-builds-the-executor-receives-them)
- Read: the plan's rulings; `crates/core-plans/contract.yaml` (`TaskContract`, `decision_pins`,
  `task_context_in`); `crates/core-plans/src/store/edit.rs` (`pins`);
  `crates/core-plans/src/store/task_context.rs`; §17 "The task contract".
- Touch: the plan task's Files.
- Needs: B-175; the shared request accepted.
- Done when: the plan task's "Done when".

### B-178 · The record keeps the pinned elements, and CodeRecord judges their state

- Status: done
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-178--the-record-keeps-the-pinned-elements-and-coderecord-judges-their-state](plans/2026-11-19-code-record-2a.md#task-b-178--the-record-keeps-the-pinned-elements-and-coderecord-judges-their-state)
- Read: the plan's rulings; `crates/code-record/contract.yaml`;
  `crates/code-record/src/store/mod.rs`; design §5.2.
- Touch: the plan task's Files.
- Needs: B-177.
- Done when: the plan task's "Done when".

### B-179 · `ProjectState`: the project's state, and the `project_state` tool

- Status: done (B-178 is done; its owner `ProjectState` was approved by Mohammed on 2026-11-19)
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-179--projectstate-the-projects-state-and-the-project_state-tool](plans/2026-11-19-code-record-2a.md#task-b-179--projectstate-the-projects-state-and-the-project_state-tool)
- Read: the plan's rulings; `crates/shadows-core/src/app.rs` (`assemble_services`);
  `crates/core-runs/contract.yaml` (`progress`); `crates/core-plans/contract.yaml` (`list`,
  `plan_parts_in`); `docs/codebase/contracts/TEMPLATE.yaml`; design §5.3, §8.
- Touch: the plan task's Files.
- Needs: B-178.
- Done when: the plan task's "Done when".

### B-187 · Runs journals each change of a task's progress

- Status: done
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-187--runs-journals-each-change-of-a-tasks-progress](plans/2026-11-19-code-record-2a.md#task-b-187--runs-journals-each-change-of-a-tasks-progress)
- Read: the plan's Ruling 11; `crates/core-runs/contract.yaml` (`transitions`, `mark_done`);
  `crates/core-runs/src/store/executor_choice.rs` (how Runs journals `TaskExecutorChosen`);
  executors design, "The plan page".
- Touch: the plan task's Files.
- Needs: nothing (its rows reach no client before B-180).
- Done when: the plan task's "Done when".

### B-180 · Routes and events for the web

- Status: done (B-179 and B-187 are done; the shared request was accepted on 2026-11-20)
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-180--routes-and-events-for-the-web](plans/2026-11-19-code-record-2a.md#task-b-180--routes-and-events-for-the-web)
- Read: the plan's rulings; `crates/shadows-http/src/contract.yaml` (decisions);
  `crates/core-events/contract.yaml`; the shared request.
- Touch: the plan task's Files.
- Needs: B-179, B-187; the shared request accepted.
- Done when: the plan task's "Done when".

### B-181 · Acceptance on a real provider (2a)

- Status: doing (run on 2026-11-21 on Claude Code, Mohammed's account; first plan, elements, live refresh and diagrams recorded in `docs/evidence/2026-11-21-code-record-2-live.md`; the diagram defects it found are fixed in `01f98d9`; left: run the second plan)
- Plan: [plans/2026-11-19-code-record-2a.md#task-b-181--acceptance-on-a-real-provider-2a](plans/2026-11-19-code-record-2a.md#task-b-181--acceptance-on-a-real-provider-2a)
- Touch: `docs/evidence/` (one new file); this entry's status.
- Needs: B-176 to B-180, B-187; the frontend's tasks under the shared request.
- Done when: the evidence file records the three steps as they happened.

Slice 2b of the code record: `code_where`. Plan: `docs/backend/plans/2026-11-19-code-record-2b.md`.
Shared request for its route: `docs/shared/requests/2026-11-19-code-where.md` (answered again on
2026-11-20 after Masada's dispute: `uses_live`; accepted by Masada on 2026-11-20, `b6b158d`). The plan's rulings and gate list bind every task below; dispatch is the controller's.

### B-182 · `code_where`, Shadows' half: the record read and ranked

- Status: done
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-182--code_where-shadows-half-the-record-read-and-ranked](plans/2026-11-19-code-record-2b.md#task-b-182--code_where-shadows-half-the-record-read-and-ranked)
- Read: the plan's rulings and reuse table; `crates/code-record/contract.yaml`;
  `crates/code-record/src/store/mod.rs`; `crates/core-design/src/layout.rs` (`owners`); design §5.3.
- Touch: the plan task's Files.
- Needs: B-178.
- Done when: the plan task's "Done when".

### B-183 · `code_where`, the code's half: confirmed within a budget

- Status: done (pyright tests with --include-ignored pass, after the LSP client declares hierarchical document symbols)
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-183--code_where-the-codes-half-confirmed-within-a-budget](plans/2026-11-19-code-record-2b.md#task-b-183--code_where-the-codes-half-confirmed-within-a-budget)
- Read: the plan's rulings; `crates/code-record/src/verify.rs` (`ask`, `document_symbols`,
  `matches`); `crates/codebase/contract.yaml` (`lsp`, `missing_inputs`, `inventory`).
- Touch: the plan task's Files.
- Needs: B-182.
- Done when: the plan task's "Done when", including its real-pyright tests run here with `--include-ignored`.

### B-184 · The `code_where` tool, and the Planner asks first

- Status: done
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-184--the-code_where-tool-and-the-planner-asks-first](plans/2026-11-19-code-record-2b.md#task-b-184--the-code_where-tool-and-the-planner-asks-first)
- Read: the plan's rulings; `crates/shadows-mcp/src/contract.yaml`;
  `crates/core-grants/contract.yaml` (`tools_for`); `crates/core-harness/contract.yaml`.
- Touch: the plan task's Files.
- Needs: B-183, B-179.
- Done when: the plan task's "Done when".

### B-185 · The `code_where` route

- Status: done
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-185--the-code_where-route](plans/2026-11-19-code-record-2b.md#task-b-185--the-code_where-route)
- Read: the plan's rulings; `crates/shadows-http/src/contract.yaml`; the shared request.
- Touch: the plan task's Files.
- Needs: B-183; the shared request accepted.
- Done when: the plan task's "Done when".

### B-186 · Acceptance on a real provider (2b)

- Status: doing (run on 2026-11-21, same evidence file: `code_where` answers and a hand-deleted symbol shows as a live gap; **the Planner did not call `code_where`** (P1; traced, fix in B-188 and B-189); left: step 1 again after B-188, and the stopped-server step)
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-186--acceptance-on-a-real-provider-2b](plans/2026-11-19-code-record-2b.md#task-b-186--acceptance-on-a-real-provider-2b)
- Touch: `docs/evidence/` (one new file); this entry's status.
- Needs: B-184; B-181 run on the same project.
- Done when: the evidence file records the three steps as they happened.

### B-188 · The Planner asks `code_where` before it reads (P1)

- Status: done (the controller's live check is still to run)
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-188--the-planner-asks-code_where-before-it-reads-p1](plans/2026-11-19-code-record-2b.md#task-b-188--the-planner-asks-code_where-before-it-reads-p1)
- Read: the plan task's root cause; `crates/core-harness/src/prompt.txt`;
  `crates/core-harness/contract.yaml`; `crates/shadows-mcp/src/record_tools.rs`.
- Touch: the plan task's Files.
- Needs: nothing.
- Done when: the plan task's "Done when"; then the controller's live check (B-186 step 1 again),
  recorded in the evidence file.

### B-189 · `code_where` gives each symbol's contract signature

- Status: done (the web's `symbol` fixture gained `inputs: []` so the regenerated schema
  typechecks; the rest of F-136 is the frontend's)
- Plan: [plans/2026-11-19-code-record-2b.md#task-b-189--code_where-gives-each-symbols-contract-signature](plans/2026-11-19-code-record-2b.md#task-b-189--code_where-gives-each-symbols-contract-signature)
- Read: `crates/code-record/contract.yaml` (`code_where`); `crates/code-record/src/model.rs`
  (`SymbolEntry`); `crates/core-plans/contract.yaml` (`SymbolWork`, `ContractValue`).
- Touch: the plan task's Files.
- Needs: B-188; the request amendment accepted.
- Done when: the plan task's "Done when".

### B-190 · A done task carried into a new version keeps its earlier verdict (P6)

- Status: done; its named tests pass. The branch gate is still to run.
- Plan: this entry. Found live on 2026-11-21 (`docs/evidence/2026-11-21-code-record-2-live.md`,
  P6): T2, carried to v3 and marked done there with no attempt of its own, recorded every
  symbol `at_done: unchecked`, because `Runs::done_record` gave a mark with no committed attempt
  in its version no attempt, commit or verdict.
- Rule (Runs supplies the facts, CodeRecord records them with `record_done_in`): with no committed
  attempt in this version, Runs follows the task's copy ancestry (`Plans::task_lineage_in`, then
  `previous` versions through `Plans::version_in`) to the newest earlier version with a committed
  attempt at it. That attempt's commit, contract and verdict are carried only when the earlier
  task is marked done no earlier than the attempt committed (the mark is for that commit) and the
  carried task's frozen contract equals the attempt's saved one. Otherwise the record stays
  `unchecked`. `record_in` drops the verdict only when this version has a committed attempt that
  is not the record's.
- Read: `crates/core-runs/contract.yaml` (`mark_done`); `crates/code-record/contract.yaml`
  (`DoneRecord`, `record_done_in`); `crates/core-plans/contract.yaml` (`task_lineage_in`).
- Touch: `crates/core-runs/src/lib.rs`, `crates/core-runs/src/store/marks.rs`,
  `crates/core-runs/contract.yaml`, `crates/code-record/src/model.rs` (the `DoneRecord` doc),
  `crates/code-record/contract.yaml`, `crates/shadows-core/tests/runs_attempts/contract_check.rs`.
- Needs: nothing.
- Done when: `cargo test -p shadows-core --features test-support --test runs_attempts`
  (`a_task_carried_with_its_contract_unchanged_keeps_its_verdict`,
  `a_task_carried_with_a_changed_contract_stays_unchecked`); the gate list.

### B-191 · The Planner asks `code_where` one short thing at a time (P8)

- Status: done in `8673b50` (tests pass). Its live check failed: in a new conversation ("people can
  change their email") the Planner loaded `code_where` and never called it; it ran `git ls-files`
  and read 7 files with `cat` before drafting (evidence, third run). The prompt alone does not
  move it; B-186 step 1 stays open on an owner decision.
- Plan: this entry. Found live on 2026-11-21 (`docs/evidence/2026-11-21-code-record-2-live.md`, P8):
  the Planner's one `code_where` call asked a 12-word sentence; the words tier needs every word in
  one file's purpose or one symbol's contract, so it answered `entries: []`, and the Planner read
  12 files with `cat`. Ruling (Mohammed, 2026-11-21, "take the best"): the words tier stays as
  it is (any-word matching would rank nearly every file on "and" or "logic"); the Planner is told
  how to ask instead.
- Rule: the `code_where` tool description, its `topic` argument's doc, and the prompt's "Look
  before you write a path" say: one short thing per call (a name the person used, a path, a symbol,
  an element id or name; one to three words), several calls rather than one sentence; an empty
  answer to a sentence is not "nothing there": ask again with each thing alone before reading.
- Touch: `crates/shadows-mcp/src/record_tools.rs`, `crates/core-harness/src/prompt.txt`, their
  contracts (`crates/shadows-mcp/contract.yaml`, `crates/core-harness/contract.yaml`) where they
  quote these lines, and any test that pins the prompt or the tool description.
- Needs: nothing.
- Done when: the tests of `core-harness` and `shadows-mcp` pass; then B-186 step 1 again, live.

### B-192 · `code_where` never answers "no uses" from a language server that has not seen the file (P9)

- Status: done in `ecbfb76`; live check passed (evidence, third run)
- Plan: this entry. Found live on 2026-11-21 (same evidence file, P9): pyright, started before the
  executors created `backend/accounts/`, answered `textDocument/references` for `SignIn` and
  `AppCore` with `[]`; `code_where` reported `uses_live: answered`, `uses: []`. pyright run by
  hand, and the same question after a restart of `shadows serve`, return the uses. The first ask
  right after a start also answered `[]` while pyright was still reading.
- Rule: an empty `uses` under `answered` means no callers. The fixer traces why the running
  server did not see the new files (watched-file notifications, the files it opens, or the
  readiness of a server just started) and fixes it in its owner; a server not yet ready gives
  `uses_live` `failed` or `skipped` with why, never `answered`.
- Touch: the owner the root cause names (`crates/codebase`, `crates/shadows-lsp` or
  `crates/code-record`), its contract, and a test through its public API.
- Needs: nothing.
- Done when: the new test fails before the fix and passes after; the touched crates' tests; then
  the live check: a Python symbol in a file made after pyright started shows its uses.

### B-193 · Shadows sends the code record with the Planner's context

- Status: doing
- Plan: this entry. Ruling (Mohammed, 2026-11-21): the Planner does not wait on itself to ask;
  Shadows feeds it. Spec: `docs/backend/specs/2026-11-04-planner-context-design.md` §4.2 ("Code
  record" layer, its cut order and owner); `docs/backend/specs/2026-11-19-project-code-record-design.md`
  §5.3 (first paragraph). Live reason: B-186 step 1 failed three runs out of three (evidence P1, P8).
- Rule:
  - `CodeRecord::digest(project) -> Result<RecordDigest, CodeRecordError>`: the record only, no
    language server. Per part of the approved map in map order (Design's `owners` for part and
    layer, as `code_where` uses), each recorded file (newest record per path) with its purpose,
    and its symbols with kind, inputs and returns from their contracts; files no part owns apart;
    the elements in `gap` or `unchecked` with why (as `project_state` reads them); `marks`: the
    count of done marks recorded. One read snapshot. Deterministic order (map order, layer, path,
    symbol name).
  - `Turns::planning_context` adds a "Code record" block after the index, rendered and budgeted
    in `planning/` (about 3 000 tokens, `budget.rs`), with the spec's cut order and `[cut: …]`
    markers; its source is `("code_record", project, marks)`, so a new done mark re-sends it
    through the existing digest rule. An empty record writes one line saying so (no code yet
    recorded: read the files the plan touches).
  - `Turns` receives `CodeRecord` in its constructor (`AppCore::start` wiring; `core-turns`'
    `Cargo.toml` gains `code-record`).
  - Prompt (`crates/core-harness/src/prompt.txt`): "Know the project before you plan" and "Look
    before you write a path" say the code record arrives in the context: plan from it; ask
    `code_where` for a symbol's line now, its uses, or what the block cut; read a file only for
    what neither gives. Keep B-191's one-short-thing line for `code_where`.
- Read: `crates/core-turns/contract.yaml` (`planning_context`), `crates/core-turns/src/planning/`,
  `crates/code-record/contract.yaml` and `src/code_where.rs` (how it reads the record and owners),
  `crates/project-state` (how it reads element states), `crates/shadows-core/src/app.rs`.
- Touch: `crates/code-record/src/{lib.rs,model.rs}` and a new `src/digest.rs` (+ store read if
  needed), `crates/code-record/contract.yaml`; `crates/core-turns/src/planning/{mod.rs,block.rs,budget.rs}`,
  `crates/core-turns/src/lib.rs` (constructor), `crates/core-turns/Cargo.toml`,
  `crates/core-turns/contract.yaml`; `crates/shadows-core/src/app.rs` (wiring only);
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`; the test pinning the
  prompt (`crates/shadows/tests/mcp_tools.rs`); tests through the public API: the digest's shape and
  order, the cut order, the empty record, and a planning context that carries it and is re-sent
  after a new done mark; the code map (`docs/codebase/README.md`) for the new module.
- Needs: nothing.
- Done when: the new tests and the touched crates' tests pass; `cargo test -p shadows --test
  architecture --test codemap --test contracts`; clippy for the touched crates; then live: a new
  Planner conversation plans "people can change their email" naming existing files and symbols
  without reading them first (B-186 step 1).
