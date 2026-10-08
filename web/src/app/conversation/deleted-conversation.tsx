// One job: a removed conversation's readable history (§16.8).

import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import type { PlanningThread } from '@/api/client'
import { entriesQuery, operationsQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { Messages } from './messages'
import { SubagentPanel } from './subagent-panel'
import { subagentOf } from './tool-text'

export function DeletedConversation({ thread, projectId }: { thread: PlanningThread; projectId: string }) {
  const entries = useQuery(entriesQuery(thread.id))
  const operations = useQuery(operationsQuery(thread.id))
  const error = entries.error ?? operations.error
  // A subagent's card still opens what it did (§22.4).
  const [agent, setAgent] = useState<string | null>(null)
  const card =
    entries.data?.map((e) => subagentOf(e)).find((c) => c !== null && c.id === agent) ?? null
  return (
    <div className="flex h-full min-h-0">
      <div className="flex min-w-0 flex-1 flex-col">
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
          subagents={[]} onOpenSubagent={setAgent}
        />
      </div>
      {card !== null && <SubagentPanel card={card} onClose={() => setAgent(null)} />}
    </div>
  )
}
