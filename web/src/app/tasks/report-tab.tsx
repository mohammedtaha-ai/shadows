import type { Attempt } from '@/api/runs'
import { attemptLabel } from './use-task-run'

/** Each attempt, newest first: its state, model, times, commit and reason. */
export function ReportTab({ attempts }: { attempts: readonly Attempt[] }) {
  if (attempts.length === 0) return <p className="px-6 py-4 text-sm text-faint-foreground">No attempts yet.</p>
  return (
    <ol aria-label="Attempts" className="space-y-2 px-6 py-4 text-xs">
      {attempts.map((a) => (
        <li key={a.id} className="rounded border border-border p-2">
          <div className="flex flex-wrap gap-3">
            <span className="font-semibold">Attempt {a.number}</span>
            <span>{attemptLabel(a)}</span>
            <span className="font-mono">{a.model}</span>
          </div>
          <div className="text-muted-foreground">
            {a.started_at}{a.ended_at !== null && ` → ${a.ended_at}`}
          </div>
          {a.commit !== null && <div className="font-mono">{a.commit}</div>}
          {a.reason !== null && <p dir="auto" className="text-destructive-foreground">{a.reason}</p>}
        </li>
      ))}
    </ol>
  )
}
