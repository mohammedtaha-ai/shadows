import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { client, unwrap } from './transport'
import type { components } from './schema'

export type WaitingPermission = components['schemas']['WaitingPermission']
export type PermissionAnswer = components['schemas']['PermissionAnswer']
export type PermissionOption = components['schemas']['PermissionOption']
export const permissionKeys = { waiting: () => ['aylul-permissions'] }
export const waitingPermissionsQuery = queryOptions({
  queryKey: permissionKeys.waiting(), retry: false,
  queryFn: () => unwrap(client.GET('/api/aylul/permissions')),
})
export function answerPermission(threadId: string, requestId: string, optionId: string, commandId: string): Promise<PermissionAnswer> {
  return unwrap(client.POST('/api/threads/{id}/session/permissions/{request_id}', {
    params: { path: { id: threadId, request_id: requestId } },
    body: { command_id: commandId, option_id: optionId },
  }))
}
export function invalidatePermissions(client: QueryClient) {
  return client.invalidateQueries({ queryKey: permissionKeys.waiting() })
}
