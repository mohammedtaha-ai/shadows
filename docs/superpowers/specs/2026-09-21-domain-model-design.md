# Section 4 — Core Domain Model and Invariants

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

## 4.1 Identity types

Domain IDs are UUID-v4 newtypes unless a later decision explicitly changes one:

> **OPEN — Milestone 0 carries `String` ids, deliberately.**
>
> Task 3 landed `DurableEvent` with `event_id`, `project_id`, `thread_id`,
> `operation_id`, and `correlation_id` as `String`, contradicting this rule. It is
> recorded here rather than left as a silent divergence.
>
> The reason is structural, not convenience: `DurableEvent` references entities whose
> modules do not exist yet. `ProjectId` belongs in `project/`, `ThreadId` in `thread/`,
> `OperationId` in `operation/` — created in Milestone 0 Tasks 5, 6, and 9. Defining
> them anywhere else to satisfy the rule earlier would break a rule that costs more:
> no module is created before the task that fills it.
>
> **Trigger that closes this:** Task 9, which creates the last of those modules and is
> also the first task whose signatures place two ids of different kinds adjacent —
> `mark_operation_started(op_id, expected_runtime)` takes two `String`s that the
> compiler cannot tell apart. That is where `String` stops being cosmetic and starts
> being a defect the type system was supposed to catch. Task 9's dispatch carries this
> block; the newtypes land with it and sweep the earlier signatures.

```text
ProjectId
ThreadId
ThreadEntryId
DecisionId
ResearchId
WorkflowId
TaskId
GateId
VerificationCheckId
OperationId
AgentInvocationId
VerificationRunId
VerdictId
EventId
CommandId
RuntimeInstanceId
CorrelationId
```

Opaque cursors and ordinals are **not** UUID IDs:

```text
EventCursor
ThreadEntryOrdinal
```

## 4.2 Core entities

### Project

```rust
struct Project {
    id: ProjectId,
    slug: ProjectSlug,
    name: String,
    default_config_ref: Option<ConfigRef>,
    created_at: Timestamp,
}
```

### PlanningThread

```rust
struct PlanningThread {
    id: ThreadId,
    project_id: ProjectId,
    title: String,
    status: PlanningThreadStatus, // Open | Closed
    created_at: Timestamp,
}
```

### ThreadEntry

```rust
struct ThreadEntry {
    id: ThreadEntryId,
    thread_id: ThreadId,
    ordinal: ThreadEntryOrdinal,
    kind: ThreadEntryKind,
    author: Principal,
    body: String,
    refs: Vec<EntryRef>,
    created_at: Timestamp,
}
```

`ThreadEntryOrdinal` is strictly increasing within one thread and is independent from durable-event sequence.

```rust
enum EntryRef {
    Decision(DecisionId),
    Research(ResearchId),
    Workflow(WorkflowId),
    Operation(OperationId),
    Verdict(VerdictId),
}
```

### Decision

Project-level:

```rust
struct Decision {
    id: DecisionId,
    project_id: ProjectId,
    title: String,
    summary: String,
    references: Vec<DecisionRef>,
    status: DecisionStatus,
    decided_by: Option<Principal>,
    created_at: Timestamp,
    updated_at: Timestamp,
}
```

A `Decision` has no `thread_id` source-of-truth field.

### ResearchArtifact

```rust
struct ResearchArtifact {
    id: ResearchId,
    project_id: ProjectId,
    title: String,
    source: Option<String>,
    summary: String,
    created_at: Timestamp,
}
```

### Workflow

```rust
struct Workflow {
    id: WorkflowId,
    thread_id: ThreadId,
    state: WorkflowState,
    previous_version_id: Option<WorkflowId>,
    created_at: Timestamp,
    updated_at: Timestamp,
    frozen_at: Option<Timestamp>,
}
```

```rust
enum WorkflowState {
    Draft,
    Approved,
    Frozen,
    Running,
    Completed,
    Failed,
}
```

The normalized task/edge/gate/check rows are the scheduler's authoritative DAG representation.

If an authored plan snapshot is preserved for provenance, it is named explicitly (for example `source_plan_json`) and is **not** consulted as a second runtime DAG source.

### Task

```rust
struct Task {
    id: TaskId,
    workflow_id: WorkflowId,
    contract: TaskContract,
    scope: DeclaredScope,
    state: TaskState,
    created_at: Timestamp,
    updated_at: Timestamp,
}
```

```rust
enum TaskState {
    Pending,
    Ready,
    InProgress,
    Completed,
    Failed,
    Blocked,
}
```

`InProgress` is business progress, not an OS fact. It begins when dispatch
atomically claims a `Ready` task and creates its `Pending` execution
`Operation`, and it continues through agent execution and required
verification. Whether a process has actually spawned is described by
`OperationStatus` alone, which keeps its own `Running`. A `Pending` Operation
must never force the Task model to claim that a process is already running.

Task state is persisted because deriving scheduler state correctly from multiple attempts, verification, blocking, and recovery is complex.

### TaskContract

Frozen with the workflow version.

Conceptually contains:

```text
intent
inputs
expected outputs
read scope
write scope
network capability
timeout/budget
required checks
```

### Gate and VerificationCheck

```text
Gate = workflow timing/grouping anchor
VerificationCheck = deterministic check definition
```

```rust
enum CheckKind {
    Build,
    Test,
    Contract,
    Baseline,
}

enum CheckPhase {
    Task,
    Regression,
    Deferred,
    Final,
}
```

A check belongs to exactly one task or gate.

### Operation

The **domain** uses a typed status enum. It does not expose persistence JSON columns.

```rust
struct Operation {
    id: OperationId,
    kind: OperationKind,
    status: OperationStatus,

    thread_id: Option<ThreadId>,
    workflow_id: Option<WorkflowId>,
    task_id: Option<TaskId>,

    runtime_instance_id: RuntimeInstanceId,

    cancellation_request: Option<CancellationRequest>,

    created_at: Timestamp,
    started_at: Option<Timestamp>,
    finished_at: Option<Timestamp>,
}
```

```rust
enum OperationKind {
    PlannerTurn,
    ExecutionRun,
    Verification,
    Research,
    Git,
}
```

Reviewer remains a separate future AI-operation seam and is not a deterministic verification kind.

```rust
enum OperationStatus {
    Pending,
    Running,
    Completed {
        outcome: OperationOutcome,
    },
    Failed {
        failure: OperationFailure,
    },
    Cancelled,
    Interrupted {
        reason: InterruptReason,
        previous_runtime_instance: RuntimeInstanceId,
    },
}
```

Storage may flatten this into columns such as `status_kind`, `outcome_json`, and failure fields. Those are storage representation, not domain shape.

### AgentInvocation

```rust
struct AgentInvocation {
    id: AgentInvocationId,
    operation_id: OperationId,

    role: Role,
    harness: HarnessKind,
    profile: HarnessProfile,

    native_session_id: Option<String>,
    created_at: Timestamp,
}
```

```rust
enum Role {
    Planner,
    Executor,
    Reviewer,
    Researcher,
}
```

Relationship lives on `AgentInvocation.operation_id`; `Operation` does not store `agent_invocation_id`.

### VerificationRun

```rust
struct VerificationRun {
    id: VerificationRunId,
    operation_id: OperationId,
    check_id: VerificationCheckId,
    created_at: Timestamp,
}
```

Its lifecycle and timestamps come from the associated `Operation`.

### Verdict

One final verdict per verification run in v1:

```rust
struct Verdict {
    id: VerdictId,
    run_id: VerificationRunId,
    passed: bool,
    evidence_ref: EvidenceRef,
    discrepancies: Vec<Discrepancy>,
    created_at: Timestamp,
}
```

### RuntimeInstance

```rust
struct RuntimeInstance {
    id: RuntimeInstanceId,
    version: String,
    started_at: Timestamp,
}
```

A runtime may touch many projects. It is not a child of `Project`.

### DurableEvent

```rust
struct DurableEvent {
    event_id: EventId,
    seq: DurableSeq,
    kind: EventKind,

    project_id: Option<ProjectId>,
    thread_id: Option<ThreadId>,
    workflow_id: Option<WorkflowId>,
    operation_id: Option<OperationId>,

    actor: Option<ActorRef>,
    causation: Option<CausationRef>,
    correlation_id: Option<CorrelationId>,

    payload: EventPayload,
    created_at: Timestamp,
}
```

`seq` is globally ordered in the SQLite v1 implementation. Gaps are allowed.

One event row may be visible through several scopes.

### CommandRecord

`CommandRecord` belongs conceptually to `command/`, not `storage/`.

It records external mutation idempotency:

```text
principal
command scope
command ID
command kind
request fingerprint
logical outcome reference
recorded_at
```

The fingerprint is derived by the command/application layer from command kind,
command schema version, and a normalized typed request payload. It is not a
digest of arbitrary raw JSON bytes.

Not every command creates an `Operation`.

---

## 4.3 Core invariants

### Workflow

- Frozen workflow definition is immutable.
- No unfreeze.
- New design version points to the previous version.
- Only one successor per version in v1.
- A version that has a successor receives no new task dispatches.
- Existing operations from the older version may finish or be explicitly cancelled.

### Operation

Normal lifecycle:

```text
Pending -> Running -> Completed
                   -> Failed
                   -> Cancelled
                   -> Interrupted

Pending -> Failed(stage=Spawn)
Pending -> Interrupted
Pending -> Cancelled         # cancellation confirmed before Running
```

Recovery uses exact CAS:

```text
operation ID
+ expected status
+ expected runtime instance
```

### Task

`Task.state` is authoritative scheduler state and is persisted.

Execution-attempt history lives in `Operation` rows.

At most one active `ExecutionRun` (`Pending` or `Running`) may exist per task.

### Scope enforcement

```text
EffectiveScope ⊆ DeclaredScope
```

The scheduler/application determines the required/effective scope.

Execution must block **before side effects** if the runtime cannot prove/enforce the required scope.

This is not a generic SQL `CHECK`; filesystem/network enforcement belongs to application/execution/process policy.

### Atomicity

Every durable mutation is committed atomically with its durable event(s).

External mutation:

```text
state
+ durable event(s)
+ CommandRecord
= one transaction
```

Internal mutation:

```text
state
+ durable event(s)
= one transaction
```

When two persisted entities form one scheduler invariant, they are updated in one use-case-specific transaction.

Example:

```text
create ExecutionRun(Pending)
+ Task Ready -> InProgress
+ OperationCreated
+ TaskTransitioned
= one transaction
```

No `Vec<MultiOp>` public transaction DSL is introduced.

### Idempotency

Same:

```text
principal
+ command scope
+ command_id
```

means the same logical effect/resource identity.

Idempotency also validates request identity:

```text
same key + same command kind/fingerprint
  -> replay stored outcome

same key + different command kind/fingerprint
  -> CommandConflict
```

The fingerprint uses a deterministic collision-resistant digest. The concrete
hash library is an implementation choice and is not fixed by this spec.

A replay may return the **current** resource state; it does not reproduce a frozen historical response body.

### Ordering

Domain-visible order uses explicit fields:

```text
durable event seq
thread-entry ordinal
created_at + explicit stable tie-breaker where necessary
```

Never:

```text
rowid
physical insertion order
implicit SELECT order
```

### Continuity

Durable continuity does not require a native harness session.

### Durable truth vs journal

Current entity tables are authoritative current state.

The journal is authoritative history/provenance/resync ordering, not a replacement for entity tables.

---
