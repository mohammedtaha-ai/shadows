# Shadows

> **AI Start Here** — Read these files to get started:
>
> | Document | Purpose |
> |----------|---------|
> | [CLAUDE.md](./CLAUDE.md) | Architecture, conventions, rules (this file) |
> | [docs/status.md](./docs/status.md) | Current project status, open items |
> | [docs/superpowers/specs/shadows_design_spec_reviewed.md](./docs/superpowers/specs/shadows_design_spec_reviewed.md) | The single authoritative design and decision source |
> | [docs/decisions/](./docs/decisions/) | Four compact decision maps for navigation |
> | [docs/superpowers/specs/2026-09-20-runtime-execution-model-draft.md](./docs/superpowers/specs/2026-09-20-runtime-execution-model-draft.md) | **Draft, not baseline.** Runtime/execution proposal with 7 open decisions |
> | [docs/future/shadows-team-direction.md](./docs/future/shadows-team-direction.md) | Non-binding future team/server direction |
> | [docs/evidence/persistence/](./docs/evidence/persistence/) | Archived persistence comparison evidence |
> | [docs/evidence/harness/](./docs/evidence/harness/) | Measured Claude harness stream contract and web-serving spike |
>
> **Documentation rules:**
> - The reviewed spec is the only active architecture and decision source.
> - The runtime draft is a proposal. Do not implement from it. Only its `TaskState::InProgress` vocabulary has been merged into the spec.
> - The four consolidated ADRs summarize the baseline; the spec carries the full semantics and wins if a summary becomes stale.
> - Do not create an ADR for every implementation detail. Amend the spec when the baseline changes; create a new ADR only for a later decision that explicitly supersedes part of the accepted baseline.
> - The deleted earlier spec and replaced ADR files remain recoverable from Git history; they are not active references.
> - Future-direction documents are non-binding and must not expand local v1 scope.
> - CLAUDE.md stays compact: links + rules + architecture. No long backlogs.

## Project

- **Slug:** shadows
- **Stack:** Rust 1.94+ candidate floor from SQLx 0.9, single crate, 15 top-level modules + cross-cutting
- **Status:** Architecture baseline accepted. The next deliverable is the first runnable browser Planner vertical slice, not the full schema or platform.
- **Purpose:** Local-first AI orchestration layer (planning + workflow + context + execution + verification + continuity). Clean rewrite of `shadow` avoiding patching pattern.

## Architecture

Single Rust crate `shadows` with library + single binary (two modes: `serve` daemon + CLI client), plus an independent browser client. `shadows serve` serves the product locally but never opens a browser automatically.

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

**Winner: SQLx 0.9 only.** The delta validated SQLx and SeaORM 2.0.3 end-to-end against PostgreSQL 16.15; SQLx retained the edge and SeaQuery had no demonstrated use case. See `docs/evidence/persistence/DELTA_VALIDATION.md`.

### First runnable milestone

The first milestone is deliberately vertical: start `shadows serve`, manually open any browser, select a local project, create/resume a planning thread, run one real Claude Planner turn with live output, stop it with confirmed process-tree termination, restart the daemon, and recover the durable conversation. Workflow scheduling, verification, MCP, team sync, and the full proposed schema do not block this milestone.

### External tools

`gcode` is an optional external executable. Shadows may invoke it through the normal process/tool boundary when code search is added, but does not vendor or depend on `gobby-cli`, `gcore`, PostgreSQL, FalkorDB, or Qdrant. `ghook` and `gwiki` are not part of the first milestone.

## Rules (project-specific)

- **Library-first:** no custom ORM, no custom migration engine. Use Rust ecosystem crates.
- **Domain stays pure:** `domain/` types carry no persistence imports (`sea_orm`, `sqlx`, `sea_query`, `Row`, `Entity`, `ActiveModel`, `Pg*`, `Sqlite*`).
- **Ordering is explicit:** durable event sequence, thread-entry ordinal, or another explicit stable domain key. No `rowid`, physical insertion order, or implicit `SELECT` order.
- **DB-specific syntax isolation:** backend-specific SQL (FTS5, tsvector, `PRAGMA`, `rowid`, `strftime`) stays inside `storage/<backend>/`. Enforced by module boundaries, contract tests, and review — not by a keyword blacklist scanned across the tree (spec §2.9).
- **Idempotency:** mutating commands carry `CommandId`, command kind, schema version, and normalized request fingerprint. Replay requires fingerprint equality; mismatch is `CommandConflict`.
- **PLAN_BLOCKED = Operation outcome, NOT HTTP error.** Structured refusal, not transport failure.

## Decisions

The canonical spec owns complete architecture semantics. Four compact ADRs group the accepted decisions for navigation; they are summaries, not a second specification.
