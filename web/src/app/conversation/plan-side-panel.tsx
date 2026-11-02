// Only a live plan-show for this tab may open the panel; replays must not move the reader.

import { X } from 'lucide-react'
import type { Plan, PlanTask } from '@/api/workflows'
import { ErrorLine } from '../error-line'
import { PlanGraph } from '@/app/workflows'
import { usePlan } from '@/app/workflows'
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
      className="rounded-lg border border-accent-line/20 bg-accent/10 p-1 text-muted-foreground transition-colors hover:bg-accent/25 hover:text-foreground"
    >
      <X className="size-3.5" />
    </button>
  )

  return (
    <aside
      aria-label="Plan beside the conversation"
      className="flex w-[min(40rem,45%)] min-w-80 shrink-0 flex-col border-l border-accent-line/20 bg-card/95 backdrop-blur-xl"
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
