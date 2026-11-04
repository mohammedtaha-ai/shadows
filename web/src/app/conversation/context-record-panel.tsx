import { useQuery } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { carriedContextQuery } from '@/api/planning-context'
import { ErrorLine } from '../error-line'
import { ContextState, type PlanningContextState } from './context-mark'

export function ContextRecordPanel({ operationId, state, onClose }: {
  operationId: string
  state: PlanningContextState
  onClose: () => void
}) {
  const record = useQuery(carriedContextQuery(operationId))
  const carried = record.data?.carried
  return <aside aria-label="Planning context beside the conversation"
    className="flex w-[min(40rem,55%)] min-w-0 shrink-0 flex-col border-l border-border/80 bg-card">
    <header className="flex items-start justify-between gap-2 border-b border-border/70 px-4 py-3">
      <div>
        <h2 className="text-sm font-semibold">Planning context</h2>
        <p role="status" className="mt-0.5 text-xs text-muted-foreground capitalize"><ContextState state={state} /></p>
      </div>
      <button type="button" onClick={onClose} aria-label="Close planning context"
        className="rounded-lg border border-border/60 p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground">
        <X aria-hidden className="size-3.5" />
      </button>
    </header>
    <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-4 py-3 text-xs">
      {record.isPending && <p>Reading planning context…</p>}
      {record.error !== null && <ErrorLine error={record.error} />}
      {record.data === null && <p>No planning context record is available for this turn.</p>}
      {carried !== undefined && <>
        <dl className="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 tabular-nums">
          <dt>Index tokens</dt><dd>{carried.index_tokens}</dd>
          <dt>Detail tokens</dt><dd>{carried.detail_tokens}</dd>
          <dt>Over-budget tokens</dt><dd>{carried.over_tokens}</dd>
        </dl>
        {carried.cuts.length > 0 && <section>
          <h3 className="mb-1 font-semibold">Cuts</h3>
          <ul className="space-y-1 text-muted-foreground">
            {carried.cuts.map((cut, index) => <li key={index} dir="auto">{cut}</li>)}
          </ul>
        </section>}
        {carried.sources.length > 0 && <section>
          <h3 className="mb-1 font-semibold">Sources</h3>
          <ul className="space-y-1 break-words text-muted-foreground">
            {carried.sources.map(({ kind, id, version }, index) =>
              <li key={index}><bdi>{kind}</bdi> · <bdi>{id}</bdi> · version {version}</li>)}
          </ul>
        </section>}
        <section>
          <h3 className="mb-1 font-semibold">Recorded text</h3>
          <pre dir="auto" className="rounded-lg border border-border/60 bg-background/60 p-3 text-start leading-relaxed whitespace-pre-wrap [overflow-wrap:anywhere]">
            {carried.text}
          </pre>
        </section>
      </>}
    </div>
  </aside>
}
