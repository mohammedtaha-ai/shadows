import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useRef, type ReactNode } from 'react'
import { answerPermission, invalidatePermissions, type WaitingPermission } from '@/api/permissions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

export function PermissionAnswer({ request, children }: { request?: WaitingPermission; children?: ReactNode }) {
  const client = useQueryClient()
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const answer = useMutation({
    retry: false,
    mutationFn: ({ request: pending, optionId }: { request: WaitingPermission; optionId: string }) => {
      const { thread_id: threadId, request_id: requestId } = pending
      attempt.current = attemptFor(attempt.current, { threadId, requestId, optionId })
      return answerPermission(threadId, requestId, optionId, attempt.current.commandId)
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
  const waiting = request && !answer.isSuccess
  return (
    <>
      {waiting && <p className="inline-flex items-center gap-1.5 rounded-full border border-current px-2.5 py-0.5 text-xs font-semibold text-warning">
        Waiting for your approval
      </p>}
      {children}
      {waiting && <div className="flex flex-wrap gap-2">
        {request.options.map(option => (
          <Button key={option.option_id} type="button" variant="outline" disabled={answer.isPending}
            onClick={() => {
              if (locked.current) return
              locked.current = true
              answer.mutate({ request, optionId: option.option_id })
            }}>
            {option.name}
          </Button>
        ))}
      </div>}
      {answer.error && <ErrorLine error={answer.error} />}
    </>
  )
}
