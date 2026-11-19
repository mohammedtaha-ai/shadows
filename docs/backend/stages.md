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
B-167 (a task may not modify or delete a file the project does not have) is done on this branch.
B-168 (a big project's file listing no longer stalls Shadows) and B-169 (a task may not create a
file the project already has) are done on this branch. Next: the code record's first slice (B-170), once its plan is written.

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
- When B-167's file check is skipped (no project folder, first listing running), the Planner is not told; it saw a like edit refused before and could not tell why.
- A Planner's `Read` (Claude) and read-only sandbox (Codex) can read any file the account can, outside the project too; only its instructions keep it in the project. Accepted as is by Mohammed on 2026-11-18.

## Open

### B-170 · The code record, slice 1: verification against the contract, and the record

- Status: blocked (waits for its plan, `docs/backend/plans/2026-11-19-code-record.md`, written once
  §4's OPEN in the design is ruled; the earlier text of this entry recorded after done, which
  Mohammed corrected on 2026-11-19: a task is verified before it is done)
- Plan: `docs/backend/plans/2026-11-19-code-record.md` (not written yet)
- Design: `specs/2026-11-19-project-code-record-design.md` §3, §4, §8, §9 slice 1.

### B-169 · A task may not create a file the project already has

- Status: done (built by the controller: `ContractRules::file_problems` checks a create the edit
  introduces; tests `a_task_may_not_create_a_file_the_project_already_has` and
  `a_create_the_task_already_had_is_not_checked_again`, the second shown to fail without its guard)
- Plan: none; the ruling is §17 "The task contract" (Mohammed, 2026-11-19), the other half of
  B-167: the Planner modifies what is there instead of writing a second file beside it.
- Done when: the two tests and `cargo test -p shadows-core --features test-support --test task_contract` pass.

### B-168 · A big project's file listing no longer stalls Shadows

- Status: done (built by the controller at Mohammed's request: `write_inventory` reads outside the write and writes batches of 5,000 rows, scan row last; listings of a project run one at a time; extensions held in memory; tests `other_writes_go_on_while_a_big_listing_is_written` and `the_wanted_servers_come_from_the_listing_not_a_second_read`, both failing on the old code; the live rerun is in the evidence file)
- Found live: `docs/evidence/2026-11-18-missing-file-refusal-live.md`, "The first listing still
  running". A first listing of 2,000,000 files took 218 s; for all of it the Planner's tool calls
  on every project got no answer (one timed out), and other projects' listings waited. After it,
  each Planner turn on that project read the whole list (2,000,005 rows, 15 s).
- Root cause, traced by the controller:
  1. `CodeStore::write_inventory` (`crates/codebase/src/store/inventory.rs`) writes the whole
     listing in one `Storage::write_txn`, one `INSERT` per row, after a `SELECT` of every stored
     row inside the same transaction. `write_txn` holds Shadows' single write lock
     (`crates/core-db/src/lib.rs`, `self.write.lock()`), so every other write in the daemon waits
     until the listing's last row.
  2. `CodeStore::file_extensions` (`crates/codebase/src/store/servers.rs`) reads every file path
     of the project to find its extensions, and is asked on each turn.
- Plan: none; the fix stays inside Codebase.
  1. Read the stored rows through the reader, outside the write transaction, and work out the
     upserts and deletes there. Write them in bounded batches (a few thousand rows per
     transaction, multi-row `INSERT`), so the write lock is released between batches. The scan
     row (`code_inventory_scan`) is written last, in its own transaction, so a first listing stays
     "indexing" to readers until it is whole, as now. A later listing that changes rows moves the
     generation (as now), so a reader paging across it gets `InventoryStale` rather than a mixed
     page; say in the contract how a reader of single paths (`files_present`) sees a listing in
     progress.
  2. Keep each project's file extensions in Codebase's memory, computed from the rows the listing
     already holds when it is recorded; `file_extensions` answers from there and reads the store
     only when nothing is held yet (after a restart), once.
- Read: `crates/codebase/contract.yaml`; `crates/codebase/src/inventory.rs` (`record_inventory`,
  `files_present`, `inventory`); `crates/codebase/src/store/inventory.rs`; `crates/codebase/src/store/servers.rs`;
  `crates/codebase/src/lib.rs` (the inner state); `crates/core-db/src/lib.rs` (`write_txn`, `reader`).
- Touch: those codebase files, `crates/codebase/contract.yaml`, codebase's tests
  (`crates/codebase/tests/`), this entry's status. Not `core-db`.
- Done when: a test lists a folder of tens of thousands of files and shows another write
  (any `write_txn`) completing while that listing is still being written; the existing inventory
  invariants tests (generation, incomplete listings, deletes only when complete, stale cursors)
  pass; a test shows `file_extensions` answering without reading the store a second time;
  `cargo test -p codebase` (with its test-support feature if needed), `cargo test -p shadows-core
  --features test-support --test task_contract` and
  `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings` pass.
  The controller reruns the 2,000,000-file journey live afterwards.

### B-167 · A task may not modify or delete a file the project does not have

- Status: done. Codebase::files_present answers which paths are listed files (None when the list cannot answer); Plans gets Codebase in new, asks it before an edit (present_files), and the pure ContractRules::missing_file_problems refuses inside the edit transaction for changed tasks. Tests in crates/shadows-core/tests/task_contract.rs.
- Plan: none; the ruling is §17 "The task contract" (`docs/core/specs/2026-10-01-task-contracts-and-evidence-design.md`,
  Mohammed, 2026-11-18). When a plan edit is saved, Plans refuses a task whose contract names a
  file with action `modify` or `delete` that is not in the project's file inventory, unless a task
  it depends on (directly or transitively, through the plan's existing dependency graph) has that
  path with action `create`. A path that such a dependency has with action `delete` is refused
  too. The refusal is the edit's existing invalid-plan answer, one problem per file, naming the
  task and the path. When the inventory cannot answer (`InventoryState::Missing` or `Indexing`),
  the check is skipped. Plans' rules stay pure: the in-plan part (creates and deletes along
  dependencies) is a pure rule over `PlanContent`; only the existence question goes to Codebase.
  Plans receives `Codebase` in its constructor (`AppCore::start`); if Codebase has no method that
  answers "which of these paths does this project have" without paging the whole inventory, add
  one there, with its contract line. Find the edit path from `crates/core-plans/contract.yaml`
  and `crates/core-plans/src/rules.rs` (`edit_problems`).
- Read: `crates/shadows-core/src/app.rs` (`Plans::new`, `Codebase::new`);
  `crates/core-plans/contract.yaml`; `crates/core-plans/src/task_contract.rs`, `rules.rs`,
  `dependency_graph.rs`; `crates/codebase/contract.yaml`; `crates/codebase/src/inventory.rs`.
- Touch: `crates/core-plans/src/lib.rs`, the edit path and rule files it names,
  `crates/core-plans/contract.yaml`, `crates/core-plans/tests/` (or the existing plan-edit tests
  the contract names), `crates/codebase/src/inventory.rs` and `crates/codebase/contract.yaml` if a
  method is added, `crates/shadows-core/src/app.rs`, `plans_for_tests` and any test fixture that
  builds `Plans`, this entry's status.
- Done when: tests show (1) `modify` of a missing path is refused naming task and path; (2) `modify`
  of a path a dependency creates is accepted; (3) `modify` of a path a dependency deletes is
  refused; (4) `modify` of an existing file is accepted; (5) a project with no folder or an
  unfinished listing skips the check; the core-plans and codebase tests and
  `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings` pass.
