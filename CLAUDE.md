# Shadows

> **AI Start Here** — Read these files to get started:
>
> | Document | Purpose |
> |----------|---------|
> | [CLAUDE.md](./CLAUDE.md) | Architecture, conventions, rules (this file) |
> | [docs/status.md](./docs/status.md) | Current project status, open items |
> | [docs/decisions/](./docs/decisions/) | Authoritative local ADR set |
> | [docs/superpowers/specs/shadows_design_spec_reviewed.md](./docs/superpowers/specs/shadows_design_spec_reviewed.md) | Canonical design candidate |
> | [sandbox/SPIKE_REPORT.md](./sandbox/SPIKE_REPORT.md) | Persistence spike report (SQLx vs SeaORM) |
>
> **Documentation rules:**
> - Accepted architecture decisions live locally in `docs/decisions/`.
> - The reviewed spec is canonical; `2026-09-20-shadows-design.md` is obsolete review history.
> - Plans/specs must remain consistent with local ADRs.
> - CLAUDE.md stays compact: links + rules + architecture. No long backlogs.

## Project

- **Slug:** shadows
- **Stack:** Rust 1.94+ candidate floor from SQLx 0.9, single crate, 15 top-level modules + cross-cutting
- **Status:** Canonical design candidate and local ADR consistency pass complete. Migrations, SQLite concurrency validation, and implementation plan remain.
- **Purpose:** Local-first AI orchestration layer (planning + workflow + context + execution + verification + continuity). Clean rewrite of `shadow` avoiding patching pattern.

## Architecture

Single Rust crate `shadows` with library + single binary (two modes: `serve` daemon + CLI client).

### Top-level modules (15)

```text
project/         thread/         command/        runtime/
agent/           planner/        workflow/       operation/
scheduler/       execution/      verification/   events/
storage/         protocol/       cli/
```

### Cross-cutting

`config`, `secrets`, `error`, `tracing`, `process`, `mcp`

### Single-ownership rules (5)

| Module | Sole owner of |
|---|---|
| `agent/` | AI subprocess harness (`AgentHarness::start`) |
| `storage/` | SQLite (and future PostgreSQL adapter) |
| `protocol/` | HTTP/SSE transport |
| `process/` | `tokio::process` / `process-wrap` (private to `process/` only) |
| `secrets/` | Secret value resolution (config holds refs only) |

### Persistence (spike outcome)

**Winner: SQLx 0.9 only.** The delta validated SQLx and SeaORM 2.0.3 end-to-end against PostgreSQL 16.15; SQLx retained the edge and SeaQuery had no demonstrated use case. See `sandbox/persistence-delta/DELTA_VALIDATION.md`.

## Rules (project-specific)

- **Library-first:** no custom ORM, no custom migration engine. Use Rust ecosystem crates.
- **Domain stays pure:** `domain/` types carry no persistence imports (`sea_orm`, `sqlx`, `sea_query`, `Row`, `Entity`, `ActiveModel`, `Pg*`, `Sqlite*`).
- **Ordering is explicit:** durable event sequence, thread-entry ordinal, or another explicit stable domain key. No `rowid`, physical insertion order, or implicit `SELECT` order.
- **DB-specific syntax isolation:** FTS5, tsvector, rowid, PRAGMA, strftime live ONLY inside `storage/<backend>/`. Domain never sees them.
- **Idempotency:** mutating commands carry `CommandId`, command kind, schema version, and normalized request fingerprint. Replay requires fingerprint equality; mismatch is `CommandConflict`.
- **PLAN_BLOCKED = Operation outcome, NOT HTTP error.** Structured refusal, not transport failure.

## Decisions

The authoritative ADR set is local under `docs/decisions/`. Resolve decisions by title/topic because consolidation changed historical numbering.
