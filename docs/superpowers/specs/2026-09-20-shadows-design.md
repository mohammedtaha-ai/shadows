# Shadows — Design Spec

- **Date:** 2026-09-20
- **Status:** **OBSOLETE** — replaced by [`shadows_design_spec_reviewed.md`](./shadows_design_spec_reviewed.md). Do not use for implementation planning.
- **Slug:** `shadows`
- **Stack:** Rust 1.87+, single crate, library + binary
- **Purpose:** Local-first AI orchestration layer (planning + workflow + context + execution + verification + continuity). Clean rewrite of `shadow` avoiding the patching pattern.

---

> This document is retained only as the pre-review design record. The reviewed
> specification is the canonical candidate and resolves known lifecycle,
> idempotency, persistence, schema, and scope issues in this version.

## Section 1 — Architecture Overview

Single Rust crate `shadows`. Library + binary (two modes: `serve` daemon + CLI client).

### 13 top-level modules

```text
project/         thread/         command/        runtime/
agent/           planner/        workflow/       operation/
scheduler/       execution/      verification/   events/
storage/         protocol/       cli/
```

### Cross-cutting modules

`config`, `secrets`, `error`, `tracing`, `process`, `mcp`

### Single-ownership rules (5)

| Module | Sole owner of |
|---|---|
| `agent/` | AI subprocess harness (`AgentHarness::start`) |
| `storage/` | SQLite (and future PostgreSQL adapter) |
| `protocol/` | HTTP/SSE transport |
| `process/` | `tokio::process` / `process-wrap` (private to `process/` only) |
| `secrets/` | `SecretResolver` value resolution (config holds refs only) |

### Persistence spike outcome

**Winner: SQLx 0.9** (over SeaORM). SeaORM's Postgres path was stubbed in the spike; SQLx passed 7/7 Postgres scenarios end-to-end via Docker. See `sandbox/SPIKE_REPORT.md` for full evidence.

### ProcessSpec boundary

- `agent/` knows `AgentInvocation`
- `process/` knows ONLY `ProcessSpec` + `ProcessHandle`
- Harness translates `AgentInvocation` → `ProcessSpec` → `process::spawn`

This boundary keeps the harness swap-clean: changing the agent subprocess layer does not require touching `process/`, and changing the OS process layer does not require touching `agent/`.

---

## Section 2 — Data Flow (12 flows)

### 1. Create/Resume PlanningThread

User issues a planning command. `cli/` → `protocol/` → `command/`. `command/` calls `storage::create_planning_thread` (external mutation → `CommandContext` → `CommandRecord` → idempotent). Thread is opened with initial state (open, root scope). ThreadEntry appended with the user's prompt. Durable event recorded. Snapshot returned to client.

Resume: client sends `event_cursor`. `storage::load_thread_snapshot` reads thread + entries + cursor in a single TX. Gap-free resync via cursor.

### 2. Planner invocation

Planner is invoked by user command or by workflow dispatch. `planner/` resolves current thread + active workflow + recent entries, produces a Plan (declarative). Plan may propose workflow operations (propose/skip). All planner writes go through `storage::append_thread_entry(WriteOrigin::Internal { actor, caused_by })`.

### 3. Agent streaming + cancellation (semantic, not SIGTERM)

Agent subprocess streams via SSE through `protocol/`. Cancellation is a **semantic** operation: client sends cancel command → `command/` → `storage::transition_operation(target=Cancelled)` → durable event. The `process/` layer observes cancellation via a watch channel and shuts down cleanly. No SIGTERM-as-truth.

### 4. Durable vs transient events (journal + bus)

- **Durable events:** committed to `durable_event` table. Single source of truth. Per-scope projection (project/thread/workflow/operation).
- **Transient events:** in-process tokio broadcast bus. For UI animation, progress hints, log lines. Never persisted.

Durable events drive resync; transient events drive live UI. Two separate channels on the SSE stream.

### 5. Workflow creation/update (Frozen immutable, supersede replaces)

Workflow states: `Draft → Ready → Frozen → Superseded`.

- `Draft` → `Ready` → `Frozen`: linear progression via `transition_workflow`.
- `Frozen`: immutable. No edits allowed.
- Update after Frozen: create new workflow with `previous_version_id = old.id`. Old transitions to `Superseded`. Lineage is linear (`UNIQUE (previous_version_id)`).

This keeps history honest and rollback trivial.

### 6. Scheduler → ExecutionIntent[]

`scheduler/` is a **pure decision function**: `(thread_state, workflow_state, task_states, runtime_registry) → ExecutionIntent[]`. No I/O. The runtime calls the scheduler, gets intents, and translates them to `create_internal_operation` calls.

`scheduler/` carries zero SQLx/sqlx/sql/Row imports. Its output is fully testable as a function.

### 7. ExecutionRun lifecycle (two-phase spawn: Pending → Running/Failed{stage:Spawn})

```text
Pending ──spawn_ok──▶ Running ──complete──▶ Completed{outcome}
   │                     │
   │                     ├──fail──▶ Failed{stage: Run}
   │                     │
   │                     ├──cancel──▶ Cancelled
   │                     │
   │                     └──interrupt──▶ Interrupted
   │
   └──spawn_fail──▶ Failed{stage: Spawn}
```

Two-phase spawn protects durability: `Pending` row exists before subprocess fork; spawn failure is recoverable (the row is real, the event is real, the orphan is detectable).

### 8. Verification flow (deterministic only; Reviewer separate, no ReviewCheck kind in v1)

`VerificationCheck` has a `CheckPhase` (Pre/Post) and binds to exactly one of `task_id` or `gate_id` (CHECK constraint). Verification runs execute deterministic checks only in v1. Reviewer is a separate agent role (ADR-0031) — there is **no `ReviewCheck` kind** in v1.

### 9. Persistence boundaries (mechanical only: imports outside storage/)

The architecture test mechanically enforces: files outside `storage/<backend>/` MUST NOT import `sqlx`, `sea_orm`, `sea_query`, `Row`, `Entity`, `ActiveModel`, `Pg*`, `Sqlite*`, `PRAGMA`, `strftime`, `rowid`, FTS5 SQL, `tsvector`. This is the **only** persistence-boundary check. Domain types stay pure.

### 10. Client disconnect/reconnect (snapshot + opaque cursor, gap-free resync)

On reconnect, client sends `event_cursor` (opaque, scope-tagged). Server returns:
1. `load_thread_snapshot` (single read TX, includes `event_cursor`)
2. `read_events_after(cursor)` (gap-free resync — server emits any events the client missed, durably committed)

Client reconstitutes state, then subscribes to live SSE.

### 11. Claude → Codex continuity (same PlanningThread, opaque native_session_id)

Continuity does NOT depend on the AI provider's native session. Reconstruction is from durable truth (thread entries, decisions, research, workflow lineage). `AgentInvocation.native_session_id` is opaque — when present it speeds up warm-start; when absent, the agent harness cold-starts from durable state.

### 12. External Agent via MCP (domain operations only, authority validated by Shadows)

External agents attached via MCP cannot write to `shadows` storage directly. They invoke Shadows **domain operations** through the MCP server (`mcp/`). Each operation goes through `command/` → `CommandRecord` → idempotency check → `storage` mutation. Authority is validated by Shadows, not by the external agent.

---

## Section 3 — Error Handling + Dependencies + Testing

### 3.1 AppFailure (transport-neutral, public-safe)

```rust
struct AppFailure {
    code: ErrorCode,              // stable registry, no string literals
    class: FailureClass,          // Domain | Infrastructure | AgentProcess | PolicyRefusal
    retry: RetryClass,            // Never | Immediate | Backoff | AfterReconfiguration | AfterUserAction
    public_details: PublicDetails, // typed enum, NO serde_json::Value
    // source separated into FailureReport (internal only)
}
```

Separation principle: `public_details` is what the client sees (typed, stable, safe). The full causal chain goes into `FailureReport` (internal-only, never serialized over the wire). This prevents accidental secret/internal-path leaks while preserving debuggability.

### 3.2 PLAN_BLOCKED = Operation outcome, not HTTP error

```rust
enum OperationOutcome {
    Success { result_ref },
    Blocked { reason, missing_capability, alternatives },  // structured refusal
    Rejected { reason },
}
```

When an operation completes with `Blocked`, the HTTP response is **200 OK** with the operation's outcome. PLAN_BLOCKED is a first-class domain outcome — never an HTTP 4xx/5xx.

### 3.3 Idempotency atomic boundary

```sql
INSERT command_record (..., outcome_ref=op_id, ...) ON CONFLICT DO NOTHING
-- if won: INSERT operation + event + UPDATE command_record
-- if lost: read existing outcome_ref, return Replayed
```

There is no `Claimed`/`Finalized` state. All three writes happen in **ONE TX**. `outcome_ref` is generated **BEFORE** the TX (so the command_record row can carry it). The storage layer returns `WriteOutcome::Applied | Replayed`.

### 3.4 ErrorCode registry (single source)

```text
WorkflowInvalidTransition
WorkflowFrozenImmutable
WorkflowValidationFailed

AgentAuthFailed
AgentUnsupportedProfile
AgentRateLimited

ProcessSpawnFailed
ProcessTerminated

StorageUnavailable
StorageMigrationFailed

CommandConflict
CommandAlreadyProcessed
IdempotencyKeyRequired

OperationBlocked       (outcome kind)
OperationRejected      (outcome kind)
PlanBlocked            (outcome kind)

InvalidCommand
```

These are stable identifiers — clients pattern-match on them. No string literals scattered through the codebase.

### 3.5 Dependency Strategy (Section 3.2.1)

| Need | Library | Decision |
|---|---|---|
| Typed errors | `thiserror` | USE |
| Startup boundary | `anyhow` | USE (binary/bootstrap only) |
| Async runtime | `tokio` | USE |
| HTTP | `axum` | USE |
| Middleware | `tower`, `tower-http` | USE |
| Persistence | `SQLx 0.9` | USE (spike decision) |
| Migrations | `SQLx`'s official `migrate!` | USE |
| Processes | `tokio::process` + `process-wrap` | USE |
| MCP | `rmcp` | USE |
| Serialization | `serde`, `serde_json` | USE |
| Tracing | `tracing`, `tracing-subscriber` | USE |
| CLI | `clap` | USE |
| Secrets | `secrecy`, `zeroize` | EVALUATE |
| CLI diagnostics | `miette` | EVALUATE |
| Retry | `tokio` primitives / `policy` | EVALUATE |

`EVALUATE` items have a concrete trigger (a real need) before adoption. No speculative dependencies.

### 3.6 Testing Strategy (Section 3.3)

**10 layers:**

1. **Unit** — module-local logic, no I/O
2. **Integration** — multi-module within a crate boundary
3. **Property** — `proptest` for invariant-bearing code (state machines, parsers, selectors)
4. **Mutation** — `cargo-mutants` on nightly, critical paths only
5. **Spec conformance** — specs become executable assertions via the architecture test harness
6. **Architecture** — mechanical rules only (imports outside their owner)
7. **Crash recovery** — kill -9 mid-write, restart, assert state
8. **Real CLI** — `#[ignore]`d tests against `claude`, `codex` binaries
9. **Concurrency** — loom/tokio-test for races, cancel-safety, channel buffering
10. **Cross-platform** — Windows + Linux for path/env/process behavior

**CI shape:**

- Linux: every push (fast feedback)
- Windows: nightly (path/env-heavy edge cases)
- Mutation: nightly on critical paths
- Real CLI: opt-in only (manual trigger or label-gated)

**Architecture test** is mechanical and import-based. Domain types must compile with zero `sqlx`/`sea_*` imports. Storage layer is the only place where these appear.

**Mutation testing** must prove that critical-path tests catch logic changes. If `cargo-mutants` finds an uncaught mutation in `storage/`, `scheduler/`, or `command/`, that is a test-gap to fix before merge.

---

## Section 4 — Core Domain Model + Invariants + Persistence

### 4.1 Core Domain Model (Section 4.1)

**Entities + IDs (all UUID v4):**

- `Project`
- `PlanningThread`
- `Decision`
- `ResearchArtifact`
- `Workflow` (with `previous_version_id` for lineage, NOT `superseded_by`)
- `Task` (state persisted — derivation too complex to recompute)
- `Gate`
- `TaskContract`
- `Scope`
- `VerificationCheck`
- `Operation` (no `agent_invocation_id` — relationship on child side)
- `AgentInvocation`
- `VerificationRun`
- `Verdict`
- `DurableEvent` (with multi-scope projection: project/thread/workflow/operation IDs nullable)
- `CommandRecord` (idempotency primitive)
- `RuntimeInstance`

**Operation (FINAL):**

```rust
struct Operation {
    id: OperationId,
    kind: OperationKind,
    status_kind: OperationStatusKind,
    status_payload_json: Option<String>,  // outcome_json for Completed
    thread_id: Option<ThreadId>,
    workflow_id: Option<WorkflowId>,
    task_id: Option<TaskId>,
    runtime_instance_id: RuntimeInstanceId,  // NOT agent_invocation_id
    created_at: Timestamp,
    started_at: Option<Timestamp>,    // None until Running
    finished_at: Option<Timestamp>,
}

enum OperationStatusKind {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

enum OperationOutcome {
    Success,
    Blocked,
    Rejected,
} // payload on Completed{outcome}

enum FailureStage {
    Spawn,
    Invocation,
    Run,
    Completion,
}
```

The relationship moves to `AgentInvocation.operation_id` (child side) — operations never carry a direct agent reference. This keeps `Operation` clean of agent-layer concerns.

### 4.2 Relationships + Invariants (Section 4.2 final)

1. **Workflow state machine** — `Draft → Ready → Frozen → Superseded`. Frozen is immutable. No unfreeze. Updates create a new workflow with `previous_version_id`.

2. **Operation lifecycle** — `Pending → Running → {Completed, Failed, Cancelled, Interrupted}`. `Pending → Failed{stage: Spawn}` is the only legal direct failure transition before `Running`.

3. **Decision scope** — project-level. Decisions are NOT workflow-scoped (ADR-0007). A workflow may reference decisions via IDs in its body, but the decision itself lives at the project.

4. **Continuity** — reconstructed from durable truth (thread entries, decisions, research, workflow lineage, operations, events). No native session required.

5. **Atomicity** — state mutation + durable event + (CommandRecord if external) in ONE TX. No partial commits.

6. **Supersede** — `previous_version_id` only. The new workflow points back; the old does not point forward.

7. **DeclaredScope ⊇ EffectiveScope** — runtime cannot exceed declared. Enforced at the storage layer (CHECK + scheduler).

8. **Idempotency** — same `(principal, scope, command_id)` → same `outcome_ref`. Same identity, not frozen-history semantics.

9. **OperationOutcome.Blocked** — structured refusal from planner/agent/execution. Carries `missing_capability` and `alternatives`.

10. **RuntimeInstance linkage** — `Operation` records the runtime that created it. Recovery reconciles orphans via exact CAS: `WHERE id=? AND status=? AND runtime_instance_id=?`.

11. **Persistence Ordering** — explicit domain keys (`durable_seq`, `ordinal`). No `rowid`, no physical insertion order, no implicit `SELECT` order.

### 4.3 Persistence Model / API (Section 4.3 Final)

**Concrete `Storage { pool: SqlitePool }`.** No runtime dependency (no `tokio` import in the type). Returns `storage::Error`.

**External vs Internal mutations:**

```text
External mutation → CommandContext → CommandRecord → idempotent
Internal mutation → no CommandContext → still atomic state + event → CAS protection
```

Both paths enforce "state + event" atomicity. External additionally requires a `CommandRecord` for idempotency.

**`WriteOrigin` enum** for thread entries, research, decisions (single method, internal/external branching):

```rust
enum WriteOrigin {
    External(CommandContext),
    Internal { actor: ActorRef, caused_by: Option<OperationId> },
}

enum WriteOutcome<T> {
    Applied(T),
    Replayed(T),
}
```

**`CausationRef` enum:**

```rust
enum CausationRef {
    Operation(OperationId),
    Command(CommandRef),
    Workflow(WorkflowId),
    Runtime(RuntimeInstanceId),
}
```

**Idempotency (no Claimed state):**

```sql
PRIMARY KEY (principal_kind, principal_id, command_scope_kind, command_scope_key, command_id)
-- Global scope: command_scope_key = '' (canonical, non-null for SQLite uniqueness)
-- outcome_ref nullable (outcome_kind='NoContent' → NULL)
-- caller generates resource_id BEFORE TX
```

**Storage API (full list):**

- **Projects:** `create_project`, `get_project`
- **PlanningThread:** `create_planning_thread`, `close_planning_thread`, `load`, `list_threads_for_project`
- **ThreadEntries:** `append_thread_entry` (WriteOrigin), `list_thread_entries` (paginated)
- **Snapshot:** `load_thread_snapshot` (single read TX, includes event_cursor)
- **Workflow:** `create_workflow_draft`, `supersede_workflow`, `transition_workflow`, `begin_workflow_dispatch` (no runtime_id), `complete_workflow_dispatch`, `fail_workflow_dispatch`, `get_workflow`, `list_workflows_for_thread`
- **Decision:** `propose_decision`, `transition_decision` (both WriteOrigin)
- **Research:** `record_research`, `search_research`, `list_research_for_project`
- **Operations:** `create_operation_from_command` (external), `create_internal_operation` (internal, stable ID, CausationRef), `mark_operation_started/completed/failed/cancelled/interrupted`, `transition_operation`, `get_operation`, `list_operations_for_thread` (with OperationStatusKind filter), `find_orphaned_operations` (Pending + Running, by exact status + runtime CAS)
- **AgentInvocation:** `create_agent_invocation`, `attach_native_session_id`, `list_invocations_for_operation`
- **Verification:** `record_verification_run`, `record_verdict`, `get_verdict`
- **Runtime:** `register_runtime_instance`, `get_runtime_instance`
- **Events:** `read_events_after`, `current_cursor` (read-only public; append is private)

**`Storage::Error` (typed sources, no string buckets):**

```rust
enum Error {
    TransitionConflict {
        entity: ConflictEntity,
        expected: ConflictState,
        actual: ConflictState,
    },
    NotFound(&'static str),
    InvalidCursor { reason: CursorError },
    Serialization(#[source] serde_json::Error),
    Constraint(#[source] sqlx::Error),
    Database(#[source] sqlx::Error),
    Migration(#[source] sqlx::migrate::MigrateError),
}

enum ConflictState {
    Operation(OperationStatusKind),
    Workflow(WorkflowState),
    Decision(DecisionStatus),
}

enum CursorError {
    WrongScope,
    Malformed,
    ExpiredOrUnavailable,
}
```

**Recovery CAS (exact, not IN):**

```sql
UPDATE operation
SET status = 'Interrupted', finished_at = now, ...
WHERE id = ? AND status = ? AND runtime_instance_id = ?
```

Both `expected_status` (Pending or Running) AND `expected_runtime` come from `find_orphaned_operations`. No `IN` clause — exact match.

**Active execution uniqueness per Task:**

```sql
CREATE UNIQUE INDEX idx_active_execution_per_task
ON operation(task_id, status_kind)
WHERE status_kind IN ('Pending', 'Running') AND kind = 'ExecutionRun';
```

**ThreadEntry ordinal allocation:**

```sql
-- same TX:
UPDATE planning_thread
SET next_entry_ordinal = next_entry_ordinal + 1
WHERE id = ?
RETURNING next_entry_ordinal - 1

INSERT INTO thread_entry (..., ordinal = <from update>, ...)
```

The ordinal is allocated and the entry is inserted in a single TX — no race window.

**Task lifecycle atomic with Operation:**

- **Scheduler dispatch:** `Operation(Pending) + Task(Ready → Running) + OperationStarted + TaskTransitioned` in ONE TX
- **Executor complete:** `Operation(Running → Completed{outcome}) + Task(Running → Completed/Failed/Blocked) + events` in ONE TX

---

## Section 5 — SQLite Schema (Section 4.4 Final Proposal)

### Tables (16 + 1 virtual)

```text
project
planning_thread
thread_entry
decision
research_artifact
workflow
task
task_parent
gate
verification_check
operation
agent_invocation
verification_run
verdict
durable_event
command_record
runtime_instance
research_fts          (virtual, FTS5)
research_fts_triggers (keep FTS5 in sync)
```

### Multi-scope `durable_event`

```sql
CREATE TABLE durable_event (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    project_id TEXT,
    thread_id TEXT,
    workflow_id TEXT,
    operation_id TEXT,
    actor_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    causation_kind TEXT,
    causation_ref TEXT,
    correlation_id TEXT,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Per-scope indexes:
CREATE INDEX idx_event_thread_seq    ON durable_event(thread_id, seq);
CREATE INDEX idx_event_project_seq   ON durable_event(project_id, seq);
CREATE INDEX idx_event_workflow_seq  ON durable_event(workflow_id, seq);
CREATE INDEX idx_event_operation_seq ON durable_event(operation_id, seq);

-- Global = all FKs NULL
-- Same event reachable from multiple scopes
-- EventCursor opaque { scope, last_seq }
```

### Command idempotency

```sql
CREATE TABLE command_record (
    principal_kind     TEXT NOT NULL,
    principal_id       TEXT NOT NULL,
    command_scope_kind TEXT NOT NULL,
    command_scope_key  TEXT NOT NULL,   -- '' = global scope (canonical)
    command_id         TEXT NOT NULL,
    outcome_kind       TEXT NOT NULL,   -- 'NoContent' → outcome_ref NULL
    outcome_ref        TEXT,            -- nullable only when outcome_kind='NoContent'
    recorded_at        TEXT NOT NULL,
    PRIMARY KEY (principal_kind, principal_id, command_scope_kind, command_scope_key, command_id)
);
```

Global canonicalization: empty string for `command_scope_key` (not NULL — SQLite uniqueness requires non-null PK columns).

### Key CHECK constraints

- All enum columns: `CHECK (... IN (...))`
- `verification_check`: `((task_id IS NOT NULL) + (gate_id IS NOT NULL)) = 1` (belongs to exactly one)
- `operation`: state-derived invariants
  - `Pending` + `finished_at IS NULL`
  - `Running` + `finished_at IS NULL`
  - `Completed` + outcome NOT NULL
  - `Failed` + failure NOT NULL
  - `started_at IS NOT NULL` ⇔ status past Pending
- `command_record`: Global canonicalization + outcome kind/ref consistency
- `durable_event`: actor pair atomic, causation pair atomic (both fields present or both NULL)
- `workflow`: `state = 'Frozen' → frozen_at IS NOT NULL`

### Workflow lineage linear

```sql
CREATE UNIQUE INDEX idx_workflow_previous ON workflow(previous_version_id);
```

Each version has at most one successor (no branching).

### Indexes (with the queries they serve)

| Index | Query |
|---|---|
| `idx_op_runtime_status` | `find_orphaned_operations` |
| `idx_op_workflow_status`, `idx_op_task_status`, `idx_op_thread_status` | scheduler + `derive_task_state` |
| `idx_event_thread_seq`, `idx_event_project_seq`, `idx_event_workflow_seq`, `idx_event_operation_seq` | per-scope cursor reads |
| `idx_invocation_op` | agent invocation lookup |
| `idx_verification_run_op`, `idx_verdict_run` | verification summaries |
| `idx_decision_project_status` | decision listings |
| `idx_thread_entry_thread_ord` | thread entry pagination |
| `idx_workflow_thread_state`, `idx_workflow_previous` | lineage traversal |
| `idx_research_project_created` | research listings |
| `idx_active_execution_per_task` (PARTIAL UNIQUE) | active-execution uniqueness |
| `idx_task_parent_parent` | DAG traversal |

### FTS5

```sql
CREATE VIRTUAL TABLE research_fts USING fts5(
    title,
    summary,
    content = 'research_artifact',
    content_rowid = 'rowid',
    tokenize = 'porter unicode61 remove_diacritics'
);
```

`AFTER INSERT/UPDATE/DELETE` triggers keep `research_fts` in sync with `research_artifact`.

**FTS5 syntax lives ONLY in `storage/sqlite/queries/research.rs`.** Domain types never see FTS5 SQL.

`rowid` exists only as FTS5's internal pointer — NEVER domain ordering. All ordering uses explicit `durable_seq` / `ordinal` keys.

### SQLite runtime config (conservative v1)

```sql
PRAGMA foreign_keys = ON;       -- required for ON DELETE RESTRICT
PRAGMA journal_mode = WAL;      -- durable, concurrent readers
PRAGMA busy_timeout = 5000;     -- 5s; protects against in-memory upgrade-deadlock
```

**NOT locked at v1** (evaluate per environment): `synchronous`, `temp_store`, `wal_autocheckpoint`.

### Domain contract

Committed `durable_event.seq` values are strictly increasing. **Gaps are allowed** (rolled-back TXs leave holes). `AUTOINCREMENT` is an implementation detail; consumers must not depend on contiguous sequences.

---

## Section 6 — Open Items (post-spec)

- [ ] Write SQLx migrations
  - `0001_core.sql` — tables + CHECK constraints
  - `0002_indexes.sql` — query indexes
  - `0003_research_fts.sql` — FTS5 virtual table + triggers
  - `0004_active_execution_unique.sql` — partial unique index
- [ ] Implement `storage` module per Section 4.3 + 4.4
- [ ] Implement domain types per Section 4.1
- [ ] Storage contract tests: same suite runs against SQLite (v1) + Postgres (future)
- [ ] Consider ADR compression: 4 merges proposed
  - `0007` + `0008` → `0004` (ResearchArtifact durable from v1)
  - `0027` + `0028` → Process (cancellation semantic)
  - `0026` → `0021` (Workflow frozen + supersede)
  - `0012` + `0031` → Verification (CheckPhase, Reviewer separate)
- [ ] `writing-plans` skill for implementation plan
- [ ] Real-CLI acceptance tests in CI (opt-in)

---

## Section 7 — Cross-cutting Rules (single-source)

1. **Domain stays pure** — no `SQLx`/`sqlx`/`sea_orm`/`sea_query`/`Row`/`Entity`/`ActiveModel`/`Pg*`/`Sqlite*` imports in domain types
2. **Ordering explicit** — `durable_seq` / `ordinal`. NEVER `rowid`. NEVER physical insertion order. NEVER implicit `SELECT` order
3. **DB-specific syntax isolation** — FTS5 / `tsvector` / `rowid` / `PRAGMA` / `strftime` only inside `storage/<backend>/`
4. **All durability mutations atomic with their durable events** — state + event in ONE TX
5. **`PLAN_BLOCKED` = Operation outcome**, NEVER HTTP error
6. **Secrets refs only** (config) → `SecretResolver` → child env at spawn. No secret values in config files
7. **Idempotency = same identity** (resource_id + current state), NOT frozen-history
8. **Type everything** — no string buckets in error contracts, no raw `Uuid` in `EntryRef`, no `serde_json::Value` in public error shapes

---

## Section 8 — Decisions Reference

**32 ADRs total (19 active, 13 archived).** Full list via `mx_search(project='shadows', doc_type='decision', status='active')`.

### Foundational ADRs (active)

| ADR | Title | Key commitment |
|---|---|---|
| ADR-0001 | Library-first with ADR discipline | Library-first; no custom ORM/migration engine; every non-trivial choice gets an ADR |
| ADR-0003 | Agent seam | `Role != Harness != Provider != Model` |
| ADR-0004 | Shadows owns truth | External agents cannot write directly; they invoke domain operations |
| ADR-0007 | Decisions project-level | Decisions NOT workflow-scoped |
| ADR-0008 | ResearchArtifact durable from v1 | Research is first-class persistent data |
| ADR-0009 | Operation lifecycle with two-phase spawn | Pending row exists before subprocess fork |
| ADR-0012 | VerificationCheck uses CheckPhase | Pre/Post phase separation; bind to exactly one of task or gate |
| ADR-0013 | Secrets use references | Config holds refs only; resolver resolves at spawn |
| ADR-0014 | Architecture tests (mechanical only) | Import-based, no semantic inference |
| ADR-0015 | Observability + module-local errors | Each module owns its error surface |
| ADR-0018 | Event durability | Durable events are the source of truth for resync |
| ADR-0021 | Scheduler pure decision | `scheduler/` is a pure function; no I/O |
| ADR-0024 | MCP-attached agents | External agents via MCP, authority validated by Shadows |
| ADR-0026 | Frozen workflow + supersede | Frozen is immutable; updates via `previous_version_id` |
| ADR-0027 | `process/` sole owner | `process/` owns `ProcessSpec` + `ProcessHandle` |
| ADR-0028 | Cancellation semantic | Cancel = operation transition, not SIGTERM-as-truth |
| ADR-0031 | Reviewer separate from Verifier | No `ReviewCheck` kind in v1 |
| ADR-0032 | Storage ports backend isolation + explicit ordering | Domain never imports storage; ordering explicit |
| ADR-0033 | Persistence strategy — SQLx 0.9 | Winner of spike (over SeaORM) |

---

*End of spec. Sections 2, 3, 4.1, 4.2, 4.3, 4.4 captured. Migrations + implementation plan to follow.*
