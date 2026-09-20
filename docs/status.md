# Project Status

_Created via /mxInitProject · Updated via /mxSave 2026-09-20_

## Implemented Features

- (none yet — architecture phase only)

## Architecture Design

| Section | Topic | Status |
|---|---|---|
| 2 | Architecture overview (15 modules, ownership rules) | ✅ Approved |
| 3.1 | Data Flow (12 flows, including MCP) | ✅ Approved (after 9 corrections) |
| 3.2 | Error Handling (AppFailure, OperationOutcome, idempotency, retry) | ✅ Approved (after 4 fixes) |
| 3.2.1 | Dependency / Library Strategy | ✅ Approved |
| 3.3 | Testing Strategy (10 layers, CI shape) | ✅ Approved |
| 4.1 | Core Domain Model (entities, IDs, states) | ✅ Approved |
| 4.2 | Relationships + Invariants (shadow lessons applied) | ✅ Approved (after 8 fixes + Persistence Ordering invariant) |
| 4.3 | Persistence delta (SeaORM 2.0.3 vs SQLx 0.9) | ✅ Completed — winner: SQLx only |
| 4.4 | Capability-oriented persistence API | ✅ Canonical candidate updated |
| 5 | SQLite schema proposal | ✅ Canonical candidate updated; DDL pending |

## Persistence Spike Outcome

- **Decision:** SQLx 0.9 only; no SeaQuery until a demonstrated dynamic-query use case.
- **Evidence:** `docs/evidence/persistence/DELTA_VALIDATION.md`
- **PostgreSQL parity:** SQLx 0.9 and SeaORM 2.0.3 exercised end-to-end on PostgreSQL 16.15.
- **Concurrent migrations:** SQLx 4/4 successful; SeaORM 1/4 successful without extra serialization.
- **Domain isolation:** verified; persistence types remain outside domain contracts.
- **Toolchain consequence:** Rust 1.94 is the current dependency floor from SQLx 0.9.

## Decisions

The authoritative consolidated ADR set is local in `docs/decisions/`.
Resolve decisions by title/topic rather than historical pre-consolidation numbers.
Workflow lineage, operation recovery, process containment, and cancellation ADRs
were aligned with the canonical spec on 2026-09-20.

## Open Items

- [ ] Write SQLx migration DDL from the accepted SQLite schema proposal.
- [ ] Run production-like file-backed SQLite WAL concurrency validation.
- [ ] Write the implementation plan with layered readiness milestones.
- [ ] Implement only after the plan review gate.

## Future Direction

- `docs/future/shadows-team-direction.md` records a non-binding direction for a
  separate PostgreSQL-backed team/organization service while `shadows` remains
  a standalone SQLite local engine.
- Team Server, synchronization, leases, offline collaboration, and shared
  protocol extraction remain explicitly outside local v1.

## Architecture Shadow Lessons Applied

From `E:\Globalprojects\shadow` (predecessor project, NOT part of shadows):

- R14 (FTS5 in domain crate) → DB-specific syntax isolation rule
- R14 (rowid ordering) → `durable_seq` explicit ordering invariant
- R14 (SQLite migration authority) → migration authority lives in `storage/sqlite/`
- R11 (Windows/Linux split defects) → process group / Job Object semantics (cancellation)
- R13 (no way to clear proposed memory) → Memory module deferred to v2 (out of scope for v1)
- 24-entry PROCESS_AUTHORITY allowlist → use module visibility + strict ownership, no allowlist

## Archived Evidence

Persistence comparison reports are retained under `docs/evidence/persistence/`.
The throwaway implementations were removed after the decision and remain
recoverable from Git history.
