import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { Check, CircleCheck } from 'lucide-react'
import { awaitingJudgment, stoppedOnAdditionCap, type Need, type TaskRun } from '@/api/runs'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { attemptLabel } from './use-task-run'

function needName(need: Need): string {
  return typeof need.task === 'number' ? `T${need.task}` : `${need.task.plan_id.slice(0, 8)} · T${need.task.task}`
}

/** What a person sees of the task before any tab: where it is, what it waits on, and its three actions. */
export function TaskHeader({ projectId, workflowId, planTitle, task, title, run, error, busy,
  onStart, onStop, onDone }: {
  projectId: string
  workflowId: string
  planTitle: string
  task: number
  title: string
  run: TaskRun
  error: Error | null
  busy: boolean
  onStart: () => void
  onStop: (attemptId: string) => void
  onDone: (done: boolean, reason?: string) => void
}) {
  const [reason, setReason] = useState('')
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
        {awaitingJudgment(run) && (
          <span className="rounded bg-muted px-1.5 py-0.5 text-2xs text-muted-foreground">
            Waiting for the Planner's judgment
          </span>
        )}
        {stoppedOnAdditionCap(run) && (
          <span className="rounded bg-muted px-1.5 py-0.5 text-2xs text-muted-foreground">
            Stopped after the Planner's third addition; back to you
          </span>
        )}
        {run.done && <span className="inline-flex items-center gap-1.5 rounded-full border border-success/35 bg-success/15 px-2.5 py-1 text-xs font-semibold text-success">
          <CircleCheck aria-hidden className="size-3.5" />Done
        </span>}
      </div>
      {run.author_waiting && (
        <p className="mt-2 text-xs text-muted-foreground">
          <span className="font-medium text-foreground">Result waits for its external author</span>
          {" "}It is judged when the agent that wrote this plan waits again.
        </p>
      )}
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
          <Button size="sm" disabled={!run.start.allowed || busy} onClick={onStart}>Start</Button>
        )}
        {!running && !run.start.allowed && run.start.reason !== null && (
          <span dir="auto" className="text-xs text-muted-foreground">{run.start.reason}</span>
        )}
        <Button size="sm" variant="outline" disabled={busy} onClick={() => { onDone(!run.done, run.done ? undefined : reason || undefined) }}>
          {run.done ? 'Mark not done' : 'Mark done'}
        </Button>
      </div>
      {!run.done && <label className="mt-3 block max-w-xl space-y-1 text-xs text-muted-foreground">
        <span>Completion reason (optional; explain any checks you accept after review)</span>
        <textarea aria-label="Completion reason" value={reason} onChange={(e) => setReason(e.target.value)} disabled={busy}
          rows={2} className="w-full resize-y rounded-md border border-border bg-background px-3 py-2 text-foreground" />
      </label>}
      {run.done && run.done_reason != null && <p dir="auto" aria-label="Completion reason" className="mt-2 whitespace-pre-wrap break-words text-sm text-muted-foreground">{run.done_reason}</p>}
      {error !== null && <div className="mt-2"><ErrorLine error={error} /></div>}
    </header>
  )
}
