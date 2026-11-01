import { useId, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { extensionsQuery, type ExtensionKind, type McpItem } from '@/api/extensions'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../../error-line'
import { ExtensionImport } from '../extension-import'
import { ExtensionList } from '../extension-list'
import { McpForm } from '../mcp-form'

export function PluginsSection() {
  const library = useQuery(extensionsQuery)
  const [kind, setKind] = useState<ExtensionKind>('plugin')
  const [adding, setAdding] = useState(false)
  const [editing, setEditing] = useState<McpItem | null>(null)
  const id = useId()
  const kinds = ['plugin', 'skill', 'mcp'] as const
  if (adding || editing) return kind === 'mcp'
    ? <McpForm item={editing} onDone={() => { setAdding(false); setEditing(null) }} />
    : <ExtensionImport kind={kind} onDone={() => setAdding(false)} />
  return <div className="space-y-4">
    <div role="tablist" aria-label="Extension library" className="flex gap-2">{kinds.map(value => <button type="button" key={value} role="tab" id={`${id}-${value}`} data-kind={value} aria-selected={kind === value} aria-controls={`${id}-panel`} tabIndex={kind === value ? 0 : -1}
      className="rounded-md px-3 py-2 text-sm aria-selected:bg-secondary" onClick={() => setKind(value)} onKeyDown={event => {
        if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
        event.preventDefault()
        const next = event.key === 'Home' ? 'plugin' : event.key === 'End' ? 'mcp' : kinds[(kinds.indexOf(kind) + (event.key === 'ArrowRight' ? 1 : 2)) % kinds.length]
        setKind(next); event.currentTarget.parentElement?.querySelector<HTMLElement>(`[data-kind="${next}"]`)?.focus()
      }}>{value === 'plugin' ? 'Plugins' : value === 'skill' ? 'Skills' : 'MCP tools'}</button>)}</div>
    <div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-${kind}`} className="space-y-4">
      {library.isPending ? <p role="status">Loading library…</p> : library.error ? <div className="space-y-3"><ErrorLine error={library.error} /><Button type="button" variant="outline" disabled={library.isFetching} onClick={() => { void library.refetch() }}>Retry</Button></div> : <>
        <Button type="button" onClick={() => setAdding(true)}>{kind === 'mcp' ? 'Add MCP tool' : 'Add'}</Button>
        <ExtensionList kind={kind} items={kind === 'plugin' ? library.data.plugins : kind === 'skill' ? library.data.skills : library.data.mcp ?? []} onEdit={setEditing} />
      </>}
    </div>
  </div>
}
