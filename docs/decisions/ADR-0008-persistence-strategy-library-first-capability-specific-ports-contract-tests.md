# Persistence strategy: library-first, capability-specific ports, contract tests

**Doc ID:** 1008
**Status:** accepted
**Tags:** architecture, library-first, persistence, portability, storage
**Source slug:** adr-0008-persistence-strategy-library-first-capability-specific-ports-contract-tests

---

# Decision — Persistence strategy: SQLx, library-first, capability-oriented storage

**Status:** accepted · 2026-09-20

## Context

Shadows v1 needs SQLite persistence while preserving a credible path to PostgreSQL without changing the domain/application model. The persistence delta documented in `docs/evidence/persistence/DELTA_VALIDATION.md` compared current SeaORM 2.0.3 and SQLx 0.9.0 against the same real PostgreSQL 16.15 semantics, closing the earlier spike's two evidence gaps: SeaORM PostgreSQL was exercised end-to-end, and both candidates used current releases and official migration paths.

This ADR retains the earlier principles: library-first, pure domain types, backend semantics contained by storage, capability-specific boundaries, and behavioral contract tests as the portability proof.

## Decision

Use **SQLx 0.9 only** as the persistence library for Shadows v1.

- **Backend for v1:** SQLite.
- **Portability requirement:** PostgreSQL compatibility remains required and was validated by the delta spike. A production PostgreSQL adapter is not required for v1.
- **Migrations:** use SQLx's official `Migrator` / `migrate!`; do not build a custom migration framework or runner.
- **Query construction:** do not add SeaQuery initially. Add a dynamic query builder only when a concrete use case demonstrates the need.
- **Alternatives:** SeaORM is viable but not selected; SQLx + SeaQuery is also not selected for v1.

The architectural boundary is:

```text
Domain/Application
        ↓
capability/use-case-oriented storage boundary
        ↓
SQLx
        ↓
SQLite v1 / PostgreSQL-compatible future
```

## Storage isolation

Domain/application types remain plain Rust types. They must not depend on or expose SQLx, SQL, driver rows, connections, transactions, backend errors, backend-specific types, or backend-specific semantics.

SQL is expected with SQLx, but it belongs inside `storage/`:

- shared/static storage SQL and mappings stay in storage-owned modules;
- SQLite-specific FTS5 syntax, PRAGMA use, locking, and configuration stay in `storage/sqlite/`;
- future PostgreSQL-specific DDL, functions, types, and search semantics stay in its backend implementation.

Storage owns translation from library/backend errors to stable application-facing errors. Architecture tests and module visibility enforce the dependency boundary; behavioral storage contract tests prove semantics and portability.

## Persistence API

The production API is capability/use-case-oriented. It must model Shadows semantics such as committing a command atomically, appending and reading durable events, persisting operation transitions, loading planning state, superseding workflows, and research search.

Do not introduce a generic `Database`, `Repository<T>`, repository-per-entity convention, transaction DSL, or public `Vec<MultiOp>`. Define traits only where a real semantic boundary, test seam, or multiple implementation need exists; concrete storage services are acceptable elsewhere.

The core atomic invariant remains:

```text
state mutation + DurableEvent + CommandRecord = one transaction
```

This is implemented with SQLx's transaction support, not a custom transaction engine.

## Migration acceptance

The SQLx migration path must prove:

- fresh database → latest schema;
- every supported old database → latest schema;
- concurrent startup is safe;
- a failed migration does not leave an invalid schema.

Reversible migrations are not a universal requirement. Backend-specific migration SQL remains inside storage-owned migrations.

## Delta evidence

Both SeaORM 2.0.3 and SQLx 0.9.0 passed the real PostgreSQL 16.15 parity contract for:

- atomic `CommandRecord + Operation + DurableEvent`;
- scoped durable-event ordering/cursor behavior;
- multi-entity transactions and rollback;
- UUID, timestamp, enum, optional-ID, and JSON roundtrips;
- fresh and supported-old-schema migration paths.

The decision favors SQLx because:

- its official migrator passed concurrent startup **4/4**, while SeaORM produced **1/4 success and 3/4 PostgreSQL 23505 failures** without additional startup serialization;
- the same atomic workload required less adapter glue;
- the measured local cold/hot build, dependency count, and target footprint were lower;
- neither the old SQLx spike nor the delta produced a concrete SeaQuery use case;
- no persistence type leaked into the shared domain contract.

Detailed reports remain under `docs/evidence/persistence/`. The throwaway spike code was removed after the decision to prevent it from being mistaken for production structure; it remains recoverable from Git history.

## Alternatives considered

### SeaORM 2.0.3

Viable and capable of the required transactional and typed PostgreSQL behavior, but rejected for Shadows v1 because it added adapter glue, dependencies, compile/target cost, and concurrent-migration startup work without a meaningful benefit for this workload.

### SQLx + SeaQuery

Rejected for v1 because the static-query workload demonstrated no need for a dynamic query builder. This can be reconsidered only for a concrete dynamic-composition use case.

## Explicit non-decisions

This ADR does not automatically require:

- a separate domain, SQLite, or PostgreSQL crate;
- a production PostgreSQL adapter now;
- `Vec<MultiOp>` or another generic transaction DSL;
- a global `durable_seq` counter or any particular cursor scope;
- a SQLite single-connection production model;
- a repository trait for every entity.

Start as a modular monolith with module boundaries, visibility controls, and architecture tests. Split crates only when a concrete ownership, reuse, compilation, or enforcement need appears.

## Toolchain consequence

SQLx 0.9 declares Rust 1.94 as its minimum supported Rust version. Record Rust 1.94 as the current candidate dependency floor, not the final workspace `rust-version`; the final floor is the maximum required by the complete confirmed v1 dependency set.

## Follow-ups

- [ ] Finalize Section 4.3 persistence model/API using SQLx and capability/use-case-oriented operations.
- [ ] Finalize Section 4.4 SQLite schema and SQLx migrations.

Do not reopen the SeaORM/SQLx comparison unless a new concrete blocker invalidates this evidence.

Related: [[storage-ports-backend-isolation-explicit-ordering-domain-semantics]], [[library-first-with-selective-custom-implementation]]
