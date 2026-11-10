# Local delivery APIs and task model

- Status: accepted for planning by the controller under Mohammed's 2026-11-09
  design approval; this is not a claim that the APIs exist.
- Screen: companion frontend plan, tasks F-99 to F-105.
- Owner spec: [17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09).
- Backend answer: existing AppCore owners implement the operations below.
- Frontend agreement: show Core's states; no duplicated readiness/acceptance rules.
- All IDs already defined by an owner remain that owner's types, never a new
  shared-id crate. All paths below are project-relative; result lines are
  one-based and UTF-16 columns zero-based. Unknown optional values are null.

## New model contracts

Rust public types live in the owner named below, derive serde/utoipa/schemars as
their transport needs require, and have private stores. This is a specification
of fields, not permission to hand-edit generated OpenAPI or schema.d.ts.

### Code: inventory and semantic access

- InventoryQuery { prefix: String = "", after: Option<String>, limit: u16 = 100 }.
- InventoryKind = directory | file | excluded.
- InventoryEntry { path: String, kind: InventoryKind, language: Option<String>,
  excluded_reason: Option<String>, has_symbols: bool }.
- InventoryPage { state: missing | indexing | ready | incomplete,
  generation: u64, entries: Vec<InventoryEntry>, next: Option<String>,
  observed_at: Option<String> }. Cursor encodes generation/prefix; reject stale
  cursor with inventory_stale, not mixed pages. Maximum limit 200.
- LspConfig { server: String, executable: String, args: Vec<String>,
  enabled: bool }. executable is an absolute executable path; args do not pass
  through a shell. Server ids are the 13 plugin slugs in section 15.
- LspStatus { server: String, state: missing | configured | starting | ready |
  unsupported | failed, version: Option<String>, capabilities: Vec<String>,
  reason: Option<String> }. Configuration alone never returns ready.
- SemanticQuery { kind: definition | references | outline | diagnostics,
  path: String, name: Option<String>, line: Option<u32>,
  character: Option<u32> }. Require path plus either name or position for
  definition/references. Multiple same-name candidates return ambiguous.
- SemanticHit { path: String, line: u32, end_line: u32, name: String,
  signature: Option<String>, excerpt: Option<String>, source_hash: String,
  matched_by: semantic | name }.
- SemanticAnswer { state: found | not_found | ambiguous | stale | unsupported |
  unavailable, hits: Vec<SemanticHit>, more: bool, backend: Option<String>,
  reason: Option<String> }. At most 50 hits and 32768 excerpt bytes; a source
  file over the existing 1 MiB indexing limit returns unsupported for excerpts.
  Read scope filtering occurs before source/metadata disclosure.

### Plans: detailed contracts

Add TaskContent.contract: Option<TaskContract>, omitted when None. Old canonical
serialized content remains byte-for-byte unchanged when no contract is supplied.
Existing reads/writes remain the permission scopes, including legitimate globs.

- TaskContract { files: Vec<FileWork>, symbols: Vec<SymbolWork>,
  produces: Vec<ProducedOutput>, uses: Vec<ContractUse>,
  steps: Vec<String>, invariants: Vec<String>, forbidden: Vec<String>,
  checks: Vec<CheckSpec>, decisions: Vec<DecisionPin> }.
- FileWork { path: String, action: read | create | modify | delete,
  purpose: String }.
- SymbolWork { id: String, path: String, kind: function | method | class,
  name: String, inputs: Vec<ContractValue>, returns: ContractValue,
  behavior: Vec<String>, errors: Vec<String> }.
- ContractValue { name: String, type_name: String, meaning: String }.
- ProducedOutput { id: String, symbol_id: Option<String>,
  artifact: Option<String>, acceptance: Vec<u32> }. Exactly one symbol_id or
  artifact; symbols/files/items must belong to the task.
- ContractTarget = Local { task: u32 } | Pinned { workflow: WorkflowId,
  version: i64, task: u32 }.
- ContractSource = Task { target: ContractTarget, output: String } |
  Existing { path: String, symbol: String } |
  Agreement { id: AgreementId, version: i64, operation: String }.
- ContractUse { source: ContractSource, purpose: String }.
- CheckSpec { id: String, program: String, args: Vec<String>, cwd: String,
  timeout_seconds: u32 = 300, acceptance: Vec<u32>, baseline: bool = false }.
  Timeout range 1..3600; program/args are separate. cwd cannot escape project.
- DecisionPin { id: String, version: i64 }.
- ResolvedContract { reference: ContractSource, producer_title: Option<String>,
  symbol: Option<SymbolWork>, artifact: Option<String> }.
  Same-version references participate in the existing dependency graph.
  Cross-plan pins are exact and same-project; shared agreement reach follows
  its existing owner. Do not copy a producer signature into the consumer.

### Design: minimal decisions

- DecisionInput { id: Option<String>, expected_version: Option<i64>,
  title: String, decision: String, rationale: String, source: String }.
- DecisionRecord { id: String, version: i64, project_id: ProjectId,
  title: String, decision: String, rationale: String, source: String,
  state: proposed | accepted | superseded, approved_by: Option<ApprovedBy> }.
  Every revision immutable; acceptance/supersession follows section 24.
  Task pins must reference an accepted revision.

### Runs: preparation, briefs, checks and reviews

- PrepareRequest { command_id: String, source_thread: ThreadId,
  source_message: ThreadEntryId, framework: String, destination: String,
  instructions: String, model: String }. Source ownership is checked; a
  model-supplied source message is not accepted without the owning thread.
- Preparation { id: String, project_id: ProjectId, thread_id: ThreadId,
  state: running | completed | failed | stopped | interrupted,
  destination: String, changed_paths: Vec<String>, report: Option<String>,
  commit: Option<String>, reason: Option<String> }.
- ContractAvailability { contract: ResolvedContract, ready: bool,
  reason: Option<String> }. Owned by Runs: Plans resolves the obligation and
  Runs joins its runtime acceptance. Plans never depends on Runs.
- ExecutionBrief { attempt_id: AttemptId, workflow_id: WorkflowId, version: i64,
  task: TaskContent, dependencies: Vec<ContractAvailability>,
  decisions: Vec<DecisionRecord>, base_commit: Option<String>,
  map_revision: i64, standards: ContextStandards, model: String }.
  Persist once per attempt; no separate hashed packet or source-copy store.
- TaskQuestion { attempt_id: AttemptId, query: SemanticQuery }.
- TaskAnswer { code: SemanticAnswer, obligations: Vec<ContractAvailability>,
  problem: Option<String> }.
- CheckRun { id: String, attempt_id: AttemptId, check: String,
  phase: before | after, state: queued | running | passed | failed |
  cancelled | interrupted | inconclusive, exit_code: Option<i32>,
  tested_head: Option<String>, output: String, reason: Option<String> }.
  Output cap 65536 bytes per stream with visible truncation.
- SourceRequest { path: String, start_line: u32 = 1, max_lines: u32 = 200 }.
  cap 500 lines/32768 bytes, read by recorded candidate SHA, not live files.
- SourceAnswer { commit: String, path: String, start_line: u32,
  text: String, truncated: bool }.
- ItemReview { number: u32, result: met | not_met | pending,
  evidence: String, waits_for: Option<ContractTarget> }.
- ReviewInput { attempt_id: AttemptId, commit: String,
  outcome: pass | pass_with_notes | fail_implementation | fail_plan,
  items: Vec<ItemReview>, summary: String }.
- ReviewRecord = ReviewInput plus id, critic_thread: ThreadId, created_at: String.
- AttemptAssessment { attempt_id: AttemptId, checks: Vec<CheckRun>,
  review: Option<ReviewRecord>, state: awaiting_checks | checking |
  awaiting_review | reviewing | accepted | needs_person | failed | interrupted }.
- RepairCommand { command_id: String, instruction: String }.
- RepairState { attempt_id: AttemptId, number: u32, state: waiting_planner |
  retrying | investigating | stopped_for_person | pending_dependency,
  reason: String, owner_thread: Option<ThreadId> }.
- ExecutionStatus { allowed: bool, reason: Option<String> }; Plans returns
  false for Draft/superseded/archive, including an approved version with a
  newer Draft on its plan.

## HTTP and MCP operations

Rust enum names for the state fields above are InventoryState, LspState,
SemanticState, PreparationState, CheckState, AssessmentState, RepairPhase and
AcceptanceSource. Their variants use PascalCase for the specified snake_case
wire values. Actions are FileAction; symbol kinds are SymbolKind. Add
CodeError::code(&self) -> &'static str for stable Code refusal mappings.

Each route handler calls one owning service. Existing route/status behavior
remains unless explicitly extended. Mutations use existing CommandId replay/
normalized fingerprint rules; this is not a new contract-fingerprint engine.

| HTTP | Request | Result / owner | Planner MCP |
|---|---|---|---|
| GET /api/projects/{id}/code/files | InventoryQuery | InventoryPage / Code | project_files |
| POST /api/projects/{id}/code/refresh | no body | InventoryPage / Code | project_refresh |
| GET /api/projects/{id}/code/servers | none | Vec<LspStatus> / Code | project_servers |
| PUT /api/projects/{id}/code/servers/{server} | command_id + LspConfig | LspStatus / Code | none |
| POST /api/projects/{id}/code/question | SemanticQuery | SemanticAnswer / Code | project_question |
| POST /api/projects/{id}/preparations | PrepareRequest | Preparation / Runs | project_prepare |
| GET /api/projects/{id}/preparations | none | Vec<Preparation> / Runs | none |
| GET /api/preparations/{id} | none | Preparation / Runs | preparation_get |
| POST /api/preparations/{id}/stop | command_id | Preparation / Runs | none |
| GET /api/projects/{id}/decisions | none | Vec<DecisionRecord> / Design | decision_list |
| POST /api/projects/{id}/decisions | command_id + DecisionInput | DecisionRecord / Design | decision_propose |
| POST /api/projects/{id}/decisions/{decision}/accept | command_id + version | DecisionRecord / Design | decision_accept under section 24 |
| GET /api/attempts/{id}/brief | none | ExecutionBrief / Runs | task_brief for executor |
| GET /api/attempts/{id}/assessment | none | AttemptAssessment / Runs | none |
| POST /api/attempts/{id}/checks | command_id | AttemptAssessment / Runs | none |
| GET /api/attempts/{id}/repair | none | RepairState or null / Runs | none |
| POST /api/attempts/{id}/retry | RepairCommand | Attempt / Runs | task_retry for owner Planner |
| POST /api/attempts/{id}/accept-notes | command_id | AttemptAssessment / Runs | none |

Existing GET workflow/task endpoints expose optional contract. Existing plan_edit
carries it inside task_add/task_update. Existing start takes optional critic_model,
stored per attempt; if omitted no automated review is claimed until a compatible
critic model is selected. Add task_question for executor, task_source/task_review
for critic, and task_stop for the owner Planner. These scoped tools call Runs.
project_question/project_files are not offered to executor/critic grants.
Rich task_report accepts attempt_id; retired-attempt reports cannot target a retry.

## Errors, lifecycle and API generation

Use current envelope/codes for existing refusal cases. Add stable codes
inventory_stale, path_outside_project, lsp_unavailable, missing_contract_input,
outside_task_reads, plan_replanning, verification_busy and stale_review as
applicable. UI shows Core's reason; unsupported capability is visible.

Backend answer: accepted with the model above; same-origin/grant boundaries
are checked in their owning services. Frontend answer: accepted for planning;
each UI task remains blocked until its endpoint exists in generated OpenAPI.
No HTTP source-reading endpoint is added for arbitrary host paths.
B-120: TaskContent refuses an unknown key (`additionalProperties: false`), as TaskContract does.

Regenerate api/openapi.json only in a task changing routes/serialized public
types; regenerate web/src/api/schema.d.ts with npm run gen:api after that task.

