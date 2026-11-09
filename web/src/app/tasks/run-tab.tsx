import { useQuery } from '@tanstack/react-query'
import type { Attempt } from '@/api/runs'
import { entriesQuery } from '@/api/threads'
import { ReplyText } from '@/app/conversation'
import { ErrorLine } from '../error-line'

/** The attempt's executor thread, read as the conversation reads its entries. */
export function RunTab({ attempt }: { attempt: Attempt | null }) {
  if (attempt === null) return <p className="px-6 py-4 text-sm text-faint-foreground">No attempt yet.</p>
  return <Entries attempt={attempt} />
}

function Entries({ attempt }: { attempt: Attempt }) {
  const running = attempt.state === 'running'
  const entries = useQuery({ ...entriesQuery(attempt.thread_id), refetchInterval: running ? 2000 : false })
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
