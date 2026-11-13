# Slice 4: task questions, conflicts and parallel groups — Implementation Plan

> **For agentic workers:** work from the named stages entry, one verified task and
> scoped commit at a time. Use superpowers:executing-plans for inline execution, or
> superpowers:subagent-driven-development when the controller selects agents. This
> planning document does not start product implementation.

**Goal:** Save the context each attempt receives, answer its code/contract questions,
escalate contract conflicts, and let the Planner start compatible independent tasks.

**Architecture:** AppCore composes the existing owners. Plans owns approved context
and contract relationships; Codebase owns disk/LSP facts; Runs owns admission,
attempt context, joined answers, conflicts and ready groups. Turns delivers notices.

**Tech Stack:** existing Rust 1.94+, SQLx 0.9/SQLite, Tokio, rmcp, axum/utoipa, React,
TanStack Query/Router and Vitest. Reuse workspace schemars for MCP input schemas;
no new third-party package or version.

**Spec:** [approved design](../specs/2026-11-14-task-questions-design.md).
**Wire agreement:** [slice 4](../../shared/requests/2026-11-14-task-questions.md).
**Baseline:** `7a52bc59`, `feature/slice-4-task-questions`, 2026-11-14.
**Replaces:** [earlier briefs draft](2026-11-09-execution-briefs.md); its old steps are not executable.
**Order:** B-108 → B-107 → B-109 → B-147 → B-148 → F-102 → B-149.

## Global Constraints

- Begin at AppCore and the named owner contracts. Stores remain private; cross-owner
  transaction work uses associated `*_in(&mut core_db::Tx, …)` methods.
- No new service, scheduler daemon, AI answerer, dependency graph, workspace or cache.
- No source-copy store, parent transcript, packet hash, range locks, automatic retry,
  cancellation cascade, general signature validator or arbitrary concurrency setting.
- `code_lsp` remains available within project/link/grant reach beyond task `reads`.
- Unaffected attempts continue across a corrective Draft and are judged against their
  original Frozen contract. No automatic copying of old attempts/marks into a new version.
- Keep existing conservative write overlap and held scopes through committed judgment.
  Never free a scope while its executor may write, including failed session termination.
- Mutations use normalized command fingerprints, schema 1 and replay before current-state
  refusals. Conflict delivery freezes its full payload, not just its command id.
- One cache per workspace; no CARGO_TARGET_DIR, worktree build, clean, release/custom
  profile/target or changed RUSTFLAGS. Full gate stays in CI; local tests are focused.
- Public wire types have existing serde/schema derives and are re-exported through
  shadows-core. Generate OpenAPI/schema.d.ts; adapters depend only on shadows-core.
- Changed owners update their contracts and code maps in the same product commit.
  Never put planned operations into current contracts during this planning delivery.
- Reserve migrations 0061_attempt_context.sql and 0062_run_conflicts.sql on this baseline.
  If another task consumes a number, renumber only these unshipped files before execution.
- Tests use real temporary Git/SQLite folders and installed servers. No new fake worker,
  adapter behavior or provider scripts. Live-provider tests are explicitly ignored until
  signed-in access is available. Existing fake-backed regression tests may be updated.

## Review Focus

1. A Draft/done-mark change racing Start cannot bypass admission: B-108's
   `admission_rechecks_a_new_draft` and `admission_rechecks_needs`.
2. A repeated brief read or addition cannot rewrite old context: B-107's fixed-context,
   replay and addition tests.
3. Planned consumers cannot be presented as source callers or globally exhaustive:
   B-109's provenance, unavailable-server and version/pagination tests.
4. Conflict/report races, failed termination and restart cannot lose delivery or release
   a live scope: B-147's race, failure and recovery tests.
5. Individually ready tasks may overlap each other: B-148's pairwise group and held-commit tests.
6. Shared-folder Git staging can include another task's staged path: B-149's
   `live_cross_scope_staging_is_not_an_accepted_report` must preserve the existing
   report refusal, never weaken scope checks or reset another task's files.

## Common Touch and verification

Every backend task additionally owns `docs/backend/stages.md` (its status only) and
`docs/codebase/README.md` (its new/changed module rows). They are part of each Files list.
Each listed contract is updated in that task's commit. F-102 owns its frontend equivalents.

After focused backend tests run `cargo fmt --all --check` and
`cargo test -p shadows --test architecture --test codemap --test contract_tests` plus
the repository's 100-column check on changed Rust. For route/wire changes run the
OpenAPI test once with UPDATE_OPENAPI=1, then without; inspect only intended API diffs.
Do not repeat the whole workspace gate. A blocked check is reported, never called passed.

Each task follows the five checkbox steps below and ends in a separate explicit-path
commit. Tests named here are requirements to implement, not evidence they already ran.

## Task B-108: authoritative admission and exact required files

**Needs:** design/wire agreement. **Owners:** Plans, Runs, Codebase.

**Files:**
- Create `crates/core-plans/src/execution.rs`, `crates/codebase/src/inputs.rs`,
  `crates/shadows-core/tests/run_admission.rs`.
- Modify `crates/core-plans/src/lib.rs`, `crates/core-plans/src/store/read.rs`,
  `crates/core-plans/contract.yaml`; `crates/core-runs/src/start.rs`,
  `crates/core-runs/src/ready.rs`, `crates/core-runs/src/amend.rs`,
  `crates/core-runs/src/store/mod.rs`, `crates/core-runs/src/store/marks.rs`,
  `crates/core-runs/src/error.rs`, `crates/core-runs/contract.yaml`.
- Modify `crates/codebase/src/lib.rs`, `crates/codebase/src/inventory.rs`,
  `crates/codebase/src/lsp/ask.rs`, `crates/codebase/contract.yaml`,
  `crates/shadows-http/src/runs.rs`, `crates/shadows-http/src/contract.yaml`, `api/openapi.json`.
- Owning-module transaction tests may live in the exact new execution/inputs files
  and existing Runs store files above; no new production test facade.

**Interfaces:**
- Plans exports `ExecutionRefusal { Archived, NotFrozen, Superseded }` and
  `ExecutionPlan { plan: Plan, refusal: Option<ExecutionRefusal> }` from execution.rs.
  `pub async fn execution_plan(&self, workflow: &WorkflowId, version: i64) -> Result<ExecutionPlan, PlansError>`;
  `pub async fn execution_plan_in(tx: &mut Tx, workflow: &WorkflowId, version: i64) -> Result<ExecutionPlan, PlansError>`.
  One loader/rule supplies both; populate existing linked_tasks through LinkedRows in
  that snapshot, preserving cross-plan needs resolution and version mismatches as NotFound.
- Runs maps those refusals to existing RunsError variants. One private transaction-aware
  dependency-mark check supplies both preflight and serialized admission. begin_attempt
  replays, checks current plan eligibility and required marks, checks held scopes, then
  inserts the executor/attempt in its existing write. continue_attempt rechecks eligibility
  before transferring a committed scope to a new addition. Existing old work still reports.
- Codebase exports `pub async fn missing_inputs(&self, asker: Asker<'_>, paths: &[String]) -> Result<Vec<String>, CodebaseError>`.
  Canonical project/grant/secret checks precede disk reads; return sorted/deduplicated
  absent regular files. IO failure, escaping/link-out and secret paths are errors, not absence.
  For an absent path, resolve its nearest existing ancestor and validate the remaining
  components; absence is not a canonicalization error, and an ancestor junction out is refused.
  One private `checked_path(dir: &Path, relative: &str, allow_missing: bool) -> Result<PathBuf, CodebaseError>`
  async associated method in inputs.rs supplies containment to missing_inputs, existing
  real_file and project_file. Reuse the inventory's secret_name predicate for required
  inputs (expose it within the crate); do not copy a second secret list or path rule.
- Required exact paths come from rich contract Read/Modify/Delete file work and Existing
  source references. Create targets need not exist; reads globs are permitted search areas,
  not mandatory files. Legacy contracts preserve their existing behavior.
  Add RunsError MissingInput { paths } / `missing_input` and InputUnavailable(String) /
  `input_unavailable`; both refuse Start before insertion. Preserve git/dirty checks.
- Disk checks run outside long SQL writes; only durable plan/mark/scope checks are atomic.
  Do not claim files are locked. Derive required inputs once for preflight and Start.

- [ ] **1. Write failing tests:** `admission_rechecks_a_new_draft`, `admission_rechecks_needs`,
  `a_create_target_need_not_exist`, `missing_existing_inputs_refuse_start`,
  `a_reads_glob_is_not_a_required_file`, `an_input_junction_out_is_refused`,
  `unrelated_dirty_files_do_not_refuse_start`, `old_work_reports_after_a_corrective_draft`.
  Assert no attempt/thread/context on refusal and replay returns the first attempt.
  Race tests use a gated owning-store barrier before admission, not timing sleeps.
- [ ] **2. Observe RED:** `cargo test -p shadows-core --test run_admission`, plus owning
  transaction tests. Only the missing operation/behavior counts as the intended failure.
- [ ] **3. Implement** the named interfaces and common admission/input rules.
- [ ] **4. Observe GREEN:** Step 2; `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test code_lsp`; common checks and OpenAPI regeneration.
- [ ] **5. Commit** `feat(backend): B-108 recheck task admission and required inputs`.

## Task B-107: saved attempt context and task_brief

**Needs:** B-108. **Owners:** Plans, Runs; grants/adapters translate.

**Files:**
- Create `crates/core-runs/src/brief.rs`, `crates/core-runs/src/store/brief.rs`,
  `crates/core-db/migrations/0061_attempt_context.sql`, `crates/shadows-core/tests/attempt_briefs.rs`.
- Modify `crates/core-plans/src/task_context.rs`, `crates/core-plans/src/store/task_context.rs`,
  `crates/core-plans/contract.yaml`, `crates/core-db/contract.yaml`;
  `crates/core-runs/src/lib.rs`, `crates/core-runs/src/start.rs`,
  `crates/core-runs/src/amend.rs`, `crates/core-runs/src/store/mod.rs`,
  `crates/core-runs/src/error.rs`, `crates/core-runs/contract.yaml`.
- Modify `crates/shadows-core/src/lib.rs`, `crates/shadows-mcp/src/run_tools.rs`,
  `crates/shadows-mcp/src/server.rs`, `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/core-harness/src/executor.md`, `crates/core-harness/contract.yaml`;
  `crates/shadows-http/src/runs.rs`, `crates/shadows-http/src/lib.rs`,
  `crates/shadows-http/src/openapi.rs`, `crates/shadows-http/src/contract.yaml`, `api/openapi.json`.
- Update changed-tool regressions `crates/shadows-core/tests/executor_session.rs`,
  `crates/shadows/tests/mcp_tools.rs`, `crates/shadows/tests/code_scope.rs`.

**Interfaces:**
- Plans: `pub async fn task_context_in(tx: &mut Tx, project: &ProjectId, workflow: &WorkflowId, task: u32, phase: Option<&PhaseId>) -> Result<TaskContext, PlansError>`.
  Lift existing assembly into this one owner method. Existing grant-facing task_context
  retains authorization and calls it; Runs calls it after admission inside the same write.
  Add missing schema derives to existing nested context types, not persistence imports.
- Migration: `run_attempt_context(attempt_id TEXT PRIMARY KEY REFERENCES run_attempts(id) ON DELETE CASCADE,
  context_json TEXT NOT NULL, captured_at TEXT NOT NULL)`; reject UPDATE of saved context.
  No historical backfill, parent transcript or source bytes.
- Runs: `AttemptBrief { attempt: Attempt, context: TaskContext, captured_at: String }`;
  `pub async fn brief(&self, attempt: &AttemptId) -> Result<AttemptBrief, RunsError>`;
  `pub async fn brief_for(&self, grant: &Grant) -> Result<AttemptBrief, RunsError>`.
  Unknown attempt => NotFound; historical attempt with no row => ContextNotRecorded
  (`context_not_recorded`, HTTP 409). Do not reconstruct historical layout.
- Private `capture_context_in(tx: &mut Tx, attempt: &AttemptId, context: &TaskContext, captured_at: &str) -> Result<(), RunsError>`
  runs after insertion in both begin_attempt and continue_attempt, before commit/session.
  Capture failure rolls back the whole insertion/transfer. Attempt remains the source of
  identity/model/base SHA/addition; no duplicated mutable metadata in context JSON.
- MCP executor `task_brief {}` replaces executor task_context; Planner/project task_context
  stays. First and addition prompts require task_brief. HTTP GET `/api/attempts/{id}/brief`.
  Re-export wire types through shadows-core. Replays never capture another context.

- [ ] **1. Tests:** `brief_stays_fixed_after_layout_change`, `an_addition_captures_its_own_brief`,
  `brief_replay_creates_one_context`, `context_failure_rolls_back_admission`,
  `historical_context_is_not_invented`, `an_executor_reads_only_its_own_brief`.
  Assert old/new exact map/standards versions, unchanged saved JSON, one row on replay.
- [ ] **2. RED:** `cargo test -p shadows-core --test attempt_briefs`.
- [ ] **3. Implement** the assembler, transactional capture, reads, grants, prompts and schemas.
- [ ] **4. GREEN:** Step 2, executor_session, `cargo test -p shadows --test mcp_tools --test code_scope`,
  common checks and OpenAPI. Check the ordinary Planner task_context still works.
- [ ] **5. Commit** `feat(backend): B-107 keep the context each attempt receives`.

## Task B-109: questions about source and planned consumers

**Needs:** B-107. **Owners:** Plans owns relationships; Runs joins Codebase facts.

**Files:**
- Create `crates/core-plans/src/contract_questions.rs`, `crates/core-runs/src/question.rs`,
  `crates/shadows-core/tests/task_questions.rs`, `crates/shadows/tests/task_question_mcp.rs`.
- Modify `crates/core-plans/src/lib.rs`, `crates/core-plans/src/contract_uses.rs`,
  `crates/core-plans/contract.yaml`, `crates/core-runs/src/lib.rs`,
  `crates/core-runs/src/error.rs`, `crates/core-runs/contract.yaml`, `crates/shadows-core/src/lib.rs`;
  `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/server.rs`,
  `crates/shadows-mcp/src/contract.yaml`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`, `crates/core-harness/src/executor.md`,
  `crates/core-harness/contract.yaml`.
- Modify `crates/core-grants/src/lib.rs`, `crates/core-grants/src/store/mod.rs`,
  `crates/core-runs/src/brief.rs`, `crates/shadows-http/src/runs.rs`,
  `crates/shadows-http/src/openapi.rs`, `crates/shadows-http/src/failure/runs.rs`,
  `crates/shadows-mcp/src/refusal.rs`, `crates/shadows-http/src/contract.yaml`, `api/openapi.json`.
- Modify `crates/core-runs/Cargo.toml`, `crates/codebase/Cargo.toml`,
  `crates/codebase/src/model.rs`, `crates/codebase/contract.yaml`, `Cargo.lock`.
  Add the existing workspace schemars dependency where input schema derives need it;
  this is infrastructure. LspQuestion gains serde/schema derives and LspOperation the
  missing JsonSchema derive, so typed TaskQuestion inputs preserve native LSP fields.

**Interfaces:** exact public fields are in the wire agreement; definitions live in the
owning job module and are exported by its lib.
- Plans owns ContractSelector, ConsumerRef, ContractFacts and
  `pub async fn contract_facts(&self, workflow: &WorkflowId, task: u32, selector: ContractSelector, after: Option<u32>, limit: u16) -> Result<ContractFacts, PlansError>`.
  Select own contract/symbol/output or an exact source declared by its uses. Resolve
  producers and reverse consumers via ContractRules in that exact Frozen version only.
  Sort consumers by (task number, uses index), offset after, limit 1..=50/default 20.
  No newer/cross-plan planned consumers, copied siblings or second dependency graph.
  ContractFacts echoes selector. Own returns the whole own contract. Symbol returns only
  that symbol, its declared file and outputs referring to it; Output returns that output
  and its declared symbol/artifact file; omit unrelated vectors/text from these projections.
  Use returns contract=null and only its matching resolved source. Consumers match exact
  local task/output references, or an exact Existing source for a Use selector; the latter
  has output=null. Never infer a consumer from a same-name function or another plan.
- Grants: `pub async fn check_live_in(tx: &mut Tx, grant: &Grant) -> Result<(), GrantsError>`.
  Its store rereads the id and checks unrevoked kind/project/thread/scope against the
  authenticated value. Runs uses it after awaited code work and B-147 inside acceptance.
  No token or Grants SQL enters Runs; revoked identities remain refusals.
- RunsError gains transparent GrantsError/CodebaseError variants. Adapters unwrap those
  into existing owner-error translations before generic RunRefusal handling; grant/path
  errors remain refusals. Only actual server-unavailable outcomes become code_error.
- B-109 extends AttemptBrief with consumers/consumers_more through that same Plans lookup
  (own selector, first 50), derived from the original Frozen version. Do not change saved
  context JSON. Update brief schemas and tests; an incomplete summary is explicitly labelled.
- Runs owns TaskQuestion, ProducerStatus, TaskAnswer and
  `pub async fn question(&self, grant: &Grant, question: TaskQuestion) -> Result<TaskAnswer, RunsError>`.
  Resolve the authenticated current Running executor attempt and saved brief. Ask Plans,
  optionally Codebase::lsp with Asker::Grant, and Runs' own done/commit reads. Recheck live
  grant/attempt after awaited code work. Never trust a client-selected attempt id.
- Separate fixed planning facts from current done/commit/LSP observations with observed_at.
  `planned_scope` is always `this_frozen_version`. An unavailable server is code_error,
  not a successful empty code answer; never infer missing implementation from no evidence.
- Whole answer <=64 KiB; preserve native LSP bounds/truncated and consumer pagination.
  If even a selected contract cannot fit, return `question_too_large` asking for a narrower
  selector; do not silently cut signatures. Invalid selector/offset/limit => invalid_question.
  No install/retry, natural-language parser or question/source ledger.
- MCP `task_question` takes TaskQuestion; executor retains code_lsp. No new HTTP question API.

- [ ] **1. Tests:** `planned_consumers_are_not_code_callers`, `question_uses_the_attempt_version`,
  `a_use_resolves_its_producer`, `unavailable_code_is_not_absence`,
  `project_code_outside_reads_can_be_asked`, `unrelated_project_and_link_out_are_refused`,
  `consumer_pages_are_stable_and_local`, `oversized_contract_is_not_silently_cut`,
  `a_non_executor_cannot_ask`. Real Go module/gopls case has an existing symbol and a
  consumer planned but not implemented; no source location is fabricated for that consumer.
- [ ] **2. RED:** `cargo test -p shadows-core --test task_questions` and
  `cargo test -p shadows --test task_question_mcp`.
- [ ] **3. Implement** the named interfaces, grants and instructions using native LSP semantics.
- [ ] **4. GREEN:** Step 2; task_questions with `-- --include-ignored` when gopls is installed;
  attempt_briefs regressions, common checks, OpenAPI and affected existing MCP tests.
- [ ] **5. Commit** `feat(backend): B-109 answer task questions from contracts and code`.
  Its body identifies reuse of workspace schemars for MCP schemas as infrastructure.

## Task B-147: durable conflicts, confirmed stops and Planner delivery

**Needs:** B-107, B-109. **Owner:** Runs; existing Grants/Turns/Harness lifecycle.

**Files:**
- Create `crates/core-runs/src/conflict.rs`, `crates/core-runs/src/store/conflict.rs`,
  `crates/core-db/migrations/0062_run_conflicts.sql`,
  `crates/shadows-core/tests/task_conflicts.rs`, `crates/shadows/tests/task_conflict_routes.rs`.
- Modify `crates/core-runs/src/lib.rs`, `crates/core-runs/src/model.rs`,
  `crates/core-runs/src/error.rs`, `crates/core-runs/src/report.rs`,
  `crates/core-runs/src/notify.rs`, `crates/core-runs/src/stop.rs`,
  `crates/core-runs/src/watch.rs`, `crates/core-runs/src/start.rs`, `crates/core-runs/src/amend.rs`,
  `crates/core-runs/src/recover.rs`, `crates/core-runs/src/store/mod.rs`,
  `crates/core-runs/src/store/recover.rs`, `crates/core-runs/contract.yaml`, `crates/core-db/contract.yaml`.
- Modify `crates/core-turns/src/queue.rs`, `crates/core-turns/src/store/queue.rs`,
  `crates/core-turns/contract.yaml`; `crates/shadows-core/src/app.rs`, `crates/shadows-core/src/lib.rs`;
  `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/server.rs`,
  `crates/shadows-mcp/src/contract.yaml`, `crates/core-grants/src/tools.rs`,
  `crates/core-grants/contract.yaml`, `crates/core-harness/src/executor.md`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`;
  `crates/shadows-http/src/runs.rs`, `crates/shadows-http/src/lib.rs`,
  `crates/shadows-http/src/openapi.rs`, `crates/shadows-http/src/contract.yaml`, `api/openapi.json`.
- Existing regressions `crates/shadows-core/tests/runs_attempts.rs`,
  `crates/shadows-core/tests/runs_attempts/planner.rs`.

**Interfaces and state:**
- Runs owns ConflictInput/Evidence/Stop/Delivery/TaskConflict (wire agreement),
  `pub async fn conflict_for(&self, grant: &Grant, input: ConflictInput) -> Result<TaskConflict, RunsError>`,
  `pub async fn conflict(&self, attempt: &AttemptId) -> Result<Option<TaskConflict>, RunsError>`,
  `pub async fn stop_for(&self, grant: &Grant, command_id: String, attempt: &AttemptId) -> Result<Attempt, RunsError>`.
- Migration run_conflict: attempt_id PK/FK with ON DELETE CASCADE, input_json, stop, stop_error, delivery,
  notice_json nullable, created_at. Stop states requested/stopped/interrupted/failed;
  delivery pending/delivered/no_planner. Keep stop outcomes in this record; no general
  pause/resume engine. Attempt's confirmed outcome is Stopped/reason `contract conflict`.
- Validate expected/observed 1..=2,000 Unicode scalars; 1..=10 evidence items, summary
  1..=1,000, path <=1,024, optional positive one-based line. Normalized input <=16 KiB.
  Evidence is a canonical project-visible non-secret reference and explanation, not a
  source-copy store. Trim text, normalize paths before fingerprinting; preserve evidence order.
  Missing paths can themselves be evidence: validate through Codebase::missing_inputs,
  permitting its returned missing list while retaining its path/secret/grant refusals.
- In one write recheck the live executor grant, current Running attempt, command replay,
  and write conflict + Pending delivery intent + command result. Command kind run.conflict,
  schema 1, scope Attempt/id, principal Executor/thread. One accepted conflict per attempt;
  another distinct command is contract_conflict; changed input under one id is CommandConflict.
  Replay still requires live authenticated identity, but precedes the stopped-state refusal.
- record_report checks no accepted conflict in its own write: report/conflict have one
  winner. Do not convert an already Committed attempt into a conflict stop.
  The watcher's generic Failed/ended-without-commit write also checks conflict presence
  inside its transaction and defers to conflict settlement. An earlier read alone must
  not race a newly accepted conflict and overwrite its stop outcome or free its scope.
- Answer the tool promptly, then stop via a Runs-owned task outside that tool's call chain.
  Stop through Turns, close its Harness session, then release its SQL scope. Change private
  Runs::release to `Result<(), RunsError>` and update every named caller: failed close/kill
  must retain the scope. Preserve dirty/uncommitted files. Failed stop is visible, never a
  false stopped claim. Planner task_stop may retry that termination.
- stop_for checks same-project Thread grant and exact attempt. Kind run.stop/schema 1,
  principal Planner/thread, scope Attempt/id, fingerprint attempt id; command_record points
  to the attempt. Use existing idempotent Stop, record successful acknowledgment, and replay
  it without another run. Disconnected tools run through the existing detached handler;
  restart settles still-Running attempts as Interrupted, never restarts them.
- Runs::notify remains the one notification entry. Conflict presence selects its structured
  notice instead of generic ended-without-commit. AttemptNotice gains optional conflict,
  default/omitted when absent. Author/principal stays Executor, never a human approval.
- Freeze recipient, complete SendTurn settings/body and Notice before first enqueue and
  save notice_json. Retry that exact payload with id conflict-<attempt_id>. If enqueue
  commits but immediate model startup fails, delivery is still recorded. Add Turns-owned
  `pub async fn notice_recorded(&self, thread: &ThreadId, turn: &SendTurn, notice: &Notice) -> Result<bool, TurnsError>`
  to reconcile through the same full command/principal/fingerprint replay classification
  as queue_notice, including an already-consumed row or derived started turn. A person's
  message with a matching id cannot count as the executor notice. Runs never queries Turns tables.
- A delivered notice is an immutable snapshot linking to the current conflict read. Stop
  updates do not create a differently fingerprinted replay payload. Resolve a missing
  invocation later as pending; only absent/deleted/external Planner is no_planner.
- Runs owns one pending-conflict delivery retry loop, delay 1 s doubled to max 30 s.
  A failed termination waits for explicit Planner task_stop. Add
  `pub async fn resume_conflicts(&self) -> Result<(), RunsError>` after startup recovery,
  and `pub async fn close_conflicts(&self)` before storage shutdown. Resume Pending notices;
  interrupted accepted conflicts keep their record and stop=interrupted. No executor restart.
- MCP executor task_conflict takes ConflictInput; Planner task_stop takes command_id,
  attempt_id. Neither is external-project authority. HTTP GET `/api/attempts/{id}/conflict`
  => TaskConflict|null, unknown attempt 404. No human-as-executor mutation endpoint.

- [ ] **1. Tests:** `conflict_and_report_admit_one_outcome`, `conflict_replay_keeps_one_notice`,
  `a_conflict_stops_only_its_attempt`, `termination_failure_keeps_the_scope`,
  `session_close_failure_keeps_the_scope`, `restart_delivers_a_pending_conflict_once`,
  `new_planner_settings_do_not_change_a_retry`, `removed_planner_leaves_a_visible_conflict`,
  `planner_can_stop_an_affected_attempt`, `executor_conflicts_cannot_approve`,
  `conflict_keeps_uncommitted_files`. Assert retained scopes and independent work throughout.
  Use real serialized SQLite races/gated owner fault hooks. Real termination/provider cases
  are ignored live tests, not new scripted fake workers.
- [ ] **2. RED:** `cargo test -p shadows-core --test task_conflicts`;
  `cargo test -p shadows --test task_conflict_routes`; owning transaction tests.
- [ ] **3. Implement** the single path, owned background lifetime, startup/shutdown, grants,
  reads and prompts. Planner assesses impact and explicitly stops affected work.
- [ ] **4. GREEN:** Step 2, existing runs_attempts (including its planner notice tests), available
  live cases, common checks and OpenAPI. Verify both stop/report race outcomes.
- [ ] **5. Commit** `feat(backend): B-147 escalate contract conflicts and stop safely`.

## Task B-148: compatible ready groups

**Needs:** B-108, B-147. **Owner:** Runs.

**Files:** Create `crates/core-runs/src/ready_group.rs`, `crates/shadows-core/tests/ready_groups.rs`.
Modify `crates/core-runs/src/lib.rs`, `crates/core-runs/src/model.rs`,
`crates/core-runs/src/planner.rs`, `crates/core-runs/contract.yaml`,
`crates/shadows-core/src/lib.rs`, `crates/shadows-mcp/src/run_tools.rs`,
`crates/shadows-mcp/src/server.rs`, `crates/shadows-mcp/src/contract.yaml`,
`crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
`crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`,
`crates/shadows-core/tests/runs_attempts/planner.rs`, `crates/shadows/tests/mcp_tools.rs`.

**Interfaces:** ReadyTask/ReadyGroup (wire agreement) and
`pub async fn ready_for(&self, grant: &Grant, workflow: &WorkflowId) -> Result<ReadyGroup, RunsError>`.
Thread grant/project only; invalid plan => existing refusal; valid empty group => tasks=[].
MCP `task_ready { workflow_id }`; DoneMark.next becomes Vec<ReadyTask>, not Option<u32>.
One private selector supplies both. Marking old work done still succeeds, with next=[]
on a superseded version; do not fail an already-applied mark or start old work.

Sort task numbers; exclude done, previously attempted, unmet needs, invalid inputs and
held scopes. Greedily skip overlap with every selected candidate using unchanged
Scope::may_overlap. A group is an observation, not reservation; every Start revalidates.
Use existing suggested_model. No batch-start transaction, scheduler, queue or range locks.

Planner prompts cover first group after run-plan request, one-task-only requests, the
next group after done, and requery after a stop/failure following impact review. Never
retry a failed task automatically or loop a stale refused list. A corrective Draft
blocks new old-version Starts without cancelling unaffected running attempts.

- [ ] **1. Tests:** `the_ready_group_is_pairwise_disjoint`, `first_group_has_two_independent_tasks`,
  `held_commits_block_groups`, `a_stopped_task_is_not_automatically_retried`,
  `a_stop_unblocks_a_never_attempted_task`, `a_stale_group_cannot_bypass_start`,
  `judging_old_work_returns_no_old_next`, `non_planner_grants_cannot_request_groups`.
- [ ] **2. RED:** `cargo test -p shadows-core --test ready_groups`.
- [ ] **3. Implement** the common selector, changed tool result, grants and prompt.
- [ ] **4. GREEN:** Step 2, affected runs_attempts/planner and MCP tool regressions; common checks.
- [ ] **5. Commit** `feat(backend): B-148 start compatible task groups through the Planner`.

## Task F-102: saved brief and conflict presentation

**Needs:** B-107/B-147 and generated API. **Owners:** web API/tasks/conversation.

**Files:** Create `web/src/app/tasks/brief-panel.tsx`, `web/src/app/tasks/brief-panel.test.tsx`,
`web/src/app/tasks/conflict-panel.tsx`, `web/src/app/tasks/conflict-panel.test.tsx`,
`web/src/app/conversation/conflict-notice.tsx`, `web/src/app/conversation/conflict-notice.test.tsx`,
`web/src/api/runs.test.ts`.
Modify `web/src/api/runs.ts`, `web/src/api/schema.d.ts`,
`web/src/api/contract.yaml`, `web/src/app/tasks/report-tab.tsx`,
`web/src/app/tasks/task-page.tsx`, `web/src/app/tasks/task-page.test.tsx`, `web/src/app/tasks/contract.yaml`,
`web/src/app/conversation/notice-item.tsx`, `web/src/app/conversation/notice-item.test.tsx`,
`web/src/app/conversation/contract.yaml`, `docs/frontend/codebase/README.md`,
`docs/frontend/stages.md` (this task only).

**Interfaces:** `attemptBriefQuery(id: string)`, key ['runs','brief',id], and
`attemptConflictQuery(id: string)`, key ['runs','conflict',id], through existing transport.
Use generated type aliases only. Conflict polls every 2 s while stop=requested/failed
or delivery=pending; no process-liveness/impact rule in the client.
`BriefPanel({attemptId}: {attemptId:string})` and `ConflictPanel` with the same props
mount in each expandable Report entry, not just the latest attempt. Show original
contract/layout/standards and Attempt's base/model/addition. context_not_recorded is a
historical-context message, not a request to rebuild current context. Show the original
version's planned consumers and label consumers_more; do not claim global exhaustiveness.

Conversation-owned `ConflictNotice({conflict,attemptId})` reads generated facts under
NoticeItem's optional conflict field, retains executor attribution, and links to the
existing task/report. Current stop/delivery is read by ConflictPanel; copied notices
are history with no mutation controls. Questions remain ordinary tool entries.
All provider text is plain preserved text/dir auto. No client-generated dependency graph.
Use the TaskConflict project/workflow/version/task ids for the existing task route and
hash `attempt-<id>`. TaskPage opens Report for that hash and ReportTab expands/anchors
the exact attempt, so the notice link actually reaches the issue without new router APIs.

- [ ] **1. Tests:** `brief_view_keeps_original_version`, `historical_context_is_labelled`,
  `report_opens_the_selected_attempt_brief`, `conflict_shows_expected_observed_and_stop`,
  `settled_conflict_stops_polling`, `conflict_notice_keeps_executor_attribution`,
  `ordinary_notices_still_render`. Test Report/Notice integration through TestApp too.
- [ ] **2. RED:** from web, `npx vitest run src/app/tasks/brief-panel.test.tsx src/app/tasks/conflict-panel.test.tsx src/app/tasks/task-page.test.tsx src/app/conversation/conflict-notice.test.tsx src/app/conversation/notice-item.test.tsx src/api/runs.test.ts`.
- [ ] **3. Generate** with npm run gen:api, then implement hooks, panels and integrations.
- [ ] **4. GREEN:** Step 2, npm run typecheck, npm run lint, npm run build,
  `npx vitest run src/codemap.test.ts src/architecture.test.ts`.
- [ ] **5. Commit** `feat(web): F-102 show attempt briefs and contract conflicts`.

## Task B-149: real slice acceptance

**Needs:** every task above. Create `crates/shadows/tests/task_questions_live.rs`;
modify backend/frontend stages for acceptance evidence only.

- [ ] **1. Write ignored live tests:** `live_executor_asks_about_a_planned_consumer`,
  `live_contract_conflict_reaches_the_planner`, `live_planner_starts_independent_tasks_together`,
  `live_cross_scope_staging_is_not_an_accepted_report`.
  Real Go/Git folder, installed gopls, signed-in Claude executor and Claude/Codex Planner
  variants following existing language_servers_live. Assert task_brief/task_question tool
  entries, distinct code/planned facts, confirmed conflict stop and two independent
  attempts overlapping in time before either reports. Test prompts carry no finished implementation.
  In the staging case, arrange a candidate commit containing a foreign staged path;
  task_report must refuse OutsideWrites, no task is auto-marked done, and existing
  other-task changes remain intact. This does not add a commit queue or temporary index.
- [ ] **2. Run** `cargo test -p shadows --test task_questions_live -- --ignored --test-threads=1`.
  Record unavailable access separately; setup failure is not feature RED or acceptance.
- [ ] **3. Browser/log acceptance:** on a disposable project, inspect old/new briefs,
  trigger a mismatch and selective stop, approve the corrected version, and judge
  unaffected old work against its original contract. Confirm no old marks transfer
  automatically and same-file tasks never run together.
- [ ] **4. Record** commit/OS/harness/model/commands/browser observations and untested
  cases in stages. CI supplies the full workspace gate, distinct from live evidence.
- [ ] **5. Commit** `test(backend): B-149 accept task questions conflicts and ready groups`.

## Handoff

This delivery is planning only. One whole-branch review follows product implementation;
do not repeat it after every step. Each task's Needs must be met before dispatch. Slice 4
is not user-runnable until F-102 and live/browser acceptance have actually passed.
