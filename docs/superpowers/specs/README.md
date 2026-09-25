# Shadows Design Specifications

- **Date:** 2026-09-21
- **Status:** Accepted architecture baseline. Delivery begins with the first
  runnable browser Planner vertical slice; the wider schema and later
  subsystems are not prerequisites for that slice.
- **Slug:** `shadows`
- **Stack:** Rust **1.94+** minimum for the selected SQLx 0.9 line; development
  validation performed on Rust 1.96. Single Rust crate, library + binary, plus
  an independent browser client.
- **Purpose:** Local-first AI software-delivery orchestration runtime for
  planning, workflow, context, execution, verification, and durable continuity
  across interchangeable agent harnesses.

This directory is the single authoritative design and decision source. It is
split by topic so each decision has one owner without recreating parallel ADR,
design-note, and status narratives.

## Ownership map

| Sections | Owner |
|---|---|
| §0 | [`2026-09-21-principles-design.md`](./2026-09-21-principles-design.md) |
| §1 | [`2026-09-21-architecture-design.md`](./2026-09-21-architecture-design.md) |
| §2 | [`2026-09-21-data-flow-design.md`](./2026-09-21-data-flow-design.md) |
| §3 | [`2026-09-21-errors-and-testing-design.md`](./2026-09-21-errors-and-testing-design.md) |
| §4 | [`2026-09-21-domain-model-design.md`](./2026-09-21-domain-model-design.md) |
| §5, §7 | [`2026-09-21-persistence-api-and-migrations-design.md`](./2026-09-21-persistence-api-and-migrations-design.md) |
| §6 | [`2026-09-21-sqlite-schema-design.md`](./2026-09-21-sqlite-schema-design.md) |
| §8 | [`2026-09-21-runtime-design.md`](./2026-09-21-runtime-design.md) |
| §9–§11 | [`2026-09-21-rules-scope-and-milestones-design.md`](./2026-09-21-rules-scope-and-milestones-design.md) |
| §12 | [`2026-09-24-harness-controls-design.md`](./2026-09-24-harness-controls-design.md) |
| §13 | [`2026-09-25-planner-workflow-design.md`](./2026-09-25-planner-workflow-design.md) |

## Maintenance rules

- Ordinary prose is **decided**. Implement it.
- An **OPEN** block is a question the project cannot answer yet. It must name
  the trigger that closes it and why it does not block current work.
- Every decision lives in exactly one owner file. Another file refers to its
  section number instead of copying or summarising it.
- Amend an owner file in place. Do not create a competing ADR or design note.
- `docs/status.md` records current progress and decides nothing.
- `docs/evidence/` contains dated measurements, not design authority. When a
  measurement changes a decision, amend the appropriate owner file.
- Do not create a design document for a struct shape, library call, test
  correction, or ordinary implementation detail.
- Keep the section numbers stable across files so existing `§` references stay
  valid.
