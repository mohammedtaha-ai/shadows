// One job: ask Core where a topic is, and show its answer as Core ordered it.
import { useQuery } from '@tanstack/react-query'
import { Link, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import {
  codeWhereQuery,
  type FileEntry,
  type Live,
  type Owner,
  type Rank,
  type SymbolEntry,
  type UnrecordedFile,
  type UseLine,
} from '@/api/code-record'
import { ElementContent } from '@/app/element-content'
import { ErrorLine } from '@/app/error-line'
import { Button } from '@/components/ui/button'
import { SkeletonContent } from '@/components/ui/skeleton'
import { StateBadge } from './state-badge'

const link = 'text-accent-line hover:underline'
const RANKS: Record<Rank, string> = { exact: 'Exact match', path: 'Path', part: 'Part', words: 'Words' }

const ownerText = (owner: Owner) => `${owner.part_title}${owner.layer != null ? ` · ${owner.layer}` : ''}`

function LiveLine({ live }: { live: Live }) {
  if (live.state === 'confirmed') return <p>Confirmed in the code</p>
  if (live.state === 'gap') return <p dir="auto">Gap: {live.why}</p>
  if (live.state === 'unchecked') return <p dir="auto">Unchecked: {live.why}</p>
  return null
}

/** The uses in Core's order, one heading per run of the same owner. */
function groupUses(uses: UseLine[]): { owner: Owner | null; lines: UseLine[] }[] {
  const groups: { owner: Owner | null; lines: UseLine[] }[] = []
  for (const use of uses) {
    const owner = use.owner ?? null
    const last = groups[groups.length - 1]
    if (last !== undefined && last.owner?.part_id === owner?.part_id && last.owner?.layer === owner?.layer) {
      last.lines.push(use)
    } else {
      groups.push({ owner, lines: [use] })
    }
  }
  return groups
}

function Uses({ symbol }: { symbol: SymbolEntry }) {
  const { uses_live: live, uses, uses_total: total, uses_more: more } = symbol
  if (live.state === 'not_asked') return null
  if (live.state === 'failed' || live.state === 'skipped') {
    return <p dir="auto">Uses not checked: {live.why}</p>
  }
  if (live.state === 'answered' && uses.length === 0) return <p>No uses.</p>
  return (
    <div className="space-y-1">
      {live.state === 'answered' && <p>{total} uses</p>}
      {groupUses(uses).map((group, index) => (
        <div key={index}>
          <p dir="auto" className="text-muted-foreground">
            {group.owner === null ? 'No part' : ownerText(group.owner)}
          </p>
          <ul>
            {group.lines.map((use, at) => (
              <li key={at} dir="auto" className="font-mono">{use.path}:{use.line}</li>
            ))}
          </ul>
        </div>
      ))}
      {more > 0 && <p>{live.state === 'truncated' ? `at least ${more} more` : `${more} more`}</p>}
      {live.state === 'truncated' && <p dir="auto">{live.why}</p>}
    </div>
  )
}

function SymbolCard({ symbol }: { symbol: SymbolEntry }) {
  return (
    <li data-where-symbol={symbol.symbol_id} className="space-y-1 rounded-lg border border-border p-2">
      <p dir="auto" className="font-medium">{symbol.name} ({symbol.kind})</p>
      <p>At done: {symbol.at_done}</p>
      <LiveLine live={symbol.live} />
      {symbol.line != null && <p>Line {symbol.line}</p>}
      {symbol.behavior.length > 0 && (
        <ul className="list-disc pl-4">
          {symbol.behavior.map((line, index) => <li key={index} dir="auto">{line}</li>)}
        </ul>
      )}
      <Uses symbol={symbol} />
    </li>
  )
}

function FileCard({ projectId, file }: { projectId: string; file: FileEntry }) {
  return (
    <article data-where-file={file.path} className="space-y-2 rounded-xl border border-border p-3 text-xs">
      <div className="flex flex-wrap items-center gap-2">
        <span dir="auto" className="font-mono text-sm">{file.path}</span>
        <span className="text-muted-foreground">{RANKS[file.rank]}</span>
      </div>
      {file.owner != null && <p dir="auto">{ownerText(file.owner)}</p>}
      <LiveLine live={file.live} />
      {file.confirmed === 'file_and_symbol' && <p>File and symbol confirmed</p>}
      {file.confirmed === 'file_only' && <p>File confirmed</p>}
      {file.tasks.length > 0 && (
        <ul className="space-y-0.5">
          {file.tasks.map((task, index) => (
            <li key={`${task.workflow_id}:${task.version}:${task.task}:${index}`} dir="auto">
              <Link to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
                params={{ projectId, workflowId: task.workflow_id, version: String(task.version), task: String(task.task) }}
                className={link}>
                T{task.task} v{task.version} · {task.action} · {task.purpose}
                {task.commit != null ? ` · ${task.commit.slice(0, 8)}` : ''}
              </Link>
            </li>
          ))}
        </ul>
      )}
      {file.symbols.length > 0 && (
        <ul className="space-y-2">
          {file.symbols.map(symbol => <SymbolCard key={`${symbol.symbol_id}:${symbol.task}:${symbol.version}`} symbol={symbol} />)}
        </ul>
      )}
    </article>
  )
}

function Unrecorded({ file }: { file: UnrecordedFile }) {
  return (
    <li className="space-y-0.5">
      <p dir="auto" className="font-mono">{file.path}</p>
      {file.owner != null && <p dir="auto">{ownerText(file.owner)}</p>}
      {file.symbols.length > 0 && (
        <ul>
          {file.symbols.map((symbol, index) => <li key={index} dir="auto">{symbol.name}:{symbol.line}</li>)}
        </ul>
      )}
    </li>
  )
}

export function WhereView({ projectId, topic }: { projectId: string; topic?: string }) {
  const navigate = useNavigate()
  const [typed, setTyped] = useState(topic ?? '')
  const answer = useQuery({ ...codeWhereQuery(projectId, topic ?? ''), enabled: topic !== undefined })
  const data = answer.data
  return (
    <div className="space-y-4 text-sm">
      <form aria-label="Where is it" className="flex flex-wrap items-center gap-2"
        onSubmit={event => {
          event.preventDefault()
          void navigate({ to: '/projects/$projectId/state', params: { projectId }, search: { view: 'where', topic: typed } })
        }}>
        <input aria-label="Topic" dir="auto" value={typed} onChange={event => setTyped(event.target.value)}
          className="min-w-64 flex-1 rounded-lg border border-border bg-input-background px-3 py-1.5" />
        <Button type="submit" size="sm">Ask</Button>
        {(answer.isSuccess || answer.isError) && (
          <Button type="button" variant="outline" size="sm" onClick={() => void answer.refetch()}>Ask again</Button>
        )}
      </form>
      {answer.isLoading && <SkeletonContent label="Asking the record, then the code…" layout="rows" />}
      {answer.isError && <ErrorLine error={answer.error} />}
      {data !== undefined && (
        <div className="space-y-3">
          <p>{data.asked} language-server questions asked</p>
          {data.budget != null && <p role="status" dir="auto">{data.budget}</p>}
          {data.entries.length === 0 && <p>Shadows has no record of this.</p>}
          {data.entries.map((entry, index) => entry.kind === 'file' ? (
            <FileCard key={`file:${entry.path}`} projectId={projectId} file={entry} />
          ) : (
            <article key={`element:${entry.element.id}:${index}`} className="space-y-2 rounded-xl border border-border p-3 text-xs">
              <div className="flex flex-wrap items-center gap-2">
                <StateBadge state={entry.state.state} />
                <Link to="/projects/$projectId/state" params={{ projectId }}
                  search={{ view: entry.element.content.kind === 'table' ? 'tables' : 'screens', element: entry.element.id }}
                  dir="auto" className={link}>
                  {entry.element.id} · {entry.element.content.kind} · {entry.element.content.name}
                </Link>
              </div>
              <ElementContent element={entry.element} />
            </article>
          ))}
          {data.entries_more > 0 && <p>{data.entries_more} more entries</p>}
          {data.not_in_record.length > 0 && (
            <section aria-label="Not in the record" className="space-y-1 text-xs">
              <h2 className="font-medium">Not in the record</h2>
              <ul className="space-y-2">
                {data.not_in_record.map(file => <Unrecorded key={file.path} file={file} />)}
              </ul>
              {data.not_in_record_more > 0 && <p>{data.not_in_record_more} more files</p>}
            </section>
          )}
        </div>
      )}
    </div>
  )
}
