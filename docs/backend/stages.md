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
`449d4f95`. The code record's slices 1 and 2 are on `main`: the contract check (PR #73, `7b7abec`), and
the project's state, its diagrams' data and `code_where` with the code record sent to the Planner
(PR #75, `4d69977d`). Codex commit recovery is on `main` in PR #75, and its live acceptance is closed
(B-196, 2026-11-23). The Claude catalog issue waits for Claude quota to return.

The Windows `testing7` run is recorded in `docs/evidence/2026-11-22-testing7-go-notes-live.md`
at `627a0e94`.

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
| #75 | Project state and the code record: Design's tables and screens pinned by tasks, element states, `project_state` (B-175 to B-181, B-187; evidence `docs/evidence/2026-11-21-code-record-2-live.md` at `4d69977d`); `code_where` and the Planner asking it, and the code record sent with the Planner's context (B-182 to B-186, B-188 to B-193); Codex commit recovery: the Git-write procedure and Start's readiness for an attempted task (B-194, B-195; B-196 closed 2026-11-23; evidence `docs/evidence/2026-11-22-codex-commit-recovery-live.md` at `4d69977d`); the web's state, diagrams and `code_where` (F-130 to F-136) | B-175 to B-195; F-130 to F-136 | `4d69977d` |

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
- A language server that exits is not started again until `shadows serve` restarts; until then `code_where` gives the record's half with `unchecked` and why (evidence, fifth run).
- A Planner's `Read` (Claude) and read-only sandbox (Codex) can read any file the account can, outside the project too; only its instructions keep it in the project. Accepted as is by Mohammed on 2026-11-18.

## Open

The `testing8` live run is finished. Open: its findings (B-199), B-200 to B-205, and B-208 to B-210 (polling to SSE) on `feature/after-75`. B-196, B-197 and B-198 are done on
`feature/after-75` and stay here until the next docs sync.

### B-196 · Verify Codex commit recovery on Windows

- Status: done (2026-11-23). The native approved retry, the existing lock, released leftovers,
  the personal-edit refusal and the Planner's judgment were observed
  (2026-11-22, [live evidence](../evidence/2026-11-22-codex-commit-recovery-live.md)).
  A person cannot reject a Codex Git approval: `ask` routes every request to Codex's
  `auto_review`, never to the person; a reviewer denial stays unobserved
  ([finding](../evidence/2026-11-23-codex-commit-recovery-live.md)).
- Plan: [Codex commit recovery, B-196](plans/2026-11-22-codex-commit-recovery.md#b-196--verify-the-recovery-with-native-codex-on-windows).
- Read: B-196's acceptance steps and `docs/evidence/2026-11-22-testing7-go-notes-live.md` at `627a0e94`.
- Touch: one `docs/evidence/<run-date>-codex-commit-recovery-live.md` and this entry's status.
- Needs: nothing. B-194 and B-195 are done. Controller and person run the live acceptance.
- Done when: the evidence records a rejected approval request for a denied Git write, and what the
  executor and the Planner did after it.

### B-197 · A hold reports only project files, and an attempt's writes are its own

- Status: done (2026-11-23, `ac7d1e3d`). Found live in `testing8`: the Planner's end-of-turn check
  asked about an executor's files, git-ignored caches and build output, folders and vanished temp
  files, and `project_change_note` refused most of them. `ActiveHold::changed()` now keeps only
  paths the project's listing holds as files (or a known file deleted), and `hold_claiming` gives
  a running attempt the paths inside its `writes`. Tests: `crates/codebase/tests/hold.rs`.
  Live: zero such notices with two Codex attempts and pytest running
  ([evidence](../evidence/2026-11-23-testing8-go-python-live.md)).

### B-198 · Adopt the Codex CLI 0.171.0 protocol

- Status: done (2026-11-23, `d5d7baba`). 0.171.0 changed `AgentMessageDisplay`, which Shadows does
  not read. The snapshot was adopted with `adopt_the_installed_codex_protocol` after the signed-in
  live Codex tests passed; the fixtures follow the new floor.

### B-199 · Finish the testing8 live run, and the findings it left

- Status: the run is finished (2026-11-23): all 14 tasks judged done, both programs run end to
  end on one data file (Go 8 packages pass, pytest 39 passed). The findings below remain.
- Findings, not fixed ([evidence](../evidence/2026-11-23-testing8-go-python-live.md)):
  - Codex's sandbox cannot run the WindowsApps `python` alias, so executors report Python checks
    `not_run` (Shadows' own checks pass them).
  - Every Codex executor's first `git add` is denied on `index.lock`, then approved: one extra
    round per task.
  - A Codex start slower than `setup_wait` (20 s) because its model-list refresh timed out fails
    to open, and the reason is only in Codex's own log.
  - Shadows' Codex model list did not offer `gpt-6.2-sol`.
  - `live_the_claude_code_catalog_lists_each_models_efforts` failed; not examined.
  - pyright is not installed on this machine (each Python contract is `unchecked`).
  - Codex's sandbox could not copy a fixture into its temporary folder (`PermissionError`), so an
    executor's subprocess test did not run there.
  - The sandbox findings are Codex's only: a Claude Code executor on the same project ran pytest,
    copied into a temp folder and committed with no denial (`6a662f5`).
  - The Planner counted 15 tasks; the plans hold 14.
- Done when: each finding is fixed, accepted or moved to future.

### B-200 · A project recheck runs when a plan is complete, or when asked

- Status: done (2026-11-24) in code and tests: only the mark that completes its plan version
  rechecks; `Runs::recheck_project` serves `POST /api/workflows/{workflow}/recheck` and the
  Planner's `project_recheck`. Live in `testing8` (2026-11-24, plan "Water CLI reset-today"):
  T1's mark started no recheck; T2's, completing the plan, started one (15:06:42 to 15:12:13 UTC,
  30 passed, 6 `contract` `not_run` for want of pyright, no notice). Notices came 20 s and 3 s
  after the commits ([evidence](../evidence/2026-11-23-testing8-go-python-live.md)).
  Follow-up (2026-11-24): the Planner prompt names `project_recheck` (when the person asks; a
  completing done mark already starts one), held by
  `mcp_tools::the_planner_is_told_how_to_run_and_judge_tasks`. An asked recheck in which every
  check passed tells its asker so (`an_asked_recheck_that_passes_says_every_check_passed`); a
  mark's batch keeps its rule, notice only on a new failure.
- Owner: [executors design, "A project recheck runs when a plan is complete, or when
  asked"](specs/2026-11-08-executors-design.md).
- Change: a done mark runs no project recheck unless it completes its plan. Add one Runs method
  that starts a project recheck on request, called by an HTTP route (the person) and an MCP tool
  (the Planner); its notice goes to the asking Planner, or to the plan's Planner when the person
  asks. Update the `core-runs` contract and the MCP tool list.
- Needs: a frontend task for the person's "Recheck the project" control (shared request to the
  frontend side).
- Done when: in a live run, marking a task that is not its plan's last starts no recheck and the
  next notice arrives without waiting; marking the last task starts one; the person and the
  Planner can each start one.

### B-201 · A Codex conversation in `ask` can commit

- Status: done (2026-11-24) in code and a signed-in live test
  (`live_a_codex_conversation_in_ask_commits_but_keeps_hooks_and_config`), and live in
  `testing8` (2026-11-24): two Codex `gpt-6-luna` executors committed first time, with no
  `index.lock` denial and no approval. A per-turn
  `sandboxPolicy` replaces a profile, so `ask` sends the experimental `permissions` field; the
  connection opens with `experimentalApi` and the snapshot was re-adopted from the experimental
  schema. The `testing8` executor run in "Done when" is not yet done.
- Owner: [language servers design, "A Codex conversation in `ask` can commit"](specs/2026-11-13-language-servers-design.md#the-planners-shell-and-its-changes).
- Change: in `crates/shadows-agent/src/codex.rs` and `codex/session.rs`, the `ask` mode's
  filesystem permissions write the project root and its `.git`, read `.git/hooks` and
  `.git/config`, deny `.git/config.lock`, write the temp folder and read the rest. The probe used
  a `[permissions.<name>.filesystem]` profile with `default_permissions`; first confirm that the
  per-turn `sandboxPolicy` does not replace it, and keep the opening's echo check true.
  `read-only` is unchanged. Update the `shadows-agent` contract.
- Done when: a live Codex executor in `testing8` commits with no `index.lock` denial, and a
  signed-in live test shows a hook write and `git config` refused.

### B-202 · A recheck interrupted by a restart runs again

- Status: done (2026-11-24). Found in `testing8` (2026-11-23): a restart mid-recheck left its
  unfinished checks `not_run`, and nothing ran them again. Cause: `recover_rechecks` marked them
  `not_run` by design. It now runs the batch through `run_recheck`, as a fresh one runs. Test:
  `a_restart_runs_an_interrupted_recheck_again`.
- Owner: [executors design, "A restart loses nothing"](specs/2026-11-08-executors-design.md).
- Change: `Runs::recover_rechecks` (`crates/core-runs/src/recheck.rs`) runs a batch's unfinished
  rows at startup instead of marking them interrupted, then ends and delivers it as usual.
- Done when: a test interrupts a recheck, restarts Runs, and sees the rows run and the batch end.

### B-203 · A slow Codex start says why it failed

- Status: done (2026-11-24). Found in `testing8`: two Setup attempts failed to open because Codex's
  model-list refresh timed out past the 20 s `setup_wait`; the reason was only in Codex's log.
  Cause: Codex's own `thread/start` waits on its model-list refresh (Codex's log, 16:19 UTC: the
  stale cache, two 5 s tries reading the 850 KB list, then `get_default_model` a third), which
  Shadows cannot skip; Shadows threw away Codex's stderr, so a timeout said only "harness setup
  timed out". The second attempt's own reason (`workspace routing discovery timed out`) was
  carried already. Now the opening's timeout ends with the last error Codex logged on stderr.
  Test: `a_codex_start_past_setup_wait_names_what_codex_logged`.
- Owner: [harness controls, `setup_wait`](../core/specs/2026-09-24-harness-controls-design.md).
- Change: find why Codex's opening waits on the model-list refresh, and either keep it off the
  opening's path or give the opening's failure Codex's reason. Root cause first.
- Done when: an opening with no network fails or succeeds within `setup_wait` and, on failure,
  the attempt's reason names the cause.

### B-204 · Shadows' Codex model list offers what Codex offers

- Status: done (2026-11-24) in code and the live Codex catalog test (it lists `gpt-6.2-sol`);
  the executor menu in a browser is not yet seen. Found in `testing8`: `gpt-6.2-sol` was in
  Codex's own cache but not in Shadows' list, so executors could not use it. Cause: the catalog
  was probed once per daemon and kept for its life. The probe at 16:12 UTC ran while Codex
  0.169.0's cache was stale and its refresh timed out (Codex's log), and Codex answered its
  built-in list (`gpt-6.1-sol` first, `gpt-5.5`, no 6.2; the Planner quoted it at 16:26);
  Codex itself listed `gpt-6.2-sol` later, Shadows never asked again. Now a read catalog stands
  for 300 s (Codex's own cache age), then is read again; a failed re-read keeps the last list.
  Tests: `an_aged_catalog_is_read_again`, `a_failed_reread_keeps_the_last_catalog`.
- Change: find where the Codex catalog drops it (`crates/core-harness/src/catalog.rs` and the
  Codex probe) and fix the cause. Root cause first.
- Done when: the live Codex catalog test lists `gpt-6.2-sol` and the task executor menu offers it.

### B-205 · The Claude Code catalog live test passes

- Status: done (2026-11-24), no code change: the test passes signed in (11 models, `fable`
  among them). `harness_catalog.rs:160` is the catalog probe's own `unwrap`, so the probe
  failed then; its error was not kept. The probe's one recorded failure of this kind
  (`docs/evidence/2026-11-22-testing7-go-notes-live.md`) is Claude Code refusing a model switch
  (`claude-fable-5-1`: `API error: 403 Request not allowed · model not changed`) while the
  account's Claude quota was out, which ends the whole probe (only "Usage credits are required"
  skips one model). Cause outside Shadows; whether a 403 should skip that one model is left open.
  `live_the_claude_code_catalog_lists_each_models_efforts` (`harness_catalog.rs:160`) failed
  during B-198's run and was not examined then.
- Change: run it, find whether the test or the catalog is wrong, fix that. Root cause first.
- Done when: the test passes signed in.

### B-206 · The plan page can read the project's last recheck

- Status: done. For F-137 (the person's "Recheck project" button). Decided by Mohammed on
  2026-11-24.
- Owner: [executors design, "A project recheck runs when a plan is complete, or when
  asked"](specs/2026-11-08-executors-design.md).
- Change: one Runs read and one route, `GET /api/workflows/{workflow}/recheck`: the project's
  newest recheck batch (the workflow names the project), or none. It answers the batch id, what
  began it (a completing mark with plan title and task, the person, or a Planner), created and
  finished times, and counts of its rows by outcome (passed, failed, timed out, inconclusive,
  not run, not finished yet). Regenerate `api/openapi.json`; update the core-runs contract.
- Done when: a test reads none, then a running batch with unfinished rows, then the finished
  batch's counts; `cargo test -p shadows --test openapi` passes.

### B-207 · The person can read a project recheck's output, live while it runs

- Status: done. For F-138 (the recheck's terminal panel on the plan page). Decided by Mohammed
  on 2026-11-24.
- Owner: [executors design, "A project recheck runs when a plan is complete, or when
  asked"](specs/2026-11-08-executors-design.md); amend it with the two reads below.
- Change: two Runs reads and two routes.
  - `GET /api/rechecks/{recheck}`: the batch's rows in order (task, then the check's ordinal):
    task, attempt id, check id, the check's command as the person reads it, outcome or none,
    started and finished times, and the stored output with its truncated flag. 404 for an
    unknown batch.
  - `GET /api/rechecks/{recheck}/output`, SSE, live only (not journaled): on connect, each
    started and unfinished row's output so far, then each new chunk as the check prints it
    (`{task, attempt_id, check_id, text}`), a row's start and finish (with its outcome), and an
    end event when the batch finishes; a finished batch answers the end event at once. The
    output a running row has printed is held in memory by Runs, capped as the stored output is;
    nothing new is written to the database.
  - Regenerate `api/openapi.json`; update the core-runs contract and the shadows-http routes.
- Done when: a test reads a finished batch's rows with their output; a test subscribes while a
  check runs and receives a chunk the check printed before it finishes, then its finish and the
  end event; `cargo test -p shadows --test openapi` passes.

### B-208 · The project stream says when a planning thread's list changes

- Status: done (2026-11-24). Decided by Mohammed on 2026-11-24 (polling to SSE; Hamoud's proposal, agreed with
  Masada). For F-d, which drops the 10 s polls of `threadsQuery` and `agentThreadsQuery`.
- Owner: [project plans design, "How the pages stay current"](../core/specs/2026-10-01-project-plans-design.md);
  amend it in place: the project tail also carries what changes a project's thread list.
- Change:
  - Events: the project tail (`read_project_events_after`, `crates/core-events/src/store.rs`)
    also carries `ThreadRetitled` and `ThreadRemoved`, which Threads already journals with the
    thread's project.
  - Operations: a turn's end (`OperationCompleted`, `OperationFailed`, `OperationCancelled`,
    `OperationInterrupted`, including recovery's) is journaled with its thread's project, read
    in the same write; the project tail carries these four kinds. A turn with no thread has no
    project and stays off the tail. The payload is unchanged.
  - Update the core-events and core-operations contracts.
- Done when: a test retitles a thread, ends a turn in it and removes it, and reads the three
  events from the project stream in sequence order; a thread of another project is not there.

### B-209 · A task's check results and conflict delivery reach the project stream

- Status: done. Decided by Mohammed on 2026-11-24. For F-b and the follow-up that drops the 2 s
  polls of `attemptChecksQuery` and `attemptConflictQuery`.
- Owner: [executors design, "Progress reaches the project stream"](specs/2026-11-08-executors-design.md);
  amend it: `TaskProgressChanged` names any change to what the task page reads of an attempt,
  not only its progress.
- Change: Runs journals one `TaskProgressChanged { project_id, workflow_id, version, task,
  attempt_id }`, same payload, in the write that finishes one of an attempt's check rows
  (outcome set, `crates/core-runs/src/store/checks.rs`), and in the write that changes a
  conflict's delivery or the scope it holds (`store/conflict.rs`). A recheck's rows
  (`run_recheck_results`) journal nothing here; B-210 covers them. A replay journals nothing.
  Update the core-runs contract.
- Done when: a test commits an attempt whose check finishes after the commit and reads a
  `TaskProgressChanged` for that attempt after the check's outcome is stored; a test moves a
  conflict's delivery from pending and reads one.

### B-210 · A project recheck's start and end reach the project stream

- Status: done. Decided by Mohammed on 2026-11-24. For F-a's follow-up: `lastRecheckQuery` reads
  again on these events and polls never.
- Owner: [executors design, "A project recheck runs when a plan is complete, or when
  asked"](specs/2026-11-08-executors-design.md); amend it with the two events.
- Change: Runs journals `RecheckStarted { project_id, workflow_id, recheck_id }` in the write that
  begins a batch (a completing done mark's and `recheck_project`'s; `workflow_id` the cause
  plan), and `RecheckFinished` with the same payload in the write that sets the batch's
  `finished_at`, also for a batch recovery runs again. A replayed ask journals nothing. The
  project tail carries both kinds. Update the core-runs and core-events contracts.
- Done when: a test asks for a recheck and reads `RecheckStarted` then `RecheckFinished` for that
  batch from the project stream; a test's completing done mark starts a batch and its
  `RecheckStarted` is there.

## Backend review fixes (2026-11-25)

Plan: [backend review fixes](plans/2026-11-25-backend-review-fixes.md); each entry is that plan's task, in its order. Executed inline by the controller on Mohammed's request (2026-11-25).

### B-211 · Fixed-width database times; check overlap compares instants

- Status: done (2026-11-25). Plan task 2. `Storage::now` and `Storage::format_time` write nine fraction digits; `Storage::instant` reads any width; `overlapped` compares instants; `core-grants` writes through the shared formatter. Old rows keep their width (accepted).

### B-212 · The ended-turn cursor reads only operation endings

- Status: done (2026-11-25). Plan task 8. The ended-turn query lists the four operation endings only; RecheckStarted/RecheckFinished stay on the project tail.

### B-213 · The database opens its literal filename

- Status: done (2026-11-25). Plan task 3. `Storage::open` uses `SqliteConnectOptions::filename`. The plan's Unix non-UTF-8 case is not added: the product runs on Windows and the check would only run on CI's Linux.

### B-214 · A failed recheck worker ends its batch

- Status: done (2026-11-25). Plan task 6. A failed worker ends its batch (`abandon_batch`): unfinished rows `not_run` with the error, no notice, watchers released. No retry loop (accepted risk: the person rechecks again).

### B-215 · A committed write is announced even if its caller is dropped

- Status: done (2026-11-25). Plan task 7. A drop guard armed from commit to announcement spawns one announcement on the writer when the caller is dropped or the sequence read fails. Test-only `Storage::pause_after_commit` (feature `test-support`).

### B-216 · A replayed recheck ask returns its own stored outcome

- Status: done (2026-11-25). Plan task 1. Replay decodes the stored outcome (empty is none). No schema change: `cause_command` keeps its UNIQUE (accepted risk in the plan).

### B-217 · The last recheck and its counts come from one snapshot

- Status: done (2026-11-25). Plan task 9. `last_recheck` reads the batch and its counts in one reader transaction; no race test (it would need a hook inside the store).

### B-218 · A cancelled LSP question closes its document

- Status: done (2026-11-25). Plan task 5. `Server::ask` holds an `OpenDocument` guard that sends `didClose` on close or drop, before the question lock is released.

### B-219 · LSP hierarchy work and startup waits are bounded

- Status: done in part (2026-11-25). Plan task 13. Contract verification shares one start wait across its files (`verify`). The hierarchy work cap in `codebase/src/lsp/ask.rs` is deferred to future: no observed failure, and its test needs a process-backed hierarchy server.

### B-220 · Extension import copies outside the writer

- Status: done (2026-11-25). Plan task 4. Replay check through a reader, copy (`.import-<id>` then rename) before the writer, one short metadata write; `import_bundled` clears leftover `.import-` folders at start. Accepted: a crash between the rename and the commit leaves an unread id folder.

### B-221 · Quiet-project waits use Operations' busy check

- Status: done (2026-11-25). Plan task 11. `wait_for_quiet` asks `Operations::thread_is_busy` (one row) per executor thread; Operations is passed into Runs by AppCore (Runs already depended on core-operations).

### B-222 · Supersession reads only the project's marks

- Status: done (2026-11-25). Plan task 10. `superseded_in` reads done marks only for the selected plans' versions (`json_each`), searched through the marks' primary key (EXPLAIN QUERY PLAN: SEARCH ... USING INDEX (workflow_id=?)).

### B-223 · Project state builds only what it returns

- Status: done in part (2026-11-25). Plan task 12. `bounded` keeps the first N and only counts the rest; part detail builds task lists only for the kept plans, with progress indexed by task. Not done: splitting element-state facts from stage/layout and batching per-plan reads (no measurement showed them needed).

### B-224 · A project subscription holds only the channels it reads

- Status: done (2026-11-25). Plan task 17. A project subscription holds only the committed watch; the bus, options, commands, UI signals and tasks receivers (`ThreadLive`) are taken only by a thread subscription, still before its replay. No new test: the change is which receivers exist, and the stream suites stay green.

### B-225 · Planner command construction is type-safe

- Status: done (2026-11-25). Plan task 18. `Approver::Planner(PlannerApprover)`; only `PlannerApprover::command` builds a derived command, so the `unreachable!` is gone; `Design::planner_approver` returns a `PlannerApprover`.

### B-226 · Startup validates executable paths and origin ports

- Status: todo. Plan task 19.

### B-227 · A clipped provider catalog says so

- Status: todo. Plan task 20.

### B-228 · An agent home's settings are published whole, never clobbered

- Status: todo. Plan task 21.

### B-229 · Architecture tests cover Runs and Messages

- Status: todo. Plan task 15.

