# MCP-attached agents are first-class

**Doc ID:** 999
**Status:** active
**Tags:** architecture, external-agents, integration, mcp
**Source slug:** mcp-attached-agents-are-first-class

---

# Decision — MCP-attached agents with Shadows-validated authority

**Status:** active · 2026-09-20

## Two distinct integrations

```text
A) Managed Invocation
   Shadows ──spawns──▶ Claude Code / Codex
   process owned by Shadows

B) MCP Attached Agent
   User starts Claude / Codex
        │
        │ MCP
        ▼
   Shadows
   process NOT owned by Shadows
```

In (B), Shadows owns no process, no model, no credentials. It still
owns Project Truth, PlanningThread, Decisions, Research, Workflow,
Operations.

`AgentHarness` (managed) and MCP (external) are different things.
Both work over the same core truth.

## Authority rule

```text
agent / model cannot authorize itself.
```

The MCP client requests a binding with a `role` and `capabilities`.
Shadows validates the configured identity / profile / policy and
returns a `GrantedMcpContext`:

```text
MCP client requested binding
          ↓
Shadows validates configured identity / profile / policy
          ↓
GrantedMcpContext {
    project_id,
    thread_id?,
    granted_role,
    granted_capabilities,
    opaque_scoped_cursor,
}
```

The model sees only what Shadows grants. It cannot elevate.

## event_cursor is opaque scoped

The cursor handed to the MCP client is opaque and scoped to the
granted context. The client cannot use it to read another project's
journal or to infer a global journal structure.

## mcp/ adapter module

```text
mcp/
├── server.rs
├── tools.rs
├── resources.rs
└── types.rs
```

Forbidden:

```text
mcp → rusqlite directly                → fail (architecture test)
mcp → mutate workflow storage directly → forbidden
mcp → spawn agents                     → forbidden
```

The invariants stay in the owning modules.

## MCP surface is domain operations, not CRUD

The MCP server exposes:

- `project_context_get`
- `thread_context_get`
- `workflow_get` / `workflow_propose`
- `task_get`
- `decision_propose` / `decision_get`
- `research_record` / `research_get`
- `events_since(opaque_cursor)`

NO `set_workflow_json(...)`, NO `write_decision_row(...)`. `propose`
operations are proposal-shaped, not raw writes; they pass through the
same validation the managed Planner / CLI / future Web use.

## Continuity does not depend on MCP notifications

```text
snapshot + revision + event_cursor + events_since(cursor)
```

If the host supports MCP notifications / resource subscriptions we
use them. Continuity does not depend on them.

## Claude → Codex continuity via MCP

```text
Yesterday:
   Claude managed invocation
        ↓
   PlanningThread #42
        ↓
   Decisions + Research + Workflow persisted

Today:
   User opens Codex CLI himself
        ↓
   Codex connects to Shadows MCP
        ↓
   thread_context_get(thread=42)
        ↓
   gets current Shadows-owned context
        ↓
   continues planning
```

Codex never needs Claude's native session ID.

Related: [[shadows-owns-truth-not-the-cli]], [[events-journal-bus-subscribe-after]]
