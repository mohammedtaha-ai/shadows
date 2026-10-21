import { useId, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { extensionsQuery, type ExtensionKind } from '@/api/extensions'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../../error-line'
import { ExtensionImport } from '../extension-import'
import { ExtensionList } from '../extension-list'

export function PluginsSection() {
  const library = useQuery(extensionsQuery)
  const [kind, setKind] = useState<ExtensionKind>('plugin')
  const [adding, setAdding] = useState(false)
  const id = useId()
  if (adding) return <ExtensionImport kind={kind} onDone={() => setAdding(false)} />
  return <div className="space-y-4">
    <div role="tablist" aria-label="Extension library" className="flex gap-2">{(['plugin', 'skill'] as const).map(value => <button type="button" key={value} role="tab" id={`${id}-${value}`} data-kind={value} aria-selected={kind === value} aria-controls={`${id}-panel`} tabIndex={kind === value ? 0 : -1}
      className="rounded-md px-3 py-2 text-sm aria-selected:bg-secondary" onClick={() => setKind(value)} onKeyDown={event => {
        if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
        event.preventDefault()
        const next = event.key === 'Home' ? 'plugin' : event.key === 'End' ? 'skill' : kind === 'plugin' ? 'skill' : 'plugin'
        setKind(next); event.currentTarget.parentElement?.querySelector<HTMLElement>(`[data-kind="${next}"]`)?.focus()
      }}>{value === 'plugin' ? 'Plugins' : 'Skills'}</button>)}</div>
    <div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-${kind}`} className="space-y-4">
      {library.isPending ? <p role="status">Loading library…</p> : library.error ? <div className="space-y-3"><ErrorLine error={library.error} /><Button type="button" variant="outline" disabled={library.isFetching} onClick={() => { void library.refetch() }}>Retry</Button></div> : <>
        <Button type="button" onClick={() => setAdding(true)}>Add</Button>
        <ExtensionList kind={kind} items={kind === 'plugin' ? library.data.plugins : library.data.skills} />
      </>}
    </div>
  </div>
}
