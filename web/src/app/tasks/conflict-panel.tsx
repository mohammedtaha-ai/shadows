// One job: the contract conflict an executor reported on one attempt, with Core's current stop and delivery facts.

import { useQuery } from '@tanstack/react-query'
import { attemptConflictQuery, type TaskConflict } from '@/api/runs'
import { ErrorLine } from '../error-line'

const DELIVERY: Record<TaskConflict['delivery'], string> = {
  pending: 'not yet delivered to the Planner',
  delivered: 'delivered to the Planner',
  no_planner: 'no Planner to tell',
}

/** The conflict of one attempt, or nothing when it reported none. */
export function ConflictPanel({ attemptId }: { attemptId: string }) {
  const conflict = useQuery(attemptConflictQuery(attemptId))
  if (conflict.error !== null) return <ErrorLine error={conflict.error} />
  const c = conflict.data
  if (c == null) return null
  return (
    <section aria-label="Contract conflict" className="mt-2 space-y-1 rounded border border-warning/40 p-2">
      <p className="font-semibold">Contract conflict</p>
      <p><span className="font-semibold">Expected</span> <span dir="auto" className="whitespace-pre-wrap break-words">{c.expected}</span></p>
      <p><span className="font-semibold">Observed</span> <span dir="auto" className="whitespace-pre-wrap break-words">{c.observed}</span></p>
      <ul className="list-disc ps-5">
        {c.evidence.map((e, i) => (
          <li key={i}>
            <span className="font-mono">{e.line === null ? e.path : `${e.path}:${e.line}`}</span>{' '}
            <span dir="auto" className="whitespace-pre-wrap break-words">{e.summary}</span>
          </li>
        ))}
      </ul>
      <p className="text-muted-foreground">
        Attempt {c.attempt_state} · {c.scope_held ? 'scope still held' : 'scope released'} · {DELIVERY[c.delivery]}
      </p>
    </section>
  )
}
