import { designKeys } from '@/api/design'
// Immutable roadmap proposal content with live person-approval state.
import {useMutation,useQuery,useQueryClient} from '@tanstack/react-query'
import {useRef} from 'react'
import type {ThreadEntry} from '@/api/client'
import {type Attempt,attemptFor} from '@/api/command-id'
import {approveRoadmapProposal,type OutcomeContent,type RoadmapProposal} from '@/api/design'
import {outcomeQuery,partQuery,roadmapProposalsQuery} from '@/api/queries'
import {Button} from '@/components/ui/button'
import {ErrorLine} from '../error-line'
import {ApprovedBy} from './approved-by'

const isContent = (c: unknown): c is OutcomeContent => !!c && typeof c === 'object' && !Array.isArray(c)
  && typeof (c as OutcomeContent).title === 'string' && Array.isArray((c as OutcomeContent).acceptance)

function snapshot(entry:ThreadEntry): RoadmapProposal | null {
  const card = entry.card
  if (!card || typeof card.id !== 'string' || !['first','add','change','remove'].includes(String(card.action))
    || !Array.isArray(card.outcomes)) return null
  if (card.outcomes.some(o => !o || typeof o !== 'object' || Array.isArray(o) || typeof o.key !== 'string'
    || !isContent(o.content) || !Array.isArray(o.parts))) return null
  if (card.old != null && !isContent(card.old)) return null
  return card as unknown as RoadmapProposal
}

const titles = {first:'First roadmap',add:'Add outcome',change:'Change outcome',remove:'Remove outcome'}

export function RoadmapProposalCard({entry,projectId}: {entry:ThreadEntry;projectId:string}) {
  const proposed = snapshot(entry)
  const client = useQueryClient()
  const query = useQuery({...roadmapProposalsQuery(projectId),enabled:proposed !== null})
  const attempt = useRef<Attempt | null>(null)
  const approve = useMutation({mutationFn:(sent:{id:string;commandId:string}) =>
    approveRoadmapProposal(projectId,sent.id,sent.commandId),onSuccess:() => {
      attempt.current=null
      void client.invalidateQueries({queryKey:designKeys.all(projectId)})
    },onError:() => {void client.invalidateQueries({queryKey:roadmapProposalsQuery(projectId).queryKey})},
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
  return <article data-roadmap-proposal={proposed.id} className="space-y-4 rounded-lg border bg-card p-4">
    <h3 className="font-medium">{titles[proposed.action]}</h3>
    {proposed.old && <section aria-label="Current outcome" className="rounded border p-3 text-sm">
      <h4>{proposed.action==='remove' ? 'Outcome to remove' : 'Current outcome'}</h4>
      <OutcomeText content={proposed.old} />
    </section>}
    {proposed.action==='change' && proposed.outcomes[0]
      && (proposed.outcomes[0].parent ?? null)!==(proposed.old_parent ?? null)
      && <MoveLine projectId={projectId} parent={proposed.outcomes[0].parent ?? null} />}
    {proposed.outcomes.length>0 && <OutcomeTree outcomes={proposed.outcomes} projectId={projectId}
      first={proposed.action==='first'} />}
    <p role="status" className="text-sm">{status}</p>
    {live?.status==='Approved' && <p className="text-sm">
      <ApprovedBy by={live.approved_by} projectId={projectId} /></p>}
    {live?.status==='Pending' && !live.outdated && query.error===null && <Button size="sm"
      disabled={approve.isPending} onClick={submit}>Approve roadmap</Button>}
    {query.error && <ErrorLine error={query.error} />}{approve.error && <ErrorLine error={approve.error} />}
  </article>
}

function OutcomeText({content}: {content:OutcomeContent}) {
  return <>
    <p dir="auto" className="font-medium">{content.title}</p>
    <p dir="auto" className="whitespace-pre-wrap">{content.intended_result}</p>
    {content.acceptance.length>0 && <ul className="list-disc ps-5 text-sm">
      {content.acceptance.map((a,i) => <li key={i} dir="auto">{a}</li>)}</ul>}
  </>
}

function MoveLine({projectId,parent}: {projectId:string;parent:string | null}) {
  const target = useQuery({...outcomeQuery(projectId,parent ?? ''),enabled:parent!==null})
  return <p role="note" dir="auto" className="text-sm font-medium">{parent===null ? 'Moves to the top level'
    : `Moves under: ${target.data?.outcome.content.title ?? parent}`}</p>
}

function PartTitle({projectId,partId}: {projectId:string;partId:string}) {
  const part = useQuery(partQuery(projectId,partId))
  return <li dir="auto">{part.data?.part.content.title ?? partId}</li>
}

function OutcomeTree({outcomes,projectId,first,parent=null,depth=0}: {
  outcomes:RoadmapProposal['outcomes'];projectId:string;first:boolean;parent?:string | null;depth?:number}) {
  if (depth>outcomes.length) return null
  return <ul className="space-y-3 border-l pl-3">{outcomes.filter(o => !first || (o.parent_key ?? null)===parent)
    .map(o => <li key={o.key} className="space-y-2">
      <OutcomeText content={o.content} />
      <section aria-label="Referenced parts" className="text-sm"><h5>Parts</h5>
        <ul className="list-disc ps-5">{o.parts.map(p => <PartTitle key={p} projectId={projectId} partId={p} />)}</ul>
      </section>
      {first && outcomes.some(child => child.parent_key===o.key) && <OutcomeTree outcomes={outcomes}
        projectId={projectId} first parent={o.key} depth={depth+1} />}
    </li>)}</ul>
}
