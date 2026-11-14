import { useLocation } from '@tanstack/react-router'
import { useEffect, useRef, useState } from 'react'
import { ChevronDown, GitCommitHorizontal } from 'lucide-react'
import type { Attempt } from '@/api/runs'
import { BriefPanel } from './brief-panel'
import { ConflictPanel } from './conflict-panel'
import { attemptLabel } from './use-task-run'

function attemptTime(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}

function AttemptEntry({ a, linked }: { a: Attempt; linked: boolean }) {
  const [open, setOpen] = useState(linked)
  const ref = useRef<HTMLLIElement>(null)
  const detailsId = `attempt-${a.id}-details`
  useEffect(() => { if (linked) ref.current?.scrollIntoView?.() }, [linked])
  return (
    <li ref={ref} id={`attempt-${a.id}`} className="overflow-hidden rounded-xl border border-border bg-card/40">
      <button type="button" aria-expanded={open} aria-controls={detailsId}
        aria-label={`${open ? 'Hide' : 'Show'} details for attempt ${a.number}`}
        onClick={() => { setOpen(!open) }}
        className="flex w-full cursor-pointer flex-wrap items-center justify-between gap-4 rounded-t-xl bg-muted/30 px-4 py-3 text-start transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring">
        <span className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <span className="text-sm font-semibold text-foreground">Attempt {a.number}</span>
          <span className="rounded-full border border-accent-line/25 bg-accent-line/10 px-2 py-0.5 text-xs font-medium text-accent-line">{attemptLabel(a)}</span>
          <span className="text-xs text-muted-foreground">Model: <span className="font-mono text-foreground">{a.model}</span></span>
        </span>
        <span className="inline-flex shrink-0 items-center gap-2 rounded-lg border border-border bg-background/60 px-3 py-1.5 text-xs font-medium text-foreground">
          {open ? 'Hide details' : 'Show details'}
          <ChevronDown aria-hidden className={`size-4 ${open ? 'rotate-180' : ''}`} />
        </span>
      </button>
      <div className="space-y-3 border-t border-border px-4 py-4 text-sm leading-relaxed">
      <p className="text-xs text-muted-foreground">
        {a.addition === null ? 'Began from Start' : "Carried on with the Planner's addition"}
      </p>
      {a.addition !== null && (
        <p dir="auto" aria-label="Planner's addition" className="mt-1 whitespace-pre-wrap break-words">{a.addition}</p>
      )}
      <div className="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground">
        <span>Started <time dateTime={a.started_at} title={a.started_at}>{attemptTime(a.started_at)}</time></span>
        {a.ended_at !== null && <span>Ended <time dateTime={a.ended_at} title={a.ended_at}>{attemptTime(a.ended_at)}</time></span>}
      </div>
      {a.commit !== null && <div className="flex items-start gap-2 text-xs text-muted-foreground">
        <GitCommitHorizontal aria-hidden className="mt-0.5 size-4 shrink-0" />
        <span>Commit <code className="break-all text-foreground">{a.commit}</code></span>
      </div>}
      {a.report !== null && (
        <div className="space-y-2">
          <h3 className="text-xs font-semibold text-muted-foreground">Executor report</h3>
          <p dir="auto" aria-label="Executor report" className="whitespace-pre-wrap break-words">{a.report}</p>
        </div>
      )}
      {a.reason !== null && <p dir="auto" className="text-destructive-foreground">{a.reason}</p>}
      <div id={detailsId} role="region" aria-label={`Details for attempt ${a.number}`} hidden={!open}>
        {open && <div className="space-y-4 border-t border-border pt-4">
          <h3 className="text-sm font-semibold">Context &amp; contract details</h3>
          <BriefPanel attemptId={a.id} />
          <ConflictPanel attemptId={a.id} />
        </div>}
      </div>
      </div>
    </li>
  )
}

/** Each attempt, newest first: its state, model, times, commit, the executor's report and reason; open one to read
 * the context it received and any contract conflict it reported. */
export function ReportTab({ attempts }: { attempts: readonly Attempt[] }) {
  const hash = useLocation({ select: (l) => l.hash }).replace(/^#/, '')
  if (attempts.length === 0) return <p className="px-6 py-4 text-sm text-faint-foreground">No attempts yet.</p>
  return (
    <ol aria-label="Attempts" className="space-y-2 px-6 py-4 text-xs">
      {attempts.map((a) => <AttemptEntry key={a.id} a={a} linked={hash === `attempt-${a.id}`} />)}
    </ol>
  )
}
