// One job: show Core's project state (stage, parts, plans, gaps, one part in full) as Core gave it.
import { useQuery } from '@tanstack/react-query'
import { Link, useNavigate } from '@tanstack/react-router'
import { accessLabel, projectStateQuery, taskStatusLabel, type ElementGap, type ElementView, type PartDetail, type PartState, type PlanProgress, type PlanTasks } from '@/api/project-state'
import { ErrorLine } from '@/app/error-line'
import { Panel } from '@/components/ui/panel'
import { Button } from '@/components/ui/button'
import { SkeletonContent } from '@/components/ui/skeleton'
import { StateBadge } from './state-badge'

const link = 'text-accent-line hover:underline'

function PlanLine({ projectId, plan }: { projectId: string; plan: PlanProgress }) {
  const target = plan.workflow_id ?? plan.draft
  return (
    <li className="text-xs text-muted-foreground">
      {target !== null ? (
        <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: target }}
          dir="auto" className={link}>
          {plan.title}{plan.workflow_id === null ? ' (draft)' : ''}
        </Link>
      ) : <span dir="auto">{plan.title}</span>}
      {' '}
      {plan.tasks.done} of {plan.tasks.total} done · {plan.tasks.started} started · {plan.tasks.not_started} not started
    </li>
  )
}

function PlanList({ projectId, plans, more }: { projectId: string; plans: PlanProgress[]; more: number }) {
  return (
    <>
      {plans.length > 0 && <ul className="mt-1 space-y-0.5">
        {plans.map(plan => <PlanLine key={plan.plan_id} projectId={projectId} plan={plan} />)}
      </ul>}
      {more > 0 && <p className="mt-1 text-xs text-muted-foreground">{more} more plans</p>}
    </>
  )
}

function PartCard({ projectId, part, parts }: { projectId: string; part: PartState; parts: PartState[] }) {
  const parent = part.parent === null ? null : parts.find(other => other.part_id === part.parent)?.title ?? part.parent
  const c = part.elements
  return (
    <article data-state-part={part.part_id} className="rounded-xl border border-border p-3">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <Link to="/projects/$projectId/state" params={{ projectId }} search={{ view: 'state', part: part.part_id }}
          dir="auto" className={`text-sm font-medium ${link}`}>
          {part.title}
        </Link>
        <span dir="auto" className="text-xs text-muted-foreground">{part.kind ?? 'No kind'}</span>
        {parent !== null && <span dir="auto" className="text-xs text-muted-foreground">in {parent}</span>}
      </div>
      <p className="mt-1 text-xs text-muted-foreground">
        {c.planned} planned · {c.built} built · {c.gap} gap · {c.unchecked} unchecked
      </p>
      <PlanList projectId={projectId} plans={part.plans} more={part.plans_more} />
    </article>
  )
}

function ElementLink({ projectId, id, kind, name }: { projectId: string; id: string; kind: string; name: string }) {
  return (
    <Link to="/projects/$projectId/state" params={{ projectId }}
      search={{ view: kind === 'table' ? 'tables' : 'screens', element: id }} className={link}>
      {id} · {kind} · <span dir="auto">{name}</span>
    </Link>
  )
}

function GapLine({ projectId, gap, parts }: { projectId: string; gap: ElementGap; parts: PartState[] }) {
  const part = parts.find(other => other.part_id === gap.part_id)?.title ?? gap.part_id
  return (
    <li className="space-y-0.5 text-xs">
      <div className="flex flex-wrap items-center gap-2">
        <StateBadge state={gap.state} />
        <ElementLink projectId={projectId} id={gap.element_id} kind={gap.kind} name={gap.name} />
        <span dir="auto" className="text-muted-foreground">{part}</span>
      </div>
      <p dir="auto" className="text-muted-foreground">{gap.why}</p>
    </li>
  )
}

function PlanTasksBlock({ projectId, plan }: { projectId: string; plan: PlanTasks }) {
  return (
    <div className="space-y-1">
      <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: plan.workflow_id }}
        dir="auto" className={`text-sm font-medium ${link}`}>
        {plan.title}
      </Link>
      <ul className="space-y-0.5 text-xs">
        {plan.tasks.map(task => (
          <li key={task.number} className="flex flex-wrap gap-x-2">
            <Link to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
              params={{ projectId, workflowId: plan.workflow_id, version: String(plan.version), task: String(task.number) }}
              className={link}>
              T{task.number} <span dir="auto">{task.title}</span>
            </Link>
            <span className="text-muted-foreground">{taskStatusLabel(task)}</span>
            {task.access != null && <span className="text-muted-foreground">{accessLabel(task.access)}</span>}
          </li>
        ))}
      </ul>
    </div>
  )
}

function PartDetailSection({ projectId, title, detail, removed }: {
  projectId: string; title: string; detail: PartDetail; removed: boolean
}) {
  const navigate = useNavigate()
  const shown = detail.elements.filter((view: ElementView) => removed || view.state.state !== 'removed')
  const hasRemoved = detail.elements.some(view => view.state.state === 'removed')
  return (
    <section aria-label="Part detail" className="space-y-4">
      <Panel className="space-y-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 dir="auto" className="text-base font-semibold">{title}</h2>
          <Link to="/projects/$projectId/state" params={{ projectId }} search={{ view: 'state' }} className={`text-xs ${link}`}>
            All parts
          </Link>
        </div>
        {detail.plans.map(plan => <PlanTasksBlock key={plan.plan_id} projectId={projectId} plan={plan} />)}
        {detail.plans_more > 0 && <p className="text-xs text-muted-foreground">{detail.plans_more} more plans</p>}
        <div className="space-y-2">
          <h3 className="text-sm font-medium">Tables and screens</h3>
          {shown.length === 0 && <p className="text-xs text-muted-foreground">No tables or screens in this part.</p>}
          <ul className="space-y-1 text-xs">
            {shown.map(({ element, state }) => (
              <li key={element.id} className="flex flex-wrap items-center gap-2">
                <StateBadge state={state.state} />
                <ElementLink projectId={projectId} id={element.id} kind={element.content.kind} name={element.content.name} />
                {state.behind && <span className="text-muted-foreground">newer revision planned</span>}
              </li>
            ))}
          </ul>
          {hasRemoved && (
            <Button variant="outline" size="sm" onClick={() => void navigate({
              to: '/projects/$projectId/state', params: { projectId },
              search: { view: 'state', part: detail.part_id, removed: removed ? undefined : true },
            })}>
              {removed ? 'Hide removed' : 'Show removed'}
            </Button>
          )}
        </div>
      </Panel>
    </section>
  )
}

export function StateView({ projectId, part, removed }: { projectId: string; part?: string; removed: boolean }) {
  const state = useQuery(projectStateQuery(projectId, part))
  if (state.isPending) return <SkeletonContent label="Loading the project's state…" layout="rows" />
  if (state.isError) {
    return part === undefined ? <ErrorLine error={state.error} /> : (
      <section aria-label="Part" className="space-y-2">
        <ErrorLine error={state.error} />
        <Link to="/projects/$projectId/state" params={{ projectId }} search={{ view: 'state' }} className={`text-xs ${link}`}>
          All parts
        </Link>
      </section>
    )
  }
  const data = state.data
  const partTitle = (id: string) => data.parts.find(other => other.part_id === id)?.title ?? id
  return (
    <div className="space-y-6">
      <Panel className="space-y-1 p-4 text-sm">
        <p>Stage: {data.stage}</p>
        <p>Map revision {data.map_revision}</p>
        {data.missing.length > 0 && <p dir="auto">Missing: {data.missing.join(', ')}</p>}
      </Panel>
      {data.parts.length === 0 && data.unassigned.length === 0 && (
        <p className="text-sm text-muted-foreground">No parts yet. The map comes from the Planner.</p>
      )}
      {data.task_totals !== undefined && (
        <section aria-label="Task totals" className="space-y-1">
          <h2 className="text-sm font-medium">Tasks</h2>
          <p className="text-xs text-muted-foreground">
            {data.task_totals.done} of {data.task_totals.total} done · {data.task_totals.started} started · {data.task_totals.not_started} not started
          </p>
        </section>
      )}
      {data.outcomes !== undefined && data.outcomes.length > 0 && (
        <section aria-label="Outcomes" className="space-y-1">
          <h2 className="text-sm font-medium">Outcomes with plans</h2>
          <ul className="space-y-0.5">
            {data.outcomes.map(outcome => (
              <li key={outcome.outcome_id} className="text-xs text-muted-foreground">
                <Link to="/projects/$projectId/workspace" params={{ projectId }}
                  search={{ view: 'roadmap', outcome: outcome.outcome_id }} dir="auto" className={link}>
                  {outcome.title}
                </Link>
                {' '}{outcome.tasks.done} of {outcome.tasks.total} tasks done
              </li>
            ))}
          </ul>
          {(data.outcomes_more ?? 0) > 0 && <p className="text-xs text-muted-foreground">{data.outcomes_more} more outcomes</p>}
        </section>
      )}
      {data.part != null && part !== undefined && (
        <PartDetailSection projectId={projectId} title={partTitle(data.part.part_id)} detail={data.part} removed={removed} />
      )}
      <section aria-label="Parts" className="space-y-2">
        {data.parts.map(item => <PartCard key={item.part_id} projectId={projectId} part={item} parts={data.parts} />)}
        {data.parts_more > 0 && <p className="text-xs text-muted-foreground">{data.parts_more} more parts</p>}
      </section>
      {data.unassigned.length > 0 && (
        <section aria-label="Plans with no part" className="space-y-1">
          <h2 className="text-sm font-medium">Plans with no part</h2>
          <PlanList projectId={projectId} plans={data.unassigned} more={data.unassigned_more} />
        </section>
      )}
      <section aria-label="Gaps" className="space-y-2">
        <h2 className="text-sm font-medium">Gaps</h2>
        {data.gaps.length === 0 && <p className="text-xs text-muted-foreground">No gaps.</p>}
        <ul className="space-y-2">
          {data.gaps.map(gap => <GapLine key={gap.element_id} projectId={projectId} gap={gap} parts={data.parts} />)}
        </ul>
        {data.gaps_more > 0 && <p className="text-xs text-muted-foreground">{data.gaps_more} more gaps</p>}
      </section>
    </div>
  )
}
