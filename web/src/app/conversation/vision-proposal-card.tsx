// One job: the proposed content, its live state, and the person's approval (§23.5).
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef } from 'react'
import type { ThreadEntry } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { approveVisionProposal, type VisionProposal } from '@/api/design'
import { visionProposalsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { VISION_FIELDS } from '../design/vision-fields'
import { ApprovedBy } from './approved-by'

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
  const client = useQueryClient()
  const query = useQuery({ ...visionProposalsQuery(projectId), enabled: snapshot !== null })
  const attempt = useRef<Attempt | null>(null)
  const approve = useMutation({
    mutationFn: (sent: { proposalId: string; commandId: string }) =>
      approveVisionProposal(projectId, sent.proposalId, sent.commandId),
    onSuccess: () => {
      attempt.current = null
      void client.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
    },
    onError: () => {
      void client.invalidateQueries({ queryKey: visionProposalsQuery(projectId).queryKey })
    },
  })
  if (snapshot === null) return <p dir="auto">{entry.body}</p>
  const live = query.data?.find(p => p.id === snapshot.id)
  const status = live?.status === 'Approved' ? `Approved${live.item_id ? ` · ${live.item_id}` : ''}`
    : live?.status === 'Superseded' ? 'Superseded'
    : live?.outdated ? 'Outdated — ask the Planner to propose again'
    : live?.status === 'Pending' ? 'Pending'
    : query.isPending ? 'Loading status…' : 'Status unavailable'
  const label = {purpose:'purpose',users:'user',goals:'goal',boundaries:'boundary',technical_direction:'technical direction'}[snapshot.field]
  const action = { add: 'Add', edit: 'Edit', remove: 'Remove' }[snapshot.action]
  const submit = () => {
    attempt.current = attemptFor(attempt.current, { projectId, proposalId: snapshot.id })
    approve.mutate({ proposalId: snapshot.id, commandId: attempt.current.commandId })
  }
  return <article data-vision-proposal={snapshot.id} className="rounded-lg border bg-card p-4 space-y-3">
    <h3 className="text-sm font-medium">{action} {label}{snapshot.item_id ? ` · ${snapshot.item_id}` : ''}</h3>
    {snapshot.action === 'edit' && <del dir="auto" className="block whitespace-pre-wrap text-muted-foreground">{snapshot.old_text}</del>}
    <p dir="auto" className="whitespace-pre-wrap">{snapshot.action === 'remove' ? snapshot.old_text : snapshot.text}</p>
    <p role="status" className="text-xs text-muted-foreground">{status}</p>
    {live?.status === 'Approved' && <p className="text-xs text-muted-foreground">
      <ApprovedBy by={live.approved_by} projectId={projectId} /></p>}
    {live?.status === 'Pending' && !live.outdated && query.error === null &&
      <Button size="sm" onClick={submit} disabled={approve.isPending}>Approve</Button>}
    {query.error !== null && <ErrorLine error={query.error} />}
    {approve.error !== null && <ErrorLine error={approve.error} />}
  </article>
}
