// Navigate outcomes without deriving completion from plan lifecycle.
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { type Outcome } from '@/api/design'
import { outcomeQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { OutcomeEditor } from './outcome-editor'
import { useOutcomePages } from './use-outcome-pages'
export function RoadmapView({ projectId, selected }: { projectId: string; selected?: string }) {
  const [destination, setDestination] = useState<Outcome | null>(null)
  const [creating, setCreating] = useState(false)
  const detail = useQuery({ ...outcomeQuery(projectId, selected ?? ''), enabled: selected !== undefined })
  return <div className="grid gap-6 lg:grid-cols-[minmax(220px,1fr)_2fr]">
    <section aria-label="Roadmap outcomes" className="space-y-3">
      <div className="flex gap-2"><Button size="sm" onClick={() => setCreating(true)}>Create outcome</Button>
        <Button size="sm" variant="secondary" onClick={() => setDestination(null)}>Use roadmap root</Button></div>
      <p className="text-sm text-muted-foreground">Destination: {destination?.content.title ?? 'Roadmap root'}</p>
      <OutcomeBranch projectId={projectId} choose={setDestination} />
    </section>
    <section className="space-y-4">
      {creating && <OutcomeEditor key="create" projectId={projectId} destination={destination} onCreated={() => setCreating(false)} />}
      {detail.isPending && selected && <p>Loading outcome…</p>}{detail.error && <ErrorLine error={detail.error} />}
      {detail.data && <OutcomeEditor key={selected} projectId={projectId} saved={detail.data} destination={destination} />}
      {detail.data && selected && <section aria-label="Selected outcome children"><h3 className="text-sm">Children</h3><OutcomeBranch key={selected} projectId={projectId} parent={selected} choose={setDestination} /></section>}
      {!creating && !selected && <p className="text-sm text-muted-foreground">Open an outcome to define its result and acceptance. Plan approval or archive does not complete it.</p>}
    </section>
  </div>
}
function OutcomeBranch({ projectId, parent, choose }: { projectId: string; parent?: string; choose: (outcome: Outcome) => void }) {
  const branch = useOutcomePages(projectId, parent)
  return <div className="space-y-1">{branch.pending && <p>Loading outcomes…</p>}
    {branch.items.map(o => <OutcomeRow key={o.id} projectId={projectId} outcome={o} choose={choose} />)}
    {branch.next && <Button size="sm" variant="secondary" disabled={branch.loading} onClick={() => void branch.more()}>Load more</Button>}{branch.error && <ErrorLine error={branch.error} />}
  </div>
}
function OutcomeRow({ projectId, outcome, choose }: { projectId: string; outcome: Outcome; choose: (outcome: Outcome) => void }) {
  const [expanded, setExpanded] = useState(false)
  return <div><div className="flex gap-2 rounded border p-2 text-sm">
    <button aria-label={`${expanded ? 'Collapse' : 'Expand'} ${outcome.content.title}`} onClick={() => setExpanded(!expanded)}>{expanded ? '−' : '+'}</button>
    <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'roadmap', outcome: outcome.id }} className="flex-1" dir="auto">{outcome.content.title}</Link>
    <button aria-label={`Destination ${outcome.content.title}`} title="Use as destination" onClick={() => choose(outcome)}>↳</button>
  </div>{expanded && <div className="ml-4 border-l pl-2"><OutcomeBranch projectId={projectId} parent={outcome.id} choose={choose} /></div>}</div>
}
