# Cross-plan Links (1b) Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline. Mohammed authorized completing the local vision on 2026-10-04; retain the standing instruction not to dispatch another reviewer automatically.

**Goal:** A task can depend on another plan's latest task, inspect that dependency in either graph, and navigate a project map without confusing task links with pinned API agreements.

**Architecture:** Plans owns the target value, resolved dependency views and approval rules. Its store owns migration 0016 and snapshot-consistent latest-version resolution. Existing petgraph SCC analysis detects cycles; HTTP/MCP translate and the browser reuses React Flow/Dagre.

**Tech Stack:** Existing Rust/SQLx/petgraph, generated OpenAPI, React Query/React Flow/Dagre. No new dependency for graph algorithms.

**Spec:** [§16.7–§16.12](../specs/2026-10-01-project-plans-design.md).

## Global constraints

- A link target names a plan/task, never a version; resolve its latest version.
- Source and target share a project, or the source project has the existing one-way project link to the target project.
- Unrelated and removed projects cannot be read through a grant. Linked-project grant access is read-only.
- Broken links remain readable and block Draft approval; Frozen content remains unchanged.
- `needs` and `completes_after` both count for cross-plan cycles.
- Preserve local-number wire format and old command fingerprints/digests.
- SQL stays in stores; public rules/value types carry no database imports.
- Use migration 0016; shared agreements begin at 0017.
- Format and Unicode-width checks, both clippy modes, full Rust/Web suites, generated API consistency and build are the commit gate.

## Review focus

- Removing or renaming a target task in a newer Draft changes resolution, not a Frozen source link's bytes.
- Duplicate command replay must not write another link or journal event.
- A newly created cross-plan edge cannot hide a multi-plan cycle from approval's transaction.
- Incoming metadata must not disclose a project outside the caller's grant reach.
- Removing a one-way project link breaks the dependency and revokes linked-project reads immediately.
- The map must show relevant external plans without silently dropping broken edges or archived references.

## Ownership

| File | Job |
|---|---|
| `plans/dependencies.rs` | cross-plan dependency value shapes |
| `plans/model.rs`, `ops.rs`, `rules.rs` | existing plan content and pure edit validation |
| `plans/store/dependencies.rs` | persisted cross-plan rows and resolution |
| `plans/store/graph.rs` | snapshot dependency graph for approval/map |
| `code/store/links.rs` | existing project-link reach shared with Plans under its contract |
| `shadows-http/workflow.rs` | plan-map route translation |
| `shadows-mcp/tools.rs` | optional linked-project read argument translation |
| Web workflow graph/map modules | navigate task dependencies and project plans |

## Task 1: Target identity and persistence

- [ ] Add `cross_plan_links.rs`: deserialize a local `after: 3` and an external `{plan_id, task}`; round-trip both without changing the local JSON. Watch external deserialization fail before changing the type.
- [ ] Add pure `TaskParent` (untagged local number/external plan-task), reused by `Link` and `LinkRemove`; keep changed task numbers source-local.
- [ ] Add migration 0016 cross-link rows tied to the source version/task, with explicit ordering/uniqueness and Frozen write guards.
- [ ] Extend existing content load/write/draft-copy paths. Local rows stay in their existing table; cross-plan links stay separate and target missing tasks can remain broken.
- [ ] Test real storage round-trip, whole-batch refusal, idempotent replay, copied Draft and database-level Frozen guards. Old Frozen rows remain byte-identical.
- [ ] Update Plans contract and code map, run the full gate, commit this independently testable slice.

## Task 2: Resolution, reach and approval

- [ ] Test target latest-version changes, absent tasks, project unlink/removal and source-local approval blockers using actual services.
- [ ] Resolve dependency metadata in a read snapshot; expose outgoing/incoming views without mutating plan content.
- [ ] Reuse the Code store's declared project-link reach, with a contract entry for the shared transaction reader. Grant reads may target a directly linked project; all mutations stay in the grant's own project.
- [ ] Test a three-task cross-plan cycle spanning both link kinds and a valid acyclic graph. Use petgraph SCCs with stable problem ordering.
- [ ] Approval recomputes blockers inside its write transaction. Replays keep their first durable outcome.
- [ ] Test foreign-project metadata isolation and immediate denial after unlink. Run the full gate and commit.

## Task 3: HTTP/MCP and project map

- [ ] Add `Plans::map(project)` returning current Active plan nodes plus referenced external plan nodes and counted dependency edges. Graph values remain transport-neutral.
- [ ] Add `GET /api/projects/{id}/plan-map`, generated OpenAPI/type updates and exact route-list assertions.
- [ ] Extend MCP workflow list/get with optional project slug using the same service reach policy, preserving current calls without it.
- [ ] Integration tests prove returned nodes/edges, inbound/outbound navigation, linked reads and refusal of linked writes.
- [ ] Update service contracts and code map; run the full gate and commit.

## Task 4: Browser graph, map and Windows acceptance

- [ ] Add sidebar Map navigation and `/projects/$projectId/map` using existing React Flow/Dagre.
- [ ] Draw resolved remote task nodes and broken-link explanations in each plan graph; navigation opens the target/source plan.
- [ ] Reuse project events and query invalidation. Include referenced project events when a displayed graph depends on them; close subscriptions on navigation/unmount.
- [ ] UI tests exercise external task navigation, broken-link rendering, map navigation and updates after target changes.
- [ ] Start disposable daemon/Web instances, create two plans, link their tasks, continue the target, observe a broken link and refused approval, restart and inspect retained data/logs.
- [ ] Run the complete gate, record exact Windows evidence and remaining untested cases, commit.

## Completion boundary

Finishing 1b advances the full objective; it does not complete agreements,
context compilation, execution, manager, diagrams or local release operations.
Next: Stage 2 shared API agreements under §18, with its separate detailed plan.
