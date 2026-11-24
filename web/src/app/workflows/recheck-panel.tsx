// One job: the plan page's terminal panel for the project's newest recheck batch: its checks on the
// left, the selected check's output on the right, live while the batch runs.

import { useQuery } from '@tanstack/react-query'
import { cn } from 'cn'
import { Check, ChevronDown, CircleDashed, Loader, Minus, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { type LastRecheck, type RecheckRow, lastRecheckQuery, recheckRowsQuery } from '@/api/runs'
import { Button } from '@/components/ui/button'
import { rowKey, useRecheckOutput } from '@/stream'
import { ErrorLine } from '../error-line'

type RowState = 'passed' | 'failed' | 'running' | 'waiting' | 'not run'

const clock = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })

function whoBegan(began: LastRecheck['began']): string {
  if (began.kind === 'person') return 'You asked'
  if (began.kind === 'planner') return 'The Planner asked'
  return `T${began.task} completed ${began.plan_title}`
}

function stateOf(row: RecheckRow, live: string | null): RowState {
  if (row.outcome === 'passed') return 'passed'
  if (row.outcome === 'not_run') return 'not run'
  if (row.outcome != null) return 'failed'
  return row.started_at !== null || rowKey(row) === live ? 'running' : 'waiting'
}

function StateIcon({ state }: { state: RowState }) {
  const className = 'size-3.5 shrink-0'
  switch (state) {
    case 'passed': return <Check aria-hidden className={cn(className, 'text-success')} />
    case 'failed': return <X aria-hidden className={cn(className, 'text-destructive')} />
    case 'running': return <Loader aria-hidden className={cn(className, 'animate-spin text-accent-line')} />
    case 'waiting': return <CircleDashed aria-hidden className={cn(className, 'text-faint-foreground')} />
    case 'not run': return <Minus aria-hidden className={cn(className, 'text-faint-foreground')} />
  }
}

/** The panel; nothing while the project has had no recheck. */
export function RecheckPanel({ workflowId, onClose }: { workflowId: string; onClose: () => void }) {
  const { data: last } = useQuery(lastRecheckQuery(workflowId))
  const recheckId = last?.recheck_id
  const rows = useQuery({ ...recheckRowsQuery(recheckId ?? ''), enabled: recheckId !== undefined })
  const output = useRecheckOutput(recheckId)
  const [picked, setPicked] = useState<{ recheck: string; key: string } | null>(null)
  const scroller = useRef<HTMLPreElement>(null)

  const list = rows.data ?? []
  const stored = list.find((row) => stateOf(row, null) === 'running')
  const running = output.running ?? (stored === undefined ? null : rowKey(stored))
  const chosen = picked !== null && picked.recheck === recheckId ? picked.key : null
  const selected = list.find((row) => rowKey(row) === (chosen ?? running)) ?? list[0]
  const key = selected === undefined ? null : rowKey(selected)
  // A finished row shows what Core stored; a running one, what has arrived.
  const text = selected === undefined || key === null ? '' : selected.output ?? output.text[key] ?? ''

  useEffect(() => {
    const element = scroller.current
    if (element !== null && key === running) element.scrollTop = element.scrollHeight
  }, [text, key, running])

  if (last == null) return null
  const { passed, failed, unfinished } = last.counts
  return (
    <section aria-label="Project recheck" className="flex h-72 shrink-0 flex-col border-t border-border bg-background">
      <header className="flex items-center gap-3 border-b border-border px-4 py-1.5 text-xs">
        <h2 className="font-semibold">Project recheck</h2>
        <span className="text-muted-foreground">
          {whoBegan(last.began)} at {clock.format(new Date(last.created_at))}
        </span>
        <span className="ml-auto tabular-nums text-muted-foreground">
          {passed} passed · <span className={failed > 0 ? 'text-destructive' : undefined}>{failed} failed</span> · {unfinished} left
        </span>
        <Button variant="ghost" size="icon-sm" aria-label="Close the recheck panel" onClick={onClose}>
          <ChevronDown aria-hidden className="size-4" />
        </Button>
      </header>
      {rows.isError && <div className="px-4 py-1"><ErrorLine error={rows.error} /></div>}
      <div className="flex min-h-0 flex-1">
        <ul aria-label="Checks" className="w-72 shrink-0 overflow-y-auto border-r border-border py-1">
          {list.map((row) => {
            const state = stateOf(row, output.running)
            const rowId = rowKey(row)
            return (
              <li key={rowId}>
                <button type="button" aria-pressed={rowId === key} title={row.command}
                  onClick={() => { if (recheckId !== undefined) setPicked({ recheck: recheckId, key: rowId }) }}
                  className={cn('flex w-full items-center gap-2 px-3 py-1 text-left text-xs hover:bg-accent/40',
                    rowId === key && 'bg-accent/60')}>
                  <StateIcon state={state} />
                  <span className="sr-only">{state}</span>
                  <span className="shrink-0 font-mono text-2xs text-muted-foreground">T{row.task}</span>
                  <span className="truncate font-mono">{row.command === '' ? row.check_id : row.command}</span>
                </button>
              </li>
            )
          })}
        </ul>
        <pre ref={scroller} aria-label="Output" tabIndex={0}
          className="min-w-0 flex-1 overflow-auto whitespace-pre-wrap break-words bg-muted/30 p-3 font-mono text-xs">
          {text}
          {selected?.truncated === true && '\n[output cut short]'}
          {output.lagged && '\n[some live output was missed]'}
        </pre>
      </div>
    </section>
  )
}
