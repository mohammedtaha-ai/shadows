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
accepted by Masada on 2026-11-28 (`c53ea0df`), with the two additions folded into B-250 and B-261.

Rules for every task here:
- An external agent is a `GrantKind::Project` grant, the one its connection selects for the
  call's `project_id` (`Grants::external_project`). It proposes, writes plans and runs tasks;
  it never approves: `vision_approve`, `map_approve`, `roadmap_approve`, `plan_approve` and
  `Design::planner_approver` keep refusing it.
- Where a Planner call records `principal_kind: "Thread"` with its thread id, an external call
  records `principal_kind: "Grant"` with its grant id (as `Writer::External` already does).
- Scopes, checks and the executor's own approvals do not change for an external caller.
- No Planner takes over an externally written plan, and no notice goes to any thread for it.
- Shadows never wakes an external agent and keeps no address for it; the agent waits
  (`task_wait`, or its client's Stop hook on the HTTP route). The clients are Claude Code and
  the Codex desktop app (Mohammed, 2026-11-28).

Not owned, so left out:
- `project_change_note` records a `ProjectChange` entry in the Planner's thread
  (`Turns::note_change`). An external agent has no thread, and no service owns where its note
  would go. It stays out of the external inventory until the controller names an owner.
- Questions an executor asks a plan's author (`task_question`) for an external plan are not in
  the spec; they keep today's path.

### B-249 · Runs accepts an external agent's grant on the Planner's task calls

- Status: done
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
  `crates/shadows-core/tests/external_runs.rs`, and the executor-grant regression in
  `crates/shadows-core/tests/runs_attempts/planner.rs`.
- Needs: nothing.
- Done when: `cargo test -p shadows-core --test external_runs` passes with
  `an_external_grant_starts_and_marks_a_task_of_its_project` (start answers the attempt;
  `mark_done_for` answers the next group), `an_external_grant_of_another_project_is_refused`
  (`NotFound`), `an_external_stop_is_recorded_under_its_grant` (the stop's command record has
  principal `Grant` and the grant id), `an_executor_grant_is_still_refused` (`NotFound`); and
  the existing `cargo test -p shadows-core --test runs_attempts --test run_admission` pass.

### B-250 · Design keeps vision, map and roadmap proposals from an external agent

- Status: done
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
  `MapProposal` and `RoadmapProposal` serialize `thread_id: Option<ThreadId>`,
  `grant_id: Option<GrantId>` and `connection_id: Option<String>`: the external connection
  whose scope grant proposed it, `None` for a Planner's proposal (Masada's addition, accepted
  request). Design reads it through a new transaction-local Grants read,
  `Grants::connections_of_in(<the same connection type as check_live_in>, grants: &[GrantId])
  -> Result<HashMap<GrantId, String>, GrantsError>` over `external_connection_project`, inside
  the proposals' list read; a grant whose connection row is gone maps to nothing, so
  `connection_id` is `None`. `VisionProposed`, `MapProposed` and `RoadmapProposed` are
  journaled for an external proposal as for the Planner's, with its project and
  `thread_id: null`, so they stay on the project stream. The person approves it through the
  existing project routes; approving it writes no thread entry.
- Touch: the four files above and their store files; new migration
  `crates/core-db/migrations/0084_external_design_proposals.sql` (rebuilds the three proposal
  tables: `thread_id`, `operation_id`, `entry_id` nullable, `grant_id TEXT NULL REFERENCES
  mcp_grant(id)`, `CHECK ((thread_id IS NULL) <> (grant_id IS NULL))`, rows copied);
  `crates/core-design/contract.yaml`; `crates/core-grants/src/lib.rs` (or `external.rs`) and
  its store for `connections_of_in`, `crates/core-grants/contract.yaml` (its
  `called_by_other_services` line); `crates/shadows/tests/architecture/table-owners.txt` if
  it names columns; `crates/shadows/tests/workspace_mcp.rs`; `api/openapi.json` (regenerated).
  The proposal context has a persistence module and code-map row. The MCP vision adapter
  forwards the command id. Existing constructors in Rust tests supply no command id for
  Planner calls. B-253 registers the external tools.
- Needs: nothing (the request is accepted).
- Done when: `cargo test -p shadows --test workspace_mcp --test vision_proposal_ids --test
  map_proposal_coverage --test design_routes` pass, with
  `the_planner_proposes_vision_cards_but_an_external_agent_cannot` renamed to
  `an_external_agent_proposes_vision_cards_but_cannot_approve` (propose answers `Pending` with
  `thread_id: null`; `vision_approve` is refused),
  `an_external_proposal_needs_its_command_id`, `an_external_proposal_replays_on_its_command_id`
  (same id, same proposal), `the_person_approves_an_external_map_proposal_with_no_entry`,
  `an_external_proposal_names_its_connection` (`connection_id` is the issuing connection's id;
  a Planner's proposal has `null`), `an_external_proposal_is_on_the_project_stream`
  (`VisionProposed` reaches the project tail with `thread_id: null`); and
  `cargo test -p shadows-core --test design_migration` passes over the new migration.

### B-251 · Design keeps decisions and design elements from an external agent

- Status: done
- Owner: `Design`.
- Read: `crates/core-design/contract.yaml`; `decisions.rs`, `elements.rs` and their stores;
  migrations `0071`, `0077` (both keep the writer as text, so no migration).
- Interfaces: `decision_propose`, `design_element_put` and `design_element_delete` accept an
  external grant. Their arguments already require `command_id: String`, so no new field: an
  external grant's call needs it nonempty, and it is the command id as given. `proposed_by` /
  `written_by` hold the principal `Writer::External` gives. Accepting a decision stays the
  person's. The Planner's optional turn provenance comes from the one proposal context
  (`proposal_context.rs`, `store/proposal_origin.rs`) that B-250's proposals use, not a second
  copy.
- Touch: `crates/core-design/src/decisions.rs`, `elements.rs`, `proposal_context.rs`,
  `crates/core-design/src/store/decisions.rs`, `store/elements.rs`, `store/proposal_origin.rs`,
  `store/proposals.rs`, `store/map_proposals.rs`, `store/roadmap_proposals.rs`,
  `crates/core-design/contract.yaml`, `crates/shadows-core/tests/design_decisions.rs`,
  `crates/shadows-core/tests/design_elements.rs`. The uncommitted hunks in
  `crates/shadows-core/tests/external_runs.rs` (B-252's test) and
  `crates/shadows/tests/mcp_server.rs` (B-253's test) are not this task's: leave them
  uncommitted for B-252 and B-253.
- Needs: B-250 (it changes `proposal_writer`).
- Done when: `cargo test -p shadows-core --test design_decisions --test design_elements` pass
  with `an_external_agent_proposes_a_decision_the_person_accepts`,
  `an_external_agent_puts_and_deletes_an_element` (it also checks that an empty `command_id`
  is refused), and `cargo test -p shadows --test workspace_mcp --test vision_proposal_ids`
  still pass (the shared proposal context changed).
- Started by Codex and stopped at its limit (2026-11-28) with the work uncommitted; finished
  from that work as it stands, not restarted.

### B-252 · Runs records an executor chosen by an external agent

- Status: done
- Owner: `Runs`.
- Read: `crates/core-runs/src/executor_choice.rs` (`save_executor`),
  `crates/core-runs/src/store/executor_choice.rs` (the read maps every value but `planner` to
  `Person`), `model.rs` (`Chooser`), migration `0073_run_task_executors.sql` (`chosen_by` is
  `CHECK IN ('person','planner')`).
- Interfaces: `Chooser::External` (`"external"` on the wire and in the table). A project
  grant's choice records `Actor { kind: "Grant", id: <grant id> }` and `by: "external"`; the
  project check stays `grant.project_id == plan.project_id`. The read matches each stored value
  explicitly (`person`, `planner`, `external`); any other is a storage error, never `Person`.
- Touch: `crates/core-runs/src/model.rs`, `executor_choice.rs`, `store/executor_choice.rs`, new
  migration `crates/core-db/migrations/<next>_external_executor_chooser.sql` (rebuilds
  `run_task_executors` with `CHECK (chosen_by IN ('person','planner','external'))`, rows
  copied), `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_runs.rs`,
  `api/openapi.json` (regenerated).
- Needs: B-249.
- Done when: `cargo test -p shadows-core --test external_runs --test design_migration` pass
  with `an_external_agent_sets_a_tasks_executor` (saved, then read back as
  `Chooser::External`), `earlier_choices_read_back_unchanged_after_the_migration` (a `person`
  and a `planner` row), and `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` changes
  only `Chooser`.

### B-253 · The external inventory grows to the Planner's, less approval and conversation tools

- Status: done
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

B-254, B-255 and B-256 (the author's reach, its token and a Codex app-server wake) were dropped
before they were built: Shadows never wakes an external agent (Mohammed, 2026-11-28, spec at
`d3497952`). Their entries are in this file at `18e72f9e`.

### B-257 · Runs keeps each attempt's report for its external author

- Status: done
- Owner: `Runs` (`notify.rs`: telling a plan's author how an attempt ended).
- Read: `crates/core-runs/contract.yaml` (notices), `crates/core-runs/src/notify.rs`
  (`Recipient`, `NoticeIntent`, the conflict notice), `crates/core-runs/src/watch.rs` (a
  committed attempt, then its checks), `checks.rs` (where the checks finish).
- Interfaces: when an attempt whose own version (`attempt.workflow_id`) is
  `WrittenBy::External` has ended and its checks have run, or its executor reports a conflict,
  Runs saves one `run_author_report` row with the body the Planner's notice would carry (one
  builder of that body for both), in the write that settles the attempt's checks or records the
  conflict. The recipient is the grant that wrote the attempt's own version, kept as the row's
  `grant_id`: a later draft by another connection does not take over a running attempt's report.
  An attempt whose failed checks are returned to its executor (`continue_attempt`, origin
  `Checks`) writes no `ended` row; its follow-up attempt writes one when it settles. Its
  `report_id` is `<attempt_id>:<kind>`, stable across every answer of it. A Planner's plan keeps
  today's thread notice and writes no row. Shadows sends the row nowhere: the author waits for
  it (B-258). A private `tokio::sync::Notify` in Runs fires after each new row commits.
- Touch: `crates/core-runs/src/notify.rs`, new `crates/core-runs/src/author.rs`, new
  `crates/core-runs/src/store/author.rs`, `crates/core-runs/src/lib.rs` (the `Notify`), new
  migration `crates/core-db/migrations/0086_run_author_report.sql` (`report_id` primary key,
  `attempt_id`, `kind` (`ended` or `conflict`), `project_id`, `grant_id`, `workflow_id`, `task`,
  `body`, `created_at`, `claimed_until NULL`, `acked_at NULL`),
  `crates/shadows/tests/architecture/table-owners.txt`, `crates/core-runs/contract.yaml`,
  `docs/codebase/README.md`, new `crates/shadows-core/tests/external_author.rs`.
- Needs: B-249.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `an_ended_attempt_of_an_external_plan_is_kept_for_its_author` (one open row, no thread
  entry), `the_report_is_kept_only_after_the_checks_ran` (a slow check: no row while it runs),
  `a_planner_plan_keeps_its_thread_notice` (no row), `a_repeated_end_keeps_one_report`,
  `the_report_goes_to_the_author_of_the_attempts_own_version` (connection A writes v1 and
  starts T1; connection B starts a v2 draft; T1's row names A's grant),
  `a_check_returned_to_the_executor_reports_once_its_follow_up_settles` (one `ended` row, for
  the follow-up attempt).
- Migration number: `0086` (B-250 took `0084`, B-252 `0085`; `0083` stays unused, since a lower number after applied ones is refused).

### B-258 · One wait for an external author, answered until acknowledged

- Status: done
- Owner: `Runs`.
- Read: B-257's rows and `Notify`; the spec's "Waiting for a task".
- Interfaces: `Runs::author_wait(&self, grant: &Grant, ack: &[String], bound: Duration) ->
  Result<AuthorWait, RunsError>` for a project grant (others `NotFound`). Delivery contract:
  a report is delivered once its author acknowledges it, and `ack` names only reports the
  author was shown by an earlier answer (a hook keeps the ids it handed to its session and
  sends them on its next run; an agent sends those it read on its next `task_wait`). There is
  no acknowledge-only call. In order:
  1. In one write that first runs `Grants::check_live_in(tx, grant)`, it marks acknowledged
     (`acked_at`) the rows in `ack` whose `grant_id` is this grant (others are ignored), then
     claims every row of this grant that is open or whose claim has lapsed
     (`claimed_until = now + 120 s`), so two waiters never hold one row at once. Rows claimed
     are answered at once as `Ended`. A claimed row not acknowledged within its lease is
     answered again by a later wait, with the same `report_id`: a report is never lost to an
     answer that did not reach the session, and the author tells a repeat by its id.
  2. With none, it answers `NoneRunning` at once only when no attempt of the versions this
     grant wrote is pending and no row of this grant is held by a claim. Pending: running, or
     committed with its checks not yet run and not returned to its executor (an attempt
     continued by a follow-up is not pending; the follow-up is, while it runs).
  3. Else it waits until a new row's `Notify`, the earliest lapse of a claim held on this
     grant's rows, or `bound`, whichever comes first, then runs step 1 again (with no `ack`);
     at `bound` with nothing claimed it answers `NothingYet`.
  A grant revoked during the wait (its connection revoked, or the project removed from it) is
  refused by the claim's check (`GrantInvalid`), and the rows stay open.
  `AuthorWait { outcome: WaitOutcome, reports: Vec<AuthorReport> }`, `WaitOutcome { Ended,
  NothingYet, NoneRunning }` (snake_case), `AuthorReport { report_id: String, attempt_id,
  workflow_id, task: u32, kind, report: String }`, oldest first.
- Touch: `crates/core-runs/src/author.rs`, `crates/core-runs/src/store/author.rs`,
  `crates/core-runs/src/model.rs`, `crates/core-runs/src/lib.rs` (re-exports),
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`.
- Needs: B-257.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `a_wait_answers_at_once_with_reports_left_while_no_one_waited`,
  `a_wait_with_nothing_pending_answers_none_running`,
  `a_wait_during_slow_checks_does_not_answer_none_running` (attempt committed, its check still
  running: the wait holds and answers `Ended` once the check ends),
  `a_wait_ends_when_an_attempt_ends`, `a_wait_passes_its_bound_with_nothing_yet`,
  `two_waiters_never_hold_one_report_at_once`,
  `an_unacknowledged_report_is_answered_again_after_its_lease` (same `report_id`),
  `an_acknowledged_report_is_not_answered_again`,
  `an_ack_for_another_grants_report_is_ignored`,
  `a_wait_after_a_returned_check_answers_none_running_once_acknowledged` (a failed check
  returns T1 to its executor; the follow-up settles; a wait answers its report; the next wait
  with that `ack` answers `none_running`),
  `a_waiting_call_wakes_when_a_claim_lapses` (a report claimed by one call and never
  acknowledged; a second call already waiting answers it once the lease lapses, with no new
  row),
  `a_grant_revoked_during_the_wait_is_refused_and_its_report_stays_open` (wait starts; the
  project is removed from the connection; an attempt ends; the wait is refused and the row is
  still open).

### B-259 · `task_wait` over MCP

- Status: done
- Owner: `Runs` (the method), `Grants` (the inventory); the MCP adapter translates.
- Read: `crates/shadows-mcp/src/run_tools.rs` (a tool calling `self.core.runs()`),
  `crates/core-grants/src/tools.rs`.
- Interfaces: tool `task_wait`, args `{ seconds?: u32, ack?: string[] }` (`seconds` default
  45; a larger value is held to 50, so Codex's `tool_timeout_sec` of about 60 never cuts it;
  `ack` names the `report_id`s an earlier answer showed it), calls `runs().author_wait(&grant, &ack, …)`,
  answers `AuthorWait`. In `PROJECT_TOOLS` only, never in
  `THREAD_TOOLS`. Its description says it answers at once when nothing runs.
- Touch: `crates/shadows-mcp/src/run_tools.rs`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`, `crates/shadows/tests/mcp_server.rs`.
- Needs: B-253, B-258.
- Done when: `cargo test -p shadows --test mcp_server` passes with
  `task_wait_is_an_external_tool_bounded_under_a_minute` (listed for a connection, not for a
  Planner; `seconds: 120` answers `nothing_yet` within 51 s while an attempt runs) and
  `the_external_inventory_is_the_planners_less_approval_and_conversation_tools` updated to 43.

### B-260 · The wait over HTTP with the connection's bearer, and its address in Settings

- Status: done
- Owner: `Runs` (the method), `Grants` (bearer, project and `wait_url`); `shadows-http`
  translates.
- Read: `crates/shadows-http/src/grants.rs`, `crates/shadows-mcp/src/auth.rs` (how `/mcp` turns
  a bearer into a connection), `Grants::authorize_external`, `Grants::external_project`,
  `crates/shadows-http/src/contract.yaml`.
- Interfaces: `POST /api/external/task-wait`, `Authorization: Bearer <connection token>`, body
  `{ project_id: string, seconds?: u32, ack?: string[] }` → `AuthorWait`. `seconds` defaults
  to 300 and is held to 540: a hook makes one call per run, and Codex's hook `timeout` (600 s)
  covers the whole hook, so one call must end inside it. `ack` carries the ids an earlier run
  handed to its session (B-258's delivery contract). A missing or revoked bearer is 401, also
  when the grant is revoked while the call waits; a project not on the connection 404. No browser origin needed. `ExternalConnection` and the issue response
  gain `wait_url: string` (`http://<bound address>/api/external/task-wait`), built by Grants
  beside `mcp_url`.
- Touch: new `crates/shadows-http/src/external_wait.rs`, `crates/shadows-http/src/lib.rs`
  (route), `crates/shadows-http/src/contract.yaml`, `crates/core-grants/src/store/external.rs` (its `load()` literal sets `wait_url`),
  `crates/core-grants/src/external.rs` (where `ExternalConnection` lives; Grants fills `wait_url` from `mcp_url`, so `Grants::new` keeps its arguments), `crates/core-grants/contract.yaml`, `api/openapi.json`
  (regenerated), `crates/shadows/tests/grants.rs`, `docs/codebase/README.md`,
  `crates/shadows-core/src/lib.rs` (re-export `AuthorWait`, `AuthorReport` and `WaitOutcome`,
  so the route names its OpenAPI body), `crates/shadows-http/src/failure/runs.rs` (only if a
  `RunsError` case needs its status). The adapter does not poll the bearer while it waits: a
  revocation during the wait is refused by `author_wait`'s own claim check (`GrantInvalid`),
  which the route answers 401; with no new report the call ends at its bound.
- Needs: B-258.
- Done when: `cargo test -p shadows --test grants --test openapi` pass with
  `the_http_wait_answers_with_the_connections_bearer` (an ended attempt's report),
  `the_http_wait_acknowledges_what_it_names` (a second call with that `ack` does not answer it),
  `the_http_wait_refuses_a_revoked_bearer` (401),
  `the_http_wait_refuses_a_connection_revoked_while_it_waits` (401; the report stays open), `the_http_wait_refuses_an_unselected_project`
  (404), `a_connection_names_its_wait_url`.

### B-261 · A task run says its result waits for its author

- Status: done
- Owner: `Runs`.
- Read: the `TaskRun` view in `crates/core-runs/src/model.rs`; B-257's rows.
- Interfaces: `TaskRun.author_waiting: bool`, true while the task's latest attempt has a
  `run_author_report` row not yet acknowledged. A `TaskProgressChanged` is journaled in the
  write that saves a row and in the write that acknowledges it, through the existing journal
  helper for an attempt (`crates/core-runs/src/store/mod.rs`), so it carries the task's
  `project_id`, `workflow_id`, `version`, `task` and `attempt_id` as today's do: the web
  refreshes only that task's run on those (Masada's condition, accepted request).
- Touch: `crates/core-runs/src/model.rs`, the view's store read, `crates/core-runs/src/author.rs`,
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`,
  `api/openapi.json` (regenerated).
- Needs: B-258.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `a_task_run_shows_its_result_waiting_for_its_author` (true after the end, still true while
  claimed, false once acknowledged), `saving_and_acknowledging_a_report_journal_the_tasks_progress`
  (two `TaskProgressChanged` events, each with that task's `workflow_id`, `version` and
  `task`).

### B-263 · Findings left by B-253 to B-261, fixed before the branch review

Each is fixed at its root by `backend-fixer`, one commit each, with only the tests it touches.

- **F1 · Lines over 100 characters.** The line check of `docs/backend/AGENTS.md` rule 5 prints
  `crates/core-runs/src/store/author.rs:50`, `crates/shadows/tests/mcp_server.rs:249`,
  `crates/shadows-core/tests/external_author.rs:768`, `crates/shadows-mcp/src/run_tools.rs:575`.
  - Status: done
  - Touch: those four files.
  - Done when: the line check prints nothing; `cargo fmt --all --check` passes;
    `cargo test -p shadows --test mcp_server` and
    `cargo test -p shadows-core --test external_author` pass.
- **F2 · A follow-up attempt that fails to open reports to no one.** In
  `crates/core-runs/src/start.rs` (around the `finish(.., AttemptState::Failed, ..)` of a
  session that does not open) the attempt is marked `Failed` but `notify` is never called: the
  Planner gets no notice, and for an external plan no `run_author_report` row is kept, so the
  report never settles and B-258's wait never answers `none_running` for that author. Fix it on
  the one path both use: a failed open settles its notice through `notify`, as every other end
  does (the Planner's thread notice, or the external author's row).
  - Status: done
  - Owner: `Runs`.
  - Touch: `crates/core-runs/src/start.rs`, `crates/core-runs/src/notify.rs` if needed,
    `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`, and the
    Planner-side test file in `crates/shadows-core/tests/` that covers attempt notices.
  - Done when: `a_follow_up_that_fails_to_open_reports_to_its_external_author` (a returned check
    whose follow-up cannot open, e.g. a model that refuses its mode: one `ended` row; the next
    wait answers it, and after its `ack` answers `none_running`) and
    `a_follow_up_that_fails_to_open_tells_the_planner` (one notice on the Planner's thread)
    pass, with the existing `external_author` and attempt-notice tests.
- **F3 · Reading the connection token is written twice.** `bearer()` in
  `crates/shadows-mcp/src/auth.rs` and in `crates/shadows-http/src/external_wait.rs` are the same
  rule. Grants owns bearer validation, so it owns the parse: one associated function
  `Grants::bearer(authorization: Option<&str>) -> Option<&str>` (the `Authorization` header's
  value; `Bearer` matched without case, the token trimmed, empty refused), called by both
  adapters, which pass the header's value and keep no parse of their own.
  - Status: done
  - Owner: `Grants`.
  - Touch: `crates/core-grants/src/lib.rs` (or `external.rs`), `crates/core-grants/contract.yaml`,
    `crates/shadows-mcp/src/auth.rs`, `crates/shadows-http/src/external_wait.rs`,
    `crates/shadows-http/src/contract.yaml`, the `shadows-mcp` contract if it names `bearer`.
  - Done when: `cargo test -p core-grants` passes with `a_bearer_header_is_parsed_once`
    (`Bearer x`, `bearer  x `, `Basic x`, `Bearer `, none), and
    `cargo test -p shadows --test mcp_server --test grants` pass.

### Every notice to its author, the author's own way (B-264 to B-267)

Spec: "Every notice goes to its author, the author's own way" in
[external connections](specs/2026-11-18-external-agent-connections-design.md#how-the-plans-author-hears-of-its-tasks)
(Mohammed, 2026-11-28, `ecbb0185`). Found by the whole-branch review (`3e5a3e5e`): a reply to
`message_agent` and a recheck's result reach only a Planner, and `project_change_note` had no
owner for an external agent. Migration `0086` is not pushed, so B-264 changes it in place.
`task_question` needs nothing: Shadows answers it, no author is told.

### B-264 · An author report carries every notice kind

- Status: done
- Owner: `Runs`.
- Read: `crates/core-runs/src/author.rs`, `store/author.rs`, `model.rs` (`AuthorReport`),
  `crates/core-db/migrations/0086_run_author_report.sql`, `crates/core-runs/contract.yaml`.
- Interfaces: `run_author_report` (changed in place in `0086`): `kind IN ('ended','conflict',
  'reply','recheck')`; new `source_id TEXT NOT NULL` (the attempt id for `ended` and
  `conflict`, the message id for `reply`, the batch id for `recheck`); `attempt_id`,
  `workflow_id` and `task` become NULL-able (a `recheck` has no attempt); `report_id` is
  `<kind>:<source_id>`. `AuthorReport` serializes `attempt_id`, `workflow_id`, `task` as
  `Option`; `kind` gains `reply` and `recheck`. One private
  `Runs::keep_for_author_in(tx, row: AuthorRow)` keeps every kind, with B-261's
  `TaskProgressChanged` only for a row that has a task. `author_waiting` counts only `ended`
  and `conflict` rows. No behaviour change for `ended` and `conflict` beyond the id format.
- Touch: `crates/core-db/migrations/0086_run_author_report.sql`, `crates/core-runs/src/author.rs`,
  `store/author.rs`, `model.rs`, `notify.rs` (the `ended` and `conflict` callers),
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`,
  `crates/shadows/tests/grants.rs` (its SQL-made rows), `api/openapi.json` (regenerated).
- Needs: nothing.
- Done when: `cargo test -p shadows-core --test external_author` and
  `cargo test -p shadows --test grants --test openapi --test mcp_server` pass, with
  `report_ids_name_their_kind_and_source` (`ended:<attempt>`, `conflict:<attempt>`).

### B-265 · An executor's reply reaches the author of the attempt's version

- Status: done
- Owner: `Runs`.
- Read: `crates/core-runs/src/notify.rs` (`notify_reply`), `message.rs` (`message_agent`, the
  executor prompt, `notify_reply_logged`), `recover.rs` (where undelivered replies retry).
- Interfaces: `notify_reply` routes by the author of the attempt's own version: a Planner as
  today; an external author keeps a `reply` row (body: the Planner's prompt text,
  `[Shadows] Reply from task agent · T<n> "<title>":\n<reply>` or the no-reply line) and marks
  the message delivered in the same write. It never answers `Unavailable` for an external
  plan, so recovery stops retrying it. The executor's prompt names no Planner: `You are asked,
  about task T<n>, without changing it:\n<message>\nAnswer in plain words. Change no file and
  make no commit for this.` B-258's pending rule also counts a message of that author's
  plans that has neither a reply nor a no-reply reason yet.
- Touch: `crates/core-runs/src/notify.rs`, `message.rs`, `author.rs`, `store/author.rs`,
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`, the test
  of the executor prompt if one names "The Planner asks you".
- Needs: B-264.
- Done when: `cargo test -p shadows-core --test external_author --test runs_attempts` pass,
  with `an_executors_reply_reaches_its_external_author` (one `reply` row; the message is
  delivered; a restart's recovery sends nothing again),
  `a_wait_holds_while_a_message_awaits_its_reply`, and the Planner's existing reply tests.

### B-266 · A recheck's result reaches whoever asked it, or the completing plan's author

- Status: done
- Owner: `Runs`.
- Read: `crates/core-runs/src/recheck.rs` (`recheck_recipient`, the batch's end and
  `deliver_recheck`), `planner.rs` (`recheck_project_for`), the automatic recheck after a done
  mark completes a plan, migrations `0070`, `0079`.
- Interfaces: a batch keeps who asked: `asker_thread_id` (a Planner) or a new
  `asker_grant_id` (an external agent; `ALTER TABLE run_rechecks ADD COLUMN asker_grant_id
  TEXT` added to `0086`); `recheck_project_for` sets it from the grant. The recipient is the
  asker, else the author of the cause plan's version that completed. A Planner hears as today;
  an external author gets a `recheck` row with the Planner's notice body, saved in the write
  that ends the batch, and the batch's notice is settled. B-258's pending rule also counts an
  unfinished batch whose recipient is that grant.
- Touch: `crates/core-runs/src/recheck.rs`, `planner.rs`, the recheck store file,
  `author.rs`, `store/author.rs`, `crates/core-db/migrations/0086_run_author_report.sql`,
  `crates/core-runs/contract.yaml`, `crates/shadows-core/tests/external_author.rs`.
- Needs: B-264.
- Done when: `cargo test -p shadows-core --test external_author` passes with
  `an_asked_recheck_reaches_its_external_asker`,
  `a_failing_automatic_recheck_reaches_the_completing_plans_external_author` (an external
  author's done mark completes the plan; a failing check gives one `recheck` row; the next
  wait answers it), `a_wait_holds_while_its_recheck_runs`; and the existing recheck tests in
  `cargo test -p shadows-core --test runs_attempts` pass.

### B-267 · An external agent notes its own changes for the project

- Status: todo
- Owner: `Turns` (it owns the note's rule); `Grants` (the inventory).
- Read: `crates/core-turns/src/changes.rs` (`note_change`), its store,
  `crates/shadows-mcp/src/turn_tools.rs` (`project_change_note`),
  `crates/core-events/src/store.rs` (the project tail's kinds), `crates/core-grants/src/tools.rs`.
- Interfaces: `Turns::note_change(&self, by: ChangeNoter, paths: Vec<String>, reason: String)`,
  `ChangeNoter::Planner(ThreadId) | ChangeNoter::External { grant: GrantId, project: ProjectId }`:
  one validation (reason 1 to 2000 characters, 1 to 100 paths through
  `Codebase::project_file`). A Planner's note stays a `ProjectChange` thread entry; an external
  note journals `ProjectChangeNoted { project_id, paths, reason }` with the grant as actor, and
  the project tail carries that kind. `project_change_note` joins `PROJECT_TOOLS` (the external
  list grows by one).
- Touch: `crates/core-turns/src/changes.rs`, `crates/core-turns/src/store/changes.rs`,
  `crates/core-turns/contract.yaml`, `crates/shadows-mcp/src/turn_tools.rs`,
  `crates/core-events/src/store.rs`, `crates/core-events/contract.yaml`,
  `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/shadows-core/tests/planner_changes.rs`, `crates/shadows/tests/mcp_server.rs`.
- Needs: nothing.
- Done when: `cargo test -p shadows-core --test planner_changes` and
  `cargo test -p shadows --test mcp_server --test project_events` pass, with
  `an_external_agent_notes_a_change_for_the_project` (one `ProjectChangeNoted` on the project
  tail, actor the grant), `an_external_note_outside_the_project_is_refused`, and the external
  inventory tests counting one more tool.

### B-262 · Acceptance: an external agent takes a project from idea to done tasks

- Status: blocked (needs B-249 to B-253, B-257 to B-261 and the web side of the request)
- Owner: the controller with Mohammed; no code.
- Run, on a fresh `testing` project through `shadows serve` and the browser:
  1. Claude Code connected from Settings with its `asyncRewake` Stop hook: it proposes vision,
     map and roadmap; Mohammed approves each with the button; it writes and starts a plan; the
     hook wakes it with each attempt's report and it judges it.
  2. The Codex desktop app with its Stop hook: busy while it waits, answered with the report;
     an attempt longer than the hook's timeout leaves its report for the next wait.
  3. An attempt that ends with no one waiting shows as waiting for its author; the next wait
     answers it at once.
- Done when: `docs/evidence/<date>-external-agents-like-the-planner-live.md` records the three
  runs and their findings.

