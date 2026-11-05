
import { useQuery } from '@tanstack/react-query'
import { useCallback, useState } from 'react'
import type { PlanningThread } from '@/api/threads'
import { entriesQuery, operationsQuery } from '@/api/threads'
import { ErrorLine } from '../error-line'
import { ContextRecordPanel } from './context-record-panel'
import { Messages } from './messages'
import { SubagentPanel } from './subagent-panel'
import { subagentOf } from './tool-text'

export function DeletedConversation({ thread, projectId }: { thread: PlanningThread; projectId: string }) {
  const entries = useQuery(entriesQuery(thread.id))
  const operations = useQuery(operationsQuery(thread.id))
  const error = entries.error ?? operations.error
  const [panel, setPanel] = useState<{ kind: 'subagent' | 'context'; id: string } | null>(null)
  const openSubagent = useCallback((id: string) => setPanel({ kind: 'subagent', id }), [])
  const openContext = useCallback((id: string) => setPanel({ kind: 'context', id }), [])
  const card = panel?.kind === 'subagent'
    ? entries.data?.map(subagentOf).find(c => c?.id === panel.id) ?? null
    : null
  const context = panel?.kind === 'context'
    ? operations.data?.find(op => op.id === panel.id)?.invocation?.planning_context
    : null
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
          subagents={[]} tools={{}} todos={{}} onOpenSubagent={openSubagent}
          onOpenContext={openContext}
        />
      </div>
      {card !== null && <SubagentPanel card={card} onClose={() => setPanel(null)} />}
      {panel?.kind === 'context' && context != null &&
        <ContextRecordPanel operationId={panel.id} state={context} onClose={() => setPanel(null)} />}
    </div>
  )
}
