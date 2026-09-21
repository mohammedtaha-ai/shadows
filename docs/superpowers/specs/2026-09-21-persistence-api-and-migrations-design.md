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
