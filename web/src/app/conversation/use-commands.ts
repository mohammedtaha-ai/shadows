// Only the stream supplies commands, so keep its list per thread without fetching it.

import { useQuery } from '@tanstack/react-query'
import type { SlashCommand } from '@/stream'
import { commandsQuery } from '@/api/sessions'

const NONE: readonly SlashCommand[] = []
export function useCommands(threadId: string | null): readonly SlashCommand[] {
  const { data } = useQuery(commandsQuery<SlashCommand>(threadId ?? ''))
  return threadId === null ? NONE : (data ?? NONE)
}
