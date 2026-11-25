import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, unwrap } from './transport'

export type ExternalConnection = components['schemas']['ExternalConnection']
export type IssuedConnection = components['schemas']['IssuedGrantBody']

export const connectionsQuery = queryOptions({
  queryKey: ['external-connections'],
  queryFn: () => unwrap(client.GET('/api/external-connections')),
})

export function issueConnection(projectIds: string[], commandId: string): Promise<IssuedConnection> {
  return unwrap(client.POST('/api/external-connections', {
    body: { command_id: commandId, project_ids: projectIds },
  }))
}

export function revokeConnection(id: string, commandId: string): Promise<ExternalConnection> {
  return unwrap(client.DELETE('/api/external-connections/{id}', {
    params: { path: { id }, query: { command_id: commandId } },
  }))
}

export function saveRevokedConnection(client: QueryClient, revoked: ExternalConnection) {
  return writeSaved<ExternalConnection[]>(client, connectionsQuery.queryKey, connections =>
    connections?.map(connection => connection.id === revoked.id ? revoked : connection))
}
