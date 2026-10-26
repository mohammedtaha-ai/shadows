import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { holdThreadRefresh } from './refresh'

/** Writer events remain available on screens with no open conversation. */
export function usePlanEvents(threadId: string | undefined, projectId?: string): void {
  const client = useQueryClient()
  useEffect(() => {
    if (threadId === undefined) return
    return holdThreadRefresh(client, threadId, { projectId, plansOnly: true })
  }, [client, threadId, projectId])
}
