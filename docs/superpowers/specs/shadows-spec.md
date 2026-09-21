# Shadows — Design Specification

- **Date:** 2026-09-21
- **Status:** Accepted architecture baseline. Delivery begins with the first runnable browser Planner vertical slice; the wider schema and later subsystems are not prerequisites for that slice.
- **Slug:** `shadows`
- **Stack:** Rust **1.94+** minimum for the selected SQLx 0.9 line; development validation performed on Rust 1.96. Single Rust crate, library + binary, plus an independent browser client.
- **Purpose:** Local-first AI software-delivery orchestration runtime for planning, workflow, context, execution, verification, and durable continuity across interchangeable agent harnesses.

> This specification is the single authoritative design and decision source. There is no second one. It intentionally avoids recreating the old `shadow` project's crate explosion, duplicated decision ledgers, and patch-driven architecture.

---

## How this document is maintained

**One document.** Decisions are amended **in place**. A decision is never revised
by adding a second document that contradicts the first, and never summarised into
a parallel file that can drift. The old `shadow` repository ended with three
simultaneously-in-force documents disagreeing about the same decision and a
status file explaining which one to believe; that failure mode is the reason this
document has no companions.

**Three kinds of statement live here, and they are marked differently:**

- Ordinary prose is **decided**. Implement it.
- A block marked **OPEN** is a question this project cannot answer yet. Every
  OPEN block names the trigger that closes it and why it does not block current
  work. An OPEN block without a trigger is rot, not a question.
- `docs/status.md` holds where the project currently is. It is operational, it
  changes often, and it decides nothing.

**Measurement records live in `docs/evidence/` and are not part of this
document.** They are dated facts, not decisions: they do not expire, do not
contradict anything, and are never cited as design authority. When a measurement
changes a decision, the decision changes *here*.

**Do not create an ADR, a design note, or a plan document for a struct shape, a
library call, a test correction, or an ordinary implementation detail.** Amend
this file.

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

### Harness identity is explicit configuration

A harness executable is resolved from explicit configuration, never from `PATH`.
The resolved path and the harness's self-reported version are read when an
Operation starts and recorded with it.

This is not defensive habit. The measured harness stream contract
(`docs/evidence/harness/SERVE_STREAM_SPIKE.md`) is the contract of one
installation at one version, and a machine can carry several: on the validation
machine the Claude desktop application bundles its own copy, at more than one
version, entirely separate from whatever `PATH` resolves. An auto-update can
therefore change the output contract underneath a running install. Without a
recorded path and version the first symptom is a blank page rather than an error,
and nothing in the durable record says which binary produced which turn.

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

`process/` receives only OS-level intent:

```text
executable
argv
cwd
explicit environment
stdio policy
timeout
containment policy
resource limits when supported
```

The child environment is built explicitly; secret values are resolved only at
spawn and never persisted, traced, or placed in command-line arguments.

`HOME`, `USERPROFILE`, `APPDATA`, temporary directories, and provider
configuration locations follow the selected isolation profile rather than leaking
from the daemon by accident. Clearing the environment wholesale is not the same
as isolating it: on Windows a child that loses `SystemRoot`, `SystemDrive`,
`ComSpec`, or `PATHEXT` fails in ways that never appear on Linux.

Persisted diagnostics about a spawn may include executable identity and version,
argument count, workspace identity, profile name, and environment key names. They
must not include prompt text, model output, secret values, complete environments,
or sensitive argument values.

A child's stdin is closed unless the harness contract requires streaming input.
An open stdin that never receives data costs a fixed stall on every turn.


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

# Section 4 — Core Domain Model and Invariants

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

# Section 5 — Persistence Model and API


## 5.1 Concrete Storage

Start with:

```rust
pub struct Storage {
    pool: SqlitePool,
}
```

No `Database` trait, no generic `Repository<T>`, and no trait per entity in v1.

SQLx types are private to `storage/`.

`storage::Error` is mapped to `AppFailure` once in the application layer.

## 5.2 External vs internal writes

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

## 5.3 Stable identity for internal creates

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

## 5.4 Causation

```rust
enum CausationRef {
    Operation(OperationId),
    Command(CommandRef),
    Workflow(WorkflowId),
    Runtime(RuntimeInstanceId),
}
```

Do not invent fake operations solely to obtain a causation ID.

## 5.5 Representative storage capabilities

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

## 5.6 No public append_event

Durable events are appended only inside use-case transaction bodies.

Public journal API is read-oriented:

```text
read_events_after
current_cursor
subscribe_after
```

A raw public `append_event` would allow event/state divergence.

## 5.7 Thread snapshot vs agent context

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

# Section 6 — SQLite Schema


No migration SQL is written until this schema proposal is accepted.

## 6.1 Table set

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

## 6.2 Common encodings

| Domain value | SQLite representation |
|---|---|
| UUID newtype | `TEXT` canonical UUID |
| Timestamp | `TEXT` RFC3339 UTC |
| Enum | `TEXT` + `CHECK` where the enum is schema-stable |
| Typed JSON payload | `TEXT`, serialized/deserialized only inside storage mapping |
| Boolean | `INTEGER` with `CHECK (value IN (0,1))` |

The domain must not expose these persistence encodings.

---

## 6.3 `project`

```text
id                  TEXT PRIMARY KEY
slug                TEXT NOT NULL UNIQUE
name                TEXT NOT NULL
default_config_ref  TEXT NULL
created_at          TEXT NOT NULL
```

---

## 6.4 `planning_thread`

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

## 6.5 `thread_entry`

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

## 6.6 `decision`

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

## 6.7 `research_artifact`

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

## 6.8 `workflow`

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

## 6.9 `task`

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

## 6.10 `task_parent`

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

## 6.11 `gate`

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

## 6.12 `verification_check`

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

## 6.13 `runtime_instance`

```text
id            TEXT PRIMARY KEY
version       TEXT NOT NULL
started_at    TEXT NOT NULL
stopped_at    TEXT NULL
stop_kind     TEXT NULL
              CHECK (stop_kind IS NULL OR stop_kind IN ('Graceful','Escalated'))
CHECK ((stopped_at IS NULL) = (stop_kind IS NULL))
```

A runtime is global daemon provenance, not project-owned.

`stopped_at` and `stop_kind` are what let startup distinguish a runtime that
ended on purpose from one that was lost, and why it ended. A row with
`stopped_at IS NULL` that is not the current runtime was lost uncleanly. The
CHECK keeps the two columns from disagreeing.

These columns do not gate recovery. Reconciliation selects on ownership alone —
every non-terminal Operation owned by a runtime other than the current one
(§8.6) — so a runtime that stopped on purpose without confirming termination
does not strand its work. See §8.1 for why there is no separate lifecycle enum.

---

## 6.14 `operation`

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

## 6.15 `agent_invocation`

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

## 6.16 `verification_run`

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

## 6.17 `verdict`

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

## 6.18 `durable_event`

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

## 6.19 `command_record`

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

## 6.20 Cross-table atomic invariants

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

## 6.21 Task lifecycle v1

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

> **OPEN — closed when the scheduler exists and has run real work.**
> The default Task transition after an attempt ends `Failed`, `Cancelled`, or
> `Interrupted`, and which layer chooses it, is not decided. See §8.9.

---

## 6.22 FTS5 strategy

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

## 6.23 SQLite connection policy

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

> **OPEN — closed by the file-backed WAL validation described immediately above.**
> The final SQLite writer strategy is undecided: whether write transactions take
> an explicit `BEGIN IMMEDIATE`, or another serialization mechanism, or neither.
> This is the one open question with a scheduled experiment rather than a distant
> trigger, and it also settles whether `durable_seq` assignment order matches
> commit order under contention (§6.18).

---

# Section 7 — Migrations


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
---

# Section 8 — Runtime and Execution Model

This section describes how Shadows actually runs work: what a runtime instance
is, how dispatch claims a task, what happens around spawn, what shutdown means,
and how a restart reconciles what a previous runtime left behind.

It does not restate decisions made elsewhere. Cancellation's two-transaction
shape is §2.3, two-phase spawn is §2.7, process-tree containment is §1.5, the
scheduler's purity is §2.6, and the durable tables are §6. This section adds only
what those do not already decide.

## 8.1 RuntimeInstance lifecycle

One `RuntimeInstance` row is created when `shadows serve` starts, and it owns
every Operation that runtime dispatches. Ownership is what makes recovery
decidable: an Operation's owning runtime is either this one or a previous one,
and the two cases are handled differently.

Startup, before any work is accepted, performs configuration load, storage
migration, **acquisition of exclusive local-runtime ownership**, process
containment setup, and recovery of what a previous runtime left behind (§8.6).
Exclusive ownership is not an optimisation; §8.6 depends on it.

Runtime rows are diagnostic and recovery provenance. They are **not leases**, and
they do not enable remote or active-active execution.

**There is no RuntimeInstance state enum.** Lifecycle is derived from
`started_at`, `stopped_at`, and `stop_kind` (§6.13):

```text
stopped_at IS NULL, and this is the current runtime   -> active
stopped_at IS NULL, and it is not                     -> lost uncleanly
stopped_at set, stop_kind = Graceful                  -> stopped, work concluded
stopped_at set, stop_kind = Escalated                 -> stopped, work abandoned
```

`stop_kind` records how a runtime ended. It does **not** decide whether that
runtime's Operations are reconciled: recovery looks at ownership alone (§8.6),
because `Escalated` and unclean loss both leave work unfinished and only the
reason differs.

An enum would be a second copy of a fact the timestamps already carry, and two
copies of one fact eventually disagree. A `Draining` state in particular has no
durable meaning: draining is something a runtime is *doing*, not something a
later runtime recovers from. What the next runtime needs to know is whether this
one concluded its work, and `stop_kind` answers that directly.

## 8.2 Dispatch

Dispatch is the step between the scheduler's decision and any side effect. The
scheduler is pure (§2.6) and its decision may be stale by the time dispatch runs,
so dispatch treats it as a proposal and **revalidates inside the claiming
transaction**:

```text
workflow version is still runnable
task is still Ready
dependencies and gates still permit execution
no active execution Operation exists for the task
effective scope is inside declared scope
workspace policy has capacity
no active conflicting write scope exists
```

If every check still holds, one transaction claims the work:

```text
TX
  Task Ready -> InProgress          (exact CAS on the observed state)
  + Operation(Pending, kind = ExecutionRun) owned by this runtime
  + AgentInvocation, frozen
  + TaskTransitioned and OperationCreated durable events
  (+ CommandRecord when externally commanded)
COMMIT
```

No `OperationStarted` event is emitted here. Nothing has spawned.

**`AgentInvocation` is frozen at claim time, not read at spawn time:**

```text
operation_id
role
harness kind, profile, and resolved executable identity + version   (§1.4)
model / provider selection when applicable
context reference or digest
effective read / write / network permissions
execution workspace identity
timeout / budget
```

It must never persist resolved secret values or raw child environment values.
Reading any of these later would let a configuration change between claim and
spawn alter what the durable record says was run.

A failed CAS is not an error. It means another dispatch won or the task moved,
and the correct response is to re-decide rather than retry the write.

**Waiting is not blocking.** If capacity or a conflicting write scope prevents
dispatch, the Task stays `Ready` and is reconsidered deterministically later. It
does not become `Blocked`, and no speculative `Pending` Operation is created.

## 8.3 Prepare, harness selection, and spawn

Between claiming work and spawning it there is a **Prepare** step: resolving the
harness executable, validating that the harness supports what the invocation
froze, building the child environment, and readying the workspace.

```text
frozen AgentInvocation
    -> select AgentHarness
    -> validate harness capabilities against the frozen invocation
    -> translate to ProcessSpec
    -> resolve secrets at the last responsible moment
    -> process::spawn(ProcessSpec)
    -> register ProcessHandle under operation_id
    -> exact CAS Pending -> Running
```

Prepare can fail, and its failure is not a spawn failure — the process never
existed. It transitions `Pending -> Failed { stage: Prepare }`, keeping "we could
not get ready" distinct from "the OS refused to start it" in the durable record
and in diagnostics. Spawn failure itself follows §2.7 unchanged.

**An unsupported capability produces a structured `Blocked` operation outcome
before spawn.** It never results in silently widening permissions, and a harness
never widens the effective scope or permissions the invocation froze.

Every managed process belongs to exactly one Operation and exactly one
RuntimeInstance. No Operation becomes `Running` before its handle is registered
and the durable compare-and-swap commits — a registered handle without a
committed transition, or a committed transition without a registered handle, is
a state the runtime must not produce.

Planner and Executor roles always go through `AgentHarness`. Deterministic
verifier commands construct a `ProcessSpec` through the verification boundary
instead, without pretending to be an AI agent.

**A persisted raw PID is never killed after a restart.** The operating system
reuses PIDs, so a number recorded by a previous runtime may now belong to an
unrelated process. Termination always goes through the containment handle that
owns the tree (§1.5), never through a PID read from the database. A runtime that
does not hold the handle cannot terminate the tree and must not pretend it can;
see §8.6.

## 8.4 The cancel / exit interlock

Handle registration, natural process exit, and cancellation all race for one
Operation. They are serialized per Operation, and exactly one of them writes the
terminal transition. The required behaviour, case by case:

1. **Cancel arrives before spawn begins.** Do not spawn. Confirm no handle or
   tree exists. `Pending -> Cancelled`.
2. **Cancel arrives while spawn is in progress.** If spawn returns a handle,
   register it for termination only; never commit `Running`. Terminate and reap
   the tree, then transition to `Cancelled`.
3. **Cancel arrives after `Running`.** Terminate and reap the registered tree,
   then transition to `Cancelled`.
4. **The process exits naturally before cancellation takes termination
   ownership.** Persist `Completed` or `Failed` from the real exit. The cancel
   command observes `AlreadyTerminal` (§2.3). Shadows does not claim to have
   stopped something that had already stopped.
5. **Cancellation takes termination ownership before a natural exit is
   observed.** A late completion must not overwrite `Cancelled`.
6. **Termination fails or cannot be confirmed.** Preserve the cancellation
   request and **do not write `Cancelled`.** The Operation stays non-terminal
   until termination is confirmed or an honest `Interrupted` becomes possible.
7. **The client disconnects.** Nothing is cancelled. The Operation continues and
   durable truth is unchanged.

There is no `Cancelling` lifecycle state in v1. A client may display "Stopping"
whenever cancellation metadata exists on a non-terminal Operation; that is a
rendering decision, not a durable state.

## 8.5 Daemon shutdown

Shutdown reuses the cancellation path in §2.3 and §8.4. There is no separate
drain mode and no per-operation shutdown policy.

```text
stop signal
  -> request cancellation on every non-terminal Operation owned by this runtime
  -> containment terminates each managed tree
  -> confirmed termination -> terminal Cancelled
  -> runtime_instance.stopped_at set, stop_kind = Graceful
  -> exit
```

**`stop_kind = Graceful` is a claim, and it is only written when it is true:
every Operation this runtime owned is terminal.** A runtime that cannot reach
that state does not get to record `Graceful`. This is the guarantee §8.6 relies
on, and §8.6 checks it rather than assuming it.

A second stop signal escalates: the runtime stops waiting for confirmation, sets
`stop_kind = Escalated`, and exits. Operations that never reached confirmed
termination are **left non-terminal on purpose** and become `Interrupted` at the
next startup (§8.6). Recovery selects them by ownership, not by `stopped_at`, so
recording a clean exit time does not hide them.

Both properties that matter here follow from reusing one mechanism instead of
adding a second. A bounded drain would still need the cancellation path when its
bound expired, so it buys a second code path and a timeout constant in exchange
for nothing. And escalation never invents a terminal state it cannot prove:
`Cancelled` keeps meaning confirmed termination exactly as §2.3 requires, and an
unconfirmed operation is recorded as interrupted rather than as cancelled.

## 8.6 Crash recovery and orphan reconciliation

Once exclusive runtime ownership is acquired (§8.1), startup scans **every
non-terminal Operation whose owning runtime is not this one**, regardless of how
that runtime ended.

```text
old Pending -> Interrupted { reason: PreviousRuntimeEndedBeforeStart }
old Running -> Interrupted { reason: PreviousRuntimeEndedDuringRun }
```

The predicate is deliberately not `stopped_at IS NULL`. Three different endings
can leave an Operation non-terminal, and only one of them is an unclean loss:

```text
stopped_at IS NULL             lost uncleanly; operations stranded by a crash
stop_kind = Escalated          stopped on purpose without confirming termination (§8.5)
stop_kind = Graceful           must own no non-terminal Operations
```

Filtering on `stopped_at IS NULL` would strand every Operation an `Escalated`
shutdown left behind — permanently, because no later runtime would ever look at
them again. Filtering on ownership alone cannot have that failure mode, and it
cannot be defeated by a `stop_kind` value added later.

The reason codes name the *phase* the attempt reached, not how the runtime
ended. How it ended is already recorded on the runtime row, and recording it
twice would create two facts that can disagree.

**A `Graceful` runtime owning a non-terminal Operation is a defect, not a case
to handle silently.** Graceful shutdown does not complete until every Operation
it owns is terminal (§8.5), so such a row means that guarantee was violated.
Recovery still reconciles it to `Interrupted` — leaving it stranded would be
worse — and emits a `recovery.anomaly` event naming the runtime and the
Operation. Recovery is the only place this invariant can be observed, so it is
the only place it can be reported.

Each transition uses an exact compare-and-swap on operation id, expected status,
and previous runtime instance, and appends its durable event in the same
transaction.

`Interrupted` is a factual statement: this attempt did not conclude, and Shadows
cannot say whether its effects landed. It is never converted into success or
failure, and it never triggers an automatic retry. Whether a new attempt happens
is decided by the owning workflow or by a later user command, which creates a
**new** Operation with causal linkage to the previous one. Terminal states never
transition again.

**Recovery is a database operation and is not process cleanup.** Before recording
the interruption of an old `Running` Operation, the runtime must have evidence
that the old runtime cannot still own a live tree: exclusive daemon ownership
plus the platform containment contract of §1.5. **If that evidence is
unavailable, startup fails closed** rather than allowing two owners. A runtime
that reconciles rows while another runtime's children are still alive has
recorded a lie.

An old Operation is never adopted as though its process handle survived. The
handle died with its runtime; only the durable row remains.

If a cancellation request was durable when the runtime died, it stays visible on
the interrupted record. The outcome is `Interrupted`, not `Cancelled`, because
the final process outcome and its exact cause are unknown after a crash.

## 8.7 Observability and tracing boundaries

Stable correlation fields, present on every runtime span where they apply:

```text
runtime_instance_id
project_id
thread_id
workflow_id
task_id
operation_id
agent_invocation_id
correlation_id
```

Spans and events the runtime emits:

```text
runtime.start / stop
scheduler.decision
dispatch.claimed / dispatch.rejected
workspace.prepare
agent.invocation.start
process.spawn / exit / terminate / reaped
operation.transition.committed
verification.start / verdict
recovery.reconcile
recovery.anomaly
```

**A durable state-transition log is emitted only after its transaction commits.**
A line emitted before commit describes something that may never have happened.

**Logs diagnose; responses, durable state, and process lifetime prove.** A
transient process log is never evidence that a durable transition occurred, and
the absence of a log is never evidence that one did not.

Never log secrets, complete environments, raw prompts, model output, provider
payloads, bootstrap credentials, or sensitive filesystem paths by default. Debug
mode may add diagnostic detail; it does not disable redaction.

Operational events originating in the harness rather than in Shadows — retries,
rate-limit signals, and the like — are forwarded to the client rather than
discarded. A harness that stalls silently while retrying is indistinguishable
from a hung daemon if nothing surfaces the retry.

## 8.8 Failure and crash matrix

| Point | Required result |
|---|---|
| Before the dispatch transaction | No Task or Operation mutation. The scheduler re-decides. |
| Dispatch transaction rolls back | Task remains `Ready`; no Operation exists. |
| Prepare fails | `Pending -> Failed { stage: Prepare }`. No process existed. |
| Spawn fails | `Pending -> Failed { stage: Spawn }` (§2.7). Task resolution follows explicit workflow policy. |
| Spawn succeeds but the `Running` CAS fails because cancel won | Terminate and reap the tree; never claim `Running` (§8.4 case 2). |
| Runtime dies after the claim, before spawn | No child existed. Next startup: `Pending -> Interrupted`. |
| Runtime dies after spawn, before `Running` commits | Containment kills the tree. Next startup: `Pending -> Interrupted`. |
| Runtime dies while `Running` | Containment kills the tree. Next startup: `Running -> Interrupted`. |
| Cancel committed, termination cannot be confirmed | Preserve the request; never write `Cancelled` (§8.4 case 6). |
| Cancel committed, then the runtime dies before the tree dies | Containment kills the tree. Next startup reconciles to `Interrupted`, because the requesting runtime never confirmed termination. |
| Process exits normally while a cancel is in flight | Natural exit wins (§8.4 case 4). |
| Process exits but the terminal commit fails | Retain the runtime result and retry persistence while the runtime lives. After runtime loss, reconcile honestly as `Interrupted` if the outcome never committed. |
| Crash during a terminal transition | Rolled back. The Operation is still non-terminal and reconciles at startup. |
| Client or SSE disconnects | The Operation continues; durable truth is unchanged. |
| Live-event publication fails after commit | The durable journal remains truth; the client resyncs (§2.10). |
| Verification fails | Evidence persists; the Task does not complete. |
| Conflicting write scope | Task remains `Ready` and waits. No speculative Operation. |
| Graceful shutdown | Every owned Operation reaches a terminal state before `stop_kind = Graceful` is written (§8.5). |
| Escalated shutdown | Unconfirmed Operations are left non-terminal on purpose. Next startup reconciles them by ownership: `Interrupted` (§8.6). |
| A `Graceful` runtime is found owning a non-terminal Operation | The §8.5 guarantee was violated. Reconcile to `Interrupted` and emit `recovery.anomaly`; never leave it stranded. |

## 8.9 Runtime questions this project cannot answer yet

Each item below names what closes it. None are reachable from the first runnable
milestone, which has no workflow, no scheduler, no Executor, and no verification.

> **OPEN — closed when the scheduler exists and has run real work.**
> After an execution attempt ends `Failed`, `Cancelled`, or `Interrupted`, does
> the Task become `Ready`, `Failed`, or `Blocked` by default, and which layer
> makes that choice? This is the same question as "what is the automatic retry
> policy", stated from the Task side. Deciding it now would mean guessing at
> failure distributions nobody has observed. The refusal in §8.6 to convert
> `Interrupted` into success or failure already rules out the dangerous answers.

> **OPEN — closed when parallel execution is built.**
> What conservative path-scope representation proves two write scopes disjoint,
> and what deterministic fairness key orders Tasks waiting on the same scope?
> Both are properties of a real workload. What is already decided regardless of
> the representation: conflicting writers never run concurrently, effective scope
> is a subset of declared scope (§4.3), and a Task waiting on a conflict stays
> `Ready` (§8.2).

> **OPEN — closed when an Executor role exists.**
> Which workspace modes are available — direct checkout, branch, worktree — what
> their capacity rules are, and what a Planner or Research operation may read
> while an Executor owns the checkout. Already decided: branch switching is
> exclusive to its Project checkout, and worktree isolation is an available
> mechanism rather than a product requirement. Milestone 0 has one mode — read
> the project directory in place — and no Executor to conflict with.

> **OPEN — closed when deterministic verification is built.**
> What workflow-policy transition follows a required `VerificationRun` that
> failed or was cancelled. §2.8 already fixes that verification is separate from
> AI review and that execution completion does not imply Task completion; what
> remains is the policy, which depends on gate semantics not yet exercised.

---

# Section 9 — Cross-cutting Rules

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

# Section 10 — Out of Scope for v1

Shadows deliberately does not build the following. These are not open questions;
they are decisions to leave things out.


The current design intentionally does **not** decide:

- separate domain/storage crates;
- PostgreSQL production adapter implementation date;
- SeaQuery adoption;
- active-active multi-runtime support;
- reviewer implementation timing;
- secret hardening crates until lifecycle requirements justify them;
- a generic DB abstraction layer;
- a generic transaction-composition DSL.

Three items that used to sit in this list have moved, because they are open
questions rather than exclusions: automatic retry policy is §8.9, the Context
Compiler's ranking algorithm is §2.11, and the SQLite writer strategy is §6.23.

The non-binding long-term collaboration direction — a future team server,
multi-user accounts, and synchronisation — is **out of scope for v1 and adds no
types or requirements to it**. It is recorded in the project's history rather
than in this document, so that a future intention cannot be mistaken for a
current requirement.

---

# Section 11 — Readiness Milestones


Readiness is incremental. Feature work may build on a completed earlier layer
without waiting for every later layer; dependencies between milestones remain
explicit.

## 11.1 First Runnable Browser Planner

Order of work:


1. scaffold the single Rust crate, `shadows serve`, structured tracing, and the independent Web client;
2. open SQLite through SQLx with only the milestone schema;
3. implement local-directory Project and durable PlanningThread/ThreadEntry;
4. implement the managed process primitive and one Claude harness;
5. implement durable Planner Operation start, live SSE output, semantic stop, and restart reconciliation;
6. connect the browser UI to create/resume a thread, Start, show output, and Stop;
7. run the real Windows debug acceptance path and record exactly what remains unverified on Linux.

The milestone is incomplete until the user can operate this path from a browser.
Persistence-only or protocol-only completion is not an acceptable substitute.

Acceptance:


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

## 11.2 Persistence Foundation Ready


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

Additionally, before the persistence foundation is considered complete:

```text
transaction rollback fault injection
active ExecutionRun uniqueness
crash recovery from Pending and Running
FTS trigger parity
```

Future PostgreSQL work must run the same semantic storage-contract suite.

## 11.3 Runtime / Execution Ready


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

## 11.4 Protocol / MCP Ready


```text
[ ] HTTP command idempotency contract passes
[ ] SSE durable replay + live no-gap handoff passes
[ ] stable public errors/outcomes pass compatibility tests
[ ] MCP authority/binding passes
[ ] MCP mutations use the same application semantics
```

## 11.5 Continuity Ready


```text
[ ] durable PlanningThread entries pass
[ ] Context Compiler is deterministic, budgeted, and scoped
[ ] Claude -> Codex fake-harness continuity passes
[ ] native_session_id remains optional
[ ] snapshot/history pagination passes under large history
```

## 11.6 Later feature slices


After Milestone 0 works, add features in user-visible slices rather than
constructing the entire platform upfront:

1. Workflow draft/freeze and task display;
2. scheduler and execution;
3. deterministic verification;
4. context compilation and Claude/Codex continuity;
5. MCP-attached agents;
6. research artifacts/search;
7. AI Reviewer and team features when separately designed.

---

*End of specification.*
