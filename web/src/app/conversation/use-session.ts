// Session opening is idempotent, so a query can cache its answer and retry safely.

import { useQuery } from '@tanstack/react-query'
import type { SessionChoices } from '@/api/sessions'
import { sessionQuery } from '@/api/sessions'

export type SessionView =
  | { state: 'connecting' }
  | { state: 'ready'; choices: SessionChoices }
  | { state: 'failed'; error: Error; retry: () => void }

export function useSession(threadId: string): SessionView {
  const session = useQuery(sessionQuery(threadId))
  if (session.data !== undefined) return { state: 'ready', choices: session.data }
  if (session.isError && !session.isFetching) {
    return { state: 'failed', error: session.error, retry: () => void session.refetch() }
  }
  return { state: 'connecting' }
}
