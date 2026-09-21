# Shadows

> **AI Start Here** — Read these files to get started:
>
> | Document | Purpose |
> |----------|---------|
> | [CLAUDE.md](./CLAUDE.md) | Architecture, conventions, rules (this file) |
> | [docs/superpowers/specs/README.md](./docs/superpowers/specs/README.md) | **Index of the authoritative design sections and their owners.** |
> | [docs/status.md](./docs/status.md) | Where the project is right now. Decides nothing. |
> | [docs/evidence/](./docs/evidence/) | Dated measurement records. Facts, not decisions. |
>
> **Documentation rules:**
> - The specs are split by topic. Every decision has exactly one owner file and
>   is amended **in place**. Other files refer to it; they do not copy it.
> - Do not create an ADR, a design note, or a plan document for a struct shape,
>   a library call, a test correction, or an implementation detail. Amend its owner spec.
> - A question the project cannot answer yet is an **OPEN** block inside its owner spec,
>   at the point it bites, naming the trigger that closes it. An OPEN block with
>   no trigger is rot, not a question.
> - Evidence files are dated facts. They never expire and are never design
>   authority. When a measurement changes a decision, amend its owner spec.
> - **Spike code is deleted once its evidence file is written.** A probe's job is
>   to produce an answer, not a codebase. Its raw output moves into
>   `docs/evidence/`; its source stays in Git history and the evidence file names
>   the commit. There is no `sandbox/` directory and no throwaway crate anywhere in
>   the tree — anything you find under `src/` is the product. Do not keep a spike
>   around because it might be useful later; that is how a rewrite acquires a second
>   codebase nobody maintains.
> - Earlier specs, the runtime draft, and the four consolidated ADRs were absorbed
>   into the topic specs and deleted. They remain in Git history and are not
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
- **A file earns its size.** At **300 lines** a file stops being free: the change that
  pushes it over states, in its commit message or report, what that file's single
  responsibility is and why the new code shares it. At **500 lines** it splits. Split by
  responsibility, never by line count — a `helpers.rs` or `utils.rs` is the same pile
  under a new name, and so is a split that leaves two files that must be read together.
  The test: name each resulting file's one job in a short phrase without using "and".
  These thresholds are a trigger for judgment, not a lint; a 520-line file with one
  genuine responsibility survives review by saying so.
- **Known accretion points.** Three files take a change from nearly every task, so they
  rot first and must be watched by name: `storage/mod.rs` (the facade — one capability
  per task), `protocol/` (one route per feature, forever), and `tests/storage_contract.rs`
  (four tasks append to it in Milestone 0 alone). When one of them grows, the split goes
  by domain — `storage/sqlite/<entity>.rs` already does this — not into a second facade.
- **Domain stays pure:** `domain/` types carry no persistence imports (`sea_orm`, `sqlx`, `sea_query`, `Row`, `Entity`, `ActiveModel`, `Pg*`, `Sqlite*`).
- **Ordering is explicit:** durable event sequence, thread-entry ordinal, or another explicit stable domain key. No `rowid`, physical insertion order, or implicit `SELECT` order.
- **DB-specific syntax isolation:** backend-specific SQL (FTS5, tsvector, `PRAGMA`, `rowid`, `strftime`) stays inside `storage/<backend>/`. Enforced by module boundaries, contract tests, and review — not by a keyword blacklist scanned across the tree (spec §2.9).
- **Idempotency:** mutating commands carry `CommandId`, command kind, schema version, and normalized request fingerprint. Replay requires fingerprint equality; mismatch is `CommandConflict`.
- **PLAN_BLOCKED = Operation outcome, NOT HTTP error.** Structured refusal, not transport failure.

## Decisions

[`docs/superpowers/specs/README.md`](./docs/superpowers/specs/README.md) maps every design section to its sole owner file. Together those owner files hold the complete architecture semantics. The rules above are a working summary; where this file and an owner spec disagree, the owner spec is right and this file is the defect.
