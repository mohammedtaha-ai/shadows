// One job: the Workflows page — one plan version reviewed as its graph
// (§13.11).

import { Link, getRouteApi } from '@tanstack/react-router'
import { ChevronLeft, ChevronRight, Lock, MessageSquare } from 'lucide-react'
import { useState } from 'react'
import type { Plan } from '@/api/client'
import { ErrorLine } from '../error-line'
import { ApproveBar } from './approve-bar'
import { InspectPanel } from './inspect-panel'
import { PlanGraph } from './plan-graph'
import { stateLabel } from './plan-state'
import { usePlan } from './use-plan'

const route = getRouteApi('/projects/$projectId/workflows/$workflowId')

/** The route's component. Keyed by version, so moving to the next one
 * closes the task the last one had open. */
export function PlanPage() {
  const { projectId, workflowId } = route.useParams()
  return <PlanView key={workflowId} projectId={projectId} workflowId={workflowId} />
}

function PlanView({ projectId, workflowId }: { projectId: string; workflowId: string }) {
  const plan = usePlan(workflowId)
  const [inspected, setInspected] = useState<number | null>(null)

  if (plan.data === undefined) {
    return (
      <div className="p-6 text-sm text-faint-foreground">
        {plan.isError ? <ErrorLine error={plan.error} /> : 'Reading the plan…'}
      </div>
    )
  }
  const p = plan.data
  const task = p.tasks.find((t) => t.number === inspected)

  return (
    <div className="flex h-full min-h-0 flex-col">
      <Header plan={p} projectId={projectId} />
      {p.state === 'Frozen' ? (
        <p className="flex items-center gap-2 border-b border-border bg-accent-softer px-6 py-2 text-xs text-secondary-foreground">
          <Lock className="size-3.5" aria-hidden />
          Approved v{p.version} · editing creates draft v{p.version + 1}
        </p>
      ) : (
        <ApproveBar plan={p} />
      )}
      {plan.isError && (
        <div className="px-6 pt-2">
          <ErrorLine error={plan.error} />
        </div>
      )}
      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          {/* One canvas throughout: when Inspect opens or closes, the graph
              refits to the width left (see `plan-camera.tsx`). */}
          <PlanGraph plan={p} onSelectTask={(t) => setInspected(t.number)} />
        </div>
        {task !== undefined && (
          <InspectPanel plan={p} task={task} onClose={() => setInspected(null)} />
        )}
      </div>
    </div>
  )
}

function Header({ plan, projectId }: { plan: Plan; projectId: string }) {
  return (
    <header className="flex items-center justify-between gap-4 border-b border-border px-6 py-3">
      <div className="min-w-0">
        <h1 dir="auto" className="truncate text-sm font-medium">
          {plan.title}
        </h1>
        <p className="flex items-center gap-2 text-xs text-faint-foreground">
          <span
            className={`rounded px-1.5 py-px ${
              plan.state === 'Frozen'
                ? 'bg-secondary text-secondary-foreground'
                : 'border border-border text-muted-foreground'
            }`}
          >
            {stateLabel(plan)}
          </span>
          <span>revision {plan.revision}</span>
          <Link
            to="/projects/$projectId/threads/$threadId"
            params={{ projectId, threadId: plan.thread_id }}
            className="flex items-center gap-1 hover:text-foreground"
          >
            <MessageSquare className="size-3" aria-hidden />
            Its conversation
          </Link>
        </p>
      </div>
      <nav aria-label="Versions" className="flex items-center gap-1 text-xs">
        <VersionLink projectId={projectId} to={plan.previous} label={`v${plan.version - 1}`} back />
        <VersionLink projectId={projectId} to={plan.next} label={`v${plan.version + 1}`} />
      </nav>
    </header>
  )
}

function VersionLink({
  projectId,
  to,
  label,
  back = false,
}: {
  projectId: string
  to: string | null | undefined
  label: string
  back?: boolean
}) {
  const content = back ? (
    <>
      <ChevronLeft className="size-3.5" aria-hidden />
      {label}
    </>
  ) : (
    <>
      {label}
      <ChevronRight className="size-3.5" aria-hidden />
    </>
  )
  if (to == null) return null
  return (
    <Link
      to="/projects/$projectId/workflows/$workflowId"
      params={{ projectId, workflowId: to }}
      className="flex items-center gap-0.5 rounded-md px-2 py-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    >
      {content}
    </Link>
  )
}
