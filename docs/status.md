# Project Status

**Updated:** 2026-09-21

## Current truth

- `shadows` is a clean rewrite in architecture/documentation phase; no product
  code exists yet.
- One canonical specification remains:
  `docs/superpowers/specs/shadows_design_spec_reviewed.md`.
- Nineteen overlapping ADRs were consolidated into four short decision maps.
- SQLx 0.9 + SQLite remains the local persistence choice.
- The old `shadow` repository is reference material, not an implementation base.
- `TaskState::Running` was replaced by `TaskState::InProgress`. Task progress
  and Operation runtime status are separate axes; `OperationStatus::Running`
  is unchanged.
- The runtime/execution draft is tracked but is **not** baseline. Seven design
  decisions in it are open. None block the first milestone.
- The Claude harness stream contract is now measured, not assumed: four stream
  classes, of which only `assistant`, `user` and `result` are durable. Serving
  the web client needs no framework. `claude.exe` spawns a process tree, which
  confirms the Job Object requirement by observation.

## Next deliverable

Build the first runnable browser Planner vertical slice:

```text
shadows serve
  -> manually open any browser
  -> select a local project
  -> create/resume a PlanningThread
  -> start one real Claude turn
  -> stream output
  -> Stop and confirm process-tree termination
  -> restart and recover durable state
```

The daemon must not open a browser automatically.

## Scope for this milestone

- one Rust crate and daemon/CLI binary;
- independent Web client;
- SQLx + minimal SQLite schema;
- Project, PlanningThread, ThreadEntry, Operation, command idempotency, and
  durable event/recovery data;
- one Claude harness;
- HTTP commands + SSE streaming;
- structured diagnostic logs;
- real Windows debug acceptance, with Linux evidence reported separately.

## External tools

- `gcode` is used as an optional external binary through the normal tool/process
  boundary.
- Do not vendor or depend on `gobby-cli`, `gcore`, PostgreSQL, FalkorDB, or
  Qdrant.
- `ghook` and `gwiki` are not part of the first milestone.

## Explicitly deferred

- full Workflow DAG and scheduler;
- execution and deterministic verification;
- AI Reviewer;
- MCP-attached agents;
- ResearchArtifact/search;
- PostgreSQL adapter, team server, and synchronization;
- schema and test layers belonging only to those deferred features.

## Immediate documentation state

- [x] canonical design baseline retained;
- [x] obsolete earlier spec removed from the working tree;
- [x] ADR set compressed to four topic-based maps;
- [x] first runnable milestone and external `gcode` boundary recorded;
- [x] `TaskState` vocabulary contradiction resolved; runtime draft tracked and
      labelled non-baseline;
- [x] spiked the two unknowns Milestone 0 had no design for: serving the web
      client, and the concrete Claude harness invocation/stream contract
      (`docs/evidence/harness/SERVE_STREAM_SPIKE.md`);
- [ ] validate file-backed SQLite WAL under concurrent writers, then fix the
      writer strategy;
- [ ] write the focused Milestone 0 implementation plan;
- [ ] create `docs/codebase/roadmap/` once that plan exists;
- [ ] implement only after that plan is reviewed;
- [ ] settle the runtime draft's seven open decisions before workflow or
      scheduler work begins.

Persistence comparison evidence remains archived under
`docs/evidence/persistence/` and is not a prerequisite for seeing the first
product path run.
