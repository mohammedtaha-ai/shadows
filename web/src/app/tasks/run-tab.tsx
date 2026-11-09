import { useQuery } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import type { Attempt } from '@/api/runs'
import { entriesQuery, type Operation, operationsQuery } from '@/api/threads'
import { ReplyText } from '@/app/conversation'
import { ErrorLine } from '../error-line'

/** The attempt's executor thread, read as the conversation reads its entries. */
export function RunTab({ attempt }: { attempt: Attempt | null }) {
  if (attempt === null) return <p className="px-6 py-4 text-sm text-faint-foreground">No attempt yet.</p>
  return <Entries attempt={attempt} />
}

/** Whether the attempt's turn may still add entries: an attempt that committed
 * keeps its turn going to report, so the turn's own end decides, as Core recorded it. */
function turnOpen(attempt: Attempt, operations: readonly Operation[] | undefined): boolean {
  if (attempt.state === 'running') return true
  if (attempt.operation_id === null) return false
  if (operations === undefined) return true
  const turn = operations.find((op) => op.id === attempt.operation_id)
  return turn !== undefined && turn.finished_at == null
}

function Entries({ attempt }: { attempt: Attempt }) {
  const operations = useQuery({
    ...operationsQuery(attempt.thread_id),
    refetchInterval: (query) => (turnOpen(attempt, query.state.data) ? 2000 : false),
  })
  const open = turnOpen(attempt, operations.data)
  const entries = useQuery({ ...entriesQuery(attempt.thread_id), refetchInterval: open ? 2000 : false })
  // The last entries can land between the final read and the turn's end: read once more.
  const wasOpen = useRef(open)
  const { refetch } = entries
  useEffect(() => {
    if (wasOpen.current && !open) void refetch()
    wasOpen.current = open
  }, [open, refetch])
  if (entries.isError) return <div className="px-6 py-4"><ErrorLine error={entries.error} /></div>
  if (entries.data === undefined) return <p className="px-6 py-4 text-sm text-faint-foreground">Reading the run…</p>
  return (
    <ol aria-label="Run" className="space-y-3 px-6 py-4">
      {entries.data.map((entry) => (
        <li key={entry.id} className="text-sm">
          {entry.kind === 'AgentMessage' ? <ReplyText text={entry.body} /> : (
            <p dir="auto" className="whitespace-pre-wrap text-muted-foreground">
              {entry.kind === 'UserMessage' ? entry.body : `${entry.kind}: ${entry.body}`}
            </p>
          )}
        </li>
      ))}
    </ol>
  )
}
