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
**Order:** B-108 → B-148 → B-107 → B-109 → B-147 → F-102 → B-149.

## Global Constraints

- Begin at AppCore and the named owner contracts. Stores remain private; cross-owner
  transaction work uses associated `*_in(&mut core_db::Tx, …)` methods.
- No new service, scheduler daemon, AI answerer, dependency graph, workspace or cache.
- No source-copy store, parent transcript, packet hash, range locks, automatic task retry,
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
- Reserve migrations 0061_attempt_context.sql and 0062_attempt_conflicts.sql on this baseline.
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
6. Shared-folder Git staging must not contaminate a normal executor commit: B-148's
   real-Git procedure tests and B-149's positive parallel-commit case use --only with
   exact paths. Keep the negative OutsideWrites test as the final guard too.

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

## Task B-148: compatible ready groups

**Needs:** B-108 only. **Owner:** Runs. No dependency on task questions or conflicts.

**Files:** Create `crates/core-runs/src/ready_group.rs`, `crates/shadows-core/tests/ready_groups.rs`,
`crates/shadows-core/tests/executor_commits.rs`.
Modify `crates/core-runs/src/lib.rs`, `crates/core-runs/src/model.rs`,
`crates/core-runs/src/planner.rs`, `crates/core-runs/src/amend.rs`, `crates/core-runs/contract.yaml`,
`crates/shadows-core/src/lib.rs`, `crates/shadows-mcp/src/run_tools.rs`,
`crates/shadows-mcp/src/server.rs`, `crates/shadows-mcp/src/contract.yaml`,
`crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
`crates/core-harness/src/prompt.txt`, `crates/core-harness/src/executor.md`,
`crates/core-harness/contract.yaml`, `crates/shadows-core/tests/runs_attempts/planner.rs`, `crates/shadows/tests/mcp_tools.rs`.

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

**Commit and verification procedure, implemented in this task:**
- Update executor.md, FIRST_PROMPT and the addition prompt consistently: stage new files
  by exact name, then `git --literal-pathspecs commit --only -m "<message>" -- <own exact paths>`.
  Use exact changed paths inside writes, not a broad writes glob or `.`; include both
  names for a rename and the old name for deletion. Other staged paths stay untouched.
  Existing task_report validation remains the final guard; no custom index or commit queue.
- A Git index/ref lock refusal retries the same command at most three times after 1/2/4 s;
  never delete another operation's lock or reset its index. If still blocked, report the
  failure honestly instead of claiming a commit or silently changing someone else's work.
- Nonoverlapping writes do not isolate build directories, ports or test databases.
  The Planner specifies shared-check ownership/order in task instructions. Executors run
  isolated checks concurrently; shared project builds/tests are coordinated serially.
  If coordination is unavailable, report that check as not run and let the Planner
  arrange verification before acceptance, using its existing shell after relevant writers
  have settled when its mode permits; otherwise ask the person and leave it unverified.
  A deferred check is not a pass or a reason to ask the executor for evidence
  the Planner can obtain itself. This is instruction-level coordination here;
  an enforced verification slot remains slice 5. No new check scheduler/lock in slice 4.

- [ ] **1. Tests:** `the_ready_group_is_pairwise_disjoint`, `first_group_has_two_independent_tasks`,
  `held_commits_block_groups`, `a_stopped_task_is_not_automatically_retried`,
  `a_stop_unblocks_a_never_attempted_task`, `a_stale_group_cannot_bypass_start`,
  `judging_old_work_returns_no_old_next`, `non_planner_grants_cannot_request_groups`.
  In executor_commits, real Git tests `only_commit_preserves_foreign_staged_paths`,
  `new_deleted_and_renamed_paths_commit_only_their_own_changes`, and
  `literal_pathspecs_do_not_select_other_files` arrange interleaved staging explicitly;
  assert each accepted commit's path set and preservation of foreign staged content.
- [ ] **2. RED:** `cargo test -p shadows-core --test ready_groups --test executor_commits`.
- [ ] **3. Implement** the selector, tool/grants, commit procedure and shared-check prompts.
- [ ] **4. GREEN:** Step 2, affected runs_attempts/planner and MCP tool regressions; common checks.
- [ ] **5. Commit** `feat(backend): B-148 start compatible task groups through the Planner`.

## Task B-107: saved attempt context and task_brief

**Needs:** B-108; scheduled after B-148. **Owners:** Plans, Runs; grants/adapters translate.

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
  Preserve B-148's exact-path commit and shared-verification instructions when updating
  FIRST_PROMPT and the addition prompt to read task_brief.

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

## Task B-147: conflicts through the existing attempt lifecycle

**Needs:** B-107, B-109. **Owner:** Runs; existing Grants/Turns/Harness lifecycle.

**Files:**
- Create `crates/core-runs/src/conflict.rs`, `crates/core-runs/src/store/conflict.rs`,
  `crates/core-db/migrations/0062_attempt_conflicts.sql`,
  `crates/shadows-core/tests/task_conflicts.rs`, `crates/shadows/tests/task_conflict_routes.rs`.
- Modify `crates/core-runs/src/lib.rs`, `crates/core-runs/src/model.rs`,
  `crates/core-runs/src/error.rs`, `crates/core-runs/src/report.rs`,
  `crates/core-runs/src/notify.rs`, `crates/core-runs/src/stop.rs`,
  `crates/core-runs/src/watch.rs`, `crates/core-runs/src/start.rs`, `crates/core-runs/src/amend.rs`,
  `crates/core-runs/src/recover.rs`, `crates/core-runs/src/store/mod.rs`,
  `crates/core-runs/src/store/recover.rs`, `crates/core-runs/contract.yaml`, `crates/core-db/contract.yaml`.
- Modify `crates/shadows-core/src/lib.rs`, `crates/shadows-mcp/src/run_tools.rs`,
  `crates/shadows-mcp/src/server.rs`, `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`,
  `crates/core-harness/src/executor.md`, `crates/core-harness/src/prompt.txt`,
  `crates/core-harness/contract.yaml`, `crates/shadows-http/src/runs.rs`,
  `crates/shadows-http/src/lib.rs`, `crates/shadows-http/src/openapi.rs`,
  `crates/shadows-http/src/contract.yaml`, `api/openapi.json`.
- Existing regressions `crates/shadows-core/tests/runs_attempts.rs` and its
  `crates/shadows-core/tests/runs_attempts/planner.rs` module.

**Interfaces:** Runs retains conflict_for, conflict and stop_for from the agreement.
ConflictInput/Evidence and bounds stay. TaskConflict exposes actual attempt_state and
scope_held, not a separate stop state machine.

- `pub async fn conflict_for(&self, grant: &Grant, input: ConflictInput) -> Result<TaskConflict, RunsError>`;
  `pub async fn conflict(&self, attempt: &AttemptId) -> Result<Option<TaskConflict>, RunsError>`;
  `pub async fn stop_for(&self, grant: &Grant, command_id: String, attempt: &AttemptId) -> Result<Attempt, RunsError>`.
  Validate evidence through Codebase::missing_inputs, permitting returned missing paths
  as evidence while preserving its canonical/grant/secret refusals.

- Migration 0062 adds nullable conflict_json, conflict_at, conflict_delivery and
  conflict_notice_json columns to run_attempts. No new table. Delivery is
  pending/delivered/no_planner; conflict_notice_json is one enqueue intent, resolved once.
  Historical attempts keep null fields. The public conflict read derives current
  AttemptState and scope_held from the attempt's state and released_at.
- In one write recheck live executor identity and classify normalized command replay
  before state refusals; only a new conflict needs the current attempt to be Running.
  Store conflict + Pending delivery + command result together. Keep run.conflict/schema 1,
  scope Attempt/id, principal Executor/thread. One conflict per attempt; a distinct
  command is contract_conflict and changed replay is CommandConflict.
- record_report checks absence of conflict_json in its own write. Report/conflict has
  one winner. A late conflict cannot overwrite a committed report. After acknowledgment
  the executor stops work and ends its turn; acknowledgment is not confirmed termination.
  A report losing that race returns contract_conflict rather than a generic NotFound,
  including when the conflict was accepted between report preflight and its final write.
- Reuse notify after recording the conflict, so the Planner hears it while the attempt
  may still be Running. An enqueue failure keeps Pending without undoing acceptance.
  Planner instructions say to assess impact and use task_stop if this or another affected
  executor has not ended. Do not wait for cooperative ending before informing the Planner.
- Existing watch confirms the turn's end. With conflict_json it settles Stopped/reason
  contract conflict instead of generic ended-without-commit, then closes the session and
  releases the scope. Conflict presence and terminal arbitration share that write.
  A prior explicit Stop retains its confirmed outcome. No automatic external Stop task.
- Change private Runs::release to Result<(), RunsError> and update its listed callers:
  failed close/termination keeps the SQL scope. A nonending conflicted executor remains
  Running and held. Keep uncommitted files; never reset or auto-commit them.
- notify stays the single notice path, returning Result<(), RunsError>. Save its complete
  recipient/SendTurn/Notice once before first enqueue; retry the same notify-<attempt_id>
  command and payload. AttemptNotice gains optional conflict; ordinary notices and
  executor provenance stay unchanged. The small intent on the attempt prevents changed
  Planner settings or terminal state from changing a replay's fingerprint.
  Private ConflictNoticeIntent contains planner ThreadId, body, model, mode, effort and
  card JSON. Rebuild existing SendTurn/Notice with fixed command/executor identity and
  empty attachments/focus/plan/client_tab; no new serde requirement on Turns' models.
- Successful queue_notice, including normal replay, marks Delivered. Errors keep Pending.
  Retry through notify at existing turn-end, explicit Stop, and one pass of existing
  Runs::recover after normal reconciliation. Recovery may defer that pass off construction;
  it never waits for a model reply or restarts an executor. No timer/backoff loop, new
  AppCore lifecycle hooks or extra Turns reconciliation method.
- A crash after enqueue but before marking Delivered replays queue_notice without a
  second message. Missing/deleted owning Planner is NoPlanner; temporary enqueue or
  invocation errors stay Pending and visible until a later lifecycle delivery attempt.
  Do not promise continuous retries without further activity.
- Startup still maps unfinished attempts to Interrupted, preserving conflict/intent.
  notify must not append a second generic terminal notice for the same conflict.
- Planner stop_for keeps same-project Thread authorization, command replay and existing
  confirmed Stop (run.stop/schema 1, Attempt/id). It can terminate a nonending executor.
  MCP task_conflict/task_stop keep agreed inputs; neither is external-project authority.
  HTTP conflict read is TaskConflict|null, unknown attempt 404.

- [ ] **1. Tests:** conflict_and_report_admit_one_outcome, conflict_ack_is_not_a_confirmed_stop,
  conflict_notifies_while_its_executor_is_running, watch_settles_conflict_without_stopping_siblings,
  failed_stop_or_close_keeps_the_scope, restart_replays_pending_conflict_once,
  changed_planner_settings_keep_original_notice, planner_can_stop_a_nonending_executor,
  conflict_keeps_uncommitted_files, executor_conflicts_cannot_approve.
  Use real SQLite/Git and existing owner lifecycle hooks; real termination cases use the
  live adapter, not a newly scripted fake worker.
- [ ] **2. RED:** `cargo test -p shadows-core --test task_conflicts`;
  `cargo test -p shadows --test task_conflict_routes`; owning transaction tests.
- [ ] **3. Implement** the attempt columns, existing watch/notify/recover integration,
  report arbitration, explicit Planner Stop, reads and prompts.
- [ ] **4. GREEN:** Step 2, existing runs_attempts/planner notice regressions, available
  live cases, common checks and OpenAPI. Verify enqueue/crash replay without duplicates.
- [ ] **5. Commit** `feat(backend): B-147 report conflicts through the attempt lifecycle`.


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
Use generated type aliases only. Conflict polls every 2 s while attempt_state=running,
scope_held=true or delivery=pending; no process-liveness/impact rule in the client.
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
  `report_opens_the_selected_attempt_brief`, `conflict_shows_expected_observed_and_attempt_state`,
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
  `live_parallel_commits_keep_their_own_files`,
  `live_cross_scope_staging_is_not_an_accepted_report`.
  Real Go/Git folder, installed gopls, signed-in Claude executor and Claude/Codex Planner
  variants following existing language_servers_live. Assert task_brief/task_question tool
  entries, distinct code/planned facts, confirmed conflict stop and two independent
  attempts overlapping in time before either reports. Test prompts carry no finished implementation.
  The positive commit case requires both tasks to report accepted own-path commits using
  the --only procedure, with no loss of a foreign staged change. Merely rejecting a
  contaminated commit does not count as success for that case.
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
