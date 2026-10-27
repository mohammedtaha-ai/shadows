// A thread's harness session in the cache: its choices and its slash commands.
import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, unwrap } from './transport'

export const sessionKeys = {
  choices: (threadId: string) => ['threads', threadId, 'session'] as const,
  commands: (threadId: string) => ['threads', threadId, 'commands'] as const,
}
const NONE: readonly never[] = []
// Never on a timer or on focus: an open session answers what it holds, and the
// stream says when that changes. A failure shows a Retry at once, never silent retries.
export function sessionQuery(threadId: string) {
  return queryOptions({ queryKey: sessionKeys.choices(threadId), queryFn: () => openSession(threadId),
    staleTime: Infinity, refetchOnWindowFocus: false, retry: false })
}
// The stream owns command frame types; the cache preserves its supplied list.
export function commandsQuery<T>(threadId: string) {
  return queryOptions({ queryKey: sessionKeys.commands(threadId), queryFn: (): readonly T[] => NONE,
    enabled: false, staleTime: Infinity })
}
export function replaceCommands<T>(client: QueryClient, threadId: string, list: readonly T[]) {
  client.setQueryData(sessionKeys.commands(threadId), list)
}
// A changed harness: the old harness's list is not the new one's.
export function clearCommands(client: QueryClient, threadId: string) {
  client.setQueryData(sessionKeys.commands(threadId), NONE)
}
export function replaceChoices(client: QueryClient, threadId: string, choices: SessionChoices) {
  const state = client.getQueryState(sessionKeys.choices(threadId))
  // Session opening emits intermediate choices; its final answer is authoritative.
  if (state?.data === undefined || state.fetchStatus === 'fetching') return
  client.setQueryData(sessionKeys.choices(threadId), choices)
}
// A changed harness: forget the old session's choices and open the new one's.
export function reopenSession(client: QueryClient, threadId: string) {
  clearCommands(client, threadId)
  void client.resetQueries({ queryKey: sessionKeys.choices(threadId) })
}
export function saveSessionAnswer(client: QueryClient, threadId: string, choices: SessionChoices) {
  return writeSaved(client, sessionKeys.choices(threadId), choices)
}

export type Choice = components['schemas']['Choice']

export type SessionChoices = components['schemas']['SessionChoices']

export type AccountLimits = components['schemas']['AccountLimits']

export type TurnSettings = components['schemas']['TurnSettings']

/** Opens the thread's harness session if it is not open; answers what it
 * offers now. Idempotent: an open session answers what it holds. */
export function openSession(threadId: string): Promise<SessionChoices> {
  return unwrap(client.POST('/api/threads/{id}/session', { params: { path: { id: threadId } } }))
}

/** Sets the thread's session to `model` at once, opening it if needed;
 * answers what it offers now, the new model's efforts included.
 * The change writes nothing durable, so it carries no command id; an opening
 * it causes issues the thread's MCP grant, as any opening does. */
export function changeModel(threadId: string, model: string): Promise<SessionChoices> {
  return unwrap(
    client.PUT('/api/threads/{id}/session/model', {
      params: { path: { id: threadId } },
      body: { model },
    }),
  )
}

/** Sets the thread's session to `effort` at once, opening it if needed;
 * answers what it offers now. Like a model change, it writes
 * nothing durable and carries no command id. */
export function changeEffort(threadId: string, effort: string): Promise<SessionChoices> {
  return unwrap(
    client.PUT('/api/threads/{id}/session/effort', {
      params: { path: { id: threadId } },
      body: { effort },
    }),
  )
}

export function changeMode(threadId: string, commandId: string, mode: string): Promise<SessionChoices> {
  return unwrap(
    client.PUT('/api/threads/{id}/session/mode', {
      params: { path: { id: threadId } },
      body: { command_id: commandId, mode },
    }),
  )
}
