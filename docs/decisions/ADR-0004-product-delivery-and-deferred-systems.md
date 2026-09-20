# ADR-0004 — Product delivery and deferred systems

- **Status:** accepted
- **Date:** 2026-09-20
- **Detail:** canonical design specification, Sections 1.0, 6.2, 9, and 10

## First product milestone

The first milestone is a runnable browser Planner, not a persistence or
architecture milestone:

1. Start `shadows serve`.
2. Manually open its local address in any browser; Shadows never opens one.
3. Select a local-directory project and create or resume a planning thread.
4. Start one real Claude Planner turn and stream live output.
5. Stop it and confirm the full managed process tree terminates before the
   durable operation becomes `Cancelled`.
6. Restart Shadows and recover the conversation and terminal operation.

## External tools

`gcode` is an optional external executable. Shadows may call it through its
normal tool/process boundary, but does not vendor or depend on `gobby-cli`,
`gcore`, PostgreSQL, FalkorDB, or Qdrant. `ghook` and `gwiki` are outside the
first milestone.

## Deferred until after the milestone

- full Workflow DAG, scheduler, execution, and deterministic verification;
- AI Reviewer;
- MCP-attached agents;
- ResearchArtifact/search subsystem;
- PostgreSQL adapter and team synchronization;
- full schema, property/fault/mutation suites, and other gates belonging to
  features that do not yet exist.

When workflow and verification arrive, frozen workflow versions remain
immutable, scheduling remains a pure decision step, and deterministic verifier
results remain separate from AI reviewer judgement.

