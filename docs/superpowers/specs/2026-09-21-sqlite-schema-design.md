# Section 6 — SQLite Schema

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

No migration SQL is written until this schema proposal is accepted.

## 6.1 Table set

**17 ordinary tables + 1 FTS5 virtual table:**

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

runtime_instance
operation
agent_invocation
verification_run
verdict

durable_event
command_record

research_fts             # FTS5 virtual table
```

FTS synchronization uses triggers; triggers are not tables.

## 6.2 Common encodings

| Domain value | SQLite representation |
|---|---|
| UUID newtype | `TEXT` canonical UUID |
| Timestamp | `TEXT` RFC3339 UTC |
| Enum | `TEXT` + `CHECK` where the enum is schema-stable |
| Typed JSON payload | `TEXT`, serialized/deserialized only inside storage mapping |
| Boolean | `INTEGER` with `CHECK (value IN (0,1))` |

The domain must not expose these persistence encodings.

---

## 6.3 `project`

```text
id                  TEXT PRIMARY KEY
slug                TEXT NOT NULL UNIQUE
name                TEXT NOT NULL
default_config_ref  TEXT NULL
created_at          TEXT NOT NULL
directory           TEXT NULL
```

`directory` (§4.2) arrived in migration 0003 and is NULL only on rows written
before it. SQLite cannot add a NOT NULL column without a default, and a default
would be an invented directory, so two triggers hold the rule for new rows
instead: a NULL `directory` can be neither inserted nor written over one.

---

## 6.4 `planning_thread`

```text
id                  TEXT PRIMARY KEY
project_id          TEXT NOT NULL FK project(id) ON DELETE RESTRICT
title               TEXT NOT NULL
status              TEXT NOT NULL CHECK status IN ('Open','Closed')
next_entry_ordinal  INTEGER NOT NULL DEFAULT 1 CHECK next_entry_ordinal > 0
created_at          TEXT NOT NULL
harness_session_id  TEXT NULL
```

`harness_session_id` (§4.2, migration 0004) is written once, by an update
guarded on `IS NULL`, and never cleared.

Index:

```text
(project_id, created_at, id)
```

---

## 6.5 `thread_entry`

```text
id           TEXT PRIMARY KEY
thread_id    TEXT NOT NULL FK planning_thread(id) ON DELETE RESTRICT
ordinal      INTEGER NOT NULL CHECK ordinal > 0
kind         TEXT NOT NULL
author_kind  TEXT NOT NULL
author_id    TEXT NOT NULL
body         TEXT NOT NULL
refs_json    TEXT NOT NULL DEFAULT '[]'
created_at   TEXT NOT NULL

UNIQUE(thread_id, ordinal)
```

Milestone 1 adds `operation_id TEXT NULL FK operation(id)`: the turn an entry
belongs to (§12.7).

> **OPEN — this table has nowhere to put the harness-side identity of the line an
> entry came from.**
>
> `StreamItem::Entry` carries a `uuid` that its own definition calls "the entry's
> harness-side identity": the id the harness assigned to the line it streamed. This
> table's columns are the entry's own `id`, its `ordinal`, and an author
> (`author_kind` + `author_id`) — and an author is who wrote the message, not which
> line of which stream it arrived on. There is no column for the latter.
>
> Found during Task 10's fix round, by a review that caught the uuid being written
> into `author_id`, which made every agent message in a turn look like a different
> author. It is dropped instead: the author is the agent whose turn it is, the same
> actor on every line, and the uuid is not recorded anywhere. This is the same
> shape as `agent_invocation`'s missing harness version above — a harness-side fact
> the schema was never given a home for — and it is recorded rather than closed by
> inventing a column, for the same reason.
>
> **This does not block Milestone 0.** The milestone streams a turn and persists its
> entries; nothing in it reads an entry back by the harness's id for that line.
>
> **Trigger that closes this:** the first feature that must match a stored entry to
> a harness-side line — resume dedupe (deciding whether a line a resumed session
> re-emits is already recorded) or replay against a live harness session. Either one
> needs the identity and cannot be written without deciding where it lives.

Ordinal allocation happens in the same transaction:

```sql
UPDATE planning_thread
SET next_entry_ordinal = next_entry_ordinal + 1
WHERE id = ?
RETURNING next_entry_ordinal - 1;
```

Then the entry is inserted using that ordinal.

No `MAX(ordinal)+1`.

---

## 6.6 `decision`

```text
id               TEXT PRIMARY KEY
project_id       TEXT NOT NULL FK project(id) ON DELETE RESTRICT
title            TEXT NOT NULL
summary          TEXT NOT NULL
status           TEXT NOT NULL
refs_json        TEXT NOT NULL DEFAULT '[]'
decided_by_kind  TEXT NULL
decided_by_id    TEXT NULL
created_at       TEXT NOT NULL
updated_at       TEXT NOT NULL
```

Constraints:

```text
status IN ('Proposed','Accepted','Rejected','Superseded')

(decided_by_kind IS NULL) == (decided_by_id IS NULL)

Proposed -> decided_by_* are NULL
Accepted/Rejected -> decided_by_* are NOT NULL
```

Decision remains project-level. There is no `decision.thread_id`.

Thread relevance is derived from explicit thread-entry/workflow references unless a future real query proves an explicit association table is necessary.

---

## 6.7 `research_artifact`

```text
id          TEXT PRIMARY KEY
project_id  TEXT NOT NULL FK project(id) ON DELETE RESTRICT
title       TEXT NOT NULL
source      TEXT NULL
summary     TEXT NOT NULL
created_at  TEXT NOT NULL
```

No domain ordering depends on SQLite `rowid`.

---

## 6.8 `workflow`

```text
id                   TEXT PRIMARY KEY
thread_id            TEXT NOT NULL FK planning_thread(id) ON DELETE RESTRICT
state                TEXT NOT NULL
previous_version_id  TEXT NULL
source_plan_json     TEXT NULL
created_at           TEXT NOT NULL
updated_at           TEXT NOT NULL
frozen_at            TEXT NULL

UNIQUE(id, thread_id)
UNIQUE(previous_version_id)
```

State:

```text
Draft
Approved
Frozen
Running
Completed
Failed
```

Timestamp constraint:

```text
state IN ('Frozen','Running','Completed','Failed')
    -> frozen_at IS NOT NULL
```

Lineage must remain inside one thread.

Use a composite FK:

```text
(previous_version_id, thread_id)
    -> workflow(id, thread_id)
```

when `previous_version_id` is non-null.

`source_plan_json`, if present, is an immutable authored/provenance snapshot only.

The scheduler reads normalized:

```text
task
task_parent
gate
verification_check
```

not `source_plan_json`.

---

## 6.9 `task`

```text
id             TEXT PRIMARY KEY
workflow_id    TEXT NOT NULL FK workflow(id) ON DELETE RESTRICT
contract_json  TEXT NOT NULL
scope_json     TEXT NOT NULL
state          TEXT NOT NULL DEFAULT 'Pending'
created_at     TEXT NOT NULL
updated_at     TEXT NOT NULL

UNIQUE(id, workflow_id)
CHECK state IN ('Pending','Ready','InProgress','Completed','Failed','Blocked')
```

`Task.state` is persisted authoritative scheduler state.

---

## 6.10 `task_parent`

Use workflow-aware composite integrity:

```text
workflow_id  TEXT NOT NULL
task_id      TEXT NOT NULL
parent_id    TEXT NOT NULL

PRIMARY KEY(workflow_id, task_id, parent_id)

FK (task_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT

FK (parent_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT

CHECK(task_id != parent_id)
```

Index:

```text
(workflow_id, parent_id)
```

This prevents DAG edges across workflow versions.

Acyclicity is a domain/scheduler validation rule, not expressible as a simple SQLite `CHECK`.

---

## 6.11 `gate`

```text
id             TEXT PRIMARY KEY
workflow_id    TEXT NOT NULL
after_task_id  TEXT NULL
created_at     TEXT NOT NULL

UNIQUE(id, workflow_id)

FK workflow_id -> workflow(id) ON DELETE RESTRICT

FK (after_task_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT
```

`Gate` is a grouping/timing anchor. It does not duplicate `CheckPhase`.

---

## 6.12 `verification_check`

```text
id            TEXT PRIMARY KEY
workflow_id   TEXT NOT NULL
task_id       TEXT NULL
gate_id       TEXT NULL
kind          TEXT NOT NULL
phase_kind    TEXT NOT NULL
config_json   TEXT NOT NULL
scope_json    TEXT NOT NULL

FK workflow_id -> workflow(id) ON DELETE RESTRICT

FK (task_id, workflow_id)
    -> task(id, workflow_id) ON DELETE RESTRICT

FK (gate_id, workflow_id)
    -> gate(id, workflow_id) ON DELETE RESTRICT
```

Kinds:

```text
Build
Test
Contract
Baseline
```

Phases:

```text
Task
Regression
Deferred
Final
```

Constraints:

```text
exactly one of task_id / gate_id is non-null

phase_kind = 'Task'
    -> task_id IS NOT NULL AND gate_id IS NULL

phase_kind = 'Deferred'
    -> gate_id IS NOT NULL AND task_id IS NULL

phase_kind IN ('Regression','Final')
    -> may bind to task_id or gate_id, but still exactly one
```

This keeps only established domain invariants in SQLite. A later accepted
decision may narrow Regression or Final ownership if real semantics require it.

---

## 6.13 `runtime_instance`

```text
id            TEXT PRIMARY KEY
version       TEXT NOT NULL
started_at    TEXT NOT NULL
stopped_at    TEXT NULL
stop_kind     TEXT NULL
              CHECK (stop_kind IS NULL OR stop_kind IN ('Graceful','Escalated'))
CHECK ((stopped_at IS NULL) = (stop_kind IS NULL))
```

A runtime is global daemon provenance, not project-owned.

`stopped_at` and `stop_kind` are what let startup distinguish a runtime that
ended on purpose from one that was lost, and why it ended. A row with
`stopped_at IS NULL` that is not the current runtime was lost uncleanly. The
CHECK keeps the two columns from disagreeing.

These columns do not gate recovery. Reconciliation selects on ownership alone —
every non-terminal Operation owned by a runtime other than the current one
(§8.6) — so a runtime that stopped on purpose without confirming termination
does not strand its work. See §8.1 for why there is no separate lifecycle enum.

---

## 6.14 `operation`

```text
id                   TEXT PRIMARY KEY
kind                 TEXT NOT NULL
status_kind          TEXT NOT NULL DEFAULT 'Pending'

thread_id            TEXT NULL
workflow_id          TEXT NULL
task_id              TEXT NULL

runtime_instance_id  TEXT NOT NULL FK runtime_instance(id) ON DELETE RESTRICT

outcome_json         TEXT NULL
failure_stage        TEXT NULL
failure_reason       TEXT NULL
interrupt_reason     TEXT NULL

cancel_requested_at       TEXT NULL
cancel_requested_by_kind  TEXT NULL
cancel_requested_by_id    TEXT NULL

created_at           TEXT NOT NULL
started_at           TEXT NULL
finished_at          TEXT NULL
```

v1 kinds:

```text
PlannerTurn
ExecutionRun
Verification
Research
Git
```

Status kinds:

```text
Pending
Running
Completed
Failed
Cancelled
Interrupted
```

Hierarchy rules:

```text
task_id != NULL     -> workflow_id != NULL
workflow_id != NULL -> thread_id != NULL
```

Use composite integrity where an ancestor is stored:

```text
(task_id, workflow_id)
    -> task(id, workflow_id)

(workflow_id, thread_id)
    -> workflow(id, thread_id)
```

State-shape constraints:

```text
Pending:
  started_at NULL
  finished_at NULL
  outcome_json NULL
  failure_stage NULL
  failure_reason NULL
  interrupt_reason NULL

Running:
  started_at NOT NULL
  finished_at NULL
  outcome_json NULL
  failure_stage NULL
  failure_reason NULL
  interrupt_reason NULL

Completed:
  started_at NOT NULL
  finished_at NOT NULL
  outcome_json NOT NULL
  failure_stage NULL
  failure_reason NULL
  interrupt_reason NULL

Failed:
  finished_at NOT NULL
  failure_stage NOT NULL
  failure_reason NOT NULL
  outcome_json NULL
  interrupt_reason NULL

Cancelled:
  finished_at NOT NULL
  outcome_json NULL
  failure_stage NULL
  interrupt_reason NULL

Interrupted:
  finished_at NOT NULL
  outcome_json NULL
  failure_stage NULL
  interrupt_reason NOT NULL
```

Cancellation-request shape is independent from lifecycle status:

```text
cancel_requested_at/by_kind/by_id are either all NULL or all NOT NULL
```

The request may coexist with `Pending` or `Running`. Terminal `Cancelled`
requires `finished_at`, and is written only after process-tree termination is
confirmed. A terminal non-cancelled status may retain request metadata as
history when completion won the race.

`runtime_instance_id` already identifies the previous runtime for an interrupted operation; the domain may expose that value in `OperationStatus::Interrupted`.

### Active ExecutionRun uniqueness

Correct v1 partial unique index:

```sql
CREATE UNIQUE INDEX idx_active_execution_per_task
ON operation(task_id)
WHERE kind = 'ExecutionRun'
  AND status_kind IN ('Pending','Running');
```

This prevents a simultaneous Pending and Running execution attempt for the same task.

---

## 6.15 `agent_invocation`

```text
id                 TEXT PRIMARY KEY
operation_id       TEXT NOT NULL FK operation(id) ON DELETE RESTRICT
role               TEXT NOT NULL
harness_kind       TEXT NOT NULL
profile_json       TEXT NOT NULL
native_session_id  TEXT NULL
created_at         TEXT NOT NULL
```

> **DECIDED 2026-09-24 — implementation waits for Milestone 1's migration.**
> Explicit harness and agent path and version columns, in the table as §12.7
> lays it out. This is no longer an open architectural question; what follows is
> the reasoning that led to it.
>
> **Was: this table has nowhere to put the resolved harness path and version.**
>
> §8.2 requires `AgentInvocation` to freeze "harness kind, profile, and resolved
> executable identity + version (§1.4)" at claim time, and §1.4 requires both to be
> recorded because the measured stream contract belongs to one installation at one
> version and a machine carries several. This table names `harness_kind` and
> `profile_json` and no column for either the resolved path or the version.
>
> Found during Task 9, by an implementer who was told to record the version per
> Operation and correctly refused to invent a column for it. `Operation` is the wrong
> owner — §8.2 puts it on the invocation — but the invocation has no home for it
> either. Two candidates: explicit `harness_path` and `harness_version` columns, or
> inside `profile_json`. Explicit columns are the better answer if the record is ever
> to be queried ("which turns ran under the version that changed?"), which §1.4's
> reasoning implies it will be.
>
> **This does not block Milestone 0.** `agent_invocation` is not in
> `migrations/0001_milestone0.sql` at all — the milestone persists seven tables and
> this is not one of them, which the plan declares as a known gap.
>
> **Trigger that closes this:** the task that first creates the `agent_invocation`
> table. It cannot be written without answering this, so the question is asked where
> it bites rather than carried as a worry.

Roles include:

```text
Planner
Executor
Reviewer
Researcher
```

`profile_json` may contain provider/model/profile configuration, but role, harness, provider, and model remain conceptually distinct.

Index:

```text
(operation_id, created_at, id)
```

No reverse `operation.agent_invocation_id` column exists.

---

## 6.16 `verification_run`

Lifecycle is owned by its associated `Operation`.

```text
id            TEXT PRIMARY KEY
operation_id  TEXT NOT NULL UNIQUE FK operation(id) ON DELETE RESTRICT
check_id      TEXT NOT NULL FK verification_check(id) ON DELETE RESTRICT
created_at    TEXT NOT NULL
```

No duplicate:

```text
status_kind
started_at
finished_at
```

fields exist here.

---

## 6.17 `verdict`

One final verdict per run in v1:

```text
id                  TEXT PRIMARY KEY
run_id              TEXT NOT NULL UNIQUE FK verification_run(id) ON DELETE RESTRICT
passed               INTEGER NOT NULL CHECK passed IN (0,1)
evidence_ref         TEXT NOT NULL
discrepancies_json   TEXT NULL
created_at           TEXT NOT NULL
```

Infrastructure failure belongs to the verification `Operation`; it is not encoded as a fake failing verdict.

---

## 6.18 `durable_event`

One row per event; the same row may be visible in several scopes.

```text
seq             INTEGER PRIMARY KEY AUTOINCREMENT
event_id        TEXT NOT NULL UNIQUE
kind            TEXT NOT NULL

project_id      TEXT NULL
thread_id       TEXT NULL
workflow_id     TEXT NULL
operation_id    TEXT NULL

actor_kind      TEXT NULL
actor_id        TEXT NULL

causation_kind  TEXT NULL
causation_ref   TEXT NULL
correlation_id  TEXT NULL

payload_json    TEXT NOT NULL
created_at      TEXT NOT NULL
```

FKs use `ON DELETE RESTRICT`.

Pair constraints:

```text
actor_kind NULL <=> actor_id NULL
causation_kind NULL <=> causation_ref NULL
```

### `seq` ordering, and why it is backend-specific

`seq` is assigned at INSERT, not at COMMIT. The §2.10 no-gap handoff therefore
depends on assignment order matching commit order, which is a property of the
backend rather than of this schema.

**On SQLite it holds, and it holds structurally.** SQLite permits one write
transaction at a time. The INSERT that assigns `seq` can only execute while the
transaction holds the write lock, and the commit happens before that lock is
released, so a lower `seq` always commits before a higher one. A reader cannot
observe N+1 while N is invisible. Measured: zero visibility inversions across
21,798 reader polls under up to 32 concurrent writers
(`docs/evidence/persistence/WAL_VALIDATION.md`).

A rolled-back transaction's `seq` is **reused**, because `AUTOINCREMENT` keeps
its high-water mark in the ordinary `sqlite_sequence` table, whose update is part
of the transaction and rolls back with it. Rollbacks therefore leave no holes for
a replaying cursor. (`AUTOINCREMENT`'s no-reuse guarantee concerns rows that were
deleted, not transactions that never committed.)

> **OPEN — closed when the PostgreSQL adapter is designed.**
> **None of the above transfers to PostgreSQL.** A PostgreSQL sequence is
> non-transactional and is assigned outside any commit ordering, so two
> transactions can take values in one order and commit in the other, and a reader
> *can* observe the higher value first. §1.6 makes PostgreSQL compatibility a
> design requirement, so that adapter must assign its ordering key inside the
> commit-ordered section or use a different ordering mechanism. Which one is not
> decided, and nothing in v1 depends on it. Do not assume the SQLite property
> when writing backend-neutral cursor code.

### Scope semantics

```text
Global cursor    = all durable_event rows ordered by seq
Project cursor   = rows where project_id = ?
Thread cursor    = rows where thread_id = ?
Workflow cursor  = rows where workflow_id = ?
Operation cursor = rows where operation_id = ?
```

Global does **not** mean “all scope FKs are NULL”.

An event is written with **every** scope its subject has, not only the narrowest.
In particular every operation event — creation and each later transition — carries
its operation's `thread_id` as well as `operation_id`, read from the operation row
inside the same write transaction; an event scoped only to its operation is
invisible to the thread cursor a client follows. An operation with no thread
leaves `thread_id` NULL.

### Event cursor

Public:

```rust
struct EventCursor(/* opaque */);
```

Logical content:

```text
scope kind
scope identity if non-global
last global seq observed for that scope
```

The cursor is opaque/validated at the events/storage boundary.

### ThreadSnapshot cursor

`ThreadSnapshot` uses a **project-scoped** event cursor because its state may include project-level Decisions/Research with no `thread_id`.

Thread-scoped cursors remain valid for narrower subscriptions.

### Event indexes

```text
(project_id, seq)
(thread_id, seq)
(workflow_id, seq)
(operation_id, seq)
```

No domain logic relies on contiguous `seq` values.

---

## 6.19 `command_record`

Use a stable logical outcome shape rather than a growing `outcome_kind` per entity type:

```text
principal_kind      TEXT NOT NULL
principal_id        TEXT NOT NULL

command_scope_kind  TEXT NOT NULL
command_scope_key   TEXT NOT NULL

command_id          TEXT NOT NULL

command_kind         TEXT NOT NULL
command_schema_ver   INTEGER NOT NULL CHECK command_schema_ver > 0
request_fingerprint  TEXT NOT NULL

outcome_kind        TEXT NOT NULL   # Entity | NoContent
entity_kind         TEXT NULL
outcome_ref         TEXT NULL

recorded_at         TEXT NOT NULL

PRIMARY KEY(
  principal_kind,
  principal_id,
  command_scope_kind,
  command_scope_key,
  command_id
)
```

Global scope:

```text
command_scope_kind = 'Global'
command_scope_key  = ''
```

Non-global scopes use a non-empty canonical entity key.

Outcome constraints:

```text
outcome_kind = 'NoContent'
    -> entity_kind IS NULL AND outcome_ref IS NULL

outcome_kind = 'Entity'
    -> entity_kind IS NOT NULL AND outcome_ref IS NOT NULL
```

Representative `EntityKind` values:

```text
Project
PlanningThread
ThreadEntry
Decision
ResearchArtifact
Workflow
Task
Operation
AgentInvocation
VerificationRun
Verdict
```

`outcome_ref` is intentionally not a polymorphic SQL FK.

DB uniqueness is the final authority for external idempotency.

After a uniqueness conflict, storage compares the stored and incoming
`command_kind`, schema version, and fingerprint:

```text
equal     -> replay stored outcome
different -> CommandConflict
```

The normalized typed request includes explicit/defaulted values and target
resource identity. Raw JSON byte order is never the fingerprint input.

---

## 6.20 Cross-table atomic invariants

These are implemented as use-case-specific storage transactions.

### External creation/transition

```text
CommandRecord claim
+ entity mutation
+ durable event(s)
= one transaction
```

### Cancellation request and terminal transition

External cancellation request:

```text
CommandRecord
+ Operation.cancel_requested_*
+ OperationCancellationRequested
= one transaction
```

Runtime completion after process-tree termination:

```text
Operation Pending/Running -> Cancelled
+ OperationCancelled
= one internal transaction
```

The spawn path and cancellation path use exact CAS conditions so a cancelled
request cannot race into a committed `Running` state. A no-op cancel against a
terminal operation still records/replays its own command outcome but performs
no process effect.

### Scheduler dispatch

For a ready task:

```text
Task Ready -> InProgress
+ insert ExecutionRun(Pending)
+ OperationCreated
+ TaskTransitioned
= one transaction
```

The process has not started yet, so this transaction must **not** emit `OperationStarted`.

After spawn succeeds:

```text
Operation Pending -> Running
+ OperationStarted
= one transaction
```

If spawn fails:

```text
Operation Pending -> Failed(stage=Spawn)
+ Task InProgress -> Failed or Ready according to explicit retry policy
+ events
= one transaction
```

### Successful execution with required Task-phase verification

```text
ExecutionRun Running -> Completed(Success)
Task remains InProgress
```

until required verification completes.

Final required passing verdict:

```text
insert Verdict
+ Task InProgress -> Completed
+ TaskTransitioned
+ verification event(s)
= one transaction
```

Failed/blocking verification follows policy and transitions Task atomically with the verdict that decides it.

### Interrupted attempt

Recovery updates the exact stale attempt:

```text
WHERE operation.id = ?
  AND operation.status_kind = expected
  AND operation.runtime_instance_id = expected_runtime
```

Then task retry/requeue state is updated in the same use-case transaction when that operation determines task scheduling.

---

## 6.21 Task lifecycle v1

Base transitions:

```text
Pending -> Ready
Pending -> Blocked

Ready -> InProgress
Ready -> Blocked

InProgress -> Completed
InProgress -> Failed
InProgress -> Blocked

InProgress -> Ready   # interrupted/retry policy explicitly permits re-dispatch

Failed -> Ready       # explicit retry command/policy only
Blocked -> Ready      # explicit unblock only
Completed -> Ready    # explicit re-run only; never automatic
```

Interpretation:

- `Pending`: dependencies/gates not yet satisfied.
- `Ready`: schedulable, no active execution attempt. A task waiting only on
  runtime capacity or a conflicting write scope stays `Ready`; waiting is not
  `Blocked`.
- `InProgress`: dispatch has claimed the task. An attempt may be `Pending`,
  spawned and `Running`, or already finished while required Task-phase
  verification is still outstanding.
- `Completed`: successful execution and required Task-phase checks passed.
- `Failed`: execution/check policy made the task terminally failed for the current scheduling decision.
- `Blocked`: required capability/ancestor/policy makes progress impossible until explicit change.

Execution-attempt history is represented by multiple `ExecutionRun` operations for the same task.

> **OPEN — closed when the scheduler exists and has run real work.**
> The default Task transition after an attempt ends `Failed`, `Cancelled`, or
> `Interrupted`, and which layer chooses it, is not decided. See §8.9.

---

## 6.22 FTS5 strategy

FTS5 remains SQLite-backend-specific.

To avoid a durable relationship through the base table's implicit physical `rowid`, use a normal FTS5 table that stores the domain research ID as an unindexed field:

```sql
CREATE VIRTUAL TABLE research_fts USING fts5(
    research_id UNINDEXED,
    title,
    summary,
    tokenize = 'porter unicode61'
);
```

Triggers keep it synchronized:

```text
research_artifact INSERT
  -> INSERT research_fts(research_id, title, summary)

research_artifact UPDATE
  -> delete old FTS row by research_id
  -> insert updated FTS row

research_artifact DELETE
  -> delete FTS row by research_id
```

Search:

```text
MATCH on research_fts
→ obtain research_id
→ load/join authoritative research_artifact by UUID
```

FTS5's own internal rowid may exist internally, but Shadows does not use it as domain identity, ordering, or cross-table durable linkage.

All FTS SQL stays under:

```text
storage/sqlite/
```

A future PostgreSQL adapter may use `tsvector` without changing domain/application APIs.

---

## 6.23 SQLite connection policy

Required v1 connection configuration:

```text
PRAGMA foreign_keys = ON
PRAGMA journal_mode = WAL
PRAGMA busy_timeout = 5000
```

Meaning:

- `foreign_keys=ON`: enforce declared FK constraints.
- WAL: production file-backed SQLite supports concurrent readers while serializing writes.
- `busy_timeout`: tolerate normal `SQLITE_BUSY` writer contention for a bounded interval.

Do **not** claim `busy_timeout` fixes the shared-cache in-memory deadlock observed in the spike; production uses file-backed SQLite.

Do not lock performance/durability tuning without evidence:

```text
synchronous
temp_store
wal_autocheckpoint
```

Stay conservative by default.

### Writer strategy

Measured, not assumed: `docs/evidence/persistence/WAL_VALIDATION.md`.

```text
all write transactions are serialized through one write connection
every write transaction opens with BEGIN IMMEDIATE
busy_timeout remains set as a backstop
reads use a separate pool and are unaffected
```

Three findings fix this, each from a pair of otherwise-identical runs.

**Deferred `BEGIN` fails, and `busy_timeout` does not rescue it.** A transaction
that reads and then writes must upgrade to a write lock mid-transaction, and a
busy handler cannot wait there because the transaction already holds a read
snapshot. Deferred mode succeeded on 3–27 % of transactions, and raising
`busy_timeout` from 0 to 5000 ms moved that by eight transactions out of 1200.
SQLite reported `SQLITE_BUSY_SNAPSHOT` (517) in every deferred run and in no
other run, which identifies the upgrade failure directly rather than by
inference. No `busy_timeout` value fixes this.

**`BEGIN IMMEDIATE` alone is not enough either.** It moves contention to `BEGIN`,
where a busy handler *can* wait — so the two are required together. With
`busy_timeout = 0`, `IMMEDIATE` succeeded on 34 of 1600 transactions; with 5000
ms, on 1575.

**One write connection beat `IMMEDIATE` on both reliability and throughput.**
Zero failures out of 1600 and 1920, against 25–55 failures for `IMMEDIATE`, and
at higher throughput. SQLite's busy handler resolves contention by sleeping and
retrying, which discards work; an application-level write queue discards none.
The residual `IMMEDIATE` failures also cost more than their rate suggests: each
is a transaction the application must detect and retry, which the single write
connection removes the need for entirely.

`BEGIN IMMEDIATE` is kept even though a single write connection makes the upgrade
race unreachable today. It costs nothing uncontended, and it stops a second write
connection — a migration, a maintenance task, a future backend — from silently
reintroducing `SQLITE_BUSY_SNAPSHOT`.

Readers were never blocked: zero reader errors across 21,798 polls taken while
writers worked, including under 32 concurrent writers.

**Windows only.** SQLite's locking primitives differ by platform, and this
validation has no Linux evidence. Re-run it before calling the storage layer
cross-platform.

---
