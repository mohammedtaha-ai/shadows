import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { ApiError, client, unwrap } from './transport'

type Schemas = components['schemas']
export type ThreadPlanningAnchor = Schemas['ThreadPlanningAnchor']
export type ContextRecord = Schemas['ContextResponse']
type AnchorResponse = Schemas['AnchorResponse']

const planningKeys = {
  anchor: (threadId: string) => ['planning-context', 'anchor', threadId] as const,
  carried: (operationId: string) => ['planning-context', 'carried', operationId] as const,
}

export function anchorQuery(threadId: string) {
  return queryOptions({
    queryKey: planningKeys.anchor(threadId),
    queryFn: ({ signal }) => unwrap(client.GET('/api/threads/{id}/anchor', {
      params: { path: { id: threadId } }, signal,
    })),
  })
}

export function setAnchor(threadId: string, commandId: string, anchor: ThreadPlanningAnchor | null) {
  return unwrap(client.PUT('/api/threads/{id}/anchor', {
    params: { path: { id: threadId } }, body: { command_id: commandId, anchor },
  }))
}

export function saveAnchorAnswer(queryClient: QueryClient, threadId: string, answer: AnchorResponse) {
  return writeSaved(queryClient, planningKeys.anchor(threadId), answer)
}

export function carriedContextQuery(operationId: string) {
  return queryOptions({
    queryKey: planningKeys.carried(operationId),
    queryFn: async ({ signal }): Promise<ContextRecord | null> => {
      try {
        return await unwrap(client.GET('/api/operations/{id}/context', {
          params: { path: { id: operationId } }, signal,
        }))
      } catch (error) {
        if (error instanceof ApiError && error.problem.kind === 'daemon' &&
            error.problem.status === 404 && error.problem.code === 'CONTEXT_NOT_CARRIED') return null
        throw error
      }
    },
  })
}

export function resendContext(threadId: string, commandId: string) {
  return unwrap(client.POST('/api/threads/{id}/context/resend', {
    params: { path: { id: threadId } }, body: { command_id: commandId },
  }))
}
