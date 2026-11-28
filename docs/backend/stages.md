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
(`docs/backend/stages.md` at `4d69977d`, 2026-11-23). PR #76 (`e7b9332`) is on `main`: the project recheck on the plan page, the
streams that replace the polls, and the backend review fixes; its tasks are in the Shipped table.
The Claude Code catalog test passes signed in (Shipped, PR #76).
PR #77 (`3b17985f`) is on `main`: the Planner's project switches (§23.9 PR 3c), its tasks in the Shipped
table. PR #78 (`261c1abe`) is on `main`: delivery links between a plan, its tasks and their roadmap
outcome (§18.3), and named external connections with an editable project scope (B-236).
PR #79 (`6680cfd8`) is on `main`: the last five backend review fixes (B-244 to B-248); the suspended
name index and its `shadows-index` crate are gone; test temp files go under `target/tmp`. Its tasks are in the Shipped table.

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
| #75 | Project state and the code record: Design's tables and screens pinned by tasks, element states, `project_state` (B-175 to B-181, B-187; evidence `docs/evidence/2026-11-21-code-record-2-live.md` at `4d69977d`); `code_where` and the Planner asking it, and the code record sent with the Planner's context (B-182 to B-186, B-188 to B-193); Codex commit recovery: the Git-write procedure and Start's readiness for an attempted task (plan `docs/backend/plans/2026-11-22-codex-commit-recovery.md` at `4d69977d`; B-194, B-195; B-196 closed 2026-11-23; evidence `docs/evidence/2026-11-22-codex-commit-recovery-live.md` at `4d69977d`); the web's state, diagrams and `code_where` (F-130 to F-136) | B-175 to B-195; F-130 to F-136 | `4d69977d` |
| #76 | The plan page's project recheck and its live panel (B-206, B-207); the project and thread streams replace the polls (B-208 to B-210); a hold reports only project files (B-197); the Codex 0.171.0 protocol (B-198); a Codex conversation in `ask` commits (B-201); an interrupted recheck runs again (B-202); a slow Codex start names its reason (B-203); Shadows' Codex model list reread (B-204); the Claude Code catalog test passes, no code change (B-205); the backend review fixes (B-211 to B-229; plan `docs/backend/plans/2026-11-25-backend-review-fixes.md` at `e7b9332`; its deferred tasks shipped in #79, `6680cfd8`) | B-197, B-198, B-200 to B-229; F-137 to F-144 | `e7b9332` |
| #77 | The Planner's project switches (§23.9 PR 3c; plan `docs/backend/plans/2026-11-25-planner-project-switches.md` at `3b17985f`; accepted on Claude Code through `shadows serve` and the browser on 2026-11-25, `docs/evidence/2026-11-25-planner-switches-probe.md` at `3b17985f`) | B-230 to B-235; F-145 to F-148 | `3b17985f` |
| #78 | Named external connections with an editable project scope (plan `docs/backend/plans/2026-11-26-external-connection-management.md` at `261c1abe`); delivery links between a plan, its tasks and its roadmap outcome (§18.3; plan `docs/backend/plans/2026-11-26-delivery-traceability.md` at `261c1abe`; live run `docs/evidence/2026-11-26-delivery-traceability-live.md`) | B-236 to B-243; F-149 to F-152 | `261c1abe` |
| #79 | The last five backend review fixes: a call-hierarchy question bounded by one deadline and a root cap (B-244); the map's parents indexed once during validation (B-245); usage rows folded as SQLite returns them (B-246); one table for the crate dependency rules (B-247); the suspended name index and the `shadows-index` crate removed, migration `0082` drops `code_tag` (B-248). Also: test temp files under `target/tmp`; `fake-acp` and `fake-codex` built once per nextest run; `.claude/settings.json` enables the Rust and TypeScript language-server plugins | B-244 to B-248 | `6680cfd8` |

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
- A reviewer's denial of a Codex Git approval is not observed: `ask` routes every approval to Codex's `auto_review`, so a person cannot reject one (`docs/evidence/2026-11-23-codex-commit-recovery-live.md`; `docs/backend/stages.md` at `4d69977d`, B-196).
- The live check of a Codex conversation in `ask` (two Codex executors commit first time, `testing8`, 2026-11-24) is recorded in `docs/evidence/2026-11-23-testing8-go-python-live.md`; a full executor run on `testing8` is not yet repeated (`docs/backend/stages.md` at `e7b9332`, B-201).
- The executor menu in a browser offers `gpt-6.2-sol` (seen by Mohammed, 2026-11-28).
- Whether a model switch refused with a 403 (quota out) should skip that one model in the Claude Code catalog probe is open (`docs/backend/stages.md` at `e7b9332`, B-205).
- Project state keeps element-state facts beside stage and layout, and reads per plan; no measurement asked for the split or the batching (`docs/backend/stages.md` at `e7b9332`, B-223).
- Whether the provider form says a clipped catalog was clipped is Masada's decision; the field is on the wire (`docs/backend/stages.md` at `e7b9332`, B-227).
- The Planner's project switches (§23.9 PR 3c) left out, on purpose: a live session picking up a switch change without reopening; `CLAUDE.md` reaching a resumed or forked conversation; a skills folder left by a daemon that stopped with a session open, which stays until that thread opens again (plan `docs/backend/plans/2026-11-25-planner-project-switches.md` at `3b17985f`, Mohammed, 2026-11-25).
- The automated `planner_switches_live.rs` of B-235 is not written: the container had no `API_KEY_MINIMAX`. The acceptance is the browser run on Claude Code (`docs/evidence/2026-11-25-planner-switches-probe.md` at `3b17985f`).
- A broad write scope such as `todo/**` does not reach a nested part: the task shows under its root part only, not under a nested part it could write (`docs/evidence/2026-11-26-delivery-traceability-live.md`, finding 3).
- State reads each linked outcome whole to show its title; a title-only Design read would be cheaper (B-241).
- Of the five findings of B-242 (`docs/evidence/2026-11-26-delivery-traceability-live.md`, "Findings"), two were fixed before #78 merged: the Approve bar no longer says "Nothing blocks approval" for a plan missing its links (`40ca34be`), and the last-link refusal names the plan by its title (`7a6e3af2`). Open: the broad scope above, and an outcome linked only to pass the gate (the Planner's prompt asks against it; nothing stops a person). The fifth, a `**` scope shown unmapped, is the design.
- Outcome acceptance is not recorded by Shadows: no owner decides an outcome's acceptance items ([§18.3 OPEN](../core/specs/2026-10-03-project-planning-workspace-design.md#183-roadmap-outcomes), 2026-11-26; [future](../core/future.md)).
- The crate dependency table (`docs/backend/stages.md` at `6680cfd8`) holds production and build edges only: a dev-only edge between service crates is no longer checked, where the old `Cargo.lock` tests held it too. A test fake (`fake-acp`, `fake-codex`) is refused as a production edge.

## Open

B-244 to B-248 are done and shipped in PR #79 (`6680cfd8`), so the backend review fixes have no task
left; their entries are in `docs/backend/stages.md` at `6680cfd8`. B-199 is open, paused: the
`testing8` run is finished and its findings below remain. The tasks that PRs #75 to #79
delivered are in the Shipped table, and their open remainders are in Known and left.

### B-199 · Finish the testing8 live run, and the findings it left

- Status: the run is finished (2026-11-23): all 14 tasks judged done, both programs run end to
  end on one data file (Go 8 packages pass, pytest 39 passed). The findings below remain.
- Findings, not fixed ([evidence](../evidence/2026-11-23-testing8-go-python-live.md)):
  - Codex's sandbox cannot run the WindowsApps `python` alias, so executors report Python checks
    `not_run` (Shadows' own checks pass them). Accepted as is (Mohammed, 2026-11-28): it is the
    machine's setup, not Shadows. The alias is Python Install Manager's (or the Store's) app
    execution alias; the shims in `AppData\Local\Python\bin` come later in PATH. A project with its
    own venv is not affected: `.venv\Scripts\python.exe` is a plain program inside the project.
    Shadows does not look for a project's `.venv`; a check uses it only when its command names it.
    Not yet seen: whether the Planner writes a venv's path into a check when the project has one.
  - Every Codex executor's first `git add` is denied on `index.lock`, then approved: one extra
    round per task. Fixed in PR #76 (`e7b9332`).
  - A Codex start slower than `setup_wait` (20 s) because its model-list refresh timed out fails
    to open, and the reason is only in Codex's own log. Fixed in PR #76 (`e7b9332`).
  - Shadows' Codex model list did not offer `gpt-6.2-sol`. Fixed in PR #76 (`e7b9332`).
  - `live_the_claude_code_catalog_lists_each_models_efforts` failed; not examined. Passes signed in
    in PR #76 (`e7b9332`); the 403 question is in Known and left.
  - pyright is not installed on this machine (each Python contract is `unchecked`).
  - Codex's sandbox could not copy a fixture into its temporary folder (`PermissionError`), so an
    executor's subprocess test did not run there.
  - The sandbox findings are Codex's only: a Claude Code executor on the same project ran pytest,
    copied into a temp folder and committed with no denial (`6a662f5`).
  - The Planner counted 15 tasks; the plans hold 14.
- Done when: each finding is fixed, accepted or moved to future.

## External agents like the Planner (B-249 to B-262)

Spec: [external connections, "An external agent works like the Planner"](specs/2026-11-18-external-agent-connections-design.md#an-external-agent-works-like-the-planner-mohammed-2026-11-28)
and [§24.2](../core/specs/2026-10-08-approval-authority-design.md) (an external agent never
approves). Every decision there is Mohammed's (2026-11-28). The tasks are in build order; this
section is the plan. Shared request:
[`2026-11-28-external-agents-like-the-planner.md`](../shared/requests/2026-11-28-external-agents-like-the-planner.md);
a task that changes what the web sees stays `blocked` until it is `accepted`.

Rules for every task here:
- An external agent is a `GrantKind::Project` grant, the one its connection selects for the
  call's `project_id` (`Grants::external_project`). It proposes, writes plans and runs tasks;
  it never approves: `vision_approve`, `map_approve`, `roadmap_approve`, `plan_approve` and
  `Design::planner_approver` keep refusing it.
- Where a Planner call records `principal_kind: "Thread"` with its thread id, an external call
  records `principal_kind: "Grant"` with its grant id (as `Writer::External` already does).
- Scopes, checks and the executor's own approvals do not change for an external caller.
- No Planner takes over an externally written plan, and no notice goes to any thread for it.
- Live tests follow the live rules at the top of "Now"; a `codex` live test needs Codex signed in.

Not owned, so left out:
- `project_change_note` records a `ProjectChange` entry in the Planner's thread
  (`Turns::note_change`). An external agent has no thread, and no service owns where its note
  would go. It stays out of the external inventory until the controller names an owner.
- Questions an executor asks a plan's author (`task_question`) for an external plan are not in
  the spec; they keep today's path.

### B-249 · Runs accepts an external agent's grant on the Planner's task calls

- Status: todo
- Owner: `Runs` (`crates/core-runs`), per `AppCore`.
- Read: `crates/core-runs/contract.yaml`; `crates/core-runs/src/planner.rs` (`planner_plan`,
  `start_status_for`, `start_for`, `recheck_project_for`, `mark_done_for`, `live_for`),
  `init.rs` (`init_git`), `stop.rs` (`stop_for`, `Stopper`), `message.rs` (`message_agent`);
  the rules above.
- Interfaces: no new public method. `planner_plan` accepts `GrantKind::Thread` and
  `GrantKind::Project` whose `project_id` is the plan's; `GrantKind::Executor` stays
  `NotFound`. `init_git` accepts both the same way. `stop_for` records
  `principal_kind: "Grant"`, `principal_id: <grant id>` for a project grant and stops with a new
  private `Stopper::External`. `recheck_project_for` passes `None` as the notice thread for a
  project grant. Executor choice is B-252, not here.
- Touch: `crates/core-runs/src/planner.rs`, `init.rs`, `stop.rs`, `message.rs` (only if its
  grant check needs it), `crates/core-runs/contract.yaml`, new
  `crates/shadows-core/tests/external_runs.rs`.
- Needs: nothing.
- Done when: `cargo test -p shadows-core --test external_runs` passes with
  `an_external_grant_starts_and_marks_a_task_of_its_project` (start answers the attempt;
  `mark_done_for` answers the next group), `an_external_grant_of_another_project_is_refused`
  (`NotFound`), `an_external_stop_is_recorded_under_its_grant` (the stop's command record has
  principal `Grant` and the grant id), `an_executor_grant_is_still_refused` (`NotFound`); and
  the existing `cargo test -p shadows-core --test runs_attempts --test run_admission` pass.

### B-250 · Design keeps vision, map and roadmap proposals from an external agent

- Status: blocked (the shared request: a proposal's `thread_id` becomes nullable on the wire)
- Owner: `Design` (`crates/core-design`).
- Read: `crates/core-design/contract.yaml`; `proposal_context.rs` (`proposal_writer`,
  `proposal_command`), `proposals.rs`, `map_proposals.rs`, `roadmap_proposals.rs` and their
  stores; migrations `0022`, `0023`, `0026`; the test
  `the_planner_proposes_vision_cards_but_an_external_agent_cannot` in
  `crates/shadows/tests/workspace_mcp.rs`.
- Interfaces: `proposal_writer` answers `(Writer, Option<OperationId>)`: a Planner keeps its
  running turn (refused with no turn, as now); an external grant has none. For an external
  grant the command id is the caller's: `VisionPropose`, `MapPropose` and `RoadmapPropose`
  gain `command_id: Option<String>`, required for an external grant ("an external agent names
  its command_id"), ignored for the Planner. A proposal row from an external grant has
  `thread_id`, `operation_id` and `entry_id` NULL and a new `grant_id` set; `VisionProposal`,
  `MapProposal` and `RoadmapProposal` serialize `thread_id: Option<ThreadId>` and
  `grant_id: Option<GrantId>`. The person approves it through the existing project routes;
  approving it writes no thread entry.
- Touch: the four files above and their store files; new migration
  `crates/core-db/migrations/0083_external_design_proposals.sql` (rebuilds the three proposal
  tables: `thread_id`, `operation_id`, `entry_id` nullable, `grant_id TEXT NULL REFERENCES
  mcp_grant(id)`, `CHECK ((thread_id IS NULL) <> (grant_id IS NULL))`, rows copied);
  `crates/core-design/contract.yaml`; `crates/shadows/tests/architecture/table-owners.txt` if
  it names columns; `crates/shadows/tests/workspace_mcp.rs`; `api/openapi.json` (regenerated).
- Needs: the request `accepted`.
- Done when: `cargo test -p shadows --test workspace_mcp --test vision_proposal_ids --test
  map_proposal_coverage --test design_routes` pass, with
  `the_planner_proposes_vision_cards_but_an_external_agent_cannot` renamed to
  `an_external_agent_proposes_vision_cards_but_cannot_approve` (propose answers `Pending` with
  `thread_id: null`; `vision_approve` is refused),
  `an_external_proposal_needs_its_command_id`, `an_external_proposal_replays_on_its_command_id`
  (same id, same proposal), `the_person_approves_an_external_map_proposal_with_no_entry`; and
  `cargo test -p shadows-core --test design_migration` passes over the new migration.

### B-251 · Design keeps decisions and design elements from an external agent

- Status: blocked (needs B-250)
- Owner: `Design`.
- Read: `crates/core-design/contract.yaml`; `decisions.rs`, `elements.rs` and their stores;
  migrations `0071`, `0077` (both keep the writer as text, so no migration).
- Interfaces: `decision_propose`, `design_element_put` and `design_element_delete` accept an
  external grant; its command id is the caller's `command_id` (a new `Option<String>` on their
  argument types, required for an external grant); `proposed_by` / `written_by` hold the
  principal `Writer::External` gives. Accepting a decision stays the person's.
- Touch: `crates/core-design/src/decisions.rs`, `elements.rs`, their argument types and store
  files, `crates/core-design/contract.yaml`, `crates/shadows-core/tests/design_decisions.rs`,
  `crates/shadows-core/tests/design_elements.rs`.
- Needs: B-250 (it changes `proposal_writer`).
- Done when: `cargo test -p shadows-core --test design_decisions --test design_elements` pass
  with `an_external_agent_proposes_a_decision_the_person_accepts`,
  `an_external_agent_puts_and_deletes_an_element`, `an_external_write_needs_its_command_id`.

### B-252 · Runs records an executor chosen by an external agent

- Status: blocked (the shared request: `Chooser` gains `external`)
- Owner: `Runs`.
- Read: `crates/core-runs/src/executor_choice.rs` (`save_executor`),
  `crates/core-runs/src/store/executor_choice.rs`, `model.rs` (`Chooser`).
- Interfaces: `Chooser::External` (`"external"` on the wire). A project grant's choice records
  `Actor { kind: "Grant", id: <grant id> }` and `by: "external"`; the project check stays
  `grant.project_id == plan.project_id`.
- Touch: `crates/core-runs/src/model.rs`, `executor_choice.rs`, `store/executor_choice.rs`,
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_runs.rs`,
  `api/openapi.json` (regenerated).
- Needs: B-249; the request `accepted`.
- Done when: `cargo test -p shadows-core --test external_runs` passes with
  `an_external_agent_sets_a_tasks_executor` (`chosen_by == Chooser::External`), and
  `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` changes only `Chooser`.

### B-253 · The external inventory grows to the Planner's, less approval and conversation tools

- Status: blocked (needs B-249 to B-252)
- Owner: `Grants` (`crates/core-grants/src/tools.rs`); the MCP adapter only routes.
- Read: `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/shadows-mcp/src/external_tools.rs` (`external_schema`, `take_project`),
  `crates/shadows-mcp/src/server.rs`, `an_external_connection_only_reaches_its_chosen_projects`
  in `crates/shadows/tests/mcp_server.rs`.
- Interfaces: `PROJECT_TOOLS` is today's 14 plus the spec's two granted groups, less
  `project_change_note` (not owned, above): `vision_propose`, `map_propose`, `roadmap_propose`,
  `decision_propose`, `decision_list`, `design_element_put`, `design_element_delete`,
  `design_elements`, `project_state`, `code_where`, `project_files`, `project_refresh`,
  `task_ready`, `task_runs`, `task_start`, `task_start_status`, `task_executor_set`,
  `executor_catalog`, `task_stop`, `task_done`, `task_amend`, `task_lessons`, `task_diff`,
  `task_file`, `task_checks`, `project_recheck`, `project_git_init`, `message_agent` (42 in
  all). Never: the four `*_approve`, `plan_show`, `person_ask`. Every external tool but
  `project_list` takes `project_id` (as `external_schema` already does); `executor_catalog`
  takes it too, for one rule.
- Touch: `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/shadows-mcp/src/external_tools.rs` (descriptions only, where a Planner wording says
  "your conversation"), `crates/shadows/tests/mcp_server.rs`.
- Needs: B-249, B-250, B-251, B-252.
- Done when: `cargo test -p shadows --test mcp_server --test plan_in_conversation` pass with
  `the_external_inventory_is_the_planners_less_approval_and_conversation_tools` (the
  connection's `tools/list` is exactly `project_list` plus the 42 names, each but
  `project_list` requiring `project_id`; none of `vision_approve`, `map_approve`,
  `roadmap_approve`, `plan_approve`, `plan_show`, `person_ask`, `project_change_note`), and
  `an_external_agent_starts_a_task_over_mcp` (vision, map and roadmap proposed over MCP and
  approved by the person through the routes; then `draft_start` no longer answers
  `STAGE_BLOCKED`; then `task_start` answers an attempt).

### B-254 · Secrets keeps a plan author's reach token

- Status: todo
- Owner: `Secrets` (`crates/core-secrets`).
- Read: `crates/core-secrets/contract.yaml` (the `mcp/{id}` rules: a value never in the
  database in clear, a command record or a log).
- Interfaces: `Secrets::set_reach_token(&self, plan: &str, token: &str) -> Result<(),
  SecretsError>`, `Secrets::reach_token(&self, plan: &str) -> Result<Option<String>,
  SecretsError>`, `Secrets::remove_reach_token(&self, plan: &str) -> Result<(), SecretsError>`;
  kept under `reach/{plan}`, sealed as `mcp/{id}` is; a set replaces.
- Touch: `crates/core-secrets/src/lib.rs` (or a new `src/reach.rs`), its store,
  `crates/core-secrets/contract.yaml`, new `crates/core-secrets/tests/reach_token.rs`,
  `docs/codebase/README.md` if a file is added.
- Needs: nothing.
- Done when: `cargo test -p core-secrets --test reach_token` passes with
  `a_reach_token_round_trips_and_a_set_replaces`, `a_removed_reach_token_reads_none`,
  `a_reach_token_is_not_stored_in_clear` (the row's bytes do not contain the token).

### B-255 · `draft_start` takes the author's reach and the plan keeps it

- Status: todo
- Owner: `Plans` (`crates/core-plans`); the token through `Secrets`.
- Read: `crates/core-plans/contract.yaml`; `scope.rs` (`external_draft`), `lib.rs`
  (`start_draft`), `model.rs` (`WrittenBy`, `DraftStart`); `app.rs` where `Plans` is built;
  migration `0012` (`workflow.written_by_*`).
- Interfaces: `DraftStart.reach: Option<ReachInput>`, `ReachInput::CodexAppServer { address:
  String, token: String, thread_id: String }` (`kind: "codex_app_server"` in JSON). Accepted
  only from a project grant; a Planner grant with a reach is `InvalidCommand` ("the Planner is
  reached in its conversation"). `address` must be `ws://` with a loopback IP literal
  (`127.0.0.0/8` or `[::1]`) and a port, else `InvalidCommand` ("only a loopback ws:// address
  is accepted"). `address` and `thread_id` are kept on the new version
  (`workflow.written_by_reach TEXT NULL`, JSON), the token by `Secrets::set_reach_token(plan_id,
  …)`; the token enters no stored command params, plan content or log (`ReachInput`'s `Debug`
  hides it). A version without a reach keeps none.
  `Plans::author_reach(&self, plan: &PlanId) -> Result<Option<AuthorReach>, PlansError>` reads
  the latest version's reach: `AuthorReach::CodexAppServer { address, thread_id, token }`, the
  token from `Secrets::reach_token`; `None` for a Planner's plan or no reach. `Plans::new` gains
  `secrets: Arc<Secrets>`.
- Touch: `crates/core-plans/src/model.rs`, `scope.rs`, `lib.rs`, a new
  `crates/core-plans/src/reach.rs`, its store, `crates/core-plans/Cargo.toml` (`core-secrets`),
  `crates/core-plans/contract.yaml`, new migration
  `crates/core-db/migrations/0084_plan_author_reach.sql`, `crates/shadows-core/src/app.rs` (the
  `Plans::new` calls), `crates/shadows/tests/plan_grants.rs`, `docs/codebase/README.md`,
  `crates/shadows/tests/architecture/table-owners.txt` if it names columns.
- Needs: B-254.
- Done when: `cargo test -p shadows --test plan_grants` passes with
  `an_external_start_keeps_its_reach_and_the_latest_version_wins` (two versions, two
  addresses; `author_reach` answers the second, with its token),
  `a_reach_outside_loopback_is_refused` (`ws://192.168.1.2:4500`, `ws://localhost:4500`,
  `http://127.0.0.1:4500`), `a_planner_start_with_a_reach_is_refused`,
  `the_reach_token_is_not_in_plan_content` (`workflow_get` and the stored command params do not
  contain the token).

### B-256 · A second client wakes a Codex thread on the person's app-server

- Status: todo
- Owner: `shadows-agent` (the AI subprocess harness; it already speaks the Codex app-server
  protocol in `src/codex`). Not a Core service: B-257 calls it from `Runs`.
- Read: `crates/shadows-agent/src/codex/protocol.rs` (`thread/resume`, `turn/start`),
  `connection.rs` (`Rpc`), `session.rs` (`start_session`: `initialize` and resume),
  the `shadows-agent` row of `docs/codebase/README.md`, the spec's "Where the plan's author is
  reached".
- Interfaces: `pub struct CodexRemote;` with `pub async fn wake(address: &str, token: &str,
  thread: &str, text: &str, deadline: Duration) -> Result<(), ConnectionError>`: one WebSocket
  connection carrying the capability token on the upgrade request (as Codex 0.171.0's
  `--ws-auth capability-token` reads it; the builder records the header in the code map row),
  then `initialize`, `thread/resume { threadId }`, `turn/start { threadId, input: [text] }`,
  answered once `turn/start` is accepted, then closed. A refused token, a missing thread or the
  deadline is a `ConnectionError` naming which. New dependency `tokio-tungstenite`
  (infrastructure).
- Touch: new `crates/shadows-agent/src/codex/remote.rs`, `crates/shadows-agent/src/codex.rs`
  (`mod` and re-export), `crates/shadows-agent/Cargo.toml`, root `Cargo.toml` (workspace
  dependency), `docs/codebase/README.md`, new `crates/shadows-agent/tests/codex_remote_live.rs`.
- Needs: nothing.
- Done when: `cargo test -p shadows-agent --test codex_remote_live -- --ignored` passes signed
  in to Codex with `live_a_second_client_starts_a_turn_on_a_codex_thread` (the test starts
  `codex app-server --listen ws://127.0.0.1:<free port> --ws-auth capability-token
  --ws-token-file <tmp>`, a first client starts a thread, `wake` answers `Ok`, and the first
  client sees `turn/started` on that thread with the text) and `live_a_wrong_token_is_refused`.

### B-257 · Runs tells an external author how an attempt ended

- Status: todo
- Owner: `Runs` (`notify.rs`: telling a plan's author how an attempt ended).
- Read: `crates/core-runs/contract.yaml` (notices), `crates/core-runs/src/notify.rs`
  (`Recipient`, `NoticeIntent`, the conflict notice), `checks.rs` (where an attempt's checks
  finish); B-255's `Plans::author_reach`; B-256's `CodexRemote::wake`.
- Interfaces: when an attempt of a plan whose latest version is `WrittenBy::External` has ended
  and its checks have run, or its executor reports a conflict, Runs saves one
  `run_author_notice` row with the body the Planner's notice would carry (one builder of that
  body for both), in the write that ends the attempt or records the conflict. Then, if
  `author_reach` is `CodexAppServer`, it calls `CodexRemote::wake` with that body (deadline
  20 s); `Ok` marks the row told (`told_by = 'wake'`). A failed wake is logged without the token
  and leaves the row untold. A Planner's plan keeps today's thread notice and writes no row.
  A private `tokio::sync::Notify` in Runs fires after each new row commits (B-258 waits on it).
- Touch: `crates/core-runs/src/notify.rs`, new `crates/core-runs/src/author.rs`, new
  `crates/core-runs/src/store/author.rs`, `crates/core-runs/src/lib.rs` (the `Notify`), new
  migration `crates/core-db/migrations/0085_run_author_notice.sql` (`attempt_id`, `kind`
  (`ended` or `conflict`), `project_id`, `grant_id`, `workflow_id`, `task`, `body`,
  `created_at`, `told_at NULL`, `told_by NULL` (`wake` or `wait`); primary key
  `(attempt_id, kind)`), `crates/shadows/tests/architecture/table-owners.txt`,
  `crates/core-runs/contract.yaml`, `docs/codebase/README.md`, new
  `crates/shadows-core/tests/external_author.rs`.
- Needs: B-249, B-255, B-256.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `an_ended_attempt_of_an_external_plan_is_kept_for_its_author` (one untold row, no thread
  entry), `a_planner_plan_keeps_its_thread_notice` (no row),
  `a_failed_wake_leaves_the_notice_untold` (a reach to a closed loopback port),
  `a_repeated_end_keeps_one_notice`.

### B-258 · One wait for an external author

- Status: todo
- Owner: `Runs`.
- Read: B-257's rows and `Notify`; the spec's "Waiting for a task".
- Interfaces: `Runs::author_wait(&self, grant: &Grant, bound: Duration) -> Result<AuthorWait,
  RunsError>` for a project grant (others `NotFound`). It first claims every untold row of the
  plans that grant wrote (`told_by = 'wait'`, in one write, so two waiters never get one row)
  and answers them at once as `Ended`. With none, if no attempt of those plans is running it
  answers `NoneRunning` at once; else it waits on the `Notify` until a row comes (claimed,
  `Ended`) or `bound` passes (`NothingYet`). `AuthorWait { outcome: WaitOutcome, reports:
  Vec<AuthorReport> }`, `WaitOutcome { Ended, NothingYet, NoneRunning }` (snake_case),
  `AuthorReport { attempt_id, workflow_id, task: u32, kind, report: String }`, oldest first.
- Touch: `crates/core-runs/src/author.rs`, `crates/core-runs/src/store/author.rs`,
  `crates/core-runs/src/model.rs`, `crates/core-runs/src/lib.rs` (re-exports),
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`.
- Needs: B-257.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `a_wait_answers_at_once_with_notices_left_while_no_one_waited`,
  `a_wait_with_nothing_running_answers_none_running`, `a_wait_ends_when_an_attempt_ends`,
  `a_wait_passes_its_bound_with_nothing_yet`, `two_waiters_never_get_one_notice_twice`,
  `a_woken_notice_is_not_answered_again`.

### B-259 · `task_wait` over MCP

- Status: todo
- Owner: `Runs` (the method), `Grants` (the inventory); the MCP adapter translates.
- Read: `crates/shadows-mcp/src/run_tools.rs` (a tool calling `self.core.runs()`),
  `crates/core-grants/src/tools.rs`.
- Interfaces: tool `task_wait`, args `{ seconds?: u32 }` (default 45; a larger value is held
  to 50, so Codex's `tool_timeout_sec` of about 60 never cuts it), calls
  `runs().author_wait(&grant, …)`, answers `AuthorWait`. In `PROJECT_TOOLS` only, never in
  `THREAD_TOOLS`. Its description says it answers at once when nothing runs.
- Touch: `crates/shadows-mcp/src/run_tools.rs`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`, `crates/shadows/tests/mcp_server.rs`.
- Needs: B-253, B-258.
- Done when: `cargo test -p shadows --test mcp_server` passes with
  `task_wait_is_an_external_tool_bounded_under_a_minute` (listed for a connection, not for a
  Planner; `seconds: 120` answers `nothing_yet` within 51 s while an attempt runs) and
  `the_external_inventory_is_the_planners_less_approval_and_conversation_tools` updated to 43.

### B-260 · The wait over HTTP with the connection's bearer, and its address in Settings

- Status: blocked (the shared request)
- Owner: `Runs` (the method), `Grants` (bearer, project and `wait_url`); `shadows-http`
  translates.
- Read: `crates/shadows-http/src/grants.rs`, `crates/shadows-mcp/src/auth.rs` (how `/mcp` turns
  a bearer into a connection), `Grants::authorize_external`, `Grants::external_project`,
  `crates/shadows-http/contract.yaml`.
- Interfaces: `POST /api/external/task-wait`, `Authorization: Bearer <connection token>`, body
  `{ project_id: string, seconds?: u32 }` (default 300; a larger value is held to 540, under
  Codex's 600 s hook timeout) → `AuthorWait`. A missing or revoked bearer is 401; a project not
  on the connection 404. No browser origin needed. `ExternalConnection` and the issue response
  gain `wait_url: string` (`http://<bound address>/api/external/task-wait`), built by Grants
  beside `mcp_url`.
- Touch: new `crates/shadows-http/src/external_wait.rs`, `crates/shadows-http/src/lib.rs`
  (route), `crates/shadows-http/contract.yaml`, `crates/core-grants/src/model.rs`,
  `crates/core-grants/src/external.rs`, `crates/core-grants/contract.yaml`, `api/openapi.json`
  (regenerated), `crates/shadows/tests/grants.rs`, `docs/codebase/README.md`.
- Needs: B-258; the request `accepted`.
- Done when: `cargo test -p shadows --test grants --test openapi` pass with
  `the_http_wait_answers_with_the_connections_bearer` (an ended attempt's report),
  `the_http_wait_refuses_a_revoked_bearer` (401), `the_http_wait_refuses_an_unselected_project`
  (404), `a_connection_names_its_wait_url`.

### B-261 · A task run says its result waits for its author

- Status: blocked (the shared request)
- Owner: `Runs`.
- Read: the `TaskRun` view in `crates/core-runs/src/model.rs`; B-257's rows.
- Interfaces: `TaskRun.author_waiting: bool`, true while the task's latest attempt has an
  untold `run_author_notice` row. A `TaskProgressChanged` is journaled when a row is saved and
  when it is told, so the project stream refreshes the page.
- Touch: `crates/core-runs/src/model.rs`, the view's store read, `crates/core-runs/src/author.rs`,
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`,
  `api/openapi.json` (regenerated).
- Needs: B-258; the request `accepted`.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `a_task_run_shows_its_result_waiting_for_its_author` (true after the end, false after a wait
  claims it).

### B-262 · Acceptance: an external agent takes a project from idea to done tasks

- Status: blocked (needs B-249 to B-261 and the web side of the request)
- Owner: the controller with Mohammed; no code.
- Run, on a fresh `testing` project through `shadows serve` and the browser:
  1. Claude Code connected from Settings with its `asyncRewake` Stop hook: it proposes vision,
     map and roadmap; Mohammed approves each with the button; it writes and starts a plan; the
     hook wakes it with each attempt's report and it judges it.
  2. Codex with `codex --remote` on `codex app-server --listen ws://127.0.0.1:<port> --ws-auth
     capability-token --ws-token-file <file>`: its `draft_start` carries the reach; Shadows
     starts a turn on its screen when an attempt ends. Check that Codex can name its own thread
     id for the reach; if it cannot, stop and report it for a ruling.
  3. Codex Desktop with its Stop hook: busy while it waits, answered with the report.
  4. An attempt that ends with no one waiting shows as waiting for its author; the next wait
     answers it at once.
- Done when: `docs/evidence/<date>-external-agents-like-the-planner-live.md` records the four
  runs and their findings.

