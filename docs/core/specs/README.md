# Shadows Design Specifications

- **Date:** 2026-09-21
- **Status:** Accepted architecture baseline. Milestones 0 to 2.5 and the later
  slices of §12 to §24 are merged on `main`. What is not built is in
  [`docs/core/future.md`](../future.md).
- **Slug:** `shadows`
- **Stack:** Rust **1.94+** minimum for the selected SQLx 0.9 line; development
  validation performed on Rust 1.96. A Cargo workspace (§14.3), plus
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
| §1 | [`2026-09-21-architecture-design.md`](./2026-09-21-architecture-design.md); §1.7 owns personal/team authority, runtime placement and synchronization |
| §2 | [`2026-09-21-data-flow-design.md`](./2026-09-21-data-flow-design.md) |
| §3 | [`2026-09-21-errors-and-testing-design.md`](./2026-09-21-errors-and-testing-design.md) |
| §4 | [`2026-09-21-domain-model-design.md`](./2026-09-21-domain-model-design.md) |
| §5, §7 | [`2026-09-21-persistence-api-and-migrations-design.md`](./2026-09-21-persistence-api-and-migrations-design.md) |
| §6 | [`2026-09-21-sqlite-schema-design.md`](./2026-09-21-sqlite-schema-design.md) |
| §8 | [`2026-09-21-runtime-design.md`](./2026-09-21-runtime-design.md) |
| §9–§11 | [`2026-09-21-rules-scope-and-milestones-design.md`](./2026-09-21-rules-scope-and-milestones-design.md) |
| §12 | [`2026-09-24-harness-controls-design.md`](./2026-09-24-harness-controls-design.md) |
| §13 | [`2026-09-25-planner-workflow-design.md`](./2026-09-25-planner-workflow-design.md) |
| §14 | [`2026-09-26-application-core-design.md`](./2026-09-26-application-core-design.md) |
| §15 | [`2026-09-30-code-index-design.md`](./2026-09-30-code-index-design.md) |
| §16 | [`2026-10-01-project-plans-design.md`](./2026-10-01-project-plans-design.md) |
| §17 | [`2026-10-01-task-contracts-and-evidence-design.md`](./2026-10-01-task-contracts-and-evidence-design.md); §17.12 records future production operations |
| §18 | [`2026-10-03-project-planning-workspace-design.md`](./2026-10-03-project-planning-workspace-design.md) — Stages 1–2 built; Stage 3 is in [`docs/core/future.md`](../future.md) |
| §19 | [`2026-10-03-agent-profiles-and-extensions-design.md`](./2026-10-03-agent-profiles-and-extensions-design.md) — draft; Planner and executor profiles not built; AYLUL parts are owned by the AYLUL specs (§19.4, §19.5) |
| §20 | [`2026-10-05-message-queue-design.md`](./2026-10-05-message-queue-design.md) — the queue and Send now (built) |
| §21 | [`2026-10-05-slash-menu-design.md`](./2026-10-05-slash-menu-design.md) — the `/` menu (built) |
| §22 | [`2026-10-05-subagent-cards-design.md`](./2026-10-05-subagent-cards-design.md) — subagent cards and their side panel (built) |
| §23 | [`2026-10-08-guided-planning-design.md`](./2026-10-08-guided-planning-design.md) — accepted; PRs 0–5 merged except PR 3c (§23.9), which waits on §24.3's OPEN |
| §24 | [`2026-10-08-approval-authority-design.md`](./2026-10-08-approval-authority-design.md) — principal and agent; enforced by §23.9 PR 3b (merged) |

## Maintenance rules

Delivered work is recorded in its PR and dated evidence; what is not built yet is in
[`docs/core/future.md`](../future.md).
The [full delivery roadmap](../roadmap.md)
retains the later local product, concurrency and company sequence.

- Within an accepted spec, ordinary prose is **decided**. Implement it.
  A file marked Draft needs its written-spec review before implementation.
- An **OPEN** block is a question the project cannot answer yet. It must name
  the trigger that closes it and why it does not block current work.
- Every decision lives in exactly one owner file. Another file refers to its
  section number instead of copying or summarising it.
- Amend an owner file in place. Do not create a competing ADR or design note.
- `docs/core/future.md` lists what is not built yet and decides nothing.
- `docs/evidence/` contains dated measurements, not design authority. When a
  measurement changes a decision, amend the appropriate owner file.
- Do not create a design document for a struct shape, library call, test
  correction, or ordinary implementation detail.
- Keep the section numbers stable across files so existing `§` references stay
  valid.
