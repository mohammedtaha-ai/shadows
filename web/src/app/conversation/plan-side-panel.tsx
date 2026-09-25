// One job: a plan version in a panel beside the conversation (§13.9), opened
// only by a live `plan-show` for this tab.

import { X } from 'lucide-react'
import type { Plan, PlanTask } from '@/api/client'
import { ErrorLine } from '../error-line'
import { PlanGraph } from '../workflows/plan-graph'
import { usePlan } from '../workflows/use-plan'
import { PlanHeading } from './plan-card'

/** What the Planner asked this tab to show beside the conversation. */
export interface SideShown {
  readonly workflowId: string
  readonly taskNumber: number | null
}

export function PlanSidePanel({
  shown,
  projectId,
  onClose,
  onPointAt,
}: {
  shown: SideShown
  projectId: string
  onClose: () => void
  onPointAt: (plan: Plan, task: PlanTask) => void
}) {
  const plan = usePlan(shown.workflowId)
  const p = plan.data
  const close = (
    <button
      type="button"
      onClick={onClose}
      aria-label="Close plan"
      className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    >
      <X className="size-3.5" />
    </button>
  )

  return (
    <aside
      aria-label="Plan beside the conversation"
      className="flex w-[min(40rem,45%)] min-w-80 shrink-0 flex-col border-l border-border bg-card"
    >
      {p === undefined ? (
        <div className="flex items-start justify-between gap-2 px-4 py-3">
          {plan.isError ? (
            <ErrorLine error={plan.error} />
          ) : (
            <p className="text-xs text-faint-foreground">Reading the plan…</p>
          )}
          {close}
        </div>
      ) : (
        <>
          <PlanHeading plan={p} projectId={projectId}>
            {close}
          </PlanHeading>
          <div className="min-h-0 flex-1">
            <PlanGraph
              plan={p}
              focusTask={shown.taskNumber ?? undefined}
              onSelectTask={(task) => onPointAt(p, task)}
            />
          </div>
        </>
      )}
    </aside>
  )
}
