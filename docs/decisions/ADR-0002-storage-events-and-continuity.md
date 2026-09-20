# ADR-0002 — Storage, events, and continuity

- **Status:** accepted
- **Date:** 2026-09-20
- **Evidence:** `docs/evidence/persistence/DELTA_VALIDATION.md`
- **Detail:** canonical design specification, Sections 2.4, 2.9, 4, and 5

## Decisions

- Use SQLx 0.9 with SQLite for local v1. Use SQLx migrations; do not create a
  migration framework, generic database trait, repository-per-entity pattern,
  or public transaction DSL.
- Keep SQL, backend types, FTS5, PRAGMA, locking, and backend-specific behavior
  inside `storage/`.
- Current entity tables are authoritative. The durable journal records history,
  provenance, and resync; Shadows is not event-sourced.
- Commit a state mutation, its durable event, and an external command record in
  one transaction when they form one logical command.
- Domain ordering uses explicit keys such as sequence or ordinal, never `rowid`
  or implicit insertion order.
- Publish durable events to the live bus only after commit. Transient output may
  be lost across disconnect; durable state must be recoverable without gaps.
- Native harness session IDs are optional metadata and never continuity truth.

## Scope constraint

The first runnable milestone migrates only the tables required for projects,
planning threads, entries, operations, command idempotency, and durable event
recovery. Later tables are added with the feature that needs them.

