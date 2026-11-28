import { SkeletonContent } from '@/components/ui/skeleton'
// Navigate outcomes without deriving completion from plan lifecycle.
import { Link } from '@tanstack/react-router'
import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { useState } from 'react'
import { type Outcome } from '@/api/design'
import { outcomeQuery } from '@/api/design'
import { accessLabel, projectStateQuery, taskStatusLabel, type OutcomeDelivery, type ProjectStateView } from '@/api/project-state'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { ExternalProposals } from './external-proposals'
import { OutcomeEditor } from './outcome-editor'
import { useOutcomePages } from './use-outcome-pages'
export function RoadmapView({ projectId, selected }: { projectId: string; selected?: string }) {
  const [destination, setDestination] = useState<Outcome | null>(null)
  const [creating, setCreating] = useState(false)
  const detail = useQuery({ ...outcomeQuery(projectId, selected ?? ''), enabled: selected !== undefined })
  const delivery = useQuery({ ...projectStateQuery(projectId, undefined, selected), enabled: selected !== undefined })
  return <div className="grid gap-6 lg:grid-cols-[minmax(220px,1fr)_2fr]">
    <section aria-label="Roadmap outcomes" className="space-y-3">
      <div className="flex gap-2"><Button size="sm" onClick={() => setCreating(true)}>Create outcome</Button>
        <Button size="sm" variant="secondary" onClick={() => setDestination(null)}>Use roadmap root</Button></div>
      <p className="text-sm text-muted-foreground">Destination: {destination?.content.title ?? 'Roadmap root'}</p>
      <ExternalProposals projectId={projectId} kind="roadmap" />
      <OutcomeBranch projectId={projectId} choose={setDestination} />
    </section>
    <section className="space-y-4">
      {creating && <OutcomeEditor key="create" projectId={projectId} destination={destination} onCreated={() => setCreating(false)} />}
      {detail.isPending && selected && <SkeletonContent label="Loading outcome…" layout="form" />}{detail.error && <ErrorLine error={detail.error} />}
      {detail.data !== undefined && <Link to="/projects/$projectId/new" params={{ projectId }}
        search={{ anchor: `outcome:${detail.data.outcome.id}` }}
        className="inline-flex text-sm text-accent-line hover:underline">Plan from here</Link>}
      {selected !== undefined && detail.data && <OutcomeTasks projectId={projectId} read={delivery} />}
      {detail.data && <OutcomeEditor key={selected} projectId={projectId} saved={detail.data} destination={destination} />}
      {detail.data && selected && <section aria-label="Selected outcome children"><h3 className="text-sm">Children</h3><OutcomeBranch key={selected} projectId={projectId} parent={selected} choose={setDestination} /></section>}
      {!creating && !selected && <p className="text-sm text-muted-foreground">Open an outcome to define its result and acceptance. Plan approval or archive does not complete it.</p>}
    </section>
  </div>
}
/** Task progress of the plans serving the selected outcome, as State reports it. It says nothing of acceptance. */
function OutcomeTasks({ projectId, read }: { projectId: string; read: UseQueryResult<ProjectStateView> }) {
  const outcome: OutcomeDelivery | null | undefined = read.data?.outcome
  return <section aria-label="Related tasks" className="space-y-2">
    <h3 className="text-sm font-medium">Related tasks</h3>
    {read.isPending && <SkeletonContent label="Loading related tasks…" />}
    {read.error && <ErrorLine error={read.error} />}
    {outcome != null && <>
      <p className="text-xs text-muted-foreground">{outcome.tasks.done} of {outcome.tasks.total} tasks done · {outcome.tasks.started} started · {outcome.tasks.not_started} not started</p>
      {outcome.plans.length === 0 && <p className="text-xs text-muted-foreground">No plan serves this outcome yet.</p>}
      {outcome.plans.map(({ plan, tasks }) => {
        const target = plan.workflow_id ?? plan.draft
        return <div key={plan.plan_id} className="space-y-1 text-xs">
          {target !== null
            ? <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: target }} dir="auto" className="text-sm font-medium text-accent-line hover:underline">{plan.title}</Link>
            : <span dir="auto" className="text-sm font-medium">{plan.title}</span>}
          {plan.workflow_id === null && <span className="ml-2 text-muted-foreground">Draft</span>}
          {plan.workflow_id !== null && <ul className="space-y-0.5">{tasks.map(task => <li key={task.number} className="flex flex-wrap gap-x-2">
            <Link to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
              params={{ projectId, workflowId: plan.workflow_id!, version: String(plan.version), task: String(task.number) }}
              className="text-accent-line hover:underline">T{task.number} <span dir="auto">{task.title}</span></Link>
            <span className="text-muted-foreground">{taskStatusLabel(task)}</span>
            {task.access != null && <span className="text-muted-foreground">{accessLabel(task.access)}</span>}
          </li>)}</ul>}
        </div>
      })}
      {outcome.plans_more > 0 && <p className="text-xs text-muted-foreground">{outcome.plans_more} more plans</p>}
    </>}
  </section>
}
function OutcomeBranch({ projectId, parent, choose }: { projectId: string; parent?: string; choose: (outcome: Outcome) => void }) {
  const branch = useOutcomePages(projectId, parent)
  return <div className="space-y-1">{branch.pending && <SkeletonContent label="Loading outcomes…" />}
    {branch.items.map(o => <OutcomeRow key={o.id} projectId={projectId} outcome={o} choose={choose} />)}
    {branch.next && <Button size="sm" variant="secondary" disabled={branch.loading} onClick={() => void branch.more()}>Load more</Button>}{branch.error && <ErrorLine error={branch.error} />}
  </div>
}
function OutcomeRow({ projectId, outcome, choose }: { projectId: string; outcome: Outcome; choose: (outcome: Outcome) => void }) {
  const [expanded, setExpanded] = useState(false)
  return <div><div className="flex gap-2 rounded border p-2 text-sm">
    <button aria-label={`${expanded ? 'Collapse' : 'Expand'} ${outcome.content.title}`} onClick={() => setExpanded(!expanded)}>{expanded ? '−' : '+'}</button>
    <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'roadmap', outcome: outcome.id }} className="flex-1" dir="auto">{outcome.content.title}</Link>
    <button aria-label={`Destination ${outcome.content.title}`} title="Use as destination" onClick={() => choose(outcome)}>↳</button>
  </div>{expanded && <div className="ml-4 border-l pl-2"><OutcomeBranch projectId={projectId} parent={outcome.id} choose={choose} /></div>}</div>
}
