# ADR-0003 — Agents, processes, and operations

- **Status:** accepted
- **Date:** 2026-09-20
- **Detail:** canonical design specification, Sections 1.4, 1.5, 2.2, 2.3, and 2.7

## Decisions

- All managed AI work starts through `AgentHarness`; the harness translates an
  invocation into a transport-neutral `ProcessSpec`.
- Only `process/` may create or terminate OS processes. It owns complete process
  tree containment on Windows and Linux.
- Every long-running Planner or later execution/review action has a durable
  `Operation`; runtime handles remain transient.
- Spawn is two phase: persist `Pending`, spawn, then atomically transition to
  `Running` or terminal failure.
- A cancellation request is durable but is not terminal cancellation.
  `Cancelled` is written only after the managed process tree is confirmed gone.
- Restart reconciliation records abandoned `Pending`/`Running` work as
  `Interrupted`; it does not invent success or failure and does not auto-retry.
- Config stores secret references only. Values are resolved at spawn, injected
  through an explicit child environment, and never persisted or logged.
- Structured tracing uses stable project/thread/operation IDs and redacts
  prompts, outputs, credentials, and provider payloads by default.

