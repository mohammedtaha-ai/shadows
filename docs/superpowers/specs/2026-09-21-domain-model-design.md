# Section 4 — Core Domain Model and Invariants

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

## 4.1 Identity types

Domain IDs are UUID-v4 newtypes unless a later decision explicitly changes one:

> **CLOSED — 2026-09-22.** Milestone 0 carried `String` ids against this rule while
> the modules that own them did not yet exist. All five now do, and the ids are
> newtypes: `ProjectId`, `ThreadId`, `ThreadEntryId`, `OperationId`,
> `RuntimeInstanceId`, each declared by the shared pattern in `src/id.rs`.
>
> The block was opened because `mark_operation_started(op_id, expected_runtime)`
> took two `String`s the compiler could not tell apart. That call now takes
> `(&OperationId, &RuntimeInstanceId)`, and passing them swapped is a compile
> error — verified, not assumed. Two further cases the sweep found and closed the
> same way: `Storage::list_thread_entries` accepted any `String`, so a project id
> reached it silently; and `DurableEvent`'s `with_project`/`with_thread`/
> `with_operation` took `impl Into<String>`, so an event could be scoped to the
> wrong entity — and since one event row is visible through several scopes
> (§4.2), that is a row appearing in the wrong replay and missing from the right
> one, with nothing failing.
>
> One case is **reduced, not closed.** `Storage::append_thread_entry` took five
> consecutive `&str`. Its fields now arrive as one named struct, so a swap must be
> written out as `kind: <body text>` instead of happening silently by position.
> `kind` was still `&str` like `body` until Milestone 2 typed it as
> `ThreadEntryKind` (§4.2's decided block), which closes it.
>
> Why the ids are not `sqlx` types: CLAUDE.md keeps persistence imports out of
> domain types, so every id converts to a column at the `storage/sqlite/` boundary
> and nowhere else.

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
    directory: Option<ProjectDirectory>,
    default_config_ref: Option<ConfigRef>,
    created_at: Timestamp,
}
```

`directory` is where the project's turns run (§8.3). It is chosen by the user,
checked by the daemon when the project is created (absolute, exists, is a
directory), and stored canonical — on Windows without the `\\?\` prefix — so two
spellings of one folder are one directory. It is not the project's identity
(§11.1). `None` only for a project created before projects owned a directory:
none can be backfilled, so such a project's turns fail at Prepare rather than
run in the daemon's working directory.

A project can be **removed** (`DELETE /api/projects/{id}`, command
`project.remove`). It is a soft remove: the durable log references the row
and is never erased, so the row stays with `removed_at` set. Removal is
refused while the project holds any planning thread (`ProjectHasThreads`,
409), and nothing is written. Otherwise, in one write, its code index and its
links both ways go (§15.4), its live project grants are revoked (§13.7), and
`ProjectRemoved` is journaled. Afterwards it is NotFound everywhere: it is not
listed, and every request naming it — a thread, a grant, a turn, a code
question, a link — is refused as not found. Its slug stays taken: adding the
same folder again needs another slug.

### PlanningThread

```rust
struct PlanningThread {
    id: ThreadId,
    project_id: ProjectId,
    title: String,
    status: PlanningThreadStatus, // Open | Closed
    harness_session_id: Option<String>,
    created_at: Timestamp,
}
```

`harness_session_id` is the harness session the thread's turns continue
(evidence `SERVE_STREAM_SPIKE.md` Finding 3: `--session-id` on the first turn,
`--resume` with the same id after). The daemon chooses it and records it when a
turn that started it reaches the harness's turn-end — not at spawn, because a
turn that fails or is stopped earlier may leave no session, and a recorded id
that `--resume` rejects would fail every later turn. Clients never supply or see
it. The harness store is the source of truth for what the model remembers; the
thread's entries are the source of truth for what the user sees (see
*Continuity* below).

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

> **Note — `author: Principal` is `events::Actor` in the implementation.**
>
> `Actor` already has exactly this shape (`kind` + `id`) and already answers "who
> did this" for durable events, which is the same question. Declaring a second
> identical struct would record one decision twice. If the two ever need to
> diverge — a principal gaining fields an event actor must not carry — that is the
> point to split them, and this note is where to say so.

> **DECIDED 2026-09-25 — Milestone 2 closes this.** The trigger had already
> fired unnoticed: Milestone 1's web client renders `UserMessage` and
> `PermissionRefused` entries differently. Milestone 2 adds `PlanView` and
> `PlanApproved` (§13.9), which the client also branches on.
> `ThreadEntryKind` becomes an enum naming every value storage already holds —
> the code writes `UserMessage`, `AgentMessage` and `PermissionRefused` at the
> time of writing, and the migration checks the database holds no other — plus
> those two. (`Agent`, `System` and `User` are `Actor` kinds, the author, not
> entry kinds.)
> Stored text is not rewritten: each variant serialises to its current string.
> What follows is the reasoning that kept it open until now.
>
> **Was: `ThreadEntryKind` has no variants anywhere in this spec.**
>
> The field is typed here and its permitted values are never listed, so the
> implementation carries it as text. That is not laziness: enumerating them in
> `thread/` would decide a question this spec has not asked, and a wrong early
> enum is worse than text because migrating stored values costs more than adding
> the type later.
>
> It also leaves one hazard open. `append_thread_entry`'s `kind` and `body` are
> both `&str`, so the compiler cannot tell them apart; the named-field struct makes
> a swap visible but not impossible (§4.1). Typing `kind` closes it completely.
>
> **Trigger that closes this:** the first feature that branches on an entry's kind
> rather than storing and displaying it — Milestone 0's web client renders every
> entry the same way, so the milestone does not reach it. The harness evidence
> report already names the candidate set it would have to cover: a durable line is
> an `assistant` or `user` message, and its content blocks are text, thinking,
> tool_use or tool_result.

```rust
enum EntryRef {
    Decision(DecisionId),
    Research(ResearchId),
    Workflow(WorkflowId),
    Task(TaskId),        // Milestone 2: a message about one task (§13.9)
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

Milestone 2 adds a plan's `version`, `revision`, `title` and `goal`, a task's
`number`, and two kinds of link with a label (§13.2, §13.3, §13.15).

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
