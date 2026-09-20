# Decisions live at the project level

**Doc ID:** 983
**Status:** accepted
**Tags:** architecture, decisions, foundation, project-level
**Source slug:** decisions-live-at-the-project-level

---

# Decision — Decisions live at the project level

**Status:** accepted · 2026-09-20

## Rule

Decisions can be workflow-scoped, project-architectural, or
cross-workflow. They are not owned by any single workflow.

A workflow references decision IDs, but does not own them.

## Why

Examples that outlive any one workflow:

```text
D-001 PostgreSQL
D-002 opaque refresh tokens
D-003 module X must not depend on Y
```

These persist across workflows and replanning. Folding them into
`workflow::decisions` makes the workflow their owner, which it is not.

## Implementation in v1

```text
project/
  decisions.rs    ← Decision CRUD, query by project_id
```

A workflow references decisions via IDs. Resolving a decision is a
project-level operation, not a workflow operation.

Related: [[shadows-owns-truth-not-the-cli]]
