# Storage ports are capability-specific, not a generic Database trait

**Doc ID:** 1007
**Status:** accepted
**Tags:**
**Source slug:** adr-0007-storage-ports-are-capability-specific-not-a-generic-database-trait

---

# Decision — Storage ports: backend isolation + explicit ordering + domain semantics

**Status:** accepted · 2026-09-20

## Rule (storage isolation principle)

Any database-specific syntax or behavior stays inside the backend
storage implementation. The domain deals only with semantics.

Backend-specific things that MUST NOT leave `storage/<backend>/`:

```text
FTS5 query syntax
tsvector query syntax
rowid (any reference)
PRAGMA calls
SQLite error type names
Postgres error type names
backend-specific SQL functions (strftime, date_trunc, MATCH, ...)
backend-specific transaction types
backend-specific connection types
```

The domain / application deals ONLY with typed semantics:

```text
SearchQuery            (not "FTS5 query string")
SearchScope            (not "MATCH expression")
DurableCursor          (not "rowid 184")
ExplicitOrder          (not "physical insertion order")
StorageTransaction     (not "rusqlite::Transaction")
```

## Domain contract rule

```text
No database dialect or implicit database behavior may become part
of a domain contract.
```

A function or type in the domain that names an SQL concept (FTS5,
rowid, PRAGMA, MATCH, ...) is by definition a leak. Search the
domain for such names — they should not be there.

## Ordering invariant

```text
Domain-visible order MUST always have an explicit domain key.
```

Allowed:

- `durable_seq`
- `position`
- `ordinal`
- `created_at + stable explicit tie-breaker`

Forbidden:

- `rowid`
- physical insertion order
- default `SELECT` order
- backend-specific sequence behavior as an unstated contract

**shadow's R14 lesson:** `ORDER BY created_at ASC, rowid ASC` for
deterministic ordering broke SQLite → Postgres portability. We
explicitly forbid this category of implicit ordering.

## Architecture test scope (mechanical only)

The architecture test forbids ONLY mechanical, observable violations:

```text
rusqlite/sea_orm/sqlx outside storage/    → fail
tokio::process outside process/           → fail
axum outside protocol/                    → fail
```

These rules catch **import boundaries**. They do NOT catch semantic
leaks.

## Semantic leaks are NOT caught by grep

We do NOT build blacklists of forbidden keywords (`strftime`,
`date_trunc`, `MATCH`, `rowid`, ...):

- Such lists grow unbounded
- They produce false positives
- They do not catch the actual semantic leaks (e.g., shadow's
  `encode_fts5_literal_query` was a *function name* the test would
  have to enumerate)

Semantic guarantees come from:

- **Storage contract tests** — same suite runs against every backend
  implementation. A failure under one backend is a portability
  defect, not an implementation choice.
- **Code review** — humans audit domain code for implicit database
  assumptions.
- **Backend parity tests** — differential testing between SQLite and
  future Postgres on the same port surface.

The architecture test is a **backstop**, not the proof of portability.

## Storage contract test

The same test suite runs against every backend implementation:

```text
v1:        StorageContract suite
              ↓
            SQLite
              ↓
            PASS / FAIL

future:    StorageContract suite
              ├── SQLite
              └── PostgreSQL
              └── ...
```

Required scenarios:

- atomic state + event + command
- idempotency
- durable cursor ordering
- workflow version lineage
- operation recovery data
- search semantics
- transaction rollback
- explicit ordering

## Initial modular-monolith shape

```text
storage/
├── mod.rs
├── transaction.rs     ← use-case atomic boundaries
├── migrations/
├── mapping/types as needed
├── sqlite/            ← v1 implementation
└── (postgres/ future)
```

The domain/application calls capability/use-case-oriented storage APIs. It
does not import SQLx, SQLite/PostgreSQL types, or backend implementations.
The exact internal folders are implementation details, not an ADR contract.

## No abstraction more than needed

We do NOT write:

```rust
trait ProjectRepository { ... }
trait ThreadRepository { ... }
trait DecisionRepository { ... }
```

just because the database might change. We define a port only when
shadows actually needs a semantic boundary.

Prefer semantic operations such as:

```text
commit_command
append/read durable events
persist_operation_transition
supersede_workflow
load bounded thread snapshot
search_research
```

Define traits only when a real semantic boundary, test seam, or multiple
implementation need exists. Concrete storage services are valid in v1. No
public transaction DSL or `Vec<MultiOp>` is adopted.

## Transactions

The atomic invariant:

```text
state mutation + durable event + CommandRecord = one transaction
```

is implemented using the persistence library's transaction support.
We do not write a transaction engine ourselves.

## What this ADR does NOT decide

- The specific library (SQLx 0.9 is decided by the persistence ADR)
- The schema (decided in Section 4.5)
- Specific query implementations (lives in `storage/sqlite/`)

Related: [[persistence-strategy-library-first-capability-specific-ports-contract-tests]], [[library-first-with-selective-custom-implementation]], [[architecture-tests-are-mechanical-defense-in-depth]]
