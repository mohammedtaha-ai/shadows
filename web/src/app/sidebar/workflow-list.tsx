// One job: the open project's plans in the sidebar — active plans first,
// then folded archived ones (§13.11, §16.8).

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { ChevronDown, ChevronRight, Workflow } from 'lucide-react'
import { useState } from 'react'
import type { PlanListing } from '@/api/client'
import { plansQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { stateLabel } from '../workflows/plan-state'

export function WorkflowList({ projectId, selected }: { projectId: string; selected?: string }) {
  const { data: plans, error } = useQuery(plansQuery(projectId, true))
  const [showArchived, setShowArchived] = useState(false)

  const activePlans = plans?.filter((p) => p.plan_state === 'Active') ?? []
  const archivedPlans = plans?.filter((p) => p.plan_state === 'Archived') ?? []

  return (
    <div className="mt-1 mb-1.5 ml-4 border-l border-sidebar-border pl-2">
      <p className="px-2 pt-1 pb-0.5 text-[11px] tracking-wide text-faint-foreground uppercase">
        Workflows
      </p>
      {error !== null && (
        <div className="px-2 py-1">
          <ErrorLine error={error} />
        </div>
      )}
      {plans !== undefined && plans.length === 0 && (
        <p className="px-2 py-1 text-xs text-faint-foreground">
          No plans yet. Ask the Planner for a plan in a conversation.
        </p>
      )}
      <ul className="space-y-0.5">
        {activePlans.map((plan) => (
          <PlanRow key={plan.id} plan={plan} projectId={projectId} selected={selected} />
        ))}
      </ul>
      {archivedPlans.length > 0 && (
        <div className="pt-1">
          <button
            type="button"
            onClick={() => setShowArchived((v) => !v)}
            aria-expanded={showArchived}
            className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-xs text-faint-foreground transition-colors hover:bg-sidebar-accent/40 hover:text-sidebar-foreground"
          >
            {showArchived ? (
              <ChevronDown className="size-3 shrink-0 transition-transform duration-150" aria-hidden />
            ) : (
              <ChevronRight className="size-3 shrink-0 transition-transform duration-150" aria-hidden />
            )}
            <span>Archived ({archivedPlans.length})</span>
          </button>
          {showArchived && (
            <ul className="mt-0.5 space-y-0.5">
              {archivedPlans.map((plan) => (
                <PlanRow key={plan.id} plan={plan} projectId={projectId} selected={selected} />
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}

function PlanRow({
  plan,
  projectId,
  selected,
}: {
  plan: PlanListing
  projectId: string
  selected?: string
}) {
  const isSelected = plan.id === selected
  return (
    <li>
      <Link
        to="/projects/$projectId/workflows/$workflowId"
        params={{ projectId, workflowId: plan.id }}
        aria-current={isSelected ? 'page' : undefined}
        className={`group flex items-center gap-2 rounded-r-md border-l-2 px-2 py-1 text-sm transition-all duration-150 ${
          isSelected
            ? 'border-accent-line bg-secondary/80 font-medium text-secondary-foreground shadow-xs'
            : 'border-transparent text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'
        }`}
      >
        <Workflow
          className={`size-3.5 shrink-0 transition-colors ${
            isSelected ? 'text-accent-line' : 'group-hover:text-sidebar-foreground'
          }`}
          aria-hidden
        />
        <span dir="auto" className="min-w-0 flex-1 truncate text-start">
          {plan.title}
        </span>
        <span className="shrink-0 text-[11px] text-faint-foreground">{stateLabel(plan)}</span>
      </Link>
    </li>
  )
}
