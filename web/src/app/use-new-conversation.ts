// One job: starting a new conversation (a planning thread) in a project and
// opening it.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useRef } from 'react'
import { createThread } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { threadsQuery } from '@/api/queries'

/** `start()` creates the next conversation; pressing it again after a failure
 * is a retry of the same command. */
export function useNewConversation(projectId: string) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const threads = useQuery(threadsQuery(projectId))
  const attempt = useRef<Attempt | null>(null)

  const create = useMutation({
    mutationFn: (request: { title: string; command_id: string }) =>
      createThread(projectId, request),
    onSuccess: (thread) => {
      attempt.current = null
      void queryClient.invalidateQueries({ queryKey: threadsQuery(projectId).queryKey })
      void navigate({
        to: '/projects/$projectId/threads/$threadId',
        params: { projectId, threadId: thread.id },
      })
    },
  })

  const start = () => {
    if (create.isPending || threads.data === undefined) return
    const request = { title: `Conversation ${threads.data.length + 1}` }
    attempt.current = attemptFor(attempt.current, request)
    create.mutate({ ...request, command_id: attempt.current.commandId })
  }

  /** `start()` does nothing until the project's conversations are read: the
   * title counts them. */
  const ready = threads.data !== undefined
  return { start, ready, pending: create.isPending, error: create.error }
}
