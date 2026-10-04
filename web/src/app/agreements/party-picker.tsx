// Browse arbitrary nesting when declaring stable participant identities.
import { useState } from 'react'
import type { AgreementContent } from '@/api/client'
import { Button } from '@/components/ui/button'
import { usePartPages } from '../design/use-part-pages'
import { ErrorLine } from '../error-line'
type Party = AgreementContent['parties'][number]
export function PartyPicker({ projectId, parties, onChange, disabled }: {
  projectId: string; parties: Party[]; onChange: (parties: Party[]) => void; disabled: boolean
}) {
  const [path, setPath] = useState<{ id: string; title: string }[]>([])
  const parent = path.at(-1)?.id
  return <fieldset className="space-y-2"><legend>Declared participants</legend>
    <nav className="flex gap-2" aria-label="Participant part path">
      <Button size="sm" variant="outline" onClick={() => setPath([])}>Project</Button>
      {path.map((p, i) => <Button key={p.id} size="sm" variant="outline"
        onClick={() => setPath(path.slice(0, i + 1))}>{p.title}</Button>)}
    </nav>
    <Branch key={parent ?? 'root'} projectId={projectId} parent={parent} parties={parties}
      disabled={disabled} onChange={onChange} onOpen={part => setPath([...path, part])} />
    <ul className="text-xs text-faint-foreground">{parties.map(p => <li key={`${p.part_id}:${p.role}`}>
      {p.role} · {p.part_id}{!disabled && <Button size="sm" variant="ghost" onClick={() =>
        onChange(parties.filter(n => !(n.part_id === p.part_id && n.role === p.role)))}>Remove party</Button>}
    </li>)}</ul>
  </fieldset>
}
function Branch({ projectId, parent, parties, disabled, onChange, onOpen }: {
  projectId: string; parent?: string; parties: Party[]; disabled: boolean;
  onChange: (parties: Party[]) => void; onOpen: (part: { id: string; title: string }) => void
}) {
  const page = usePartPages(projectId, parent)
  return <div className="space-y-2">
    {page.error && <ErrorLine error={page.error} />}
    {page.items.map(part => <div key={part.id} className="flex flex-wrap items-center gap-3">
      <Button variant="outline" size="sm" onClick={() => onOpen({ id: part.id, title: part.content.title })}>
        Browse {part.content.title}</Button>
      {(['provides', 'uses'] as const).map(role => <label key={role}><input type="checkbox"
        disabled={disabled} aria-label={`${part.content.title} ${role}`}
        checked={parties.some(p => p.part_id === part.id && p.role === role)} onChange={e => {
          const remaining = parties.filter(p => !(p.part_id === part.id && p.role === role))
          onChange(e.target.checked ? [...remaining, { part_id: part.id, role }] : remaining)
        }} /> {role}</label>)}
    </div>)}
    {page.next && <Button onClick={() => void page.more()} disabled={page.loading}>More parts</Button>}
  </div>
}
