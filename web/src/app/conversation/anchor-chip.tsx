import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { outcomeQuery, partQuery } from '@/api/design'
import { ApiError } from '@/api/error'
import { anchorQuery, resendContext, saveAnchorAnswer, setAnchor, type ThreadPlanningAnchor } from '@/api/planning-context'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { AnchorPicker } from './anchor-picker'

export function AnchorChip({ projectId, threadId }: { projectId: string; threadId: string }) {
  const queryClient = useQueryClient()
  const selected = useQuery(anchorQuery(threadId))
  const anchor = selected.data?.anchor
  const kind = anchor?.kind
  const part = useQuery({ ...partQuery(projectId, anchor?.id ?? ''), enabled: kind === 'part' })
  const outcome = useQuery({ ...outcomeQuery(projectId, anchor?.id ?? ''), enabled: kind === 'outcome' })
  const name = kind === 'part' ? part.data?.part.content.title : outcome.data?.outcome.content.title
  const detailError = kind === 'part' ? part.error : outcome.error
  const removed = detailError instanceof ApiError && 'status' in detailError.problem && detailError.problem.status === 404

  const [picking, setPicking] = useState(false)
  const changeAttempt = useRef<Attempt | null>(null)
  const resendAttempt = useRef<Attempt | null>(null)
  const change = useMutation({
    mutationFn: (value: ThreadPlanningAnchor | null) => {
      changeAttempt.current = attemptFor(changeAttempt.current, { threadId, anchor: value })
      return setAnchor(threadId, changeAttempt.current.commandId, value)
    },
    onSuccess: async answer => {
      changeAttempt.current = null
      await saveAnchorAnswer(queryClient, threadId, answer)
      setPicking(false)
    },
  })
  const resend = useMutation({
    mutationFn: () => {
      resendAttempt.current = attemptFor(resendAttempt.current, { threadId })
      return resendContext(threadId, resendAttempt.current.commandId)
    },
    onSuccess: () => { resendAttempt.current = null },
  })
  const busy = change.isPending || resend.isPending
  const error = selected.error ?? (removed ? null : detailError) ?? resend.error ?? (picking ? null : change.error)
  const openPicker = () => { change.reset(); setPicking(true) }

  return <div className="space-y-1">
    <div className="flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
      {anchor === null && <span>Not anchored</span>}
      {anchor != null && <span>
        {removed ? `Anchored ${kind} was removed` : name === undefined ? 'Planning anchor' : <>Anchored at <bdi>{name}</bdi></>}
      </span>}
      {anchor === null && <Button variant="ghost" size="xs" disabled={busy} onClick={openPicker}>Anchor…</Button>}
      {anchor != null && <>
        {!removed && <Button variant="ghost" size="xs" disabled={busy} onClick={openPicker}>Change</Button>}
        <Button variant="ghost" size="xs" disabled={busy} onClick={() => change.mutate(null)}>Clear</Button>
      </>}
      <Button variant="ghost" size="xs" disabled={busy} onClick={() => resend.mutate()}>Send context again</Button>
    </div>
    {resend.isSuccess && <p role="status" className="text-xs text-muted-foreground">Your next normal message carries it again.</p>}
    {error && <ErrorLine error={error} />}
    {picking && <AnchorPicker projectId={projectId} pending={change.isPending} error={change.error}
      onChoose={value => change.mutate(value)} onClose={() => setPicking(false)} />}
  </div>
}
