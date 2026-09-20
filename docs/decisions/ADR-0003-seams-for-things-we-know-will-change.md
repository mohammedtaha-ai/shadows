# Seams for things we know will change

**Doc ID:** 979
**Status:** active
**Tags:** architecture, extensibility, foundation, seams
**Source slug:** adr-0003-seams-for-things-we-know-will-change

---

# Decision — Agent seam: Role, AgentHarness::start, AgentRunHandle

**Status:** active · 2026-09-20

## What this is

The agent seam is the only place AI work happens. It encodes:

```text
Role != Harness != Provider != Model
```

Examples:

```text
Role: Planner
Harness: Claude Code CLI
Provider: MiniMax
Model: MiniMax-M3
```

Another:

```text
Role: Executor
Harness: Codex CLI
Provider: OpenAI
Model: ...
```

The same workflow, the same task contracts, the same project truth
must not change because the agent changes.

## Trait surface

```rust
trait AgentHarness {
    async fn start(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentRunHandle>;
}

struct AgentInvocation {
    role: Role,                  // Planner | Executor | Reviewer (future)
    profile: HarnessProfile,     // harness-specific config
    input: Input,                // role-specific
    context: Context,            // role-specific
    permissions: Permissions,    // read/write scope
}

struct AgentRunHandle {
    operation_id: OperationId,
    events: Stream<Item = AgentEvent>,
    wait:   Future<Output = Result<AgentOutput>>,
    cancel: Future<Output = Result<()>>,
}
```

## Translation boundary (harness, not process/)

`agent/` owns `AgentInvocation` (it is the agent seam contract).
The harness — `ClaudeCodeHarness`, `CodexHarness`, etc. — translates
`AgentInvocation` into a `ProcessSpec` and calls `process::spawn`.

```text
agent::start(AgentInvocation)
   ↓
selected AgentHarness
   ↓ translate AgentInvocation → ProcessSpec
process::spawn(ProcessSpec) → ProcessHandle
```

`process/` does NOT know about `AgentInvocation`. It only knows about
`ProcessSpec`. See [[process-owns-child-process-primitive-with-processspec]].

## Why this shape

We know from day one that an agent invocation:

- streams output
- can be cancelled
- survives client disconnect
- emits events
- may live minutes or hours

A unary `invoke() -> Output` would force us to redesign the seam the
moment we add streaming or cancellation. So the seam is a
long-running process with an event stream, not a function call.

## Who goes through agent::start

ALL AI subprocess work goes through `agent::start`. This is
non-negotiable:

- `planner/` → `agent::start(role: Planner, ...)`
- `execution/` → `agent::start(role: Executor, ...)`
- `verification/` does NOT use `agent/`; deterministic checks use
  `process::spawn` directly with their own `ProcessSpec` (different
  concern).
- `review/` (future) → `agent::start(role: Reviewer, ...)` when it lands.

Planner and Executor NEVER call `process::` directly. They go through
`agent::start`, which selects the harness, which builds the
`ProcessSpec`, which `process::` turns into reality.

`process/` does NOT know about Planner / Executor / Claude / Codex /
Reviewer. It is a low-level primitive.

## Layering

```text
planner/   ──▶ agent::start ──▶ selected AgentHarness ──▶ process::
execution/ ──▶ agent::start ──▶ selected AgentHarness ──▶ process::
review/    ──▶ agent::start ──▶ selected AgentHarness ──▶ process::  (future)

verification/ ──▶ builds ProcessSpec ──▶ process::spawn ──▶ process::
```

`Operation` = durable lifecycle in shadows.
`AgentRunHandle` = runtime handle on the live process.

They are not the same thing. Operation outlives any client connection
and any process restart. AgentRunHandle is per-process.

## What stays enforced

- All AI subprocess work goes through `AgentHarness`.
- Role is explicit in the invocation.
- Harness / Provider / Model are configuration values, not code paths.
- `process::` is the only owner of `tokio::process`.
- `AgentInvocation` only exists inside `agent/`.

Related: [[process-owns-child-process-primitive-with-processspec]], [[operation-lifecycle-with-two-phase-spawn]]
