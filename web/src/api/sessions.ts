import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { openSession, type SessionChoices } from './client'
import { writeSaved } from './saved-answer'

export const sessionKeys = {
  choices: (threadId: string) => ['threads', threadId, 'session'] as const,
  commands: (threadId: string) => ['threads', threadId, 'commands'] as const,
}
export const sessionKey = sessionKeys.choices
export const commandsKey = sessionKeys.commands
const NONE: readonly never[] = []
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
export function clearCommands(client: QueryClient, threadId: string) {
  client.setQueryData(sessionKeys.commands(threadId), NONE)
}
export function replaceChoices(client: QueryClient, threadId: string, choices: SessionChoices) {
  const state = client.getQueryState(sessionKeys.choices(threadId))
  // Session opening emits intermediate choices; its final answer is authoritative.
  if (state?.data === undefined || state.fetchStatus === 'fetching') return
  client.setQueryData(sessionKeys.choices(threadId), choices)
}
export function reopenSession(client: QueryClient, threadId: string) {
  clearCommands(client, threadId)
  void client.resetQueries({ queryKey: sessionKeys.choices(threadId) })
}
export function saveSessionAnswer(client: QueryClient, threadId: string, choices: SessionChoices) {
  return writeSaved(client, sessionKeys.choices(threadId), choices)
}
