# Shadows

> **AI Start Here** — Read these files to get started:
>
> | Document | Purpose |
> |----------|---------|
> | [CLAUDE.md](./CLAUDE.md) | Architecture, conventions, rules (this file) |
> | [docs/superpowers/specs/shadows-spec.md](./docs/superpowers/specs/shadows-spec.md) | **The design and decision source. There is no second one.** |
> | [docs/status.md](./docs/status.md) | Where the project is right now. Decides nothing. |
> | [docs/evidence/](./docs/evidence/) | Dated measurement records. Facts, not decisions. |
>
> **Documentation rules:**
> - The spec is amended **in place**. Never revise a decision by adding a
>   second document, and never summarise it into a parallel file that can drift.
> - Do not create an ADR, a design note, or a plan document for a struct shape,
>   a library call, a test correction, or an implementation detail. Amend the spec.
> - A question the project cannot answer yet is an **OPEN** block inside the spec,
>   at the point it bites, naming the trigger that closes it. An OPEN block with
>   no trigger is rot, not a question.
> - Evidence files are dated facts. They never expire and are never design
>   authority. When a measurement changes a decision, change the decision in the spec.
> - Earlier specs, the runtime draft, and the four consolidated ADRs were absorbed
>   into the spec and deleted. They remain in Git history and are not
>   active references.
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

[`docs/superpowers/specs/shadows-spec.md`](./docs/superpowers/specs/shadows-spec.md) owns the complete architecture semantics and is the only place a decision lives. The rules above are a working summary of what it says about module boundaries; where this file and the spec disagree, the spec is right and this file is the defect.
