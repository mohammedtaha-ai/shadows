// Immutable map proposal content with live person-approval state.
import {useMutation,useQuery,useQueryClient} from '@tanstack/react-query'
import {useRef} from 'react'
import type {ThreadEntry} from '@/api/client'
import {type Attempt,attemptFor} from '@/api/command-id'
import {approveMapProposal,type MapProposal} from '@/api/design'
import {mapProposalsQuery} from '@/api/queries'
import {Button} from '@/components/ui/button'
import {ErrorLine} from '../error-line'
import {MapPartDetails} from '../design/map-part-details'
import {ApprovedBy} from './approved-by'

function snapshot(entry:ThreadEntry): MapProposal | null {
  const card = entry.card
  if (!card || typeof card.id !== 'string' || !['first','add','change'].includes(String(card.action))
    || !Array.isArray(card.parts) || !Array.isArray(card.waivers)) return null
  if (card.parts.some(p => !p || typeof p !== 'object' || Array.isArray(p)
    || typeof p.key !== 'string' || !p.content || typeof p.content !== 'object'
    || Array.isArray(p.content) || typeof p.content.title !== 'string')) return null
  if (card.waivers.some(w => !w || typeof w !== 'object' || Array.isArray(w)
    || typeof w.part !== 'string' || typeof w.reason !== 'string')) return null
  return card as unknown as MapProposal
}

export function MapProposalCard({entry,projectId}: {entry:ThreadEntry;projectId:string}) {
  const proposed = snapshot(entry)
  const client = useQueryClient()
  const query = useQuery({...mapProposalsQuery(projectId),enabled:proposed !== null})
  const attempt = useRef<Attempt | null>(null)
  const approve = useMutation({mutationFn:(sent:{id:string;commandId:string}) =>
    approveMapProposal(projectId,sent.id,sent.commandId),onSuccess:() => {
      attempt.current=null
      void client.invalidateQueries({queryKey:['projects',projectId,'design']})
    },onError:() => {void client.invalidateQueries({queryKey:mapProposalsQuery(projectId).queryKey})},
  })
  if (!proposed) return <p dir="auto">{entry.body}</p>
  const live = query.data?.find(p => p.id===proposed.id)
  const status = live?.status==='Approved' ? 'Approved' : live?.status==='Superseded' ? 'Superseded'
    : live?.outdated ? 'Outdated — ask the Planner to propose again' : live?.status==='Pending' ? 'Pending'
    : query.isPending ? 'Loading status…' : 'Status unavailable'
  const submit = () => {
    attempt.current=attemptFor(attempt.current,{projectId,proposalId:proposed.id})
    approve.mutate({id:proposed.id,commandId:attempt.current.commandId})
  }
  return <article data-map-proposal={proposed.id} className="space-y-4 rounded-lg border bg-card p-4">
    <h3 className="font-medium">{proposed.action==='first' ? 'First project map' : proposed.action==='add' ? 'Add part' : 'Change part'}</h3>
    {proposed.old && <section className="rounded border p-3 text-sm"><h4>Changed fields</h4>
      {Object.keys(proposed.old).filter(key => JSON.stringify(proposed.old?.[key as keyof typeof proposed.old])
        !== JSON.stringify(proposed.parts[0]?.content[key as keyof typeof proposed.old])).map(key =>
        <p key={key}><strong>{key}</strong>: <del dir="auto">{JSON.stringify(proposed.old?.[key as keyof typeof proposed.old])}</del>
          {' → '}{JSON.stringify(proposed.parts[0]?.content[key as keyof typeof proposed.old])}</p>)}
    </section>}
    <PartTree parts={proposed.parts} projectId={projectId} first={proposed.action==='first'} />
    {proposed.waivers.length>0 && <section aria-label="Map waivers" className="space-y-2 rounded border border-amber-500 bg-amber-500/10 p-3">
      <h4 className="font-medium">This approval includes these waivers</h4>
      {proposed.waivers.map(w => <p key={w.part} dir="auto">Without {w.part} — reason: {w.reason}</p>)}
    </section>}
    <p role="status" className="text-sm">{status}</p>
    {live?.status==='Approved' && <p className="text-sm">
      <ApprovedBy by={live.approved_by} projectId={projectId} /></p>}
    {live?.status==='Pending' && !live.outdated && query.error===null && <Button size="sm"
      disabled={approve.isPending} onClick={submit}>{proposed.waivers.length ? 'Approve map and waivers' : 'Approve map'}</Button>}
    {query.error && <ErrorLine error={query.error} />}{approve.error && <ErrorLine error={approve.error} />}
  </article>
}

function PartTree({parts,projectId,first,parent=null,depth=0}: {parts:MapProposal['parts'];projectId:string;
  first:boolean;parent?:string | null;depth?:number}) {
  if (depth>parts.length) return null
  return <ul className="space-y-3 border-l pl-3">{parts.filter(p => !first || (p.parent_key ?? null)===parent).map(p =>
    <li key={p.key} className="space-y-2"><h4 dir="auto" className="font-medium">{p.content.title}</h4>
      <p dir="auto" className="whitespace-pre-wrap">{p.content.responsibility}</p>
      <p dir="auto" className="whitespace-pre-wrap text-sm">{p.content.design}</p>
      <MapPartDetails content={p.content} projectId={projectId} />
      {first && parts.some(child => child.parent_key===p.key) && <PartTree parts={parts} projectId={projectId}
        first parent={p.key} depth={depth+1} />}
    </li>)}</ul>
}
