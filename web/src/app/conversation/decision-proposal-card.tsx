// One job: a Planner's proposed decision as a card, with Accept while Core says that revision
// is still the latest proposed one.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef } from 'react'
import { acceptDecision, decisionsQuery, designKeys } from '@/api/design'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import type { ThreadEntry } from '@/api/threads'
import { Button } from '@/components/ui/button'
import { panelVariants } from '@/components/ui/panel'
import { ErrorLine } from '../error-line'
import { ApprovedBy } from './approved-by'

interface Snapshot { id: string; version: number; title: string; decision: string; rationale: string; source: string }

function snapshotOf(entry: ThreadEntry): Snapshot | null {
  const c = entry.card
  if (c === null || c === undefined || typeof c.id !== 'string' || typeof c.version !== 'number' ||
      typeof c.title !== 'string' || typeof c.decision !== 'string' || typeof c.rationale !== 'string') return null
  return { id: c.id, version: c.version, title: c.title, decision: c.decision, rationale: c.rationale,
    source: typeof c.source === 'string' ? c.source : '' }
}

export function DecisionProposalCard({ entry, projectId }: { entry: ThreadEntry; projectId: string }) {
  const snapshot = snapshotOf(entry)
  const client = useQueryClient()
  const query = useQuery({ ...decisionsQuery(projectId), enabled: snapshot !== null })
  const attempt = useRef<Attempt | null>(null)
  const accept = useMutation({
    mutationFn: (sent: { id: string; version: number; commandId: string }) =>
      acceptDecision(projectId, sent.id, sent.version, sent.commandId),
    onSuccess: () => {
      attempt.current = null
      void client.invalidateQueries({ queryKey: designKeys.all(projectId) })
    },
    onError: () => { void client.invalidateQueries({ queryKey: designKeys.decisions(projectId) }) },
  })
  if (snapshot === null) return <p dir="auto">{entry.body}</p>
  const revisions = query.data?.filter(d => d.id === snapshot.id) ?? []
  const live = revisions.find(d => d.version === snapshot.version)
  const latest = revisions.every(d => d.version <= snapshot.version)
  const submit = () => {
    attempt.current = attemptFor(attempt.current, { projectId, id: snapshot.id, version: snapshot.version })
    accept.mutate({ id: snapshot.id, version: snapshot.version, commandId: attempt.current.commandId })
  }
  const stale = accept.error instanceof ApiError && accept.error.problem.kind === 'daemon' &&
    accept.error.problem.code === 'DECISION_STALE'
  const textClass = 'text-xs text-muted-foreground'
  return <article data-decision-proposal={`${snapshot.id}:${snapshot.version}`}
    className={panelVariants({ className: 'space-y-3' })}>
    <h3 className="text-sm font-semibold text-foreground">Decision {snapshot.id} · version {snapshot.version}</h3>
    <p dir="auto" className="text-sm font-medium text-foreground">{snapshot.title}</p>
    <p dir="auto" className="whitespace-pre-wrap text-sm text-foreground">{snapshot.decision}</p>
    <p dir="auto" className={`whitespace-pre-wrap ${textClass}`}>{snapshot.rationale}</p>
    {snapshot.source !== '' && <p dir="auto" className={textClass}>Source: {snapshot.source}</p>}
    {live?.state === 'accepted' && <p role="status" className={textClass}>
      Accepted · <ApprovedBy by={live.approved_by} projectId={projectId} /></p>}
    {live !== undefined && (live.state === 'superseded' || (live.state === 'proposed' && !latest)) &&
      <p role="status" className={textClass}>Superseded</p>}
    {live === undefined && query.error === null && <p role="status" className={textClass}>
      {query.isPending ? 'Loading status…' : 'Status unavailable'}</p>}
    {live?.state === 'proposed' && latest && query.error === null &&
      <Button size="sm" onClick={submit} disabled={accept.isPending}>Accept</Button>}
    {query.error !== null && <ErrorLine error={query.error} />}
    {stale ? <p role="alert" className="text-xs text-destructive-foreground">
      This decision changed. Reload to see its latest version.</p>
      : accept.error !== null && <ErrorLine error={accept.error} />}
  </article>
}
