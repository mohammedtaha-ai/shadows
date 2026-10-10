# Shadows Design Specifications

- **Date:** 2026-09-21
- **Status:** Accepted architecture baseline. Delivery begins with the first
  runnable browser Planner vertical slice; the wider schema and later
  subsystems are not prerequisites for that slice.
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
| §14 | [`2026-09-26-application-core-design.md`](./2026-09-26-application-core-design.md) |
| §15 | [`2026-09-30-code-index-design.md`](./2026-09-30-code-index-design.md) |
| §16 | [`2026-10-01-project-plans-design.md`](./2026-10-01-project-plans-design.md) |
| §17 | [`2026-10-01-task-contracts-and-evidence-design.md`](./2026-10-01-task-contracts-and-evidence-design.md) |
| §18 | [`2026-10-03-project-planning-workspace-design.md`](./2026-10-03-project-planning-workspace-design.md) — draft for review |
| §19 | [`2026-10-03-agent-profiles-and-extensions-design.md`](./2026-10-03-agent-profiles-and-extensions-design.md) — draft profiles, specialist dispatch and extensions |
| §20 | [`2026-10-05-message-queue-design.md`](./2026-10-05-message-queue-design.md) — the queue and Send now; merged through PR #19 (2026-10-05) |
| §21 | [`2026-10-05-slash-menu-design.md`](./2026-10-05-slash-menu-design.md) — the `/` menu; merged through PR #21 (2026-10-05) |
| §22 | [`2026-10-05-subagent-cards-design.md`](./2026-10-05-subagent-cards-design.md) — subagent cards and their side panel; merged through PR #22 (2026-10-08) |
| §23 | [`2026-10-08-guided-planning-design.md`](./2026-10-08-guided-planning-design.md) — accepted in conversation; owner-file review remains noted there. PRs 0–1 merged through PRs #23–#24; PR 2 is next (§23.9) |
| §24 | [`2026-10-08-approval-authority-design.md`](./2026-10-08-approval-authority-design.md) — principal and agent; accepted in conversation, owner-file review and real-adapter probe remain open. Enforcement is planned for §23.9 PR 3b |
| — | [`2026-10-04-structure-hygiene-design.md`](./2026-10-04-structure-hygiene-design.md) — implemented and Windows-verified; independent review pending |

## Maintenance rules

Delivery and acceptance details live in [`docs/status.md`](../../status.md).
The [full delivery roadmap](../plans/2026-10-03-project-planning-roadmap.md)
retains the later local product, concurrency and company sequence.

- Within an accepted spec, ordinary prose is **decided**. Implement it.
  A file marked Draft needs its written-spec review before implementation.
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
