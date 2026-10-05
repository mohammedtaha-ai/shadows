// One job: the `/` list each conversation's harness last sent (spec §21.3),
// kept per thread in the query cache. Only the stream fills it; nothing
// fetches it, so it is never stale and never refetched.

import { type QueryClient, useQuery } from '@tanstack/react-query'
import type { SlashCommand } from '@/stream/frames'

const NONE: readonly SlashCommand[] = []

export function commandsKey(threadId: string) {
  return ['threads', threadId, 'commands'] as const
}

export function replaceCommands(qc: QueryClient, threadId: string, list: readonly SlashCommand[]) {
  qc.setQueryData(commandsKey(threadId), list)
}

/** The thread's harness changed (§12.6): the old harness's list is not the new one's. */
export function clearCommands(qc: QueryClient, threadId: string) {
  qc.setQueryData(commandsKey(threadId), NONE)
}

export function useCommands(threadId: string | null): readonly SlashCommand[] {
  const { data } = useQuery({
    queryKey: commandsKey(threadId ?? ''),
    queryFn: () => NONE,
    enabled: false,
    staleTime: Infinity,
  })
  return threadId === null ? NONE : (data ?? NONE)
}
