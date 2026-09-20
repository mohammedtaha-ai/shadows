# Cancellation is semantic, not signal-level

**Doc ID:** 1003
**Status:** accepted
**Tags:** architecture, cancellation, portability, process
**Source slug:** cancellation-is-semantic-not-signal-level

---

# Decision — Cancellation is semantic, not signal-level

**Status:** accepted · 2026-09-20

## Rule

Cancellation request and terminal cancellation are separate facts.
`handle.cancel()` semantically terminates the managed process tree; it is
not `SIGTERM`. `OperationStatus::Cancelled` is persisted only after
Shadows confirms that no managed descendant remains alive.

## Durable flow

```text
External cancel command TX:
  CommandRecord
  + Operation cancellation-request metadata
  + OperationCancellationRequested

runtime/process layer:
  observe request
  terminate complete managed tree
  confirm no descendant remains alive

internal terminal TX:
  exact CAS Pending/Running → Cancelled
  + OperationCancelled
```

Cancellation request metadata is orthogonal to operation status and
contains the requester and request time. No `Cancelling` state is added in
v1 unless implementation evidence proves it necessary.

The spawn path checks cancellation before spawn and again in the exact
`Pending → Running` CAS, preventing a durable request from racing into a
committed Running state.

## Idempotency and terminal operations

- Same cancel command key and request fingerprint replays the stored result.
- A different cancel command against an already-terminal operation records
  its own `AlreadyTerminal`/current-operation outcome without process effects.
- If the daemon dies after the request TX, the next runtime can still observe
  and reconcile the durable request.

## Why

Writing the contract as `SIGTERM` makes it Unix-specific and says nothing
about runtime-loss containment or descendants. Platform mechanics remain
inside `process/`.

## What stays

- The `cancel: Future<Output = Result<()>>` field on
  `AgentRunHandle`.
- Durable request and terminal events are distinct.
- The supervisor's responsibility to terminate the entire process
  tree, not just the direct child.

Related: [[agentharnessstart-returns-agentrunhandle]], [[parallel-execution-isolation]]
