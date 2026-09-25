// One job: a `PlanView` entry as a card (§13.9) — the real graph of the
// version it shows, live, with the way to its page.

import { Link } from '@tanstack/react-router'
import { ExternalLink } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Plan, PlanTask, ThreadEntry } from '@/api/client'
import { ErrorLine } from '../error-line'
import { PlanGraph } from '../workflows/plan-graph'
import { usePlan } from '../workflows/use-plan'

/** The plan version an entry refers to, and the task if it names one. */
function planRefs(entry: ThreadEntry): { workflowId: string | null; taskId: string | null } {
  let workflowId: string | null = null
  let taskId: string | null = null
  for (const ref of entry.refs) {
    if ('Workflow' in ref) workflowId = ref.Workflow
    if ('Task' in ref) taskId = ref.Task
  }
  return { workflowId, taskId }
}

export function PlanCard({
  entry,
  projectId,
  onPointAt,
}: {
  entry: ThreadEntry
  projectId: string
  onPointAt: (plan: Plan, task: PlanTask) => void
}) {
  const { workflowId, taskId } = planRefs(entry)
  // A card without its version cannot draw one; it still says what it was.
  if (workflowId === null) {
    return (
      <p dir="auto" className="text-xs text-faint-foreground">
        {entry.body}
      </p>
    )
  }
  return (
    <LiveCard
      workflowId={workflowId}
      taskId={taskId}
      body={entry.body}
      projectId={projectId}
      onPointAt={onPointAt}
    />
  )
}

function LiveCard({
  workflowId,
  taskId,
  body,
  projectId,
  onPointAt,
}: {
  workflowId: string
  taskId: string | null
  body: string
  projectId: string
  onPointAt: (plan: Plan, task: PlanTask) => void
}) {
  // Pinned to its version: it follows that version's edits, never the next one.
  const plan = usePlan(workflowId)
  const p = plan.data
  const shownTask = p?.tasks.find((t) => t.id === taskId)?.number

  return (
    <section
      data-plan-card={workflowId}
      aria-label={body}
      className="overflow-hidden rounded-xl border border-border bg-card"
    >
      {p === undefined ? (
        <div className="space-y-2 px-4 py-3">
          <p dir="auto" className="text-xs text-muted-foreground">
            {body}
          </p>
          {plan.isError ? (
            <ErrorLine error={plan.error} />
          ) : (
            <p className="text-xs text-faint-foreground">Reading the plan…</p>
          )}
        </div>
      ) : (
        <>
          <PlanHeading plan={p} projectId={projectId} />
          <PlanGraph
            plan={p}
            compact
            focusTask={shownTask}
            onSelectTask={(task) => onPointAt(p, task)}
          />
        </>
      )}
    </section>
  )
}

/** "Plan v2 · revision 7", the plan's title, and **Open plan**: the head of
 * a card and of the side panel. `children` go at its end. */
export function PlanHeading({
  plan,
  projectId,
  children,
}: {
  plan: Plan
  projectId: string
  children?: ReactNode
}) {
  return (
    <header className="flex items-center justify-between gap-3 border-b border-border px-4 py-2">
      <div className="min-w-0">
        <h2 dir="auto" className="truncate text-sm font-medium text-card-foreground">
          {plan.title}
        </h2>
        <p dir="auto" className="text-xs text-faint-foreground">
          Plan v{plan.version} · revision {plan.revision}
          {plan.state === 'Frozen' && ' · Approved'}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <Link
          to="/projects/$projectId/workflows/$workflowId"
          params={{ projectId, workflowId: plan.id }}
          className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        >
          <ExternalLink aria-hidden className="size-3" />
          <span dir="auto">Open plan</span>
        </Link>
        {children}
      </div>
    </header>
  )
}
