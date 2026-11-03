import { queryOptions } from '@tanstack/react-query'
import type { components } from './schema'
import { client, unwrap } from './transport'

export type Group = components['schemas']['Group']
export type Message = components['schemas']['Message']
export type Speaker = components['schemas']['Speaker']

export const groupKeys = {
  list: ['aylul-groups'] as const,
  detail: (id: string) => ['aylul-group', id, 'detail'] as const,
  messages: (id: string) => ['aylul-group', id, 'messages'] as const,
}

export const groupsQuery = queryOptions({
  queryKey: groupKeys.list,
  queryFn: () => unwrap(client.GET('/api/groups')),
})

export function groupQuery(id: string) {
  return queryOptions({
    queryKey: groupKeys.detail(id),
    queryFn: () => unwrap(client.GET('/api/groups/{id}', { params: { path: { id } } })),
  })
}

export function groupMessagesQuery(id: string) {
  return queryOptions({ queryKey: groupKeys.messages(id), queryFn: () => listGroupMessages(id) })
}

export function listGroupMessages(id: string, after?: number) {
  return unwrap(client.GET('/api/groups/{id}/messages', { params: { path: { id }, query: { after } } }))
}

export function createGroup(body: components['schemas']['CreateGroup']) {
  return unwrap(client.POST('/api/groups', { body }))
}

export function sendToGroup(id: string, text: string, commandId: string) {
  return unwrap(client.POST('/api/groups/{id}/messages', {
    params: { path: { id } }, body: { text, command_id: commandId },
  }))
}
