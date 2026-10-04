// Select existing part identities for an outcome, traversing bounded branches.
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { type Part } from '@/api/design'
import { partQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { usePartPages } from './use-part-pages'
interface ChoiceProps { projectId: string; selected: string[]; change: (ids: string[]) => void }
export function PartReferences(props: ChoiceProps) {
  return <section className="space-y-2" aria-label="Referenced parts"><h4 className="text-sm">Referenced parts</h4>
    <div className="flex flex-wrap gap-2">{props.selected.map(id => <PartReference key={id} projectId={props.projectId} id={id} />)}</div>
    <PartChoices {...props} />
  </section>
}
function PartReference({ projectId, id }: { projectId: string; id: string }) {
  const part = useQuery(partQuery(projectId, id))
  return <div className="text-sm">{part.data && <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map', part: id }} dir="auto">{part.data.part.content.title}</Link>}
    {part.isPending && <span>Loading part…</span>}{part.error && <ErrorLine error={part.error} />}</div>
}
function PartChoices({ parent, ...props }: ChoiceProps & { parent?: string }) {
  const branch = usePartPages(props.projectId, parent)
  return <div className="space-y-1">{branch.pending && <p>Loading parts…</p>}
    {branch.items.map(part => <PartChoice key={part.id} part={part} {...props} />)}
    {branch.next && <Button size="sm" variant="secondary" disabled={branch.loading} onClick={() => void branch.more()}>More parts</Button>}
    {branch.error && <ErrorLine error={branch.error} />}
  </div>
}
function PartChoice({ part, ...props }: ChoiceProps & { part: Part }) {
  const [expanded, setExpanded] = useState(false)
  return <div><div className="flex gap-2 text-sm"><button type="button" aria-label={`${expanded ? 'Collapse' : 'Expand'} reference ${part.content.title}`} onClick={() => setExpanded(!expanded)}>{expanded ? '−' : '+'}</button>
    <label><input type="checkbox" checked={props.selected.includes(part.id)} onChange={e => props.change(e.target.checked ? [...props.selected, part.id].sort() : props.selected.filter(id => id !== part.id))} /> {part.content.title}</label>
  </div>{expanded && <div className="ml-4 border-l pl-2"><PartChoices parent={part.id} {...props} /></div>}</div>
}
