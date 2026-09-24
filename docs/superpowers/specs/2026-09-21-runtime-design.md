# Section 8 — Runtime and Execution Model

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

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

A Planner turn freezes this as §12.7 states. Which of these permissions Shadows
enforces itself, rather than leaving to the harness's mode, is §12.5's OPEN block.

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

In Milestone 0 readying the workspace means the thread's project has a directory
(§4.2) and it is still a directory; the turn runs there. Neither is supplied by the
client, and there is no fallback to the daemon's working directory: a failure is a
`Prepare` failure naming which of the two was missing.

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
terminal transition. From Milestone 1 a Planner turn is a prompt on a live ACP
connection rather than a process of its own; §12.3 states how these cases read
there, including when the harness's own `cancelled` answer confirms a stop. The
required behaviour, case by case:

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

Shutdown waits for that confirmation up to a bound. When the bound expires, or a
second stop signal arrives, the runtime stops waiting, sets
`stop_kind = Escalated`, and exits. The bound can only turn a would-be
`Graceful` into `Escalated`, never the reverse; without it, one reader that
never finishes would hang a single Ctrl-C forever. Operations that never reached confirmed
termination are **left non-terminal on purpose** and become `Interrupted` at the
next startup (§8.6). Recovery selects them by ownership, not by `stopped_at`, so
recording a clean exit time does not hide them.

Both properties that matter here follow from reusing one mechanism instead of
adding a second. A bounded drain — letting turns finish before cancelling them —
would still need the cancellation path when its bound expired, so it buys a
second code path in exchange for nothing; the confirmation bound above is not a
drain, since every turn is cancelled at once. And escalation never invents a terminal state it cannot prove:
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

Work that outlives the request that started it runs under its own root span,
never as a child of the request's: a Planner turn's lines carry `planner.turn`
and its ids, not the `http` span of a POST that answered 202 long before.

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

**Debug mode.** `shadows serve --debug` raises the default filter to
`shadows=debug` (`RUST_LOG`, when set, still decides) and writes every line to
stderr **and** to a new plain-text file,
`<data dir>/logs/shadows-<UTC start>-<pid>.log`, where the data directory is the
one holding the database. One file per daemon start; `tracing-appender` writes
it, and the daemon holds its flush guard for its whole lifetime so the lines
that explain how a run ended reach disk. `serve` prints the file's path on a
second stdout line, after its address. Without `--debug`: `shadows=info`, stderr
only, no file. Every HTTP request logs one `http.response` line (method, path
without query, status, latency); a request's size is logged at debug, its body
never.

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
