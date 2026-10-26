// Shown on hover and on keyboard focus, so a reader is not distracted by a
// row of buttons under every message. A refused fork's reason is not hidden
// with them.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { GitBranch } from 'lucide-react'
import { useRef } from 'react'
import { agentThreadsQuery } from '@/api/aylul-profiles'
import { forkThread } from '@/api/threads'
import { type Attempt, attemptFor } from '@/api/command-id'
import { threadsQuery } from '@/api/threads'
import { Button } from '@/components/ui/button'
import { CopyButton } from '../copy-button'
import { ErrorLine } from '../error-line'

export interface ForkPoint {
  projectId: string
  threadId: string
  entryId: string
}

export function MessageActions({
  text,
  forkPoint,
}: {
  text: string
  /** Present on the one message a fork may start from. The daemon decides
   * whether it is a valid fork point, and says why when it is not. */
  forkPoint?: ForkPoint
}) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const pending = useRef<Attempt | null>(null)
  const fork = useMutation({
    mutationFn: ({ at, commandId }: { at: ForkPoint; commandId: string }) =>
      forkThread(at.threadId, commandId, at.entryId),
    onSuccess: (created, { at }) => {
      pending.current = null
      void queryClient.invalidateQueries({ queryKey: threadsQuery(at.projectId).queryKey })
      // An AYLUL fork keeps its agent, so it also shows under that agent.
      if (created.profile != null) {
        void queryClient.invalidateQueries({ queryKey: agentThreadsQuery(created.profile).queryKey })
      }
      void navigate({
        to: '/projects/$projectId/threads/$threadId',
        params: { projectId: created.project_id, threadId: created.id },
      })
    },
  })

  const startFork = (at: ForkPoint) => {
    pending.current = attemptFor(pending.current, { at_entry_id: at.entryId })
    fork.mutate({ at, commandId: pending.current.commandId })
  }

  return (
    <div className="space-y-1">
      <div className="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100 motion-reduce:transition-none">
        <CopyButton text={text} size="icon-xs" />
        {forkPoint !== undefined && (
          <Button
            variant="ghost"
            size="icon-xs"
            disabled={fork.isPending}
            onClick={() => startFork(forkPoint)}
            aria-label="Fork"
            title="Fork into a new conversation from here"
          >
            <GitBranch />
          </Button>
        )}
      </div>
      {forkPoint !== undefined && fork.error !== null && <ErrorLine error={fork.error} />}
    </div>
  )
}
