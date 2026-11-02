import { useProposalApproval } from './use-proposal-approval'
import { ProposalApproval } from './proposal-approval'
import {useQuery} from '@tanstack/react-query'
import type { ThreadEntry } from '@/api/threads'
import {approveRoadmapProposal,type OutcomeContent,type RoadmapProposal} from '@/api/design'
import { outcomeQuery, partQuery, roadmapProposalsQuery } from '@/api/design'

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
  const approval = useProposalApproval({
    projectId, proposalId: proposed?.id ?? null,
    proposalsQuery: roadmapProposalsQuery(projectId), approveProposal: approveRoadmapProposal,
  })
  if (!proposed) return <p dir="auto">{entry.body}</p>
  return <article data-roadmap-proposal={proposed.id} className="surface-panel space-y-4 rounded-2xl p-4">
    <h3 className="font-semibold text-foreground">{titles[proposed.action]}</h3>
    {proposed.old && <section aria-label="Current outcome" className="rounded-xl border border-accent-line/20 bg-background/70 p-3 text-sm">
      <h4>{proposed.action==='remove' ? 'Outcome to remove' : 'Current outcome'}</h4>
      <OutcomeText content={proposed.old} />
    </section>}
    {proposed.action==='change' && proposed.outcomes[0]
      && (proposed.outcomes[0].parent ?? null)!==(proposed.old_parent ?? null)
      && <MoveLine projectId={projectId} parent={proposed.outcomes[0].parent ?? null} />}
    {proposed.outcomes.length>0 && <OutcomeTree outcomes={proposed.outcomes} projectId={projectId}
      first={proposed.action==='first'} />}
    <ProposalApproval approval={approval} buttonLabel="Approve roadmap" textClass="text-sm" />
  </article>
}

function OutcomeText({content}: {content:OutcomeContent}) {
  return <>
    <p dir="auto" className="font-medium text-foreground">{content.title}</p>
    <p dir="auto" className="whitespace-pre-wrap">{content.intended_result}</p>
    {content.acceptance.length>0 && <ul className="list-disc ps-5 text-sm text-muted-foreground">
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
  return <ul className="space-y-3 border-l border-accent-line/25 pl-3">{outcomes.filter(o => !first || (o.parent_key ?? null)===parent)
    .map(o => <li key={o.key} className="space-y-2">
      <OutcomeText content={o.content} />
      <section aria-label="Referenced parts" className="text-sm"><h5>Parts</h5>
        <ul className="list-disc ps-5">{o.parts.map(p => <PartTitle key={p} projectId={projectId} partId={p} />)}</ul>
      </section>
      {first && outcomes.some(child => child.parent_key===o.key) && <OutcomeTree outcomes={outcomes}
        projectId={projectId} first parent={o.key} depth={depth+1} />}
    </li>)}</ul>
}
