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

export function issueConnection(
  name: string, projectIds: string[], commandId: string,
): Promise<IssuedConnection> {
  return unwrap(client.POST('/api/external-connections', {
    body: { command_id: commandId, name, project_ids: projectIds },
  }))
}

export function updateConnectionProjects(
  id: string, expectedRevision: number, projectIds: string[], commandId: string,
): Promise<ExternalConnection> {
  return unwrap(client.PATCH('/api/external-connections/{id}', {
    params: { path: { id } },
    body: { command_id: commandId, expected_revision: expectedRevision, project_ids: projectIds },
  }))
}

export function revokeConnection(id: string, commandId: string): Promise<ExternalConnection> {
  return unwrap(client.DELETE('/api/external-connections/{id}', {
    params: { path: { id }, query: { command_id: commandId } },
  }))
}

export function saveConnectionAnswer(client: QueryClient, saved: ExternalConnection) {
  return writeSaved<ExternalConnection[]>(client, connectionsQuery.queryKey, connections =>
    connections?.map(connection => connection.id === saved.id ? saved : connection))
}
