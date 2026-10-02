// One job: a removed conversation's readable history (§16.8).

import { useQuery } from '@tanstack/react-query'
import type { PlanningThread } from '@/api/client'
import { entriesQuery, operationsQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { Messages } from './messages'

export function DeletedConversation({ thread, projectId }: { thread: PlanningThread; projectId: string }) {
  const entries = useQuery(entriesQuery(thread.id))
  const operations = useQuery(operationsQuery(thread.id))
  const error = entries.error ?? operations.error
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="border-b border-border px-6 py-3">
        <h1 dir="auto" className="truncate text-start text-sm font-medium">{thread.title}</h1>
      </header>
      <p className="border-b border-border bg-muted/40 px-6 py-3 text-sm text-muted-foreground">
        This conversation was deleted. You can read it, but not write in it.
      </p>
      {error !== null && <div className="px-6 pt-3"><ErrorLine error={error} /></div>}
      <Messages
        entries={entries.data} loading={entries.isPending}
        reply={null} running={false} thinking={false} label={undefined}
        operations={operations.data} models={[]} harness={thread.harness}
        forkFrom={null} projectId={projectId} onPointAt={() => {}}
      />
    </div>
  )
}
