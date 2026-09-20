# Shadows Team — Future Architecture Direction

- **Status:** Future direction; not a v1 commitment or implementation plan
- **Date:** 2026-09-20
- **Local product:** `shadows`
- **Future team product:** working name `shadows-team`

## Intent

Keep `shadows` a standalone local-first engine while allowing a future,
separate team service for organizations, shared engineering truth, and
multi-person coordination.

Personal use must never require a server:

```text
Personal Workspace
  shadows local engine
  + SQLite authoritative truth
  + local agents/processes/worktrees/secrets
```

Future team use adds a server without replacing the local engine:

```text
Team Workspace
  shadows local engine
  + local SQLite runtime/cache
  + Shadows Team protocol
  + shadows-team shared authority
  + PostgreSQL
```

## Product boundary

### `shadows` remains local

Local authoritative responsibilities include:

- Git clone, branches, and worktrees;
- Claude/Codex and other agent processes;
- `ProcessHandle`, PID/process-tree containment, and runtime cancellation;
- provider credentials, API keys, and secret resolution;
- native harness session IDs;
- local paths, configuration, caches, sandboxes, and heavy logs;
- stdout/stderr and transient agent events;
- local-only notes/drafts and personal workspace truth;
- local `Operation` and `AgentInvocation` runtime details.

Secrets are never uploaded to the team service. Each user runs agents with
their own local credentials.

### `shadows-team` becomes shared coordination authority

Server-authoritative responsibilities may include:

- organizations, memberships, roles, and permissions;
- shared workspaces and project metadata;
- shared vision, roadmap, planning threads, decisions, and research;
- workflow versions, task DAGs, verification definitions, and assignments;
- task claims/leases and coordination;
- shared project events, audit/provenance, and revision history;
- shared run summaries and artifact/Git references.

The team service does not become a Git hosting platform and does not own local
process execution.

## Data ownership classes

Every future Team entity/field must be assigned to exactly one ownership
class:

| Class | Meaning | Examples |
|---|---|---|
| Local authoritative | Exists only on a person's device | secrets, process tree, worktree paths |
| Server authoritative | Shared truth committed by Team Server | shared decisions, workflows, assignments |
| Server projection cached locally | Read model/cache, never equal co-authority | roadmap, shared threads, task state |
| External reference | Identity/reference owned by another system | Git remote, commit SHA, PR URL, artifact URL |

The local SQLite database and PostgreSQL must never be co-authoritative for the
same shared entity.

## Protocol boundary, not database replication

Do not build SQLite/PostgreSQL row replication:

```text
FORBIDDEN:
local SQLite <-> PostgreSQL table/row synchronization
```

Team mutations use versioned domain operations:

```text
decision.propose(...)
decision.accept(...)
workflow.create(...)
thread.append_entry(...)
task.claim(...)
run.report_transition(...)
```

The server validates identity, authorization, idempotency, expected revision,
and current state before committing to PostgreSQL and publishing a shared
event. Local caches consume server-owned snapshots/events through an opaque
cursor.

## Concurrency and coordination

Shared mutations require both idempotency and optimistic concurrency:

```text
command_id + request_fingerprint
expected_revision
```

Task execution uses an atomic server-issued lease with fencing:

```rust
struct TaskLease {
    lease_id: LeaseId,
    task_id: TaskId,
    owner_user_id: UserId,
    owner_runtime_id: RuntimeId,
    fencing_token: FencingToken,
    expires_at: Timestamp,
}
```

Every lease-bound status/result update includes the fencing token. The server
rejects updates from an expired or superseded lease, preventing an old runtime
from publishing after reassignment.

## Shared execution vs local runtime

Do not make every local `Operation` detail shared.

```text
Server authority:
  TaskAssignment / TaskLease / SharedRunSummary

Local authority:
  Operation / AgentInvocation / ProcessHandle
```

They correlate through stable IDs such as:

```text
shared_run_id
local_operation_id
runtime_id
correlation_id
```

The server receives status, result summary, evidence/artifact references, and
Git commit/PR references. It does not receive secrets, PIDs, local paths,
native-session credentials, or full stdout/stderr by default.

## Local cache and offline behavior

For a Team Workspace, local SQLite may contain:

- server projections/read cache;
- the last accepted server cursor/revision;
- local runtime state;
- personal drafts that are not shared truth;
- an explicit mutation outbox.

An offline proposal has an explicit state:

```text
LocalDraft -> PendingSync -> Accepted | Rejected | Conflict
```

It must not appear as committed shared truth before server acknowledgement.

Initial Team mode should be online-only for shared writes. Server-unavailable
behavior may permit cached reads and local drafts, while shared mutations are
disabled or explicitly queued. General offline conflict resolution, CRDTs, and
multi-master writes are deferred until a demonstrated use case justifies them.

## Repository and shared-code direction

`shadows-team` will likely become a separate repository/product when its
requirements are ready. Do not create it during local v1 work.

Do not manually duplicate the local domain model into the team repository.
Extract only proven shared protocol material when real Team use cases exist,
potentially as one small versioned package containing:

- wire IDs and DTOs;
- protocol envelopes and capability negotiation;
- shared enums only where wire compatibility requires them;
- generated schemas/clients where useful.

Do not pre-create a family of domain/ports/adapters crates.

## Compatibility choices to preserve now

Current local v1 should only preserve these low-cost properties:

1. globally unique typed IDs;
2. actor, causation, correlation, and idempotency metadata;
3. storage-neutral application/domain operations;
4. no SQLite types or semantics in public domain contracts;
5. versionable command/event/protocol shapes;
6. local runtime details remain separable from shareable summaries.

Do not add `TeamWorkspace`, `ServerId`, sync engines, CRDTs, PostgreSQL
production adapters, leases, or fencing tokens to local v1 merely for this
future direction.

## Explicit non-goals for local v1

- Team Server implementation;
- organization/auth/billing systems;
- PostgreSQL production adapter;
- SQLite/PostgreSQL replication;
- offline collaborative mutation/conflict resolution;
- active-active multi-server coordination;
- Git hosting;
- cloud execution of local agent processes;
- premature shared-domain crate extraction.

## Trigger for a future Team specification

Create a separate Team design/spec only when concrete collaboration use cases,
security/tenant requirements, deployment ownership, and protocol consumers are
known. That future spec must define entity ownership, authorization, revision
conflicts, lease/fencing behavior, event/cursor semantics, cache invalidation,
privacy, and compatibility/versioning before implementation planning.
