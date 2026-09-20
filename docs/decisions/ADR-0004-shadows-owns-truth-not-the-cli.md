# Shadows owns truth, not the CLI

**Doc ID:** 980
**Status:** active
**Tags:** architecture, foundation, ownership
**Source slug:** adr-0004-shadows-owns-truth-not-the-cli

---

# Decision — Shadows owns truth, not the CLI

**Status:** active · 2026-09-20

## Rule

Claude Code, Codex, and any other CLI are workers / harnesses. They
are not the source of project truth.

We do not bind project state to a native session belonging to any CLI.

## What shadows owns

- Project state
- PlanningThread (the planning/conversation thread)
- Decisions (project-level)
- Research artifacts
- Workflow / DAG
- Task contracts
- Operations
- Runs
- Evidence
- Verification results
- Events journal
- Continuity

## State model: not event-sourced

```text
authoritative current state → entity / state tables
journal                    → history + provenance + replay / resync
```

The journal is NOT the source of current state. The current state
lives in the entity tables (projects, threads, workflows, decisions,
research, operations, etc).

The journal exists for:

- history (who did what when)
- provenance (this plan came from this AgentInvocation)
- replay (rebuild a snapshot)
- resync (a client that reconnects after disconnect)

Codex does NOT "load latest Plan from journal". Codex loads current
Plan from the planner / thread state table. The journal supplies
provenance and recent changes for the snapshot / cursor.

## Continuity is a cross-module invariant

```text
Continuity =
  Project Truth
  + Decisions
  + Workflow
  + Planning Threads
  + Research
  + Operations / Events
```

It is not a single module's responsibility. Each owning module keeps
its slice persistent. Session-style concepts only read from them.

```text
Claude session dies
Codex starts a new session
→ project continuity must still exist
```

## Vocabulary (no ambiguous "Session")

```text
PlanningThread         ← shadows-owned, durable, references Decisions + Tasks
AgentInvocation        ← shadows-owned, records a single agent call
NativeHarnessSession   ← harness-owned, disposable, opaque to shadows
ClientConnection       ← transport-owned, disposable, per-request or per-stream
```

The internal module is `thread/` (with public type `PlanningThread`).
The protocol surface may later expose `/session` for user familiarity;
protocol vocabulary and internal domain vocabulary do not need to
match literally.

Claude → Codex continuity is structural:

```text
PlanningThread #42
   │
   ├── AgentInvocation #91
   │      native_session_id = "claude-abc-123"
   │
   └── AgentInvocation #92
          native_session_id = "codex-xyz-789"
```

Codex never needs Claude's native session ID.

## What CLIs cannot do

- Cannot own project state.
- Cannot be the source of truth for plans, decisions, workflows,
  tasks, runs, evidence.
- Cannot dictate how a workflow resumes.
- Cannot decide the format of task contracts or verification
  results.

## What CLIs do

- Receive a frozen Task Contract.
- Execute the implementation intent inside the allowed scope.
- Return structured evidence (diff, test results, decisions used).
- Stop. They do not decide what comes next.

Related: [[continuity-was-archived]], [[vocabulary-was-archived]], [[operation-is-a-cross-cutting-core-concept]]
