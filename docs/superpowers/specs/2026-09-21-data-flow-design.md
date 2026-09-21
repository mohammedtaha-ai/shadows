# Section 2 — Data Flow

## 2.1 Create / Resume PlanningThread

A user creates a planning thread through an external command:

```text
CLI / HTTP / MCP
      ↓
application command handler
      ↓
storage atomic capability
```

The creation transaction includes, as applicable:

```text
PlanningThread state
+ initial ThreadEntry
+ durable event(s)
+ CommandRecord
= one transaction
```

If thread creation and the initial user prompt are one logical command, they must not be committed as two independently recoverable half-operations.

A resumed thread is reconstructed from durable Shadows-owned truth, not a native Claude/Codex session.

## 2.2 Planner invocation

The planner receives context compiled from durable truth:

```text
Project
PlanningThread
ThreadEntries
Decisions
Research
active/frozen Workflow state
relevant Operations / Verification summaries
```

The planner produces proposals; it does not write directly to storage.

Planner-produced durable changes go through owning application/domain operations. Examples:

```text
append AgentMessage / PlanRevision ThreadEntry
propose Decision
record ResearchArtifact
propose Workflow version
```

The planner cannot authorize its own capabilities.

## 2.3 Agent streaming and cancellation

Agent output has two classes:

```text
Transient stream deltas/logs/progress
Durable state transitions/events
```

Transient output may be lost across disconnect by design.

Cancellation request and terminal cancellation are distinct facts:

```text
External cancel command
        ↓
TX #1
  CommandRecord
  + persist cancellation-request metadata on Operation
  + OperationCancellationRequested durable event
COMMIT

runtime/process layer observes the request
        ↓
terminates the managed process tree
        ↓
confirms that no managed descendant remains alive
        ↓
TX #2 internal
  Pending/Running -> Cancelled
  + OperationCancelled durable event
COMMIT
```

`Cancelled` means Shadows confirmed that the managed execution is no longer
running and persisted the terminal transition. It never means only that a user
requested cancellation.

Cancellation-request metadata is orthogonal to `OperationStatus`:

```rust
struct CancellationRequest {
    requested_at: Timestamp,
    requested_by: Principal,
}
```

No `Cancelling` lifecycle state is required in v1 unless implementation
evidence demonstrates a need.

### Spawn/cancel interlock

The spawn path checks durable cancellation state before spawning and again in
the exact compare-and-swap that attempts `Pending -> Running`. A request that
wins either race prevents the Running transition; a process already spawned
must be contained and terminated before `Cancelled` is persisted.

If the daemon dies after TX #1, the request remains durable and the next
runtime reconciles it. Repeating the same cancel command ID replays its stored
outcome. A different cancel command against an already-terminal operation
records its own idempotent `AlreadyTerminal`/current-operation result without
re-running process effects.

The protocol does not define cancellation as “send SIGTERM”. Unix process-group/session handling and Windows Job Objects are process-layer implementation details.

## 2.4 Durable journal vs transient live bus

### Authoritative current truth

Current domain state lives in entity/state tables:

```text
planning_thread
workflow
task
operation
decision
...
```

### Durable journal

`durable_event` records:

```text
history
provenance
replay/resync ordering
diagnostics/audit context
```

### Transient bus

An in-process broadcast mechanism carries:

```text
token deltas
progress hints
temporary logs
UI animation events
```

The durable transaction commits first. Live publication happens after commit and is best-effort.

A live publication failure never rolls back already committed durable truth.

## 2.5 Workflow lifecycle and supersession

Workflow lifecycle:

```text
Draft → Approved → Frozen → Running → Completed
                           └────────→ Failed
```

Rules:

- `Draft` may be edited through domain operations.
- `Approved` is ready for freeze.
- `Frozen` is immutable.
- `Running`, `Completed`, and `Failed` retain the same frozen definition.
- There is **no unfreeze**.

A design change after freeze creates a new workflow version:

```text
new.previous_version_id = old.id
```

The old workflow is **not mutated into a “Superseded” lifecycle state** merely to express lineage.

Supersession is represented by lineage. A workflow version with a successor receives no new task dispatches. Existing operations from the old version may finish or may be cancelled by explicit policy.

Lineage is linear in v1:

```text
UNIQUE(previous_version_id)
```

## 2.6 Scheduler → ExecutionIntent[]

`scheduler/` is a pure deterministic decision engine:

```rust
scheduler::decide(ScheduleInput) -> ScheduleDecision
```

Conceptually:

```text
ScheduleInput:
  workflow state
  task states
  dependency completion
  gates / verification state
  active operations
  declared/effective scopes
  runtime capacity

ScheduleDecision:
  ExecutionIntent[]
  task-state intents
  no-op / blocked reasons
```

The scheduler performs no storage I/O and spawns nothing.

Parallel execution requires:

```text
DAG readiness
+ no conflicting write scopes
+ enforceable effective scope
+ available runtime capacity
```

## 2.7 ExecutionRun lifecycle

Two-phase spawn:

```text
TX #1
Operation(Pending)
+ OperationCreated event
(+ CommandRecord when external)
COMMIT

spawn via agent -> process

spawn succeeds:
TX #2
Pending -> Running
+ OperationStarted
COMMIT

spawn fails:
TX #2
Pending -> Failed(stage=Spawn)
+ OperationFailed
COMMIT
```

If the daemon dies after spawn succeeds but before TX #2, OS-level containment
must terminate the child tree. Startup recovery then changes the old runtime's
still-`Pending` operation to `Interrupted` using an exact CAS. Database
reconciliation alone is not considered process cleanup.

Terminal runtime transitions:

```text
Running -> Completed { outcome }
Running -> Failed { failure }
Running -> Cancelled
Running -> Interrupted

Pending -> Interrupted       # crash between TX #1 and spawn
Pending -> Cancelled         # durable request won before/while spawn; no tree remains
Running -> Interrupted       # crash while process is live
```

Startup recovery finds non-terminal operations owned by an older runtime and reconciles them using exact compare-and-swap conditions.

## 2.8 Verification flow

Deterministic verification is distinct from AI review.

`VerificationCheck` v1 kinds:

```text
Build
Test
Contract
Baseline
```

`CheckPhase`:

```text
Task
Regression
Deferred
Final
```

Reviewer is a separate AI role/operation seam and is not a deterministic `VerificationCheck` kind.

A `VerificationRun` is long-running work and therefore has an `Operation` for lifecycle. `VerificationRun` stores verification-specific metadata, not a duplicate lifecycle.

A final deterministic result is represented by `Verdict`.

## 2.9 Persistence boundary

The compiler/module structure is the first line of defense.

Mechanical architecture tests may verify obvious ownership rules such as:

```text
no sqlx imports outside storage/
no tokio::process outside process/
no HTTP transport types inside domain/application modules
```

Architecture tests are defense-in-depth, not semantic proof.

Do **not** maintain a giant blacklist of SQL keywords such as `rowid`, `strftime`, or FTS syntax across the entire source tree. Semantic portability is established by storage contract tests, backend-specific containment, code review, and future SQLite/PostgreSQL parity tests.

## 2.10 Disconnect / reconnect / resync

`ClientConnection` is runtime transport state only. It does not own durable thread identity.

A reconnect request provides:

```text
thread_id
last opaque durable cursor
```

The server produces a consistent `ThreadSnapshot` and a project-scoped journal cursor from the **same read transaction**.

Why project-scoped for the snapshot: the snapshot may contain project-level Decisions and Research that have no `thread_id`.

Then the event layer performs:

```text
subscribe_after(snapshot_cursor, scope)
```

with guarantees:

```text
durable replay
+ no-gap handoff to live
+ durable-sequence de-duplication
```

Transient token deltas from before disconnect are not reconstructed.

## 2.11 Claude → Codex continuity

Continuity comes from:

```text
Project truth
PlanningThread entries
Decisions
Research
Workflow versions
Task/Operation state
Verification state
Durable journal
Context Compiler
```

Native harness session IDs are optional opaque metadata:

```text
AgentInvocation.native_session_id: Option<String>
```

A missing native session must never make durable continuity impossible.

The Context Compiler, not `ThreadSnapshot` itself, selects and budgets what an agent receives.

> **OPEN — closed when continuity across harnesses is built.**
> The exact ranking and summarisation algorithm the Context Compiler uses to
> select and budget context is undecided. What is decided is the boundary: the
> Compiler is distinct from `ThreadSnapshot`, it reads durable truth, and it is
> deterministic, budgeted, and scoped (§11.5). The algorithm depends on real
> thread histories and real context budgets, neither of which exists yet.
> Nothing in the first runnable milestone reaches it — one Planner turn compiles
> a thread small enough that selection is not yet a problem.

## 2.12 External agent via MCP

MCP is a first-class adapter, separate from managed harnesses and HTTP.

An externally started Claude/Codex can connect to Shadows through MCP and use Shadows as the project-truth/planning/workflow/memory layer.

MCP exposes domain operations, not storage CRUD. Examples:

```text
project_context_get
thread_context_get
workflow_get
workflow_propose
task_get
decision_get
decision_propose
research_get
research_record
events_since
```

Authority flow:

```text
MCP client requests binding
      ↓
Shadows validates configured identity/profile/policy
      ↓
GrantedMcpContext
      ↓
allowed domain operations
```

The external agent cannot self-grant capabilities.

Mutating MCP calls are external commands and use normal command idempotency. Read calls do not create `CommandRecord`s.

---
