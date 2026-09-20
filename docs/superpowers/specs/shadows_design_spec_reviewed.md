# Shadows — Design Specification

- **Date:** 2026-09-20
- **Status:** Accepted architecture baseline. Delivery begins with the first runnable browser Planner vertical slice; the wider schema and later subsystems are not prerequisites for that slice.
- **Slug:** `shadows`
- **Stack:** Rust **1.94+** minimum for the selected SQLx 0.9 line; development validation performed on Rust 1.96. Single Rust crate, library + binary, plus an independent browser client.
- **Purpose:** Local-first AI software-delivery orchestration runtime for planning, workflow, context, execution, verification, and durable continuity across interchangeable agent harnesses.

> This specification is the single authoritative design and decision baseline. It intentionally avoids recreating the old `shadow` project's crate explosion, duplicated decision ledgers, and patch-driven architecture.

---

## 0. Design Principles

1. **Runnable vertical slice first.** A user-visible end-to-end path is delivered before building later platform layers.
2. **Modular monolith first.** Module boundaries are cheap; crate boundaries are expensive. Split crates only for a concrete build, distribution, reuse, or compile-time isolation reason.
3. **Library-first, not dependency-first.** Prefer mature libraries where they solve the problem; do not add wrappers or dependencies without a real need.
4. **Abstract volatility, not possibility.** Introduce seams where change is already known: storage backend, agent harness, process runtime, protocol adapter, secrets resolution.
5. **Durable current state + durable journal.** Entity/state tables are authoritative current truth. The durable event journal provides history, provenance, replay, and resync. Shadows is **not** full event sourcing.
6. **Native agent sessions are caches, not truth.** Claude/Codex-native session IDs are optional optimization metadata.
7. **Planner ≠ Executor ≠ Reviewer ≠ Orchestrator.**
8. **Role ≠ Harness ≠ Provider ≠ Model.**
9. **The model cannot authorize itself.** Authority comes from Shadows policy/configuration.
10. **Frozen workflow versions are immutable.** Design changes create a new version; old versions are never edited in place.
11. **Every side effect is attributable.** Durable work carries operation/task/runtime/actor/causation provenance as applicable.
12. **Execution is not completion.** Deterministic verification and workflow gates decide completion.
13. **Repository content is untrusted input.**
14. **Ordering is explicit.** Never use physical insertion order or SQLite `rowid` as a domain ordering key.
15. **Secrets are references until spawn.** Secret values never live in durable config/state or logs.

---

# Section 1 — Architecture Overview

`shadows` starts as one Rust crate exposing a library and a binary. The product client is an independent browser application that communicates only through the local protocol.

The binary has two primary modes:

```text
shadows serve    # long-running local daemon/runtime
shadows ...      # CLI client
```

`shadows serve` exposes the local API and product Web client at one local address. It prints the address and never opens a browser automatically; the user chooses which browser to use.

## 1.0 First runnable product boundary

Before the wider architecture is implemented, Shadows must prove one complete product path:

```text
shadows serve
  -> user manually opens the local Web client
  -> selects a local-directory project
  -> creates or resumes a PlanningThread
  -> starts one real Claude Planner turn
  -> receives live output
  -> stops the turn
  -> process layer confirms the complete child tree is gone
  -> Operation becomes Cancelled
  -> daemon restart restores the durable thread and terminal operation
```

Only modules and tables needed by this path are implemented initially. Workflow scheduling, execution DAGs, deterministic verification, MCP, ResearchArtifact, team sync, PostgreSQL, and the complete proposed schema remain later work and do not block the runnable milestone.

## 1.1 Core top-level modules

There are **15 core domain/application/adapter modules**:

```text
project/
thread/
command/
runtime/

agent/
planner/
workflow/
operation/

scheduler/
execution/
verification/
events/

storage/
protocol/
cli/
```

## 1.2 Cross-cutting / infrastructure modules

```text
process/
mcp/
config/
secrets/
error/
tracing/
```

These are still normal modules in the same crate; “cross-cutting” describes responsibility, not a separate architectural layer.

## 1.3 Ownership rules

| Module | Sole owner of |
|---|---|
| `agent/` | Agent-harness abstraction and harness-specific translation from `AgentInvocation` to `ProcessSpec` |
| `process/` | OS process primitives: `tokio::process`, process trees/groups/job objects, `ProcessSpec`, `ProcessHandle` |
| `storage/` | SQLx, SQLite schema/query code, migrations, future backend adapters |
| `protocol/` | HTTP/SSE transport |
| `mcp/` | MCP adapter and MCP request/response translation |
| `secrets/` | `SecretRef` resolution; secret values are resolved only at spawn |
| `scheduler/` | Pure scheduling decision logic; no I/O |
| `events/` | Durable-event vocabulary, cursor semantics, transient live-event abstraction |

### Mechanical boundary examples

- `agent/` may construct `ProcessSpec`, but only `process/` may call `tokio::process`.
- `verification/` may construct a deterministic process check, but process spawning still goes through `process/`.
- `storage/` may import SQLx; application/domain modules may not.
- `protocol/` and `mcp/` call application/domain operations; they do not access SQL directly.

## 1.4 Agent abstraction

The fundamental model is:

```text
Role != Harness != Provider != Model
```

Examples:

```text
Role: Executor
Harness: Claude Code CLI
Provider: MiniMax
Model: MiniMax-M3
```

```text
Role: Executor
Harness: Codex CLI
Provider: OpenAI
Model: configured OpenAI model
```

The same executable may be launched as multiple isolated workers with different environment/profile/model settings.

Conceptual runtime seam:

```rust
trait AgentHarness {
    async fn start(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentRunHandle, agent::Error>;
}
```

`AgentRunHandle` is runtime-only. Durable lifecycle is represented by `Operation`.

## 1.5 Process boundary

```text
AgentInvocation
      ↓
agent/<harness>
      ↓ translates
ProcessSpec
      ↓
process::spawn(ProcessSpec)
      ↓
ProcessHandle
```

`process/` knows nothing about `Role`, `Claude`, `Codex`, planning, workflows, or verification.

Child environment is built per process. The daemon's own global environment is never mutated.

Worker isolation may include:

```text
HOME / USERPROFILE
APPDATA / config dirs
temporary directory
provider-specific environment
secret values
working directory / worktree
```

### Managed-process containment invariant

A managed child process must not survive loss of its owning Shadows runtime
indefinitely. `process/` owns this OS-level guarantee for the complete managed
process tree, including grandchildren.

The contract is capability-based rather than tied to one signal or API:

```text
Windows:
  Job Object kill-on-owner-close semantics, with breakaway prevented,
  or an equivalent mechanism proven by tests

Linux:
  parent-death/supervisor containment plus process-tree cleanup,
  or an equivalent mechanism proven by tests
```

A Unix process group alone is not proof that descendants die when the daemon
crashes. The implementation plan must verify what `process-wrap` provides and
add platform-specific support where it does not satisfy this invariant.

## 1.6 Persistence decision

Persistence selection is closed:

```text
SQLx 0.9
SQLite v1
PostgreSQL compatibility is a design requirement
SeaQuery is NOT included initially
```

The final delta validation compared current candidates rather than relying only on the original spike:

- SeaORM `2.0.3`
- SQLx `0.9.0`
- PostgreSQL 16
- Rust 1.96

Both passed the required PostgreSQL semantic contract. SQLx remained preferred because it required less adapter glue, had a smaller measured dependency/build footprint for this workload, and its official migrator handled concurrent PostgreSQL startup in the validation without custom locking. SeaQuery demonstrated no necessary value for the current static-query workload.

This decision does **not** imply separate storage/domain crates, generic repository traits, or a custom transaction DSL.

---

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

# Section 3 — Errors, Dependencies, and Testing

## 3.1 Module-local errors

Each module owns typed errors using `thiserror`.

Storage example:

```rust
pub enum storage::Error {
    TransitionConflict { /* typed state */ },
    NotFound(EntityKind),
    InvalidCursor { reason: CursorError },
    Serialization(serde_json::Error),
    Constraint(sqlx::Error),
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
}
```

No God Error inside the core.

## 3.2 AppFailure

`AppFailure` is transport-neutral and public-safe:

```rust
struct AppFailure {
    code: ErrorCode,
    class: FailureClass,
    retry: RetryClass,
    public_details: PublicDetails,
}
```

The internal causal chain is retained separately in `FailureReport` and cannot accidentally serialize.

Retry classes:

```text
Never
Immediate
Backoff
AfterReconfiguration
AfterUserAction
```

Retry classification does not itself authorize automatic retry.

## 3.3 Negative domain outcome != system error

Long-running commands normally return:

```text
HTTP 202 + operation_id
```

The operation later reaches a terminal outcome.

```rust
enum OperationOutcome {
    Success {
        result_ref: Option<EntityRef>,
    },
    Blocked {
        reason: BlockReason,
        missing_capability: Option<Capability>,
        alternatives: Vec<Alternative>,
    },
    Rejected {
        reason: RejectionReason,
    },
}
```

`Blocked` and `Rejected` are domain outcomes, not HTTP/protocol failures.

Reading a completed operation may return HTTP 200 with that outcome.

## 3.4 ErrorCode registry

Clients pattern-match on stable codes, not human text.

Representative error codes:

```text
WorkflowInvalidTransition
WorkflowFrozenImmutable
WorkflowValidationFailed

AgentAuthFailed
AgentUnsupportedProfile
AgentRateLimited

ProcessSpawnFailed
ProcessTerminated

StorageUnavailable
StorageMigrationFailed
StorageConstraintViolation

CommandConflict
IdempotencyKeyRequired

InvalidCommand
InvalidCursor
```

`Blocked`/`Rejected` outcome kinds do **not** appear in the error-code registry.

Changing/removing an existing public code is a protocol compatibility concern.

## 3.5 Upstream auth vs Shadows auth

Do not conflate:

```text
Shadows client authentication/authorization
```

with:

```text
upstream provider / harness authentication failure
```

The first maps to normal client auth semantics. The second is infrastructure/agent failure and is reported synchronously or through an asynchronous `Operation` failure depending on when it occurs.

## 3.6 Dependency strategy

| Need | Library | Decision |
|---|---|---|
| Typed errors | `thiserror` | USE |
| Startup/binary boundary | `anyhow` | USE only at bootstrap/top-level boundaries |
| Async runtime | `tokio` | USE |
| HTTP | `axum` | USE |
| Middleware | `tower`, `tower-http` | USE |
| Persistence | `SQLx 0.9` | USE |
| Migrations | SQLx official `Migrator` / `migrate!` | USE |
| Process runtime | `tokio::process` + `process-wrap` | USE behind `process/` |
| MCP | `rmcp` | USE |
| Serialization | `serde`, `serde_json` | USE |
| Tracing | `tracing`, `tracing-subscriber` | USE |
| CLI | `clap` | USE |
| Secrets hardening | `secrecy`, `zeroize` | EVALUATE when actual secret-lifetime benefit is demonstrated |
| Rich CLI diagnostics | `miette` | EVALUATE |
| Retry helper | library or tokio primitives | EVALUATE from concrete retry policy |
| DAG library | none initially | ADD only if it materially reduces scheduler complexity |
| Dynamic SQL builder | none initially | SeaQuery only if real dynamic composition appears |

### 3.6.1 External developer tools

`gcode` is integrated, if needed, as an optional external executable. Shadows
invokes it through its ordinary tool/process boundary and consumes a bounded,
version-checked output contract. The Shadows repository does not vendor or take
a Rust dependency on `gobby-cli` or `gcore`, and does not inherit Gobby's
PostgreSQL, FalkorDB, Qdrant, home-directory, daemon, or configuration model.

An unavailable or incompatible `gcode` disables that optional search
capability; it does not prevent the Planner from starting. `ghook` and `gwiki`
are outside the first runnable milestone. Any future hook adapter is a small
Shadows-owned protocol adapter rather than a copied Gobby dispatcher.

## 3.7 Testing strategy

These layers describe the eventual system. They activate only when the feature
they protect exists. The first runnable milestone uses focused unit tests plus
one real Windows browser/Claude Start/stream/Stop/restart acceptance path; it
does not wait for scheduler property tests, MCP compatibility, research FTS,
or mutation testing. Linux process-containment and core gates are required
before calling the runtime cross-platform, not before the first Windows debug
run.

### Layer 1 — Unit tests

Colocated under Rust source modules.

Use for:

```text
state machines
scheduler decisions
context-selection rules
parsers
scope comparisons
error mapping
```

### Layer 2 — Black-box / cross-module integration tests

Use `tests/` for:

```text
storage contracts
process behavior
protocol flows
resync
cross-module application behavior
```

### Layer 3 — Property tests

Use `proptest` where invariants matter:

```text
DAG scheduling
cursor rules
scope relations
state transitions
idempotency keys
```

### Layer 4 — Fault injection / crash recovery

Inject failures around transaction boundaries and run real daemon-process recovery tests.

Use a platform-neutral unclean-termination helper rather than assuming Unix `kill -9`.

Critical crash points include:

```text
before TX
during TX
after TX #1 / before spawn
after spawn / before TX #2
during terminal transition
```

Required cross-platform containment probes use a real hierarchy:

```text
daemon -> long-lived child -> long-lived grandchild
uncleanly terminate daemon
assert child and grandchild do not remain alive indefinitely
restart daemon
assert old-runtime Pending/Running Operation becomes Interrupted by exact CAS
```

Run this on Windows and Linux. A direct-child-only test is insufficient.

### Layer 5 — Concurrency tests

Cover:

```text
duplicate external commands
parallel scheduler decisions
active ExecutionRun uniqueness
event resync handoff
SQLite writer contention
cancellation races
cancel command committed + daemon crash before process exit
spawn succeeds + daemon crash before Running commit
```

### Layer 6 — Architecture tests

Mechanical only:

```text
SQLx import containment
process primitive containment
no persistence types in domain signatures
```

Do not use architecture scanners as a substitute for semantic tests.

### Layer 7 — Protocol compatibility

Test stable public:

```text
ErrorCode
event envelopes
cursor encoding
command envelopes
operation outcomes
```

### Layer 8 — Migration tests

Required:

```text
fresh DB -> latest
supported old fixture -> latest
failed migration safety
concurrent startup behavior
```

Reversible/down migrations are not a blanket requirement.

### Layer 9 — Real CLI acceptance

Scheduled/manual/release or harness-change gated tests against actual Claude/Codex binaries.

Assertions target observable contracts, not exact natural-language output.

Credentials are dedicated CI credentials only.

### Layer 10 — Mutation testing

Use `cargo-mutants` selectively on deterministic critical code:

```text
scheduler
idempotency
state transitions
cursor logic
scope enforcement
```

Not the entire codebase.

### CI

If Windows is a supported runtime, **Linux and Windows core suites are both required merge/PR gates**.

Additional scheduled jobs:

```text
property-heavy suites
mutation tests
stress/concurrency tests
real CLI acceptance
```

`cargo-nextest` is preferred for normal CI execution. Retries are disabled for the core correctness suite.

A requirement-test matrix may point from requirements/ADRs to executable evidence. There is no rule requiring “one test per ADR”.

---

# Section 4 — Core Domain and Persistence Model

## 4.1 Identity types

Domain IDs are UUID-v4 newtypes unless a later decision explicitly changes one:

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

# Section 4.4 — Persistence Model / API

## 4.4.1 Concrete Storage

Start with:

```rust
pub struct Storage {
    pool: SqlitePool,
}
```

No `Database` trait, no generic `Repository<T>`, and no trait per entity in v1.

SQLx types are private to `storage/`.

`storage::Error` is mapped to `AppFailure` once in the application layer.

## 4.4.2 External vs internal writes

```rust
enum WriteOrigin {
    External(CommandContext),

    Internal {
        actor: ActorRef,
        causation: Option<CausationRef>,
        correlation_id: Option<CorrelationId>,
    },
}
```

External:

```text
CommandRecord
+ DB-enforced idempotency
+ command-kind/request-fingerprint equality check on replay
```

Internal:

```text
no CommandRecord
+ stable caller-generated entity identity when retryable
+ CAS/unique invariants
+ provenance
```

Storage does not interpret generic business commands.

## 4.4.3 Stable identity for internal creates

For any retryable internal durable creation, the owning application module generates the entity ID **before** calling storage.

Example:

```rust
struct NewOperation {
    id: OperationId,
    kind: OperationKind,
    // ...
}
```

A retry uses the same ID.

This prevents:

```text
commit succeeds
response is lost
caller retries
new random UUID creates duplicate logical work
```

## 4.4.4 Causation

```rust
enum CausationRef {
    Operation(OperationId),
    Command(CommandRef),
    Workflow(WorkflowId),
    Runtime(RuntimeInstanceId),
}
```

Do not invent fake operations solely to obtain a causation ID.

## 4.4.5 Representative storage capabilities

### External idempotent

```text
create_project
create_planning_thread
close_planning_thread
create_workflow_draft
supersede_workflow
start_workflow_dispatch_from_command
transition_workflow_from_command
create_operation_from_command
```

For artifacts that can be written by external or internal actors:

```text
append_thread_entry(origin, ...)
record_research(origin, ...)
propose_decision(origin, ...)
transition_decision(origin, ...)
```

### Internal

```text
create_internal_operation
mark_operation_started
mark_operation_completed
mark_operation_failed
mark_operation_cancelled
mark_operation_interrupted

begin_workflow_dispatch_internal
complete_workflow_dispatch
fail_workflow_dispatch

create_agent_invocation
attach_native_session_id
record_verification_run
record_verdict
register_runtime_instance
```

### Reads

```text
get_project
load_planning_thread
list_threads_for_project

list_thread_entries
load_thread_snapshot

get_workflow
list_workflows_for_thread

search_research
list_research_for_project

get_operation
list_operations_for_thread

list_invocations_for_operation
get_verdict
get_runtime_instance

find_orphaned_operations

read_events_after
current_cursor
```

The exact Rust method list may be refined during implementation, but the semantic boundaries above are fixed.

## 4.4.6 No public append_event

Durable events are appended only inside use-case transaction bodies.

Public journal API is read-oriented:

```text
read_events_after
current_cursor
subscribe_after
```

A raw public `append_event` would allow event/state divergence.

## 4.4.7 Thread snapshot vs agent context

`ThreadSnapshot` is authoritative resync state, not the prompt sent directly to an agent.

Conceptually:

```rust
struct ThreadSnapshot {
    thread: PlanningThread,
    as_of_cursor: EventCursor, // project-scoped, same read TX

    recent_entries: Page<ThreadEntry>,
    relevant_decisions: Page<DecisionSummary>,
    current_workflows: Vec<WorkflowSummary>,
    relevant_research: Page<ResearchSummary>,
    recent_operations: Page<OperationSummary>,
    active_invocations: Vec<AgentInvocationSummary>,
    verification_summary: VerificationSummary,
}
```

The snapshot is a bounded authoritative resync view, not a dump of project
history. Every collection has an explicit maximum size, and all included rows
plus the **project-scoped `as_of_cursor`** come from one consistent read
transaction.

Historical detail is loaded through separate paginated APIs. Continuation
tokens use stable ordering keys and are bound to an `as_of_cursor`/revision
when repeatable historical paging is promised; otherwise the API explicitly
documents that it reads current state and may drift.

The Context Compiler derives role-specific `AgentContext` from durable truth under token/scope/budget constraints.

---

# Section 5 — SQLite Schema Proposal

No migration SQL is written until this schema proposal is accepted.

## 5.1 Table set

**17 ordinary tables + 1 FTS5 virtual table:**

```text
project
planning_thread
thread_entry
decision
research_artifact

workflow
task
task_parent
gate
verification_check

runtime_instance
operation
agent_invocation
verification_run
verdict

durable_event
command_record

research_fts             # FTS5 virtual table
```

FTS synchronization uses triggers; triggers are not tables.

## 5.2 Common encodings

| Domain value | SQLite representation |
|---|---|
| UUID newtype | `TEXT` canonical UUID |
| Timestamp | `TEXT` RFC3339 UTC |
| Enum | `TEXT` + `CHECK` where the enum is schema-stable |
| Typed JSON payload | `TEXT`, serialized/deserialized only inside storage mapping |
| Boolean | `INTEGER` with `CHECK (value IN (0,1))` |

The domain must not expose these persistence encodings.

---

## 5.3 `project`

```text
id                  TEXT PRIMARY KEY
slug                TEXT NOT NULL UNIQUE
name                TEXT NOT NULL
default_config_ref  TEXT NULL
created_at          TEXT NOT NULL
```

---

## 5.4 `planning_thread`

```text
id                  TEXT PRIMARY KEY
project_id          TEXT NOT NULL FK project(id) ON DELETE RESTRICT
title               TEXT NOT NULL
status              TEXT NOT NULL CHECK status IN ('Open','Closed')
next_entry_ordinal  INTEGER NOT NULL DEFAULT 1 CHECK next_entry_ordinal > 0
created_at          TEXT NOT NULL
```

Index:

```text
(project_id, created_at, id)
```

---

## 5.5 `thread_entry`

```text
id           TEXT PRIMARY KEY
thread_id    TEXT NOT NULL FK planning_thread(id) ON DELETE RESTRICT
ordinal      INTEGER NOT NULL CHECK ordinal > 0
kind         TEXT NOT NULL
author_kind  TEXT NOT NULL
author_id    TEXT NOT NULL
body         TEXT NOT NULL
refs_json    TEXT NOT NULL DEFAULT '[]'
created_at   TEXT NOT NULL

UNIQUE(thread_id, ordinal)
```

Ordinal allocation happens in the same transaction:

```sql
UPDATE planning_thread
SET next_entry_ordinal = next_entry_ordinal + 1
WHERE id = ?
RETURNING next_entry_ordinal - 1;
```

Then the entry is inserted using that ordinal.

No `MAX(ordinal)+1`.

---

## 5.6 `decision`

```text
id               TEXT PRIMARY KEY
project_id       TEXT NOT NULL FK project(id) ON DELETE RESTRICT
title            TEXT NOT NULL
summary          TEXT NOT NULL
status           TEXT NOT NULL
refs_json        TEXT NOT NULL DEFAULT '[]'
decided_by_kind  TEXT NULL
decided_by_id    TEXT NULL
created_at       TEXT NOT NULL
updated_at       TEXT NOT NULL
```

Constraints:

```text
status IN ('Proposed','Accepted','Rejected','Superseded')

(decided_by_kind IS NULL) == (decided_by_id IS NULL)

Proposed -> decided_by_* are NULL
Accepted/Rejected -> decided_by_* are NOT NULL
```

Decision remains project-level. There is no `decision.thread_id`.

Thread relevance is derived from explicit thread-entry/workflow references unless a future real query proves an explicit association table is necessary.

---

## 5.7 `research_artifact`

```text
id          TEXT PRIMARY KEY
project_id  TEXT NOT NULL FK project(id) ON DELETE RESTRICT
title       TEXT NOT NULL
source      TEXT NULL
summary     TEXT NOT NULL
created_at  TEXT NOT NULL
```

No domain ordering depends on SQLite `rowid`.

---

## 5.8 `workflow`

```text
id                   TEXT PRIMARY KEY
thread_id            TEXT NOT NULL FK planning_thread(id) ON DELETE RESTRICT
state                TEXT NOT NULL
previous_version_id  TEXT NULL
source_plan_json     TEXT NULL
created_at           TEXT NOT NULL
updated_at           TEXT NOT NULL
frozen_at            TEXT NULL

UNIQUE(id, thread_id)
UNIQUE(previous_version_id)
```

State:

```text
Draft
Approved
Frozen
Running
Completed
Failed
```

Timestamp constraint:

```text
state IN ('Frozen','Running','Completed','Failed')
    -> frozen_at IS NOT NULL
```

Lineage must remain inside one thread.

Use a composite FK:

```text
(previous_version_id, thread_id)
    -> workflow(id, thread_id)
```

when `previous_version_id` is non-null.

`source_plan_json`, if present, is an immutable authored/provenance snapshot only.

The scheduler reads normalized:

```text
task
task_parent
gate
verification_check
```

not `source_plan_json`.

---

## 5.9 `task`

```text
id             TEXT PRIMARY KEY
workflow_id    TEXT NOT NULL FK workflow(id) ON DELETE RESTRICT
contract_json  TEXT NOT NULL
scope_json     TEXT NOT NULL
state          TEXT NOT NULL DEFAULT 'Pending'
created_at     TEXT NOT NULL
updated_at     TEXT NOT NULL

UNIQUE(id, workflow_id)
CHECK state IN ('Pending','Ready','InProgress','Completed','Failed','Blocked')
```

`Task.state` is persisted authoritative scheduler state.

---

## 5.10 `task_parent`

Use workflow-aware composite integrity:

```text
workflow_id  TEXT NOT NULL
task_id      TEXT NOT NULL
parent_id    TEXT NOT NULL

PRIMARY KEY(workflow_id, task_id, parent_id)

FK (task_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT

FK (parent_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT

CHECK(task_id != parent_id)
```

Index:

```text
(workflow_id, parent_id)
```

This prevents DAG edges across workflow versions.

Acyclicity is a domain/scheduler validation rule, not expressible as a simple SQLite `CHECK`.

---

## 5.11 `gate`

```text
id             TEXT PRIMARY KEY
workflow_id    TEXT NOT NULL
after_task_id  TEXT NULL
created_at     TEXT NOT NULL

UNIQUE(id, workflow_id)

FK workflow_id -> workflow(id) ON DELETE RESTRICT

FK (after_task_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT
```

`Gate` is a grouping/timing anchor. It does not duplicate `CheckPhase`.

---

## 5.12 `verification_check`

```text
id            TEXT PRIMARY KEY
workflow_id   TEXT NOT NULL
task_id       TEXT NULL
gate_id       TEXT NULL
kind          TEXT NOT NULL
phase_kind    TEXT NOT NULL
config_json   TEXT NOT NULL
scope_json    TEXT NOT NULL

FK workflow_id -> workflow(id) ON DELETE RESTRICT

FK (task_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT

FK (gate_id, workflow_id)
    -> gate(id, workflow_id) ON DELETE RESTRICT
```

Kinds:

```text
Build
Test
Contract
Baseline
```

Phases:

```text
Task
Regression
Deferred
Final
```

Constraints:

```text
exactly one of task_id / gate_id is non-null

phase_kind = 'Task'
    -> task_id IS NOT NULL AND gate_id IS NULL

phase_kind = 'Deferred'
    -> gate_id IS NOT NULL AND task_id IS NULL

phase_kind IN ('Regression','Final')
    -> may bind to task_id or gate_id, but still exactly one
```

This keeps only established domain invariants in SQLite. A later accepted
decision may narrow Regression or Final ownership if real semantics require it.

---

## 5.13 `runtime_instance`

```text
id          TEXT PRIMARY KEY
version     TEXT NOT NULL
started_at  TEXT NOT NULL
```

A runtime is global daemon provenance, not project-owned.

---

## 5.14 `operation`

```text
id                   TEXT PRIMARY KEY
kind                 TEXT NOT NULL
status_kind          TEXT NOT NULL DEFAULT 'Pending'

thread_id            TEXT NULL
workflow_id          TEXT NULL
task_id              TEXT NULL

runtime_instance_id  TEXT NOT NULL FK runtime_instance(id) ON DELETE RESTRICT

outcome_json         TEXT NULL
failure_stage        TEXT NULL
failure_reason       TEXT NULL
interrupt_reason     TEXT NULL

cancel_requested_at       TEXT NULL
cancel_requested_by_kind  TEXT NULL
cancel_requested_by_id    TEXT NULL

created_at           TEXT NOT NULL
started_at           TEXT NULL
finished_at          TEXT NULL
```

v1 kinds:

```text
PlannerTurn
ExecutionRun
Verification
Research
Git
```

Status kinds:

```text
Pending
Running
Completed
Failed
Cancelled
Interrupted
```

Hierarchy rules:

```text
task_id != NULL     -> workflow_id != NULL
workflow_id != NULL -> thread_id != NULL
```

Use composite integrity where an ancestor is stored:

```text
(task_id, workflow_id)
    -> task(id, workflow_id)

(workflow_id, thread_id)
    -> workflow(id, thread_id)
```

State-shape constraints:

```text
Pending:
  started_at NULL
  finished_at NULL
  outcome_json NULL
  failure_stage NULL
  failure_reason NULL
  interrupt_reason NULL

Running:
  started_at NOT NULL
  finished_at NULL
  outcome_json NULL
  failure_stage NULL
  failure_reason NULL
  interrupt_reason NULL

Completed:
  started_at NOT NULL
  finished_at NOT NULL
  outcome_json NOT NULL
  failure_stage NULL
  failure_reason NULL
  interrupt_reason NULL

Failed:
  finished_at NOT NULL
  failure_stage NOT NULL
  failure_reason NOT NULL
  outcome_json NULL
  interrupt_reason NULL

Cancelled:
  finished_at NOT NULL
  outcome_json NULL
  failure_stage NULL
  interrupt_reason NULL

Interrupted:
  finished_at NOT NULL
  outcome_json NULL
  failure_stage NULL
  interrupt_reason NOT NULL
```

Cancellation-request shape is independent from lifecycle status:

```text
cancel_requested_at/by_kind/by_id are either all NULL or all NOT NULL
```

The request may coexist with `Pending` or `Running`. Terminal `Cancelled`
requires `finished_at`, and is written only after process-tree termination is
confirmed. A terminal non-cancelled status may retain request metadata as
history when completion won the race.

`runtime_instance_id` already identifies the previous runtime for an interrupted operation; the domain may expose that value in `OperationStatus::Interrupted`.

### Active ExecutionRun uniqueness

Correct v1 partial unique index:

```sql
CREATE UNIQUE INDEX idx_active_execution_per_task
ON operation(task_id)
WHERE kind = 'ExecutionRun'
  AND status_kind IN ('Pending','Running');
```

This prevents a simultaneous Pending and Running execution attempt for the same task.

---

## 5.15 `agent_invocation`

```text
id                 TEXT PRIMARY KEY
operation_id       TEXT NOT NULL FK operation(id) ON DELETE RESTRICT
role               TEXT NOT NULL
harness_kind       TEXT NOT NULL
profile_json       TEXT NOT NULL
native_session_id  TEXT NULL
created_at         TEXT NOT NULL
```

Roles include:

```text
Planner
Executor
Reviewer
Researcher
```

`profile_json` may contain provider/model/profile configuration, but role, harness, provider, and model remain conceptually distinct.

Index:

```text
(operation_id, created_at, id)
```

No reverse `operation.agent_invocation_id` column exists.

---

## 5.16 `verification_run`

Lifecycle is owned by its associated `Operation`.

```text
id            TEXT PRIMARY KEY
operation_id  TEXT NOT NULL UNIQUE FK operation(id) ON DELETE RESTRICT
check_id      TEXT NOT NULL FK verification_check(id) ON DELETE RESTRICT
created_at    TEXT NOT NULL
```

No duplicate:

```text
status_kind
started_at
finished_at
```

fields exist here.

---

## 5.17 `verdict`

One final verdict per run in v1:

```text
id                  TEXT PRIMARY KEY
run_id              TEXT NOT NULL UNIQUE FK verification_run(id) ON DELETE RESTRICT
passed               INTEGER NOT NULL CHECK passed IN (0,1)
evidence_ref         TEXT NOT NULL
discrepancies_json   TEXT NULL
created_at           TEXT NOT NULL
```

Infrastructure failure belongs to the verification `Operation`; it is not encoded as a fake failing verdict.

---

## 5.18 `durable_event`

One row per event; the same row may be visible in several scopes.

```text
seq             INTEGER PRIMARY KEY AUTOINCREMENT
event_id        TEXT NOT NULL UNIQUE
kind            TEXT NOT NULL

project_id      TEXT NULL
thread_id       TEXT NULL
workflow_id     TEXT NULL
operation_id    TEXT NULL

actor_kind      TEXT NULL
actor_id        TEXT NULL

causation_kind  TEXT NULL
causation_ref   TEXT NULL
correlation_id  TEXT NULL

payload_json    TEXT NOT NULL
created_at      TEXT NOT NULL
```

FKs use `ON DELETE RESTRICT`.

Pair constraints:

```text
actor_kind NULL <=> actor_id NULL
causation_kind NULL <=> causation_ref NULL
```

### Scope semantics

```text
Global cursor    = all durable_event rows ordered by seq
Project cursor   = rows where project_id = ?
Thread cursor    = rows where thread_id = ?
Workflow cursor  = rows where workflow_id = ?
Operation cursor = rows where operation_id = ?
```

Global does **not** mean “all scope FKs are NULL”.

### Event cursor

Public:

```rust
struct EventCursor(/* opaque */);
```

Logical content:

```text
scope kind
scope identity if non-global
last global seq observed for that scope
```

The cursor is opaque/validated at the events/storage boundary.

### ThreadSnapshot cursor

`ThreadSnapshot` uses a **project-scoped** event cursor because its state may include project-level Decisions/Research with no `thread_id`.

Thread-scoped cursors remain valid for narrower subscriptions.

### Event indexes

```text
(project_id, seq)
(thread_id, seq)
(workflow_id, seq)
(operation_id, seq)
```

No domain logic relies on contiguous `seq` values.

---

## 5.19 `command_record`

Use a stable logical outcome shape rather than a growing `outcome_kind` per entity type:

```text
principal_kind      TEXT NOT NULL
principal_id        TEXT NOT NULL

command_scope_kind  TEXT NOT NULL
command_scope_key   TEXT NOT NULL

command_id          TEXT NOT NULL

command_kind         TEXT NOT NULL
command_schema_ver   INTEGER NOT NULL CHECK command_schema_ver > 0
request_fingerprint  TEXT NOT NULL

outcome_kind        TEXT NOT NULL   # Entity | NoContent
entity_kind         TEXT NULL
outcome_ref         TEXT NULL

recorded_at         TEXT NOT NULL

PRIMARY KEY(
  principal_kind,
  principal_id,
  command_scope_kind,
  command_scope_key,
  command_id
)
```

Global scope:

```text
command_scope_kind = 'Global'
command_scope_key  = ''
```

Non-global scopes use a non-empty canonical entity key.

Outcome constraints:

```text
outcome_kind = 'NoContent'
    -> entity_kind IS NULL AND outcome_ref IS NULL

outcome_kind = 'Entity'
    -> entity_kind IS NOT NULL AND outcome_ref IS NOT NULL
```

Representative `EntityKind` values:

```text
Project
PlanningThread
ThreadEntry
Decision
ResearchArtifact
Workflow
Task
Operation
AgentInvocation
VerificationRun
Verdict
```

`outcome_ref` is intentionally not a polymorphic SQL FK.

DB uniqueness is the final authority for external idempotency.

After a uniqueness conflict, storage compares the stored and incoming
`command_kind`, schema version, and fingerprint:

```text
equal     -> replay stored outcome
different -> CommandConflict
```

The normalized typed request includes explicit/defaulted values and target
resource identity. Raw JSON byte order is never the fingerprint input.

---

## 5.20 Cross-table atomic invariants

These are implemented as use-case-specific storage transactions.

### External creation/transition

```text
CommandRecord claim
+ entity mutation
+ durable event(s)
= one transaction
```

### Cancellation request and terminal transition

External cancellation request:

```text
CommandRecord
+ Operation.cancel_requested_*
+ OperationCancellationRequested
= one transaction
```

Runtime completion after process-tree termination:

```text
Operation Pending/Running -> Cancelled
+ OperationCancelled
= one internal transaction
```

The spawn path and cancellation path use exact CAS conditions so a cancelled
request cannot race into a committed `Running` state. A no-op cancel against a
terminal operation still records/replays its own command outcome but performs
no process effect.

### Scheduler dispatch

For a ready task:

```text
Task Ready -> InProgress
+ insert ExecutionRun(Pending)
+ OperationCreated
+ TaskTransitioned
= one transaction
```

The process has not started yet, so this transaction must **not** emit `OperationStarted`.

After spawn succeeds:

```text
Operation Pending -> Running
+ OperationStarted
= one transaction
```

If spawn fails:

```text
Operation Pending -> Failed(stage=Spawn)
+ Task InProgress -> Failed or Ready according to explicit retry policy
+ events
= one transaction
```

### Successful execution with required Task-phase verification

```text
ExecutionRun Running -> Completed(Success)
Task remains InProgress
```

until required verification completes.

Final required passing verdict:

```text
insert Verdict
+ Task InProgress -> Completed
+ TaskTransitioned
+ verification event(s)
= one transaction
```

Failed/blocking verification follows policy and transitions Task atomically with the verdict that decides it.

### Interrupted attempt

Recovery updates the exact stale attempt:

```text
WHERE operation.id = ?
  AND operation.status_kind = expected
  AND operation.runtime_instance_id = expected_runtime
```

Then task retry/requeue state is updated in the same use-case transaction when that operation determines task scheduling.

---

## 5.21 Task lifecycle v1

Base transitions:

```text
Pending -> Ready
Pending -> Blocked

Ready -> InProgress
Ready -> Blocked

InProgress -> Completed
InProgress -> Failed
InProgress -> Blocked

InProgress -> Ready   # interrupted/retry policy explicitly permits re-dispatch

Failed -> Ready       # explicit retry command/policy only
Blocked -> Ready      # explicit unblock only
Completed -> Ready    # explicit re-run only; never automatic
```

Interpretation:

- `Pending`: dependencies/gates not yet satisfied.
- `Ready`: schedulable, no active execution attempt. A task waiting only on
  runtime capacity or a conflicting write scope stays `Ready`; waiting is not
  `Blocked`.
- `InProgress`: dispatch has claimed the task. An attempt may be `Pending`,
  spawned and `Running`, or already finished while required Task-phase
  verification is still outstanding.
- `Completed`: successful execution and required Task-phase checks passed.
- `Failed`: execution/check policy made the task terminally failed for the current scheduling decision.
- `Blocked`: required capability/ancestor/policy makes progress impossible until explicit change.

Execution-attempt history is represented by multiple `ExecutionRun` operations for the same task.

The default Task transition after an attempt ends `Failed`, `Cancelled`, or
`Interrupted` — and which layer chooses it — is **not decided**. See open
decision 1 in
[`2026-09-20-runtime-execution-model-draft.md`](./2026-09-20-runtime-execution-model-draft.md).

---

## 5.22 FTS5 strategy

FTS5 remains SQLite-backend-specific.

To avoid a durable relationship through the base table's implicit physical `rowid`, use a normal FTS5 table that stores the domain research ID as an unindexed field:

```sql
CREATE VIRTUAL TABLE research_fts USING fts5(
    research_id UNINDEXED,
    title,
    summary,
    tokenize = 'porter unicode61'
);
```

Triggers keep it synchronized:

```text
research_artifact INSERT
  -> INSERT research_fts(research_id, title, summary)

research_artifact UPDATE
  -> delete old FTS row by research_id
  -> insert updated FTS row

research_artifact DELETE
  -> delete FTS row by research_id
```

Search:

```text
MATCH on research_fts
→ obtain research_id
→ load/join authoritative research_artifact by UUID
```

FTS5's own internal rowid may exist internally, but Shadows does not use it as domain identity, ordering, or cross-table durable linkage.

All FTS SQL stays under:

```text
storage/sqlite/
```

A future PostgreSQL adapter may use `tsvector` without changing domain/application APIs.

---

## 5.23 SQLite connection policy

Required v1 connection configuration:

```text
PRAGMA foreign_keys = ON
PRAGMA journal_mode = WAL
PRAGMA busy_timeout = 5000
```

Meaning:

- `foreign_keys=ON`: enforce declared FK constraints.
- WAL: production file-backed SQLite supports concurrent readers while serializing writes.
- `busy_timeout`: tolerate normal `SQLITE_BUSY` writer contention for a bounded interval.

Do **not** claim `busy_timeout` fixes the shared-cache in-memory deadlock observed in the spike; production uses file-backed SQLite.

Do not lock performance/durability tuning without evidence:

```text
synchronous
temp_store
wal_autocheckpoint
```

Stay conservative by default.

Before storage implementation is considered complete, run a production-like validation with:

```text
file-backed SQLite
WAL
multiple readers
concurrent writers
atomic use-case transactions
```

Then decide whether write transactions require an explicit `BEGIN IMMEDIATE` strategy or another writer-serialization mechanism.

---

# Section 6 — Migrations and Implementation Work Remaining

## 6.1 SQLx migrations

Use SQLx official migration machinery.

The first migration contains only the first runnable milestone's durable
requirements: project, planning thread, thread entry, operation, command record,
runtime instance, and durable event data. Tables for Workflow, Task, Decision,
ResearchArtifact, VerificationRun, and Verdict are added only with those
features.

The eventual logical split may become:

```text
0001_core.sql
  ordinary tables
  foreign keys
  CHECK/UNIQUE constraints

0002_indexes.sql
  query indexes
  partial active-execution unique index

0003_research_fts.sql
  FTS5 virtual table
  synchronization triggers
```

Do not create a custom migration runner.

Required migration tests:

```text
fresh -> latest
supported old fixture -> latest
failed migration safety
concurrent startup
```

## 6.2 Implementation order

### Milestone 0 — runnable browser Planner

1. scaffold the single Rust crate, `shadows serve`, structured tracing, and the independent Web client;
2. open SQLite through SQLx with only the milestone schema;
3. implement local-directory Project and durable PlanningThread/ThreadEntry;
4. implement the managed process primitive and one Claude harness;
5. implement durable Planner Operation start, live SSE output, semantic stop, and restart reconciliation;
6. connect the browser UI to create/resume a thread, Start, show output, and Stop;
7. run the real Windows debug acceptance path and record exactly what remains unverified on Linux.

The milestone is incomplete until the user can operate this path from a browser.
Persistence-only or protocol-only completion is not an acceptable substitute.

### Later milestones

After Milestone 0 works, add features in user-visible slices rather than
constructing the entire platform upfront:

1. Workflow draft/freeze and task display;
2. scheduler and execution;
3. deterministic verification;
4. context compilation and Claude/Codex continuity;
5. MCP-attached agents;
6. research artifacts/search;
7. AI Reviewer and team features when separately designed.

## 6.3 Required persistence evidence

Before considering persistence foundation complete:

```text
atomic state+event+command
idempotent replay under concurrency
same idempotency key + different request -> CommandConflict
transaction rollback fault injection
event ordering/cursors
thread ordinal allocation under concurrency
active ExecutionRun uniqueness
crash recovery from Pending and Running
consistent snapshot+cursor
bounded snapshot under large history + stable pagination
file-backed WAL concurrency
FTS trigger parity
migration upgrade
```

Future PostgreSQL work must run the same semantic storage-contract suite.

---

# Section 7 — Cross-cutting Rules

1. **Domain types stay storage-agnostic.** No SQLx/SQLite/PostgreSQL types in domain/application signatures.
2. **SQL is allowed inside storage.** It is not limited to migration files.
3. **Backend-specific syntax remains backend-local.**
4. **Ordering is explicit.** Never rely on physical insertion order.
5. **Durable state mutation + durable event is atomic.**
6. **External idempotent mutation also includes CommandRecord in the same transaction.**
7. **Internal retryable creates use stable caller-generated IDs.**
8. **No generic command interpreter in storage.**
9. **No public transaction DSL.**
10. **No public raw `append_event`.**
11. **No generic `Repository<T>` without demonstrated need.**
12. **No native agent session is required for continuity.**
13. **No global environment mutation for child agents.**
14. **Secrets are references until spawn.**
15. **Reviewer and deterministic Verifier remain separate.**
16. **Task state and related operation/verdict transitions are atomically maintained when they form one scheduler invariant.**
17. **Current entity tables are authoritative state; the journal is history/provenance/resync.**
18. **Context Compiler is distinct from ThreadSnapshot.**
19. **Architecture scanners are mechanical defense-in-depth, not semantic proof.**
20. **Schema does not redefine domain semantics merely because SQLite makes another shape easier.**

---

# Section 8 — Decision Reference Policy

This specification is the complete authoritative architecture baseline. The
nineteen early ADR files were consolidated into four compact navigation maps:

1. `ADR-0001-foundation-and-ownership.md`
2. `ADR-0002-storage-events-and-continuity.md`
3. `ADR-0003-agents-processes-and-operations.md`
4. `ADR-0004-product-delivery-and-deferred-systems.md`

The consolidated ADRs summarize related decisions and point back here for full
semantics. They are not independent specifications. If a summary and this spec
diverge, fix the summary; this spec remains authoritative.

Do not create an ADR for a struct shape, library call, test correction, or
ordinary implementation detail. A future ADR is justified only when a new
decision has credible alternatives, long-term consequences, and explicitly
supersedes part of this baseline.

---

# Section 9 — Explicit Non-Decisions / Deferred Items

The non-binding long-term collaboration direction is documented separately in
[`docs/future/shadows-team-direction.md`](../../future/shadows-team-direction.md).
It does not add Team/Server/sync types or requirements to local v1.

## Runtime model still under review

[`2026-09-20-runtime-execution-model-draft.md`](./2026-09-20-runtime-execution-model-draft.md)
proposes the detailed runtime and execution model. Only its accepted items have
been merged here: the `TaskState::InProgress` vocabulary and the separation of
Task progress from Operation runtime status. Its RuntimeInstance lifecycle,
workspace modes, write-scope conflict rules, and failure matrix remain a draft
and are not baseline. Its seven open decisions are unresolved and must be
settled before it merges. None of them block the first runnable milestone,
which has no workflow, scheduler, or verification.

The current design intentionally does **not** decide:

- separate domain/storage crates;
- PostgreSQL production adapter implementation date;
- SeaQuery adoption;
- active-active multi-runtime support;
- automatic retry policy;
- exact Context Compiler ranking/summarization algorithm;
- reviewer implementation timing;
- final SQLite writer strategy before the file-backed WAL validation;
- secret hardening crates until lifecycle requirements justify them;
- a generic DB abstraction layer;
- a generic transaction-composition DSL.

---

# Section 10 — Layered Readiness Milestones

Readiness is incremental. Feature work may build on a completed earlier layer
without waiting for every later layer; dependencies between milestones remain
explicit.

## 10.1 First Runnable Browser Planner

```text
[ ] `shadows serve` starts and prints one local address without opening a browser
[ ] the user can manually open the Web client in any browser
[ ] a local-directory project can be selected without exposing a path as project identity
[ ] a PlanningThread can be created and resumed
[ ] one real Claude Planner turn starts and streams output
[ ] Stop terminates and reaps the managed process tree before durable Cancelled
[ ] daemon restart restores the durable thread and terminal operation
[ ] structured logs correlate project, thread, and operation without sensitive payloads
[ ] the exact Windows acceptance run is recorded; Linux gaps are named honestly
```

## 10.2 Persistence Foundation Ready

```text
[ ] accepted SQLx migrations exist
[ ] SQLite opens with production connection policy
[ ] state + event atomicity passes
[ ] command idempotency replay and request-fingerprint conflict pass
[ ] durable journal/cursor passes
[ ] bounded consistent snapshot + history pagination pass
[ ] thread ordinal allocation passes under concurrency
[ ] SQLite file-backed WAL concurrency validation passes
[ ] fresh/upgrade/failed/concurrent migration tests pass
[ ] storage architecture import checks pass
```

## 10.3 Runtime / Execution Ready

```text
[ ] scheduler is deterministic/pure
[ ] Task/Operation atomic invariants pass
[ ] two-phase spawn passes
[ ] Pending/Running crash recovery passes
[ ] managed child + grandchild containment passes on Windows + Linux
[ ] cancellation request -> termination -> Cancelled flow passes
[ ] spawn/cancel race tests pass
[ ] provider/harness environment isolation passes
```

## 10.4 Protocol / MCP Ready

```text
[ ] HTTP command idempotency contract passes
[ ] SSE durable replay + live no-gap handoff passes
[ ] stable public errors/outcomes pass compatibility tests
[ ] MCP authority/binding passes
[ ] MCP mutations use the same application semantics
```

## 10.5 Continuity Ready

```text
[ ] durable PlanningThread entries pass
[ ] Context Compiler is deterministic, budgeted, and scoped
[ ] Claude -> Codex fake-harness continuity passes
[ ] native_session_id remains optional
[ ] snapshot/history pagination passes under large history
```

---

*End of specification.*
