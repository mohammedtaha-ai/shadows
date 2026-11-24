import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { holdProjectRefresh } from './refresh'
import { subscribeProjectStream } from './thread-registry'
import type { Connection } from './thread-stream'

/** Visible project views share delivery; replay reconciliation belongs to stream/. */
export function useProjectEvents(projectId: string | undefined): void {
  const client = useQueryClient()
  useEffect(() => {
    if (projectId === undefined) return
    return holdProjectRefresh(client, projectId)
  }, [client, projectId])
}

/** The open project's stream connection, from the channel useProjectEvents holds;
 * undefined with no project. */
export function useProjectConnection(projectId: string | undefined): Connection | undefined {
  const client = useQueryClient()
  const [connection, setConnection] = useState<Connection>('connecting')
  useEffect(() => {
    if (projectId === undefined) return
    return subscribeProjectStream(client, projectId, { onState: state => setConnection(state.connection) })
  }, [client, projectId])
  return projectId === undefined ? undefined : connection
}
