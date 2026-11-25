import { SkeletonContent } from '@/components/ui/skeleton'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import type { Attempt } from '@/api/runs'
import { entriesQuery } from '@/api/threads'
import { holdThreadRefresh } from '@/stream'
import { ReplyText } from '@/app/conversation'
import { ErrorLine } from '../error-line'

/** The attempt's executor thread, read as the conversation reads its entries. */
export function RunTab({ attempt }: { attempt: Attempt | null }) {
  if (attempt === null) return <p className="px-6 py-4 text-sm text-faint-foreground">No attempt yet.</p>
  return <>
    <dl className="flex flex-wrap gap-x-6 gap-y-2 border-b border-border px-6 py-3 text-xs">
      <div><dt className="text-muted-foreground">CLI</dt><dd>{attempt.harness}</dd></div>
      <div><dt className="text-muted-foreground">Model</dt><dd>{attempt.model}</dd></div>
      <div><dt className="text-muted-foreground">Effort</dt><dd>{attempt.effort ?? 'None recorded'}</dd></div>
    </dl>
    <Entries attempt={attempt} />
  </>
}

function Entries({ attempt }: { attempt: Attempt }) {
  const queryClient = useQueryClient()
  // The executor's thread stream refreshes its entries and operations while the turn goes on.
  useEffect(() => holdThreadRefresh(queryClient, attempt.thread_id, {}), [queryClient, attempt.thread_id])
  const entries = useQuery(entriesQuery(attempt.thread_id))
  if (entries.isError) return <div className="px-6 py-4"><ErrorLine error={entries.error} /></div>
  if (entries.data === undefined) return <SkeletonContent label="Reading the run…" layout="conversation" className="px-6 py-4" />
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
