# Delivery traceability

Status: accepted

**Owner spec** §18.3 [Delivery traceability](../../core/specs/2026-10-03-project-planning-workspace-design.md#delivery-traceability-approved-design-2026-11-26).

## Need

The user follows a roadmap result into its actual plans and tasks, traces a task
back to the relevant outcome and parts, and sees real attempt state. They can
attach a result to an exploratory draft through the existing Roadmap editor.

## Contracts

Use existing requests with additive fields. A `plan_id` is stable across versions and
`workflow_id` selects exactly one version. A task link uses the version's
`workflow_id`, numeric `version` and task `number` together.

| Request | Addition |
|---|---|
| `GET /api/workflows/{id}`, MCP `workflow_get` | `delivery?: PlanDelivery` for the requested version |
| MCP `task_context`, attempt brief and MCP `task_brief` | `TaskContext.outcomes?: DeliveryOutcome[]`, `TaskContext.ownership?: TaskOwnership` |
| `GET /api/projects/{id}/state`, Planner MCP `project_state` | Accept optional `outcome_id` alongside the existing selector. Send at most one selector. |
| Existing State response | `outcomes?: OutcomeProgress[]`, `outcomes_more?: number`, `task_totals?: TaskCounts`, `outcome?: OutcomeDelivery` |
| Existing part detail | `plans[i].tasks` are scoped to the selected part; each relevant `TaskLine` includes `access?: ScopeAccess` |
| Existing plan approval and design edit | `STAGE_BLOCKED` with an actionable missing-part/outcome or final-unlink refusal |

```text
DeliveryOutcome { outcome_id: string, title: string }
DeliveryPart { part_id: string, title: string }
ScopeOwner { part_id: string, via: "Folder" | { Layer: string } }
ScopePath { path: string, owners: ScopeOwner[] }
TaskOwnership { reads: ScopePath[], writes: ScopePath[] }
TaskDelivery { number: number, ownership: TaskOwnership }
DeliveryRequirement = "part" | "outcome"
PlanDelivery {
  map_revision: number, outcomes: DeliveryOutcome[], parts: DeliveryPart[],
  task_parts: DeliveryPart[], tasks: TaskDelivery[], approval_missing: DeliveryRequirement[]
}
ScopeAccess = "read" | "write" | "read_write"
OutcomeProgress { outcome_id: string, title: string, tasks: TaskCounts }
OutcomePlanProgress { plan: PlanProgress, tasks: TaskLine[] }
OutcomeDelivery { outcome_id: string, title: string, plans: OutcomePlanProgress[],
                  plans_more: number, tasks: TaskCounts }
```

- `PlanDelivery.parts` are all effective affected parts. `task_parts` names read/write owners
  from the exact version for task links. A scope path with an empty owner list is unmapped.
  `approval_missing` is empty outside an Active Draft.
- A newly served project State supplies the outcome summary, remaining count and totals,
  including empty lists or zero counts. The fields are additive for compatibility.
  Summary lists outcomes referenced by Active plans. It is capped at 100 by outcome ID,
  with the remainder reported. A selected saved outcome may have zero plans.
- An outcome detail includes at most the existing detail cap of 50 Active plans, in the
  established Plans order. Draft-only rows have the existing `PlanProgress.draft` ID, null
  live identity and an empty task list. Archived and manually linked historical plans
  remain accessible through the existing Outcome Editor list.
- Count exact live tasks before applying display caps. The same task can contribute to two
  outcomes, with one contribution to the project total. An unapproved newer draft adds no
  live tasks. `TaskLine.done` and `attempt` come from Runs.
- A foreign or missing outcome is refused through Design. Both selectors in the same
  call yield `INVALID_COMMAND` (HTTP 422). Existing part, gap and element semantics apply.
- Existing `TaskProgressChanged`, `ProjectDesignChanged` and plan lifecycle events refresh
  reads. The plan read must also refresh on workspace design edits that affect associations
  or layout. Use the current web transport and event manager.

## Backend answer

Accepted with an additive read projection and the existing workspace write. Plans and Design
retain ownership of links, Runs retains ownership of task progress, and ProjectState composes
the read. No new event or task write is required. Outcome acceptance still uses the outcome's
acceptance items and evidence in the related task reports.

## Frontend reply

Accepted. Start from the existing Approve bar, Roadmap outcome selection, Part details and
State view. Render Core's reads and keep stored links editable through the existing editor.
Task cards use Runs progress and exact version links. Keep labels for task progress distinct
from result acceptance.
