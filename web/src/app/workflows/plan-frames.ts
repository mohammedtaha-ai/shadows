import { type QueryClient, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { workflowsKey } from '@/api/queries'
import { subscribeThreadStream } from '@/stream'

/** The durable events after which a plan, its neighbours or its list read differently. */
const PLAN_KINDS: ReadonlySet<string> = new Set([
  'WorkflowEdited',
  'WorkflowFrozen',
  'WorkflowDraftStarted',
])

/** Whether a durable event of `kind` changes how a plan, its neighbours or
 * its list read. */
export function isPlanEvent(kind: string): boolean {
  return PLAN_KINDS.has(kind)
}

/** Reads every plan on screen again. */
export function refetchPlans(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: workflowsKey })
}

/** Keeps every plan query fresh from `threadId`'s stream while mounted;
 * `undefined` (the plan is not read yet) watches nothing, and neither does a
 * a thread id is not available yet. */
export function usePlanFrames(threadId: string | undefined): void {
  const queryClient = useQueryClient()
  useEffect(() => {
    if (threadId === undefined) return
    return watch(queryClient, threadId)
  }, [queryClient, threadId])
}

function watch(queryClient: QueryClient, threadId: string): () => void {
  return subscribeThreadStream(queryClient, threadId, {
    onCaughtUp: () => refetchPlans(queryClient),
    onDurable: (event, live) => {
      if (live && isPlanEvent(event.kind)) refetchPlans(queryClient)
    },
  })
}
