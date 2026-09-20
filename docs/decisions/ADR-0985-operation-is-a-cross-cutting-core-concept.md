# Operation is a cross-cutting core concept

**Doc ID:** 985
**Status:** accepted
**Tags:** architecture, foundation, long-running, operation
**Source slug:** operation-is-a-cross-cutting-core-concept

---

# Decision — Operation lifecycle with two-phase spawn

**Status:** accepted · 2026-09-20

## Rule

`Operation` is its own concept. `Run = Operation` is wrong.

Many things are long-running operations and must survive a client
disconnect:

```text
PlannerTurn
ExecutionRun
Verification
Reviewer (future)
Research operation
Git / worktree operation
```

ALL long-running actions in shadows enter this lifecycle — including
Planner and Verification. This is not optional.

## Two-phase spawn

`Operation` has a Pending phase that separates "created in DB" from
"spawn succeeded". This avoids durable events claiming a start that
did not actually happen.

```text
TX #1:
  insert Operation{status: Pending}
  append OperationCreated
COMMIT

attempt spawn:
  re-check no durable cancellation request
  process::spawn(spec) →
    Ok →
      TX #2:
        exact CAS Pending → Running only if cancellation is still absent
        append OperationStarted
      COMMIT
    Err →
      TX #2:
        Pending → Failed
        append OperationFailed { stage: Spawn, reason }
      COMMIT

Pending → { Failed(stage=Spawn) | Cancelled | Interrupted }
Running → { Completed | Failed | Cancelled | Interrupted }
```

The same pattern is used for:

- `PlannerTurn`
- `ExecutionRun`
- `Verification`
- `Reviewer` (future)

`Operation` is a real primitive, not a record around execution.

## Full lifecycle

```rust
enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed { stage: FailureStage, reason: String },
    Cancelled,
    Interrupted, // see below
}

enum FailureStage {
    Spawn,
    Invocation,
    Run,
    Completion,
}
```

## Why these states

```text
Pending → Running → { Completed | Failed | Cancelled | Interrupted }
   │
   ├─ confirmed cancellation before Running → Cancelled
   └─ old runtime / crash window → Interrupted
                │
                └─ timeout → Cancelled
```

`Interrupted` is used when the runtime disappeared (process restart,
crash, kill -9) and we cannot honestly say the operation finished.
We do NOT lie by leaving the row as `Running` after restart. We do
NOT auto-convert to `Failed` because failure is a specific outcome
with specific evidence.

On startup, Shadows reconciles operations owned by an older runtime that
were `Pending` or `Running`, using an exact compare-and-swap over operation
id, expected status, and runtime instance:

```text
Pending/Running at restart
   ↓ reconcile
Interrupted (with reconciliation metadata)
```

## Spawn crash window

If spawn succeeds and the daemon dies before TX #2, the database still
shows `Pending` while a child may exist. Database reconciliation does not
terminate that child. `process/` must contain the complete managed process
tree so it cannot survive loss of the owning runtime indefinitely.

The startup CAS records `Interrupted`; OS-level containment closes the
child/grandchild process side of the same failure.

## Spawn/cancel interlock

Cancellation request is durable metadata orthogonal to lifecycle status.
The spawn path checks it before spawn and again in the `Pending → Running`
CAS. If cancellation wins the race, Running is not committed; any spawned
tree is terminated before terminal `Cancelled` is persisted.

A future resume can either retry (creating a new Operation) or
continue based on policy decided by the Workflow / Gate.

## F-06 lesson

shadow's `Running` row was never reset merely because the application
restarted. That was correct on the audit side (the record stays), but
wrong on the runtime status side (we should not claim it is `Running`
when no handle exists). This rule splits the two concerns.

## Implementation rule

- `execution::ExecutionRun` references `operation_id`. Same for
  `verification::VerificationRun`, future `review::ReviewRun`.
- `Operation` outlives any client connection and any process restart.
- `AgentRunHandle` and `ProcessHandle` are per-process.

Related: [[agent-seam-role-agentharnessstart-agentrunhandle]], [[events-journal-bus-subscribe-after]], [[process-owns-child-process-primitive-with-processspec]]
