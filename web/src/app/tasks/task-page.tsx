import { useQuery } from '@tanstack/react-query'
import { getRouteApi } from '@tanstack/react-router'
import { useState } from 'react'
import { planQuery } from '@/api/workflows'
import { ErrorLine } from '../error-line'
import { DetailsTab } from './details-tab'
import { DiffTab } from './diff-tab'
import { ReportTab } from './report-tab'
import { RunTab } from './run-tab'
import { TaskHeader } from './task-header'
import { useTaskRun } from './use-task-run'

const route = getRouteApi('/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task')
const TABS = ['Details', 'Run', 'Diff', 'Report'] as const
type Tab = (typeof TABS)[number]

/** The route's component: one task of one plan version, to run and to read. */
export function TaskPage() {
  const { projectId, workflowId, version, task } = route.useParams()
  return <TaskView key={`${workflowId}:${version}:${task}`} projectId={projectId} workflowId={workflowId}
    version={Number(version)} taskNumber={Number(task)} />
}

function TaskView({ projectId, workflowId, version, taskNumber }: {
  projectId: string; workflowId: string; version: number; taskNumber: number
}) {
  const plan = useQuery(planQuery(workflowId))
  const { run, attempts, start, stop, done } = useTaskRun(workflowId, version, taskNumber)
  const [tab, setTab] = useState<Tab>('Details')
  // The person's choice, once they make one; until then Core's suggestion.
  const [chosen, setChosen] = useState<string | null>(null)

  const failure = run.error ?? plan.error
  if (run.data === undefined || plan.data === undefined) {
    return (
      <div className="p-6 text-sm text-faint-foreground">
        {failure !== null ? <ErrorLine error={failure} /> : 'Reading the task…'}
      </div>
    )
  }
  const planTask = plan.data.tasks.find((t) => t.number === taskNumber)
  if (planTask === undefined) return <div className="p-6 text-sm text-faint-foreground">This plan has no T{taskNumber}.</div>
  const model = chosen ?? run.data.suggested_model ?? ''
  const actionError = start.error ?? stop.error ?? done.error
  const busy = start.isPending || stop.isPending || done.isPending

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <TaskHeader projectId={projectId} workflowId={workflowId} planTitle={plan.data.title} task={taskNumber}
        title={planTask.title} run={run.data} model={model} error={actionError} busy={busy}
        onStart={(m) => { start.mutate(m) }} onStop={(id) => { stop.mutate(id) }}
        onDone={(value) => { done.mutate(value) }} />
      <div role="tablist" className="flex gap-1 border-b border-border px-6">
        {TABS.map((name) => (
          <button key={name} type="button" role="tab" aria-selected={tab === name}
            onClick={() => { setTab(name) }}
            className={`px-3 py-2 text-sm ${tab === name ? 'border-b-2 border-accent-line font-semibold' : 'text-muted-foreground'}`}>
            {name}
          </button>
        ))}
      </div>
      <div role="tabpanel" className="min-h-0 flex-1 overflow-y-auto">
        {tab === 'Details' && <DetailsTab task={planTask} model={model} onModel={setChosen} />}
        {tab === 'Run' && <RunTab attempt={run.data.latest} />}
        {tab === 'Diff' && <DiffTab attempt={run.data.latest} />}
        {tab === 'Report' && <ReportTab attempts={attempts.data ?? []} />}
      </div>
    </div>
  )
}
