# Slice 4: saved attempt context and contract conflicts

- Status: accepted for planning by the controller under Mohammed's design approval and
  request to prepare the implementation plan, 2026-11-14. No API is claimed implemented.
- Screen: F-102, attempt Report entries and the Planner's executor notices.
- Owner: [slice 4 design](../../backend/specs/2026-11-14-task-questions-design.md).
- Plan: [implementation](../../backend/plans/2026-11-14-slice-4-task-questions.md).
- Backend answer: existing AppCore owners implement these shapes and generate OpenAPI.
  Frontend agreement: show Core facts; no reconstructed context or client stop/readiness rule.
- Replaces only the earlier local-delivery request's slice 4 brief/question assumptions.

## Need and calls

The person inspects an attempt's original context and its contract problem. Executors
ask about code and approved producers/consumers. The Planner receives durable executor
notices, explicitly stops affected attempts and discovers compatible ready groups.

| Surface | Input | Result / owner |
|---|---|---|
| GET /api/attempts/{id}/brief | attempt id | AttemptBrief / Runs |
| GET /api/attempts/{id}/conflict | attempt id | TaskConflict or null / Runs |
| MCP task_brief | {} | AttemptBrief of authenticated executor's current attempt |
| MCP task_question | TaskQuestion | TaskAnswer of that same attempt |
| MCP task_conflict | ConflictInput | accepted TaskConflict; stop proceeds outside tool handler |
| MCP task_ready | { workflow_id } | ReadyGroup / Runs; same-project Planner Thread only |
| MCP task_stop | { command_id, attempt_id } | Attempt after confirmed Stop; same-project Planner Thread only |
| MCP task_done | existing input | DoneMark { task: TaskRun, next: ReadyTask[] }; [] replaces null |

No new HTTP question/mutation API. Existing HTTP Start/Stop/done keep their payloads.
Executor task_context becomes task_brief; Planner/external task_context stays unchanged.

## Final public types

Existing ids, TaskContext/TaskContract, ResolvedContract, ContractSource/Target,
ContextPlan, LspQuestion/Answer and Attempt keep their owning types. Adapters do not
copy fields. New types live in the job modules named by the plan and are re-exported.

| Type | Fields / wire rules |
|---|---|
| AttemptBrief | attempt: Attempt; context: TaskContext; captured_at: string; consumers: ConsumerRef[]; consumers_more: boolean. B-107 builds the first three; B-109 adds exact-version derived consumers (first 50). |
| ContractSelector | internally tagged kind: own; symbol { symbol_id }; output { output_id }; use { source: ContractSource }. Symbols/outputs belong to the own task; use matches a declared source. |
| ConsumerRef | task: u32; title: string; output: string or null (producer output id for local task outputs; null for an exact Existing-source use); use_index: u32; purpose: string. Planned reference, never source location. |
| ContractFacts | selector: ContractSelector; contract: TaskContract or null (selected projection); resolved: ResolvedContract[]; producers: ContractTarget[]; consumers: ConsumerRef[]; next: u32 or null. |
| TaskQuestion | selector: ContractSelector default own; code: LspQuestion or null default null; after: u32 or null default null; limit: u16 default 20, range 1..50. after is a consumer-list offset. |
| ProducerStatus | target: ContractTarget; done: boolean; latest_commit: string or null. Neither mark nor commit proves a source lookup found the symbol. |
| TaskAnswer | attempt_id: AttemptId; plan: ContextPlan; task: u32; planning: ContractFacts; producer_status: ProducerStatus[]; code: LspAnswer or null; code_error: string or null; observed_at: RFC3339 string; planned_scope: literal this_frozen_version. |
| ConflictEvidence | path: project-relative string; line: positive one-based u32 or null; summary: string. Reference/explanation, not copied whole source. |
| ConflictInput | command_id: string; expected: string; observed: string; evidence: ConflictEvidence[]. |
| ConflictStop | snake_case requested, stopped, interrupted or failed. Failed stop never implies scope release. |
| ConflictDelivery | snake_case pending, delivered or no_planner. Delivered means durable enqueue, not a model reply. |
| TaskConflict | attempt_id: AttemptId; project_id: ProjectId; workflow_id: WorkflowId; version: i64; task: u32; expected: string; observed: string; evidence: ConflictEvidence[]; stop: ConflictStop; stop_error: string or null; delivery: ConflictDelivery; created_at: RFC3339 string. Identity comes from the attempt, never the mutation input. |
| ReadyTask | task: u32; title: string; suggested_model: string or null. |
| ReadyGroup | workflow_id: WorkflowId; version: i64; tasks: ReadyTask[] ascending by task number, pairwise write-compatible. |
| AttemptNotice | existing fields plus conflict?: TaskConflict, omitted for ordinary notices; never the person's approval. |

Saved TaskContext is immutable. Attempt outcome/report/commit and producer statuses
are current facts. Brief consumers derive only from its original Frozen version; later
versions/plans do not enter the list. Order is (task, uses index); consumers_more exposes
the bounded summary. The executor pages task_question; the person can inspect the
referenced Frozen plan. Empty local results do not mean the whole project has no consumer.
Code locations/encoding preserve native Codebase semantics and bounds.

## Bounds and errors

- TaskAnswer <=64 KiB serialized, code_error <=2 KiB UTF-8. Preserve LSP truncation and
  consumer pagination. An oversized selected contract is question_too_large, never a cut signature.
- Conflict: trimmed expected/observed each 1..2,000 Unicode scalars; 1..10 evidence items;
  summary 1..1,000; path <=1,024; optional line positive. Normalize paths before fingerprint;
  preserve evidence order. Normalized payload <=16 KiB. Secret/escaping paths are refused.
- Unknown attempt: existing 404. Historical attempt without context: HTTP 409 RunRefusal
  { code: context_not_recorded, reason }; show that limitation rather than reconstructing.
- Invalid selector/offset/limit: invalid_question. Invalid conflict/evidence: invalid_conflict.
  Another distinct command after accepted conflict: contract_conflict. Same id with changed
  normalized payload: command_conflict. Existing grant/path refusals remain refusals.
- Start: missing_input for absent required files; input_unavailable for unreadable/invalid
  required inputs. IO errors do not imply absence. Unavailable LSP remains code_error.
- Stop failure retains held scope and exposes stop=failed/stop_error. Null conflict means
  no recorded issue, not successful work. no_planner is visible to the person.

## Presentation and refresh

Each Report entry opens its own BriefPanel/ConflictPanel, independent of the latest
plan. Show provider text as plain preserved text/dir auto. NoticeItem's optional conflict
links to that task/attempt Report. Copied notices are history with no mutation controls.

Use ['runs','brief',id] and ['runs','conflict',id]. Poll conflict every 2 s while
stop=requested/failed or delivery=pending. Stop polling when settled/no_planner. Current
conflict read is authoritative; the delivered card is an immutable snapshot. No new
EventSource, frontend scheduler, automatic sibling stops or retry button.
