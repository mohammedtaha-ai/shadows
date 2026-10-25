// One job: the `/` list each conversation's harness last sent (spec §21.3),
// kept per thread in the query cache (`api/sessions.ts`). Only the stream
// fills it; nothing fetches it, so it is never stale and never refetched.

import { useQuery } from '@tanstack/react-query'
import type { SlashCommand } from '@/stream/frames'
import { commandsQuery } from '@/api/sessions'

const NONE: readonly SlashCommand[] = []
export function useCommands(threadId: string | null): readonly SlashCommand[] {
  const { data } = useQuery(commandsQuery<SlashCommand>(threadId ?? ''))
  return threadId === null ? NONE : (data ?? NONE)
}
