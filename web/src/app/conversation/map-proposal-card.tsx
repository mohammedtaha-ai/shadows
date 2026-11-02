import { useProposalApproval } from './use-proposal-approval'
import { ProposalApproval } from './proposal-approval'
import type { ThreadEntry } from '@/api/threads'
import {approveMapProposal,type MapProposal} from '@/api/design'
import { mapProposalsQuery } from '@/api/design'
import {MapPartDetails} from '@/app/design'

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
  const approval = useProposalApproval({
    projectId, proposalId: proposed?.id ?? null,
    proposalsQuery: mapProposalsQuery(projectId), approveProposal: approveMapProposal,
  })
  if (!proposed) return <p dir="auto">{entry.body}</p>
  return <article data-map-proposal={proposed.id} className="surface-panel space-y-4 rounded-2xl p-4">
    <h3 className="font-semibold text-foreground">{proposed.action==='first' ? 'First project map' : proposed.action==='add' ? 'Add part' : 'Change part'}</h3>
    {proposed.old && <section className="rounded-xl border border-accent-line/20 bg-background/70 p-3 text-sm"><h4>Changed fields</h4>
      {Object.keys(proposed.old).filter(key => JSON.stringify(proposed.old?.[key as keyof typeof proposed.old])
        !== JSON.stringify(proposed.parts[0]?.content[key as keyof typeof proposed.old])).map(key =>
        <p key={key}><strong>{key}</strong>: <del dir="auto">{JSON.stringify(proposed.old?.[key as keyof typeof proposed.old])}</del>
          {' → '}{JSON.stringify(proposed.parts[0]?.content[key as keyof typeof proposed.old])}</p>)}
    </section>}
    <PartTree parts={proposed.parts} projectId={projectId} first={proposed.action==='first'} />
    {proposed.waivers.length>0 && <section aria-label="Map waivers" className="space-y-2 rounded-xl border border-warning/40 bg-warning/10 p-3">
      <h4 className="font-medium text-warning">This approval includes these waivers</h4>
      {proposed.waivers.map(w => <p key={w.part} dir="auto">Without {w.part} — reason: {w.reason}</p>)}
    </section>}
    <ProposalApproval approval={approval} buttonLabel={proposed.waivers.length ? 'Approve map and waivers' : 'Approve map'} textClass="text-sm" />
  </article>
}

function PartTree({parts,projectId,first,parent=null,depth=0}: {parts:MapProposal['parts'];projectId:string;
  first:boolean;parent?:string | null;depth?:number}) {
  if (depth>parts.length) return null
  return <ul className="space-y-3 border-l border-accent-line/25 pl-3">{parts.filter(p => !first || (p.parent_key ?? null)===parent).map(p =>
    <li key={p.key} className="space-y-2"><h4 dir="auto" className="font-medium text-foreground">{p.content.title}</h4>
      <p dir="auto" className="whitespace-pre-wrap">{p.content.responsibility}</p>
      <p dir="auto" className="whitespace-pre-wrap text-sm text-muted-foreground">{p.content.design}</p>
      <MapPartDetails content={p.content} projectId={projectId} />
      {first && parts.some(child => child.parent_key===p.key) && <PartTree parts={parts} projectId={projectId}
        first parent={p.key} depth={depth+1} />}
    </li>)}</ul>
}
