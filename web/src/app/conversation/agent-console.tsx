// One job: the agent board's console. The edits, commands, files and
// failures of the conversation, one tab each, drawn from the steps Core reported.

import { FileCode2, GitCompare, ScrollText, Terminal } from 'lucide-react'
import { useState } from 'react'
import { CopyButton } from '../copy-button'
import type { ToolStep } from './activity-rows'
import { stepLine } from './activity-text'
import { ToolDiff } from './tool-diff'

type Tab = 'diff' | 'terminal' | 'files' | 'logs'
type Filter = 'all' | 'completed' | 'failed'

const TABS = [
  { id: 'diff', label: 'Diff', icon: GitCompare },
  { id: 'terminal', label: 'Terminal', icon: Terminal },
  { id: 'files', label: 'Files', icon: FileCode2 },
  { id: 'logs', label: 'Logs', icon: ScrollText },
] as const

const FILTERS: readonly Filter[] = ['all', 'completed', 'failed']

export function AgentConsole({ steps }: { steps: readonly ToolStep[] }) {
  const [tab, setTab] = useState<Tab>('diff')
  const [filter, setFilter] = useState<Filter>('all')
  const [pickedEdit, setPickedEdit] = useState<string | null>(null)

  const edits = steps.filter((s) => s.kind === 'edit' || s.diff !== null)
  const commands = steps.filter((s) => s.kind === 'execute')
  const files = steps.filter((s) => s.target !== null)
  const logs = steps.filter((s) => s.error !== null || s.status === 'failed' || s.status === 'stopped')
  const counts = { diff: edits.length, terminal: commands.length, files: files.length, logs: logs.length }
  const edit = edits.find((s) => s.key === pickedEdit) ?? edits.at(-1) ?? null
  const shownCommands = filter === 'all' ? commands : commands.filter((s) => s.status === filter)

  return (
    <section aria-label="Console" className="surface-panel rounded-2xl p-4">
      <div role="tablist" className="mb-3 flex flex-wrap items-center gap-1.5 border-b border-accent-line/15 pb-3">
        {TABS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            onClick={() => setTab(id)}
            className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors ${
              tab === id ? 'pill-active' : 'text-muted-foreground hover:bg-accent/10 hover:text-foreground'
            }`}
          >
            <Icon aria-hidden className="size-3.5" />
            <span>
              {label} ({counts[id]})
            </span>
          </button>
        ))}
      </div>

      {tab === 'diff' &&
        (edit === null ? (
          <Empty>No edits in this conversation yet.</Empty>
        ) : (
          <div className="space-y-2">
            <div className="flex flex-wrap items-center gap-1.5">
              {edits.map((s) => (
                <button
                  key={s.key}
                  type="button"
                  aria-pressed={s.key === edit.key}
                  onClick={() => setPickedEdit(s.key)}
                  className={`max-w-48 truncate rounded-md border px-2 py-0.5 font-mono text-[11px] ${
                    s.key === edit.key
                      ? 'border-accent-line/45 bg-accent/15 text-foreground'
                      : 'border-border/60 text-muted-foreground hover:text-foreground'
                  }`}
                >
                  {s.target ?? s.title}
                </button>
              ))}
              {edit.target !== null && <CopyButton text={edit.target} size="icon-xs" />}
            </div>
            {(edit.added !== null || edit.removed !== null) && (
              <p className="font-mono text-xs tabular-nums">
                <span className="text-success">+{edit.added ?? 0}</span>{' '}
                <span className="text-destructive">−{edit.removed ?? 0}</span>
              </p>
            )}
            <ToolDiff diff={edit.diff} />
          </div>
        ))}

      {tab === 'terminal' &&
        (commands.length === 0 ? (
          <Empty>No commands ran in this conversation yet.</Empty>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-1.5">
              {FILTERS.map((f) => (
                <button
                  key={f}
                  type="button"
                  aria-pressed={filter === f}
                  onClick={() => setFilter(f)}
                  className={`rounded-md px-2 py-0.5 text-[11px] capitalize ${
                    filter === f ? 'pill-active' : 'text-muted-foreground hover:text-foreground'
                  }`}
                >
                  {f}
                </button>
              ))}
            </div>
            <ul className="space-y-1.5">
              {shownCommands.map((s) => (
                <li key={s.key} className="rounded-lg border border-border/50 bg-background/60 px-3 py-2">
                  <div className="flex items-start justify-between gap-2">
                    <code dir="ltr" className="font-mono text-xs break-all whitespace-pre-wrap">
                      {s.title}
                    </code>
                    <CopyButton text={s.title} size="icon-xs" />
                  </div>
                  <p className="mt-0.5 text-[11px] text-muted-foreground">{s.status}</p>
                  {s.error !== null && (
                    <p className="mt-1 font-mono text-xs break-words whitespace-pre-wrap text-destructive">
                      {s.error}
                    </p>
                  )}
                </li>
              ))}
            </ul>
          </div>
        ))}

      {tab === 'files' &&
        (files.length === 0 ? (
          <Empty>No files touched in this conversation yet.</Empty>
        ) : (
          <ul className="space-y-1">
            {files.map((s) => (
              <li
                key={s.key}
                className="flex items-center justify-between gap-2 rounded-lg border border-border/40 bg-background/50 px-2.5 py-1.5 text-xs"
              >
                <code dir="ltr" className="min-w-0 font-mono break-all">
                  {s.target}
                </code>
                <span className="shrink-0 text-[11px] text-muted-foreground">{s.kind ?? 'tool'}</span>
              </li>
            ))}
          </ul>
        ))}

      {tab === 'logs' &&
        (logs.length === 0 ? (
          <Empty>Nothing failed or stopped.</Empty>
        ) : (
          <ul className="space-y-1.5">
            {logs.map((s) => (
              <li key={s.key} className="rounded-lg border border-border/50 bg-background/60 px-3 py-2 text-xs">
                <p dir="auto">
                  {stepLine(s).text} <span className="text-muted-foreground">· {s.status}</span>
                </p>
                {s.error !== null && (
                  <p className="mt-1 font-mono break-words whitespace-pre-wrap text-destructive">{s.error}</p>
                )}
              </li>
            ))}
          </ul>
        ))}
    </section>
  )
}

function Empty({ children }: { children: string }) {
  return (
    <p className="rounded-xl border border-border/50 bg-background/50 p-6 text-center text-xs text-muted-foreground">
      {children}
    </p>
  )
}
