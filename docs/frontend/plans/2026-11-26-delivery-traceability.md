# Delivery Traceability Web Plan

> **Execution:** Continue the [backend plan](../../backend/plans/2026-11-26-delivery-traceability.md)
> in its stated order. Each task has one scoped commit.

**Spec:** [§18.3 Delivery traceability](../../core/specs/2026-10-03-project-planning-workspace-design.md#delivery-traceability-approved-design-2026-11-26).
**Fields:** [Accepted shared request](../../shared/requests/2026-11-26-delivery-traceability.md).
**Stack:** React, TanStack Router, TanStack Query, existing openapi-typescript client and event stream.

## Existing modules and their roles

| Module | Reuse | Add |
|---|---|---|
| `web/src/api/design.ts` | Outcome/part reads and `editDesign` used by the saved editors | Use those editors as the destination from approval. |
| `web/src/api/workflows.ts` | `planQuery`, `planProgressQuery`, `approvePlan` | Consume the generated Plan fields and exact dependency progress. |
| `web/src/api/project-state.ts` | `projectStateQuery`, same project-state invalidation namespace | Add outcome selection and DTO aliases. |
| `web/src/app/design` | `RoadmapView`, `OutcomeEditor`, `PartsView` | Read-only delivery sections beside the existing editors. |
| `web/src/app/project-state` | State view, selected-part detail, plan and task links | Outcome-level summaries and the unique project total. |
| `web/src/stream` | Plan, design and task progress invalidations | Invalidate existing plan reads for a committed design association or layout change. |

**All tasks:** queries stay in `web/src/api/`; navigation uses existing route IDs; the
UI uses saved/read-only Core facts. Each touched folder's contract belongs in its commit.
Use `npm run typecheck`, `npm run lint`, `npm run build` and
`npx vitest run src/codemap.test.ts src/architecture` from `web/` for the final frontend
slice, with focused tests and typecheck in each preceding slice.

## Task 6 · F-150 · Show approval requirements and return links

**Why:** An exploratory draft should plainly show how to become approvable. Both the exact
plan version and each task need a path back to Roadmap and its owners.

**Needs:** B-240 and B-241 OpenAPI. Generate the web schema here once for both additions.

**Touch:** `web/src/api/{schema.d.ts,workflows.ts,contract.yaml}`,
`web/src/app/workflows/{approve-bar.tsx,plan-page.tsx,plan-page.test.tsx,contract.yaml}`,
`web/src/app/tasks/{task-page.tsx,details-tab.tsx,task-page.test.tsx,contract.yaml}`,
`web/src/stream/{refresh.ts,refresh.test.tsx,contract.yaml}`, `docs/frontend/stages.md`.

- [ ] In the existing plan page test, serve an Active Draft with `approval_missing: ['outcome']`.
  Assert the **Link a roadmap outcome** action resolves to
  `/projects/$projectId/workspace?view=roadmap`, and the plan uses the existing approve
  request. Once the refreshed record has an outcome, it links to that exact outcome and
  the missing-outcome action disappears. In the current task-page test serve two tasks and
  show only the exact task's read and write paths, owner links and inherited outcome.
- [ ] Run `npx vitest run src/app/workflows/plan-page.test.tsx src/app/tasks/task-page.test.tsx`.
  Expected RED at the missing links/requirements.
- [ ] Run `npm run gen:api`. Re-export `PlanDelivery` from the workflow API when needed.
  In the existing Approve bar render the requirements by key, with an actionable Roadmap
  link and a Project Map link for a missing part. Leave Core's authoritative refusal reachable
  through the existing Approve action. The normal outcome editor supplies the link write.
- [ ] Add the plan's outcome and affected-part links near its plan metadata. In the task Details
  tab receive the matching `TaskDelivery`, its owner-title references and the plan's outcomes.
  Display read and write scopes separately. Map links use `view=map&part=<id>`, outcome links
  use `view=roadmap&outcome=<id>`. An ownerless path is visible as unmapped.
- [ ] Extend the existing `ProjectDesignChanged` refresh path to invalidate plan detail reads
  as well, using the existing `invalidateWorkflows` target. Keep the existing editor/cache flow
  and assert a later related plan fetch observes the saved link. No added polling.
- [ ] Run the two focused tests, `npx vitest run src/stream/refresh.test.tsx`, and
  `npm run typecheck`. Expected GREEN. Commit `feat(web): surface delivery links on plans and tasks`.

## Task 7 · F-151 · Display related tasks in Roadmap, Map and State

**Why:** These screens should lead to the work they describe and agree about what is done.

**Needs:** B-241 and the generated schema from F-150.

**Touch:** `web/src/api/{project-state.ts,project-state.test.ts,contract.yaml}`,
`web/src/app/design/{roadmap-view.tsx,parts-view.tsx,roadmap-view.test.tsx,parts-view.test.tsx,contract.yaml}`,
`web/src/app/project-state/{state-view.tsx,state-view.test.tsx,contract.yaml}`,
`docs/frontend/stages.md`.

- [ ] Extend existing tests with a selected outcome showing one active approved plan and a
  Draft-only row, a selected map part with one write task and one read task, and a project
  State summary with a shared task. The approved task link uses its exact version and number.
  Toggling a result's data from started to done updates the badge without changing acceptance.
- [ ] Run `npx vitest run src/app/design/roadmap-view.test.tsx src/app/design/parts-view.test.tsx src/app/project-state/state-view.test.tsx`.
  Expected RED at the missing progress sections.
- [ ] Extend the state API options with `projectStateQuery(projectId, partId?, outcomeId?)` and
  add `outcomeId ?? null` after the existing part slot in the key. Keep all state selectors
  under the existing invalidation prefix. Test the exact query parameters and both namespaced
  keys. Treat additive absent fields as unavailable. Display Core's zero as zero.
- [ ] On a selected Roadmap outcome, query outcome State detail and render its approved tasks
  under their plan, with the link to the task (whose Report tab holds available execution
  evidence) and Runs status. Show a Draft-only link with a Draft label and no task count.
  Keep outcome acceptance items and existing archived plan links in OutcomeEditor. Keep the
  read section separate from unsaved editor inputs. Use existing skeleton and inline error
  components for pending/failure states.
- [ ] On a selected Map part, use the selected-part State detail, list its scoped tasks and
  render the access label as Reads, Writes, or Reads and writes. Reuse the official task URL
  `/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task`.
  No task-loading query for every part card.
- [ ] In State overview, render Core's `task_totals` once and list `outcomes` with Roadmap links.
  The selected-part State detail also shows each task's access label. Keep the same source
  for status in each view, use existing event refresh, and surface the API's remaining counts.
- [ ] Run the three focused tests, `npx vitest run src/api/project-state.test.ts` and
  `npm run typecheck`. Expected GREEN. Commit `feat(web): show related delivery progress`.

## Task 8 · F-152 · Read real dependency status in the plan graph

**Why:** In testing8 an already committed storage task appears as `Pending v1` beside the plan
that uses it. A dependency card needs Runs progress for the version displayed on the card.

**Needs:** F-150 and F-151.
**Touch:** `web/src/app/workflows/{linked-node.tsx,plan-graph.test.tsx,contract.yaml}`,
`docs/frontend/stages.md`.

- [ ] Extend the existing plan-graph test with a Frozen dependency whose preview still says
  Pending and whose existing plan-progress response marks the exact task `done: true`,
  `state: 'committed'`. Assert the card shows Done and the exact related plan version.
  Cover the existing broken-link case in the same suite.
- [ ] Run `npx vitest run src/app/workflows/plan-graph.test.tsx`. Expected RED at the stale
  preview label.
- [ ] In the dependency card, use existing `planProgressQuery(workflow_id, version)` only
  for a resolved, reachable Frozen version with an ID and version. Resolve the entry by
  its exact task number. All cards for the same version share the query key. A draft shows
  Draft, a missing endpoint keeps the broken reason, and an unavailable progress read
  shows status unavailable. A finished query with an existing task and no attempt shows
  Not started; a committed attempt with `done: false` shows Committed. Keep the task
  progress invalidation from `TaskProgressChanged`.
- [ ] Run the graph test and F-150/F-151 tests again. Run `npm run typecheck`,
  `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture`.
  Expected GREEN. Commit `fix(web): show execution status on linked task cards`.
