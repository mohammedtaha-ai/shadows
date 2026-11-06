# Task phases

- Status: accepted
- Asked by: controller implementing both sides under Mohammed’s instruction, 2026-11-07
- Owner specification: `docs/backend/specs/2026-11-05-structure-and-depth-design.md` §5.5 and §6.2

## Need

Show ordered phases in a task view and optionally in the plan graph, with exact approved-version foreign waits. Execution status remains deferred.

## Fields

- TaskContent gains optional/default-empty `phases`. Empty lists are omitted on serialization to preserve existing fingerprints.
- Phase: opaque server `id`, server-derived contiguous `number`, `title`, `goal`, acceptance item numbers and `waits_for`.
- Phase refs use `kind: local` with `task, phase`, or `kind: plan` with `plan_id, version, task, phase`. Version is a positive i64 and must name a Frozen version in this project.
- `phases_set` replaces one task’s ordered list using optional IDs for existing/new phases. It is atomic and conflicts with another edit to that task in one batch.
- Plan gains derived `linked_phases` previews: exact `source` metadata, raw `reference`, nullable exact `target` metadata and nullable `broken` reason. Missing metadata is never invented. This is display data, excluded from PlanContent.
- MCP `task_context` gains optional `phase_id`; task acceptance remains complete and dependency pins never follow latest versions.

## Errors

Existing error transport. Unknown/duplicate IDs, invalid acceptance, missing/unapproved/foreign pins and mixed cycles are named refusals. Removed target versions remain named blockers; no fabricated metadata.

## Backend answer

Accepted: Plans owns phase values, persistence, edit validation, projections, graph and context. Design owns layout. Adapters translate service APIs. Regenerate OpenAPI and web schema at each serialized change.

## Frontend reply

Accepted by the controller for both sides under the explicit instruction to refine and execute these plans on this branch. Reuse API transport, query hooks and graph conventions; preserve navigation, selection, filters and viewport. Display no completion state.
