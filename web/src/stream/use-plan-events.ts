import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { refreshSubscriber } from './refresh'
import { subscribeThreadStream } from './thread-registry'

/** Writer events remain available on screens with no open conversation. */
export function usePlanEvents(threadId: string | undefined, projectId?: string): void {
  const client = useQueryClient()
  useEffect(() => {
    if (threadId === undefined) return
    return subscribeThreadStream(client, threadId, refreshSubscriber(client, { threadId, projectId, plansOnly: true }))
  }, [client, threadId, projectId])
}
