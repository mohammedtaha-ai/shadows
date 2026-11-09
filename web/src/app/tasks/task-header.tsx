import { Link } from '@tanstack/react-router'
import { Check } from 'lucide-react'
import type { Need, TaskRun } from '@/api/runs'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { attemptLabel } from './use-task-run'

function needName(need: Need): string {
  return typeof need.task === 'number' ? `T${need.task}` : `${need.task.plan_id.slice(0, 8)} · T${need.task.task}`
}

/** What a person sees of the task before any tab: where it is, what it waits on, and its three actions. */
export function TaskHeader({ projectId, workflowId, planTitle, task, title, run, model, error, busy,
  onStart, onStop, onDone }: {
  projectId: string
  workflowId: string
  planTitle: string
  task: number
  title: string
  run: TaskRun
  model: string
  error: Error | null
  busy: boolean
  onStart: (model: string) => void
  onStop: (attemptId: string) => void
  onDone: (done: boolean) => void
}) {
  const running = run.latest?.state === 'running'
  return (
    <header className="border-b border-border px-6 py-4">
      <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId }}
        className="text-xs text-faint-foreground hover:underline">{planTitle}</Link>
      <div className="mt-1 flex flex-wrap items-center gap-3">
        <h1 dir="auto" className="text-lg font-semibold">T{task} · {title}</h1>
        <span className="rounded bg-accent-line/15 px-1.5 py-0.5 text-2xs text-accent-line">
          {attemptLabel(run.latest)}
        </span>
        {run.done && <span className="text-xs text-muted-foreground">Done</span>}
      </div>
      {run.needs.length > 0 && (
        <ul aria-label="Needs" className="mt-2 flex flex-wrap gap-2 text-xs text-muted-foreground">
          {run.needs.map((need) => (
            <li key={needName(need)} className="flex items-center gap-1 rounded border border-border px-1.5 py-0.5">
              {need.done && <Check aria-label="done" className="size-3" />}
              <span>{needName(need)} {need.title}</span>
            </li>
          ))}
        </ul>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-2">
        {running && run.latest !== null ? (
          <Button size="sm" disabled={busy} onClick={() => { onStop(run.latest!.id) }}>Stop</Button>
        ) : (
          <Button size="sm" disabled={!run.start.allowed || busy || model.trim() === ''}
            onClick={() => { onStart(model.trim()) }}>Start</Button>
        )}
        {!running && !run.start.allowed && run.start.reason !== null && (
          <span dir="auto" className="text-xs text-muted-foreground">{run.start.reason}</span>
        )}
        <Button size="sm" variant="outline" disabled={busy} onClick={() => { onDone(!run.done) }}>
          {run.done ? 'Mark not done' : 'Mark done'}
        </Button>
      </div>
      {error !== null && <div className="mt-2"><ErrorLine error={error} /></div>}
    </header>
  )
}
