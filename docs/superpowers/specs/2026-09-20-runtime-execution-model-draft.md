# Section 5 Draft — Runtime & Execution Model

- **Status:** review draft; not yet part of the canonical specification
- **Date:** 2026-09-20
- **Scope:** runtime, scheduling, managed agents, processes, cancellation,
  recovery, filesystem strategy, parallelism, verification execution, and
  runtime observability
- **Out of scope:** HTTP/SSE wire design, MCP, final config syntax,
  implementation plan, and product code

This document isolates the runtime proposal so the canonical specification does
not grow while the design is still under review. Accepted items are marked
explicitly. Everything else remains a proposal until reviewed.

## 5.0 Goals and vocabulary

The runtime must turn durable intent into managed work without confusing three
different kinds of state:

```text
Task       = business progress through a workflow
Operation  = durable lifecycle of one long-running attempt
Process    = runtime-only OS resource owned by one RuntimeInstance
```

One Task may have several sequential Operations across retries or rework. One
Operation owns at most one managed process tree. A process handle is never
durable truth.

### Accepted: Task and Operation do not mirror each other

`TaskState::InProgress` is a broad business state. It begins when dispatch
atomically claims a Ready task and creates its Pending execution Operation. It
continues through agent execution and required verification.

`OperationStatus` alone describes whether a particular attempt has actually
spawned, completed, failed, been cancelled, or been interrupted.

```text
Task:
  Pending -> Ready -> InProgress -> Completed
                                -> Failed
                                -> Blocked

Operation:
  Pending -> Running -> Completed
                     -> Failed
                     -> Cancelled
                     -> Interrupted
  Pending -> Failed | Cancelled | Interrupted
```

`TaskState::Running` is removed. A Pending Operation must never force the Task
model to claim that an OS process is already running.

## 5.1 RuntimeInstance lifecycle

### Proposal

Every daemon start creates one durable `RuntimeInstanceId`. A runtime passes
through:

```text
Starting -> Active -> Draining -> Stopped
             |
             +-- abrupt process loss leaves the durable row non-terminal
```

Suggested durable fields:

```text
id
version
started_at
accepting_work_at?
draining_at?
stopped_at?
stop_kind?       # Clean | StartupFailure | UncleanDetected
```

Rules:

- `Starting` performs configuration, storage migration, exclusive daemon
  ownership acquisition, process-containment setup, and recovery.
- `Active` may accept new Operations.
- `Draining` accepts no new work and supervises existing handles according to
  shutdown policy.
- `Stopped` is written only by a controlled shutdown.
- A newly started runtime may classify an older non-terminal runtime as
  unclean only after it has acquired the same exclusive local-runtime
  ownership boundary.
- Runtime rows are diagnostic and recovery provenance; they are not leases for
  remote or active-active execution.

Open review point: whether v1 needs the four-state enum or only timestamps plus
`stop_kind`. The lifecycle semantics above are required either way.

## 5.2 Operation execution lifecycle

### Proposal

Normal lifecycle:

```text
Pending -> Running -> Completed { outcome }
                   -> Failed { stage, failure }
                   -> Cancelled
                   -> Interrupted { previous_runtime, reason }

Pending -> Failed { stage = Prepare | Spawn }
Pending -> Cancelled
Pending -> Interrupted
```

Meanings:

- `Pending`: durable attempt exists, but Shadows has not committed that a
  managed process is running.
- `Running`: spawn succeeded, the handle is registered under the owning
  runtime, and the exact Pending-to-Running compare-and-swap committed.
- `Completed`: the operation produced a structured domain outcome. A
  successful Execution Operation does not by itself complete the Task.
- `Failed`: infrastructure or invocation failed with a typed failure stage.
- `Cancelled`: termination was requested and Shadows confirmed no managed
  descendant remains alive.
- `Interrupted`: the owning runtime disappeared or lost the ability to prove
  the attempt's terminal outcome.

Terminal states never transition again. A retry or rework creates a new
Operation with causal linkage to the previous attempt.

## 5.3 Scheduler -> ExecutionIntent -> Operation

### Proposal

The scheduler remains pure:

```text
scheduler::decide(ScheduleInput) -> ScheduleDecision
```

It may propose `ExecutionIntent`s, but a proposal grants no authority and
changes no state. The execution coordinator performs a transactional
revalidation immediately before dispatch:

```text
workflow version is still runnable
task is still Ready
dependencies and gates still permit execution
no active execution Operation exists for the task
effective scope is inside declared scope
workspace policy has capacity
no active conflicting write scope exists
```

If all checks still hold, one transaction commits:

```text
Task Ready -> InProgress
+ Operation Pending(kind = ExecutionRun)
+ AgentInvocation with frozen harness/profile/scope inputs
+ durable TaskTransitioned and OperationCreated events
```

No `OperationStarted` event is emitted here. Spawn has not occurred.

If capacity or scope conflict prevents dispatch, the Task remains `Ready` and
is reconsidered deterministically later. Waiting behind a conflicting Task is
not `Blocked` and does not create a Pending Operation.

## 5.4 AgentInvocation and AgentHarness lifecycle

### Proposal

`AgentInvocation` freezes what is handed to one harness attempt:

```text
operation_id
role
harness kind and profile
model/provider selection when applicable
context reference or digest
effective read/write/network permissions
execution workspace identity
timeout/budget
```

It must not persist resolved secret values or raw child environment values.

Flow:

```text
durable AgentInvocation
    -> select AgentHarness
    -> validate harness capabilities
    -> translate to ProcessSpec
    -> resolve secrets at the last responsible moment
    -> process::spawn(ProcessSpec)
    -> register ProcessHandle under operation_id
```

Planner and Executor always go through `AgentHarness`. Deterministic verifier
commands construct a `ProcessSpec` through the verification boundary without
pretending to be an AI agent.

An unsupported capability produces a structured `Blocked` operation outcome
before spawn, not silent permission widening.

## 5.5 ProcessSpec and child environment isolation

### Proposal

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

Rules:

- The daemon's global environment is never mutated for a child.
- The child environment is built explicitly; credentials are resolved only at
  spawn and never persisted, traced, or placed in command-line arguments.
- `HOME`, `USERPROFILE`, `APPDATA`, temporary directories, and provider config
  locations follow the selected isolation profile rather than leaking from the
  daemon by accident.
- Persisted diagnostics may include executable identity, argument count,
  workspace identity, profile name, and environment key names. They must not
  include prompt text, output text, secret values, or sensitive argument
  values.
- `process/` knows nothing about Task, Planner, Claude, Codex, Git, or
  verification semantics.

## 5.6 Cancellation state machine and spawn race

### Proposal

Cancellation request and terminal cancellation remain separate:

```text
TX cancel request:
  CommandRecord
  + Operation.cancel_requested_at/by
  + OperationCancellationRequested

runtime:
  prevent spawn or terminate managed tree
  + confirm no managed descendant remains

TX terminal:
  exact CAS Pending/Running -> Cancelled
  + OperationCancelled
```

Per-operation coordination serializes handle registration, process exit, and
cancellation ownership. The required race behavior is:

1. **Cancel before spawn begins:** do not spawn; confirm no handle/tree exists;
   transition Pending to Cancelled.
2. **Cancel while spawn is in progress:** if spawn returns a handle, register it
   only for termination; never commit Running; terminate and reap the tree,
   then transition to Cancelled.
3. **Cancel after Running:** terminate and reap the registered tree, then
   transition to Cancelled.
4. **Natural exit observed before cancellation takes termination ownership:**
   persist Completed or Failed; the cancel command observes AlreadyTerminal.
5. **Cancellation takes termination ownership before natural exit is
   observed:** terminal completion cannot overwrite Cancelled.
6. **Termination fails or cannot be confirmed:** preserve the cancellation
   request and do not write Cancelled. The Operation remains non-terminal until
   recovery or an honest Interrupted transition is possible.
7. **Client disconnect:** never implies cancellation.

The state machine does not expose `Cancelling` in v1. The UI may present
"Stopping" when cancellation metadata exists on a non-terminal Operation.

## 5.7 Process-tree containment on Windows and Linux

### Proposal

The portable contract is stronger than sending a signal:

> A managed child and every managed descendant must not survive loss of the
> owning Shadows runtime indefinitely.

Windows requirements:

- the process is assigned to a containment primitive such as a Job Object
  before it can escape supervision;
- breakaway is prevented unless a future explicit capability requires it;
- closing the owning runtime's containment handle terminates the managed tree;
- cancellation waits for confirmed tree termination/reaping.

Linux requirements:

- process group/session cleanup alone is not accepted as parent-death proof;
- the selected supervisor/parent-death mechanism must cover grandchildren or
  pair parent-death containment with verified tree cleanup;
- cancellation waits for the supervised tree, not only the direct child.

The implementation may use `process-wrap` where it satisfies the contract, but
the library name is not the architecture. Cross-platform readiness requires a
real daemon -> child -> grandchild probe on Windows and Linux.

After restart, Shadows must not kill an arbitrary raw PID from durable storage;
PID reuse makes that unsafe. Recovery relies on runtime-loss containment and
durable reconciliation, not blind PID killing.

## 5.8 Crash recovery and orphan reconciliation

### Proposal

After exclusive runtime ownership is acquired, startup scans non-terminal
Operations owned by older RuntimeInstances.

```text
old Pending -> Interrupted(reason = RuntimeLostBeforeStart)
old Running -> Interrupted(reason = RuntimeLostDuringRun)
```

Each transition uses exact CAS on operation ID, expected status, and previous
runtime instance, and appends a durable event in the same transaction.

Recovery does not claim that the work failed, succeeded, or was cancelled. It
also does not auto-retry. The owning workflow or a later user command decides
whether to create a new Operation.

Before recording interruption of an old Running operation, the runtime must
have evidence that the old runtime cannot still own a live tree: exclusive
daemon ownership plus the platform containment contract. If that evidence is
unavailable, startup fails closed instead of allowing two owners.

If a cancellation request existed when the runtime died, it remains visible in
the interrupted record. The draft recommends `Interrupted`, not `Cancelled`,
because the final process outcome and exact cause may be unknown after a crash.

## 5.9 Workspace and filesystem strategy

### Accepted

Workspace handling is user-configurable:

```text
Direct    # use the selected checkout as-is
Branch    # use a dedicated branch in the selected checkout
Worktree  # use a dedicated branch and separate worktree directory
```

`Branch` is the recommended default; `Worktree` is optional.

Rules:

- `Direct` never changes branches automatically.
- `Branch` may create/switch a branch only while Shadows holds exclusive
  filesystem ownership for that Project checkout.
- Shadows never switches a branch while another filesystem-bound operation is
  reading or writing that checkout.
- `Direct` and `Branch` admit one filesystem-bound Operation at a time per
  Project checkout.
- `Worktree` may admit parallel Operations only when their effective write
  scopes do not conflict and project concurrency capacity is available.
- A non-Git project may use `Direct`, but mutating work is serialized.
- Failure to create or prepare the selected workspace fails the Pending
  Operation at `Prepare`; no agent is spawned.

The Project identity remains neutral. A local path belongs to a workspace
binding/configuration, not to the Project ID itself.

## 5.10 Parallel execution and write-scope conflicts

### Accepted direction; exact scope algebra remains reviewable

Conflicting work runs sequentially. Non-conflicting work may run in parallel
only when the selected workspace mode provides independent filesystem roots.

Conflict evaluation is fail-closed:

```text
unknown/unbounded write scope conflicts with every writer
same path conflicts
ancestor/descendant paths conflict
repository-wide or Git-metadata mutation conflicts with every operation
scope pairs that cannot be proven disjoint conflict
```

The scheduler may identify parallel-safe intents, but the execution coordinator
rechecks active scopes and workspace capacity in the dispatch transaction.
Conflicting Tasks remain Ready and are reconsidered after the active scope is
released. They are not failed or marked Blocked merely for waiting.

Ordering among waiting conflicting Tasks uses an explicit stable key, proposed
as readiness sequence followed by Task ID. It never relies on query order.

An Operation's effective scope is frozen before spawn and must be a subset of
the Task's declared scope. A harness cannot widen it.

## 5.11 Verification execution lifecycle

### Proposal

Execution completion and Task completion remain different facts:

```text
Execution Operation completes successfully
    -> Task remains InProgress
    -> required Task-phase VerificationRun Operations are created
    -> checks execute in the same execution workspace/snapshot
    -> required verdicts commit
    -> Task becomes Completed only when all required checks pass
```

Each VerificationRun references its own Operation. Deterministic verification
uses the process boundary but not `AgentHarness`. AI review remains a separate,
later Operation kind.

A failed required check records its evidence and prevents Task completion. The
exact default transition after failure (Failed, Blocked for rework, or a new
Ready attempt) remains an open workflow-policy decision; the verifier does not
choose it.

Cancelling verification does not retroactively change the completed execution
Operation. The Task remains non-terminal until workflow policy resolves the
missing required verdict.

## 5.12 Runtime observability and tracing boundaries

### Proposal

Stable correlation fields:

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

Important spans/events:

```text
runtime.start / active / drain / stop
scheduler.decision
execution.dispatch.accepted / deferred
workspace.prepare
agent.invocation.start
process.spawn / exit / terminate / reaped
operation.transition.committed
verification.start / verdict
recovery.reconcile
```

Durable state-transition logs are emitted only after the transaction commits.
Transient process logs are diagnostic and never treated as proof of durable
state.

Never log secrets, complete environments, raw prompts, model output, provider
payloads, bootstrap credentials, or sensitive filesystem paths by default.
Debug mode may increase diagnostic detail but does not disable redaction.

## 5.13 Failure and crash matrix

| Failure point | Required result |
|---|---|
| Before dispatch transaction | No Task or Operation mutation |
| Dispatch transaction rolls back | Task remains Ready; no Operation |
| After Pending commit, before spawn | Startup/current runtime resolves to Interrupted or Cancelled if cancellation safely won before spawn |
| Workspace preparation fails | Operation Failed at Prepare; no agent process |
| Spawn fails | Operation Failed at Spawn; Task resolution follows explicit workflow policy |
| Spawn succeeds, Running CAS fails because cancel won | Terminate/reap tree; never claim Running |
| Runtime dies after spawn, before Running commit | Containment kills tree; next runtime records Interrupted |
| Client/SSE disconnects | Operation continues; durable truth is unchanged |
| Live-event publication fails after commit | Durable journal remains truth; client resyncs |
| Cancel command commits, termination cannot be confirmed | Preserve request; never write Cancelled |
| Process exits, terminal DB commit temporarily fails | Retain runtime result and retry persistence while runtime lives; after runtime loss reconcile honestly as Interrupted if outcome was not committed |
| Runtime dies while Running | Containment closes tree; next runtime records Interrupted |
| Verification fails | Evidence persists; Task does not complete |
| Conflicting write scope | Task remains Ready and waits; no speculative Operation |

## 5.14 Runtime invariants

1. A Task is never marked Completed solely because an agent process exited 0.
2. `TaskState::InProgress` and Operation runtime status are different axes.
3. At most one active execution Operation exists per Task.
4. Every managed process belongs to exactly one Operation and RuntimeInstance.
5. No Operation becomes Running before a handle is registered and the durable
   CAS commits.
6. No Operation becomes Cancelled before the managed tree is confirmed gone.
7. No client disconnect cancels work.
8. No old runtime Operation is adopted by a new runtime as if the same process
   handle survived.
9. No raw persisted PID is sufficient authority to terminate a process.
10. No AgentHarness widens frozen effective permissions or scope.
11. Conflicting writers never run concurrently.
12. Waiting for capacity or a scope conflict leaves a Task Ready.
13. Branch switching is exclusive to its Project checkout.
14. Worktree isolation is optional, not a product requirement.
15. Durable transition events are committed atomically with state.
16. Logs diagnose; responses, durable state, and process lifetime prove.

## 5.15 Open review decisions

The following are intentionally unresolved in this draft:

1. After an execution attempt is Failed, Cancelled, or Interrupted, should the
   Task become Ready, Failed, or Blocked by default, and which layer chooses?
2. Does RuntimeInstance need an explicit four-state enum, or are lifecycle
   timestamps and `stop_kind` sufficient?
3. What is the exact conservative path-scope representation used to prove two
   write scopes disjoint?
4. What deterministic fairness key should order Tasks waiting on the same
   scope?
5. During graceful daemon shutdown, are active Operations cancelled, allowed a
   bounded drain, or selected by per-operation shutdown policy?
6. Which workspace modes are available to Planner/Research operations that
   need filesystem reads while a Branch-mode Executor owns the checkout?
7. What workflow-policy transition follows a failed or cancelled required
   VerificationRun?

These decisions must be reviewed before this draft is merged into the canonical
specification. They are design questions, not implementation-plan tasks.

## 5.16 Canonical-spec reconciliation audit

This audit was run after the draft was written.

### Direct contradictions — RESOLVED 2026-09-21

All five vocabulary contradictions below were applied to the canonical
specification. `TaskState::Running` no longer exists anywhere; `OperationStatus`
keeps its own `Running`, which was never in conflict.

1. ~~The canonical model still defines `TaskState::Running`~~ — replaced with
   `TaskState::InProgress`, with prose explaining that Task progress and
   Operation runtime status are different axes.
2. ~~Canonical scheduler transactions still say `Task Ready -> Running`~~ — both
   occurrences now read `Task Ready -> InProgress`.
3. ~~Canonical execution/verification completion still says the Task remains
   `Running`~~ — now `Task remains InProgress` until required verification
   resolves.
4. ~~The canonical task-table constraint still permits `Running`~~ — the `task`
   CHECK constraint now lists `InProgress`.
5. ~~The canonical Task lifecycle section contains Running transitions~~ — every
   transition and its prose meaning were corrected, and open decision 1 is now
   referenced from that section.

Nothing else from this draft has been merged. Sections 5.1 through 5.15 remain
a proposal, including the parts marked "Accepted" inside them, which are
accepted *for this draft* and not yet baseline.

### Structural numbering conflict

The canonical document currently uses:

```text
Section 4.4  Persistence Model / API
Section 5    SQLite Schema Proposal
Section 6    Migrations and Implementation Work Remaining
```

The reviewed structure requested for the completed architecture is:

```text
4.3 Persistence API
4.4 SQLite schema
5   Runtime & Execution Model
6   Agent Harness & Context Compiler
7   Protocol / HTTP / SSE
8   MCP
9   Config / Secrets / Profiles
10  Git / Worktree / Workspace Isolation
11  Observability & Operational Recovery
```

Therefore the canonical persistence API and schema headings/subheadings need a
mechanical renumbering. Migration mechanics belong under the persistence
section. The current implementation-order text must not occupy Section 6;
implementation planning happens only after the architecture sections close.

### Missing canonical detail supplied by this draft

- RuntimeInstance startup, drain, controlled stop, and unclean-loss semantics;
- atomic AgentInvocation creation/freeze during dispatch;
- explicit `Prepare` failure before spawn;
- per-operation serialization of exit/cancel/handle registration;
- cancel-vs-natural-exit winner semantics;
- refusal to kill a persisted raw PID after restart;
- Direct/Branch/Worktree modes and their capacity rules;
- deterministic waiting behavior for conflicting write scopes;
- branch-switch exclusion while another operation reads/writes the checkout;
- environment isolation and redacted ProcessSpec diagnostics;
- concrete runtime tracing boundaries and failure matrix.

### Existing canonical rules already compatible

- scheduler is pure and dispatch revalidates before side effects;
- Operation uses two-phase Pending -> Running spawn;
- cancellation request is distinct from terminal Cancelled;
- Cancelled requires confirmed process-tree termination;
- client disconnect does not cancel an Operation;
- old-runtime Operations reconcile by exact CAS to Interrupted;
- effective scope is a subset of declared scope;
- execution completion does not imply Task completion;
- deterministic verification is separate from AI review;
- durable transition state and events commit atomically.

### Canonical ambiguities to resolve during review

1. `create ExecutionRun(Pending)` sometimes reads like a separate durable
   ExecutionRun entity, while the schema models it as
   `Operation(kind = ExecutionRun)`. The canonical text should use the latter
   consistently unless a separate entity is deliberately introduced.
2. Canonical spawn-failure text delegates Task transition to an unspecified
   retry policy. This remains Open Decision 1 in this draft.
3. The current RuntimeInstance schema stores only ID/version/start time and
   cannot distinguish controlled drain/stop from a stale unclean runtime.
4. The canonical AgentInvocation does not yet freeze effective scope, workspace
   identity, context reference/digest, or timeout/budget.
5. The full schema is a later architecture target, while Milestone 0 requires
   only its minimal subset; the canonical schema section must make that
   distinction consistently.
