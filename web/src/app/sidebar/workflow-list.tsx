// One job: the open project's plans in the sidebar — each conversation's
// latest version with its state (§13.11).

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { Workflow } from 'lucide-react'
import { plansQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { stateLabel } from '../workflows/plan-state'

export function WorkflowList({ projectId, selected }: { projectId: string; selected?: string }) {
  // Polled every 10 s while shown: no stream carries the list (§13.10).
  const { data: plans, error } = useQuery(plansQuery(projectId))

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
        {plans?.map((plan) => {
          const isSelected = plan.id === selected
          return (
            <li key={plan.id}>
              <Link
                to="/projects/$projectId/workflows/$workflowId"
                params={{ projectId, workflowId: plan.id }}
                aria-current={isSelected ? 'page' : undefined}
                className={`flex items-center gap-2 rounded-r-md border-l-2 px-2 py-1 text-sm transition-colors ${
                  isSelected
                    ? 'border-accent-line bg-secondary text-secondary-foreground'
                    : 'border-transparent text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
                }`}
              >
                <Workflow className="size-3.5 shrink-0" aria-hidden />
                <span dir="auto" className="min-w-0 flex-1 truncate">
                  {plan.title}
                </span>
                <span className="shrink-0 text-[11px] text-faint-foreground">{stateLabel(plan)}</span>
              </Link>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
