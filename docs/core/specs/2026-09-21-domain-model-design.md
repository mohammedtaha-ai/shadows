# Section 4 — Core Domain Model and Invariants

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

## 4.1 Identity types

Domain IDs are UUID-v4 newtypes unless a later decision explicitly changes one:

> **CLOSED — 2026-09-22.** Milestone 0 carried `String` ids against this rule while
> the modules that own them did not yet exist. All five now do, and the ids are
> newtypes: `ProjectId`, `ThreadId`, `ThreadEntryId`, `OperationId`,
> `RuntimeInstanceId`, each declared by the shared pattern in `crates/core-db/src/id.rs`; each other id is
> owned by its service's `model.rs`.
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
> domain types, so every id converts to a column at a service's store boundary
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
refused while the project holds a planning thread that is not removed
(`ProjectHasThreads`, 409; §16.5), and nothing is written. Otherwise, in one
write, its code index and its links both ways go (§15.4), its live project
grants are revoked (§13.7), and
`ProjectRemoved` is journaled. Afterwards it is NotFound everywhere: it is not
listed, every request that acts on it — a thread, a grant, a turn, a code
question, a link — is refused as not found, and a read of what it held (its
threads, grants, instructions, links) answers as for an unknown project.
Its slug stays taken: adding the same folder again needs another slug.

### PlanningThread

```rust
struct PlanningThread {
    id: ThreadId,
    project_id: ProjectId,
    title: String,
    status: PlanningThreadStatus, // Open | Closed
    harness_session_id: Option<String>,
    created_at: Timestamp,
    removed_at: Option<Timestamp>,
}
```

**The title.** A thread is created with the name its client gave it; the web
client's draft gives "New conversation" (§13.11). The name then follows the
conversation, the way Claude names its own sessions, and the row keeps
`title_source`, who wrote the current title:

| `title_source` | written by | replaces |
|---|---|---|
| `client` | create; every thread before migration 0010 | — |
| `plan` | legacy external drafts that created a thread before §16.4; migration 0010 marks the earlier ones: a version 1 written with the thread, same timestamp and title | — (nothing automatic replaces it: the plan named it) |
| `first_message` | the turn that appends the thread's first UserMessage, in its own write: the message's first non-blank line, whitespace collapsed, at most 60 characters ending in "…" | `client` |
| `harness` | the title the harness sends for the thread's session (§12.3), sanitized the same way; a placeholder ("New session", "New conversation", "Untitled", "New chat"), which Claude Code sends for a conversation too short to name, is no title; one equal to the current title writes nothing | `client`, `first_message`, `harness` |
| `person` | a rename by a person. None exists yet | — (nothing automatic replaces it) |

`harness` replaces `harness` because the adapter may first send the raw first
prompt as a fallback title and generate a real one at a later turn. A thread
that already had messages before migration 0010 is `client` but has had its
first message, so only a harness title renames it. Each change
journals `ThreadRetitled` { title, source } in the same write; neither carries a
`CommandId`, since neither is a person's command. A fork is titled after its
source with " (fork)" (§12.9) and keeps its source's `title_source`; its
messages are copies, so none of them is its first. Clients see the title only;
`title_source` is not in the API until a client needs it.

`harness_session_id` is the harness session the thread's turns continue
(evidence `SERVE_STREAM_SPIKE.md` Finding 3: `--session-id` on the first turn,
`--resume` with the same id after). The daemon chooses it and records it when a
turn that started it reaches the harness's turn-end — not at spawn, because a
turn that fails or is stopped earlier may leave no session, and a recorded id
that `--resume` rejects would fail every later turn. Clients never supply or see
it. The harness store is the source of truth for what the model remembers; the
thread's entries are the source of truth for what the user sees (see
*Continuity* below).

A thread can be removed under
[§16.5](./2026-10-01-project-plans-design.md#165-deleting-a-conversation).
That section owns the command's ordering, grant revocation and retained
history; §16.8 owns the deleted conversation's read-only page.

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

> **Note — `author: Principal` is `core_db::Actor` in the implementation.**
>
> `Actor` already has exactly this shape (`kind` + `id`) and already answers "who
> did this" for durable events, which is the same question. Declaring a second
> identical struct would record one decision twice. If the two ever need to
> diverge — a principal gaining fields an event actor must not carry — that is the
> point to split them, and this note is where to say so.

> **DECIDED 2026-09-25, and grown since.** `ThreadEntryKind` is an enum; each
> variant serialises to its stored string, and stored text is never rewritten.
> The list is the enum in `crates/core-threads/src/model.rs`; each value's owner:
> `UserMessage`, `AgentMessage`, `PermissionRefused` (§12.10); `PlanView`,
> `PlanApproved` (§13.9); `Subagent` (§22.2); `VisionProposal`, `MapProposal`,
> `RoadmapProposal` (§23.5); `ToolCall`, `TodoList`
> ([tool activity](../../backend/specs/2026-10-31-tool-activity-design.md));
> `PermissionAsked`, `PermissionAnswered`
> ([AYLUL agents and panel §7.8](../../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md)).
> A new kind names its owner here. The reasoning that kept the field as text until
> Milestone 2 is in Git at `f1459dcc`.

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

Not built ([future list](../future.md)).

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

Not built ([future list](../future.md)).

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
    plan_id: PlanId,
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

Project ownership and version writers are owned by §16.2 and §16.3; the
migration that replaces `thread_id` with `plan_id` is §16.9's.

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

Not built ([future list](../future.md)).

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

Built for `PlannerTurn` only: no `workflow_id` or `task_id`, and no other kind is
written yet (§2.7).

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

Not built ([future list](../future.md)).

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

Not built ([future list](../future.md)).

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

### Attachment

(Added 2026-10-22, Mohammed's ruling: a person pastes or picks images for a message, and
`Threads` owns them.)

```rust
struct Attachment {
    id: AttachmentId,          // opaque, owned by threads
    thread_id: ThreadId,
    media_type: String,        // image/png, image/jpeg, image/gif, image/webp first
    byte_len: u64,
    sha256: [u8; 32],
    name: Option<String>,      // the file name the person gave, display only
    created_at: Timestamp,
}
```

- **Owner:** `Threads`. It stores the bytes and the record; no other service writes them.
- **Upload is its own command** (`POST /api/threads/{id}/attachments`, a `CommandId`, the
  bytes). It checks the media type by content, not by name (PNG, JPEG, GIF, WebP), and a
  size limit of 5 MiB (5 242 880 bytes) per image, the limit the model providers accept;
  over it the upload is refused before anything is written. It answers the `AttachmentId`. An attachment is immutable: a new paste is a new attachment.
- **A message names its attachments in order.** A turn start, a queued message and a
  steered message carry `attachments: [AttachmentId]` beside their text; the ids are part
  of the command fingerprint, and each must belong to the same thread. The user entry
  records them, so the transcript shows them and a fork copies the references.
- **A message may be image-only.** Empty text with at least one attachment is a message.
- **Backend-neutral input.** `Turns`' prompt composition stays the one place that builds
  the model's input; it builds neutral blocks (text, image) from the entry. The ACP
  adapter alone turns them into ACP content blocks, so `runtime/` later reads the same
  blocks. No HTTP shape and no stored entry holds an ACP type.
- **Capability is the harness's answer.** Core keeps the prompt capabilities the harness
  reports when its session starts; when it does not accept images, an attachment-bearing
  turn is refused before anything is written, with that reason.
- **Removed with the thread.** Removing a thread removes its attachments' bytes.
- **Later, not now:** files other than images, attaching a folder, and connectors.

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

Not built: no executor exists yet.

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
