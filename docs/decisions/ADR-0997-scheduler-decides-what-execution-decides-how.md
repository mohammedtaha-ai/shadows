# Scheduler decides WHAT, Execution decides HOW

**Doc ID:** 997
**Status:** accepted
**Tags:** architecture, execution, scheduler, separation
**Source slug:** scheduler-decides-what-execution-decides-how

---

# Decision — Scheduler is pure decision engine

**Status:** accepted · 2026-09-20

## Rule

```text
Scheduler decides WHAT may run.
Execution decides HOW it runs.
```

The scheduler is a deterministic decision function. It does NOT spawn
processes, write to SQLite, create worktrees, run verifiers, or hold
execution state.

## Pure signature

```rust
fn decide(input: ScheduleInput) -> ScheduleDecision;

struct ScheduleInput {
    workflow: WorkflowSnapshot,
    task_states: Vec<TaskState>,
    active_runs: Vec<OperationId>,
    resource_scopes: Vec<Scope>,
    // caller builds this from authoritative entity tables
}

struct ScheduleDecision {
    ready: Vec<TaskId>,
    blocked: Vec<(TaskId, BlockReason)>,
    parallel_safe: Vec<TaskId>,
    execution_intents: Vec<ExecutionIntent>, // direct, not re-derived
}
```

The caller (typically `execution::dispatch`) builds `ScheduleInput`
from the authoritative entity tables and passes it in. The scheduler
does no I/O.

This keeps the scheduler testable as a pure function and lets us
fan-out to multiple executors later by adding only dispatch logic,
without changing the scheduler shape.

## Superseded workflows

Workflow lineage has one authoritative stored direction:

```text
new.previous_version_id = old.id
```

The old workflow does not store `superseded_by`. In v1,
`UNIQUE(previous_version_id)` permits at most one successor. The caller
derives whether a workflow is superseded by looking up a successor whose
`previous_version_id` equals that workflow's id and includes that fact
in `WorkflowSnapshot`.

The scheduler MUST return zero ready tasks when that derived successor
exists. The dispatch transaction MUST also re-check and reject a workflow
version that already has a successor, so a stale scheduling decision
cannot dispatch work after supersession.

Existing runs on the old version either:

- complete on the old version (default), or
- are cancelled by an explicit policy decided at supersede time

This is decided by the supersede operation, not by the scheduler.

## Where it lives

```text
execution::dispatch(input)
   │
   ├── builds ScheduleInput from authoritative state
   ├── scheduler::decide(input) → ScheduleDecision
   └── for each ExecutionIntent:
        operation::start(...)
```

Related: [[frozen-workflow-immutable-with-supersede]], [[operation-lifecycle-with-recovery]]
