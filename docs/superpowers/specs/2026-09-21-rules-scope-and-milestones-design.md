# Section 9 — Cross-cutting Rules

> Part of the [Shadows design specification](./README.md). Section numbers are
> stable across files, and every `§x.y` reference resolves through the ownership
> map there — many of them point into a different file.

1. **Domain types stay storage-agnostic.** No SQLx/SQLite/PostgreSQL types in domain/application signatures.
2. **SQL is allowed inside storage.** It is not limited to migration files.
3. **Backend-specific syntax remains backend-local.**
4. **Ordering is explicit.** Never rely on physical insertion order.
5. **Durable state mutation + durable event is atomic.**
6. **External idempotent mutation also includes CommandRecord in the same transaction.**
7. **Internal retryable creates use stable caller-generated IDs.**
8. **No generic command interpreter in storage.**
9. **No public transaction DSL.**
10. **No public raw `append_event`.**
11. **No generic `Repository<T>` without demonstrated need.**
12. **No native agent session is required for continuity.**
13. **No global environment mutation for child agents.**
14. **Secrets are references until spawn.**
15. **Reviewer and deterministic Verifier remain separate.**
16. **Task state and related operation/verdict transitions are atomically maintained when they form one scheduler invariant.**
17. **Current entity tables are authoritative state; the journal is history/provenance/resync.**
18. **Context Compiler is distinct from ThreadSnapshot.**
19. **Architecture scanners are mechanical defense-in-depth, not semantic proof.**
20. **Schema does not redefine domain semantics merely because SQLite makes another shape easier.**

---

# Section 10 — Out of Scope for v1

Shadows deliberately does not build the following. These are not open questions;
they are decisions to leave things out.


The current design intentionally does **not** decide:

- separate domain/storage crates;
- PostgreSQL production adapter implementation date;
- SeaQuery adoption;
- active-active multi-runtime support;
- reviewer implementation timing;
- secret hardening crates until lifecycle requirements justify them;
- a generic DB abstraction layer;
- a generic transaction-composition DSL.

Three items that used to sit in this list have moved, because they are open
questions rather than exclusions: automatic retry policy is §8.9, the Context
Compiler's ranking algorithm is §2.11, and the SQLite writer strategy is §6.23.

The non-binding long-term collaboration direction — a future team server,
multi-user accounts, and synchronisation — is **out of scope for v1 and adds no
types or requirements to it**. It is recorded in the project's history rather
than in this document, so that a future intention cannot be mistaken for a
current requirement.

---

# Section 11 — Readiness Milestones


Readiness is incremental. Feature work may build on a completed earlier layer
without waiting for every later layer; dependencies between milestones remain
explicit.

## 11.1 First Runnable Browser Planner

Order of work:


1. scaffold the single Rust crate, `shadows serve`, structured tracing, and the independent Web client;
2. open SQLite through SQLx with only the milestone schema;
3. implement local-directory Project and durable PlanningThread/ThreadEntry;
4. implement the managed process primitive and one Claude harness;
5. implement durable Planner Operation start, live SSE output, semantic stop, and restart reconciliation;
6. connect the browser UI to create/resume a thread, Start, show output, and Stop;
7. run the real Windows debug acceptance path and record exactly what remains unverified on Linux.

The milestone is incomplete until the user can operate this path from a browser.
Persistence-only or protocol-only completion is not an acceptable substitute.

Acceptance:


```text
[ ] `shadows serve` starts and prints one local address without opening a browser
[ ] the user can manually open the Web client in any browser
[ ] a local-directory project can be selected without exposing a path as project identity
[ ] a PlanningThread can be created and resumed
[ ] one real Claude Planner turn starts and streams output
[ ] Stop terminates and reaps the managed process tree before durable Cancelled
[ ] daemon restart restores the durable thread and terminal operation
[ ] structured logs correlate project, thread, and operation without sensitive payloads
[ ] the exact Windows acceptance run is recorded; Linux gaps are named honestly
```

## 11.2 Persistence Foundation Ready


```text
[ ] accepted SQLx migrations exist
[ ] SQLite opens with production connection policy
[ ] state + event atomicity passes
[ ] command idempotency replay and request-fingerprint conflict pass
[ ] durable journal/cursor passes
[ ] bounded consistent snapshot + history pagination pass
[ ] thread ordinal allocation passes under concurrency
[ ] SQLite file-backed WAL concurrency validation passes
[ ] fresh/upgrade/failed/concurrent migration tests pass
[ ] storage architecture import checks pass
```

Additionally, before the persistence foundation is considered complete:

```text
transaction rollback fault injection
active ExecutionRun uniqueness
crash recovery from Pending and Running
FTS trigger parity
```

Future PostgreSQL work must run the same semantic storage-contract suite.

## 11.3 Runtime / Execution Ready


```text
[ ] scheduler is deterministic/pure
[ ] Task/Operation atomic invariants pass
[ ] two-phase spawn passes
[ ] Pending/Running crash recovery passes
[ ] managed child + grandchild containment passes on Windows + Linux
[ ] cancellation request -> termination -> Cancelled flow passes
[ ] spawn/cancel race tests pass
[ ] provider/harness environment isolation passes
```

## 11.4 Protocol / MCP Ready


```text
[ ] HTTP command idempotency contract passes
[ ] SSE durable replay + live no-gap handoff passes
[ ] stable public errors/outcomes pass compatibility tests
[ ] MCP authority/binding passes
[ ] MCP mutations use the same application semantics
```

## 11.5 Continuity Ready


```text
[ ] durable PlanningThread entries pass
[ ] Context Compiler is deterministic, budgeted, and scoped
[ ] Claude -> Codex fake-harness continuity passes
[ ] native_session_id remains optional
[ ] snapshot/history pagination passes under large history
```

## 11.6 Later feature slices


After Milestone 0 works, add features in user-visible slices rather than
constructing the entire platform upfront:

1. Workflow draft/freeze and task display — Milestone 2, §13;
2. scheduler and execution;
3. deterministic verification;
4. context compilation and Claude/Codex continuity;
5. MCP-attached agents;
6. research artifacts/search;
7. AI Reviewer and team features when separately designed.

---

*End of specification.*
