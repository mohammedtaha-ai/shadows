// Bounded navigation through a project's primary containment tree.
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { type Part } from '@/api/design'
import { partQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { PartEditor } from './part-editor'
import { usePartPages } from './use-part-pages'

export function PartsView({ projectId, selected }: { projectId: string; selected?: string }) {
  const [destination, setDestination] = useState<Part | null>(null)
  const [creating, setCreating] = useState(false)
  const detail = useQuery({ ...partQuery(projectId, selected ?? ''), enabled: selected !== undefined })
  return <div className="grid gap-6 lg:grid-cols-[minmax(220px,1fr)_2fr]">
    <section className="space-y-3" aria-label="Project parts">
      <div className="flex gap-2"><Button size="sm" onClick={() => setCreating(true)}>Create part</Button>
        <Button size="sm" variant="secondary" onClick={() => setDestination(null)}>Use project root</Button></div>
      <p className="text-sm text-muted-foreground">Destination: {destination?.content.title ?? 'Project root'}. Choose a part in the tree to create or move inside it.</p>
      <Branch projectId={projectId} choose={setDestination} />
    </section>
    <section className="space-y-4">
      {creating && <PartEditor key="create" projectId={projectId} destination={destination} onCreated={() => setCreating(false)} />}
      {selected !== undefined && detail.isPending && <p>Loading part…</p>}
      {detail.error !== null && <ErrorLine error={detail.error} />}
      {detail.data !== undefined && <PartEditor key={selected} projectId={projectId} saved={detail.data} destination={destination} />}
      {selected !== undefined && detail.data !== undefined && <section aria-label="Selected part children" className="space-y-2">
        <h3 className="text-sm font-medium">Children</h3>
        <Branch key={selected} projectId={projectId} parent={selected} choose={setDestination} />
      </section>}
      {!creating && selected === undefined && <p className="text-sm text-muted-foreground">Open a part to inspect its design and linked plans.</p>}
    </section>
  </div>
}

function Branch({ projectId, parent, choose }: { projectId: string; parent?: string; choose: (part: Part) => void }) {
  const branch = usePartPages(projectId, parent)
  return <div className="space-y-1">
    {branch.pending && <p>Loading parts…</p>}
    {branch.items.map(part => <TreeRow key={part.id} projectId={projectId} part={part} choose={choose} />)}
    {branch.next && <Button size="sm" variant="secondary" disabled={branch.loading} onClick={() => void branch.more()}>Load more</Button>}
    {branch.error && <ErrorLine error={branch.error} />}
  </div>
}

function TreeRow({ projectId, part, choose }: { projectId: string; part: Part; choose: (part: Part) => void }) {
  const [expanded, setExpanded] = useState(false)
  return <div>
    <div data-part-id={part.id} className="flex items-center gap-2 rounded border p-2 text-sm">
      <button aria-label={`${expanded ? 'Collapse' : 'Expand'} ${part.content.title}`} onClick={() => setExpanded(!expanded)}>{expanded ? '−' : '+'}</button>
      <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map', part: part.id }} className="flex-1" dir="auto">{part.content.title}</Link>
      <button aria-label={`Destination ${part.content.title}`} title="Use as destination" onClick={() => choose(part)}>↳</button>
    </div>
    {expanded && <div className="ml-4 border-l pl-2"><Branch projectId={projectId} parent={part.id} choose={choose} /></div>}
  </div>
}
