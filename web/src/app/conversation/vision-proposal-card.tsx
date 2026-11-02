import { useProposalApproval } from './use-proposal-approval'
import { ProposalApproval } from './proposal-approval'
import type { ThreadEntry } from '@/api/threads'
import { approveVisionProposal, type VisionProposal } from '@/api/design'
import { visionProposalsQuery } from '@/api/design'
import { VISION_FIELDS } from '@/app/design'

type Snapshot = Pick<VisionProposal, 'id' | 'action' | 'field' | 'item_id' | 'text' | 'old_text'>
function proposed(entry: ThreadEntry): Snapshot | null {
  const c = entry.card
  if (!c || typeof c.id !== 'string' || !['add', 'edit', 'remove'].includes(String(c.action)) ||
      !VISION_FIELDS.includes(c.field as Snapshot['field'])) return null
  for (const key of ['item_id', 'text', 'old_text']) {
    if (c[key] !== null && c[key] !== undefined && typeof c[key] !== 'string') return null
  }
  return c as unknown as Snapshot
}

export function VisionProposalCard({ entry, projectId }: { entry: ThreadEntry; projectId: string }) {
  const snapshot = proposed(entry)
  const approval = useProposalApproval({
    projectId, proposalId: snapshot?.id ?? null,
    proposalsQuery: visionProposalsQuery(projectId), approveProposal: approveVisionProposal,
    approvedSuffix: live => live.item_id ? ` · ${live.item_id}` : '',
  })
  if (snapshot === null) return <p dir="auto">{entry.body}</p>
  const label = {purpose:'purpose',users:'user',goals:'goal',boundaries:'boundary',technical_direction:'technical direction'}[snapshot.field]
  const action = { add: 'Add', edit: 'Edit', remove: 'Remove' }[snapshot.action]
  return <article data-vision-proposal={snapshot.id} className="surface-panel rounded-2xl p-4 space-y-3">
    <h3 className="text-sm font-semibold text-foreground">{action} {label}{snapshot.item_id ? ` · ${snapshot.item_id}` : ''}</h3>
    {snapshot.action === 'edit' && <del dir="auto" className="block rounded-lg bg-destructive/10 px-2.5 py-1.5 whitespace-pre-wrap text-muted-foreground">{snapshot.old_text}</del>}
    <p dir="auto" className="whitespace-pre-wrap text-sm text-foreground">{snapshot.action === 'remove' ? snapshot.old_text : snapshot.text}</p>
    <ProposalApproval approval={approval} buttonLabel="Approve" textClass="text-xs text-muted-foreground" />
  </article>
}
