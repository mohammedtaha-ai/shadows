import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { refreshSubscriber } from './refresh'
import { subscribeProjectStream } from './thread-registry'

/** Visible project views share delivery; replay reconciliation belongs to stream/. */
export function useProjectEvents(projectId: string | undefined): void {
  const client = useQueryClient()
  useEffect(() => {
    if (projectId === undefined) return
    return subscribeProjectStream(client, projectId, refreshSubscriber(client, { projectId }))
  }, [client, projectId])
}
