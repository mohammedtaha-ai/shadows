import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef } from 'react'
import { answerPermission, invalidatePermissions, waitingPermissionsQuery } from '@/api/permissions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { ApiError } from '@/api/error'
import type { ThreadEntry } from '@/api/threads'
import { useProjectEvents } from '@/stream'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

export function PermissionCard({ entry, projectId }: { entry: ThreadEntry; projectId: string }) {
  const requestId = typeof entry.card?.request_id === 'string' ? entry.card.request_id : null
  const client = useQueryClient()
  useProjectEvents(projectId)
  const waiting = useQuery({ ...waitingPermissionsQuery, enabled: requestId !== null })
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const request = waiting.error === null ? waiting.data?.find(value => value.thread_id === entry.thread_id && value.request_id === requestId) : undefined
  const answer = useMutation({ retry: false,
    mutationFn: (optionId: string) => {
      attempt.current = attemptFor(attempt.current, { threadId: entry.thread_id, requestId, optionId })
      return answerPermission(entry.thread_id, requestId!, optionId, attempt.current.commandId)
    },
    onSuccess: async () => { attempt.current = null; await invalidatePermissions(client) },
    onError: async error => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' &&
        (error.problem.code === 'PERMISSION_NOT_FOUND' || error.problem.code === 'PERMISSION_SETTLED')) {
        await invalidatePermissions(client)
      }
    },
    onSettled: () => { locked.current = false },
  })
  return <section aria-label="Permission request" className="surface-glow space-y-3 rounded-lg p-4">
    {request && !answer.isSuccess && <p className="inline-flex items-center gap-1.5 rounded-full border border-current px-2.5 py-0.5 text-xs font-semibold text-warning">Waiting for your approval</p>}
    <p dir="auto" className="text-sm font-medium whitespace-pre-wrap">{entry.body}</p>
    {waiting.isPending && requestId !== null && <p role="status" className="text-xs text-muted-foreground">Checking request…</p>}
    {request && !answer.isSuccess && <div className="flex flex-wrap gap-2">{request.options.map(option => <Button
      key={option.option_id} type="button" variant="outline" disabled={answer.isPending}
      onClick={() => { if (locked.current) return; locked.current = true; answer.mutate(option.option_id) }}
    >{option.name}</Button>)}</div>}
    {!request && waiting.isSuccess && <p className="text-xs text-muted-foreground">This request is no longer waiting.</p>}
    {waiting.error && <ErrorLine error={waiting.error} />}
    {answer.error && <ErrorLine error={answer.error} />}
  </section>
}
