import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { holdProjectRefresh } from './refresh'

/** Visible project views share delivery; replay reconciliation belongs to stream/. */
export function useProjectEvents(projectId: string | undefined): void {
  const client = useQueryClient()
  useEffect(() => {
    if (projectId === undefined) return
    return holdProjectRefresh(client, projectId)
  }, [client, projectId])
}
