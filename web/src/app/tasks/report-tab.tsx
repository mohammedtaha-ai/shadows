import { useLocation } from '@tanstack/react-router'
import { useEffect, useRef, useState } from 'react'
import type { Attempt } from '@/api/runs'
import { BriefPanel } from './brief-panel'
import { ConflictPanel } from './conflict-panel'
import { attemptLabel } from './use-task-run'

function AttemptEntry({ a, linked }: { a: Attempt; linked: boolean }) {
  const [open, setOpen] = useState(linked)
  const ref = useRef<HTMLLIElement>(null)
  useEffect(() => { if (linked) ref.current?.scrollIntoView?.() }, [linked])
  return (
    <li ref={ref} id={`attempt-${a.id}`} className="rounded border border-border p-2">
      <button type="button" aria-expanded={open} onClick={() => { setOpen(!open) }} className="flex w-full flex-wrap gap-3 text-start">
        <span className="font-semibold">Attempt {a.number}</span>
        <span>{attemptLabel(a)}</span>
        <span className="font-mono">{a.model}</span>
      </button>
      <p className="mt-1 text-muted-foreground">
        {a.addition === null ? 'Began from Start' : "Carried on with the Planner's addition"}
      </p>
      {a.addition !== null && (
        <p dir="auto" aria-label="Planner's addition" className="mt-1 whitespace-pre-wrap break-words">{a.addition}</p>
      )}
      <div className="text-muted-foreground">
        {a.started_at}{a.ended_at !== null && ` → ${a.ended_at}`}
      </div>
      {a.commit !== null && <div className="font-mono">{a.commit}</div>}
      {a.report !== null && (
        <p dir="auto" aria-label="Executor report" className="mt-1 whitespace-pre-wrap break-words">{a.report}</p>
      )}
      {a.reason !== null && <p dir="auto" className="text-destructive-foreground">{a.reason}</p>}
      {open && (
        <div className="mt-2 border-t border-border pt-2">
          <BriefPanel attemptId={a.id} />
          <ConflictPanel attemptId={a.id} />
        </div>
      )}
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
